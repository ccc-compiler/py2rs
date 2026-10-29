//! Python AST (ruff) → typed IR. Resolves names, checks the subset and infers
//! the types of locals (annotations on functions, unification inside them).

use std::collections::{HashMap, HashSet};

use ruff_python_ast::{self as ast, Expr as AExpr, Stmt as AStmt};
use ruff_source_file::LineIndex;
use ruff_text_size::Ranged;

use crate::diag::{err, Error, Res};
use crate::ir::*;

mod dunder;
mod generic;
mod matching;
mod nested;
mod re;
mod stdlib;

/// Where crate stubs (`regex.pyi` for `from rust.regex import ...`) are looked up.
pub fn default_stub_dirs() -> Vec<std::path::PathBuf> {
    let mut v = vec![std::path::PathBuf::from("stubs")];
    if let Ok(p) = std::env::var("PY2RS_STUBS") {
        v.extend(std::env::split_paths(&p));
    }
    v.push(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("stubs"));
    v
}

pub fn lower(src: &str) -> Res<Program> {
    lower_with(src, &default_stub_dirs())
}

pub fn lower_with(src: &str, stub_dirs: &[std::path::PathBuf]) -> Res<Program> {
    lower_modules(src, stub_dirs, &[])
}

/// `src` with the local modules it imports appended (`modules::combine`).
pub fn lower_modules(src: &str, stub_dirs: &[std::path::PathBuf], modules: &[String]) -> Res<Program> {
    let parsed = match ruff_python_parser::parse_module(src) {
        Ok(p) => p,
        Err(e) => {
            let lines = LineIndex::from_source_text(src);
            let line = lines.line_index(e.location.start()).get() as u32;
            return err(line, format!("syntax error: {}", e.error));
        }
    };
    let module = parsed.into_syntax();
    let mut f = Front {
        lines: LineIndex::from_source_text(src),
        src: src.to_string(),
        p: Program::default(),
        funcs: HashMap::new(),
        classes: HashMap::new(),
        consts: HashMap::new(),
        module_vars: HashSet::new(),
        imports: HashSet::new(),
        from_names: HashMap::new(),
        loop_keys: HashMap::new(),
        class_attrs: HashMap::new(),
        enclosing: vec![],
        exc_vars: HashMap::new(),
        typevars: HashSet::new(),
        local_modules: HashSet::new(),
        protocols: HashMap::new(),
        templates: HashMap::new(),
        instances: HashMap::new(),
        pending: vec![],
        type_env: HashMap::new(),
        local_funcs: vec![],
        tvars: Vec::new(),
        next_id: 0,
        cur: 0,
        scopes: Vec::new(),
        active: HashSet::new(),
        loops: 0,
        narrow: Vec::new(),
        handlers: Vec::new(),
        stub_dirs: stub_dirs.to_vec(),
        in_stub: None,
    };
    f.local_modules = modules.iter().cloned().collect();
    f.module(&module.body)?;
    f.finish()?;
    Ok(f.p)
}

struct Front {
    lines: LineIndex,
    /// the source text (`match` patterns are rewritten as Python)
    src: String,
    p: Program,
    funcs: HashMap<String, FuncId>,
    classes: HashMap<String, ClassId>,
    consts: HashMap<String, usize>,
    module_vars: HashSet<String>,
    imports: HashSet<String>,
    /// `from M import x`: x → `M.x`
    from_names: HashMap<String, String>,
    /// loop variable → the dict whose keys it runs over
    loop_keys: HashMap<VarId, VarId>,
    /// class attributes (constants): (class, name) → `p.consts` index
    class_attrs: HashMap<(ClassId, String), usize>,
    /// variable names of the functions being lowered (for nested functions' captures)
    enclosing: Vec<HashSet<String>>,
    /// `except C as e` where C has fields: e → C
    exc_vars: HashMap<VarId, ClassId>,
    /// `T = TypeVar("T")`
    typevars: HashSet<String>,
    /// local modules of a multi-file program: `m.x` is `x`
    local_modules: HashSet<String>,
    /// `class P(Protocol)`: its method names
    protocols: HashMap<String, Vec<String>>,
    /// generic functions (instantiated per call) and their instances by type key
    templates: HashMap<String, &'static ast::StmtFunctionDef>,
    instances: HashMap<String, FuncId>,
    pending: Vec<generic::Pending>,
    /// type variables (and union parameters, `U:<source>`) of the instance being lowered
    type_env: HashMap<String, Ty>,
    /// nested functions defined in the functions being lowered: (name, the `funcs` entry it hides)
    local_funcs: Vec<Vec<(String, Option<FuncId>)>>,
    tvars: Vec<Option<Ty>>,
    next_id: u32,
    cur: FuncId,
    scopes: Vec<HashMap<String, VarId>>,
    /// loop / comprehension variables currently bound
    active: HashSet<VarId>,
    loops: u32,
    /// narrowed Optional places: (root var, fields) → variable holding the value
    narrow: Vec<((VarId, Vec<String>), VarId)>,
    /// the exception variables of the enclosing except clauses
    handlers: Vec<VarId>,
    stub_dirs: Vec<std::path::PathBuf>,
    /// lowering a crate stub: the crate name
    in_stub: Option<String>,
}

/// `usize`, `u32`, ... in stubs: Rust number types of extern parameters / results.
const WIDTHS: &[&str] = &["usize", "isize", "u64", "u32", "u16", "u8", "i32", "i16", "i8", "f32"];

fn width_of(a: &AExpr) -> Option<String> {
    match a {
        AExpr::Name(n) if WIDTHS.contains(&n.id.as_str()) => Some(n.id.to_string()),
        _ => None,
    }
}

/// `x` or `x.a.b` as (root variable, field names).
fn place_key(e: &Expr) -> Option<(VarId, Vec<String>)> {
    match &e.kind {
        ExprKind::Var(v) => Some((*v, vec![])),
        ExprKind::Field(b, f) => {
            let (r, mut p) = place_key(b)?;
            p.push(f.clone());
            Some((r, p))
        }
        _ => None,
    }
}

pub const EXCEPTIONS: &[&str] = &[
    "Exception", "ValueError", "TypeError", "IndexError", "KeyError", "RuntimeError",
    "ZeroDivisionError", "AssertionError", "NotImplementedError", "ArithmeticError",
    "OverflowError", "LookupError", "StopIteration", "AttributeError", "EOFError", "BaseException", "NameError",
    "OSError", "FileNotFoundError", "PermissionError", "IsADirectoryError",
];

fn snake_name(n: &str) -> String {
    let mut o = String::new();
    for (i, c) in n.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                o.push('_');
            }
            o.extend(c.to_lowercase());
        } else {
            o.push(c);
        }
    }
    o
}

/// Names `m` of `super().m(...)` calls.
fn super_calls(body: &[AStmt], out: &mut Vec<String>) {
    use ruff_python_ast::visitor::{self, Visitor};
    struct V<'b>(&'b mut Vec<String>);
    impl<'x> Visitor<'x> for V<'_> {
        fn visit_expr(&mut self, e: &'x AExpr) {
            if let AExpr::Attribute(a) = e {
                if let AExpr::Call(c) = &*a.value {
                    if matches!(&*c.func, AExpr::Name(n) if n.id.as_str() == "super") && !self.0.contains(&a.attr.id.to_string()) {
                        self.0.push(a.attr.id.to_string());
                    }
                }
            }
            visitor::walk_expr(self, e);
        }
    }
    let mut v = V(out);
    v.visit_body(body);
}

fn is_docstring(s: &AStmt) -> bool {
    matches!(s, AStmt::Expr(e) if matches!(*e.value, AExpr::StringLiteral(_)))
}

fn is_main_guard(test: &AExpr) -> bool {
    if let AExpr::Compare(c) = test {
        if let ([AExpr::Name(n), AExpr::StringLiteral(s)], [ast::CmpOp::Eq]) = (&*c.operands, &*c.ops)
        {
            return n.id.as_str() == "__name__" && s.value.to_str() == "__main__";
        }
    }
    false
}

/// Names assigned anywhere in `body` (not descending into defs).
fn assigned_names(body: &[AStmt], out: &mut HashMap<String, usize>) {
    fn target(t: &AExpr, out: &mut HashMap<String, usize>) {
        match t {
            AExpr::Name(n) => *out.entry(n.id.to_string()).or_default() += 1,
            AExpr::Tuple(t) => t.elts.iter().for_each(|e| target(e, out)),
            AExpr::List(t) => t.elts.iter().for_each(|e| target(e, out)),
            _ => {}
        }
    }
    for s in body {
        match s {
            AStmt::Assign(a) => a.targets.iter().for_each(|t| target(t, out)),
            AStmt::AnnAssign(a) if a.value.is_some() => target(&a.target, out),
            AStmt::AugAssign(a) => target(&a.target, out),
            AStmt::For(f) => {
                target(&f.target, out);
                assigned_names(&f.body, out);
                assigned_names(&f.orelse, out);
            }
            AStmt::While(w) => {
                assigned_names(&w.body, out);
                assigned_names(&w.orelse, out);
            }
            AStmt::If(i) => {
                assigned_names(&i.body, out);
                for c in &i.elif_else_clauses {
                    assigned_names(&c.body, out);
                }
            }
            AStmt::Try(t) => {
                assigned_names(&t.body, out);
                for h in &t.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = h;
                    assigned_names(&h.body, out);
                }
                assigned_names(&t.orelse, out);
                assigned_names(&t.finalbody, out);
            }
            _ => {}
        }
    }
}

/// Does a function body yield (outside nested functions)?
fn yields(body: &[AStmt]) -> bool {
    use ruff_python_ast::visitor::{walk_expr, walk_stmt, Visitor};
    struct V(bool);
    impl<'a> Visitor<'a> for V {
        fn visit_stmt(&mut self, s: &'a AStmt) {
            if !matches!(s, AStmt::FunctionDef(_) | AStmt::ClassDef(_)) {
                walk_stmt(self, s);
            }
        }
        fn visit_expr(&mut self, e: &'a AExpr) {
            match e {
                AExpr::Yield(_) | AExpr::YieldFrom(_) => self.0 = true,
                AExpr::Lambda(_) => {}
                _ => walk_expr(self, e),
            }
        }
    }
    let mut v = V(false);
    body.iter().for_each(|s| v.visit_stmt(s));
    v.0
}

fn is_const_expr(e: &AExpr, consts: &HashMap<String, usize>) -> bool {
    match e {
        AExpr::NumberLiteral(_) | AExpr::StringLiteral(_) | AExpr::BooleanLiteral(_) | AExpr::NoneLiteral(_) => true,
        AExpr::UnaryOp(u) => is_const_expr(&u.operand, consts),
        AExpr::BinOp(b) => is_const_expr(&b.left, consts) && is_const_expr(&b.right, consts),
        AExpr::Name(n) => consts.contains_key(n.id.as_str()),
        _ => false,
    }
}

impl Front {
    fn line(&self, r: &impl Ranged) -> u32 {
        self.lines.line_index(r.range().start()).get() as u32
    }

    fn id(&mut self) -> u32 {
        self.next_id += 1;
        self.next_id
    }

    fn mk(&mut self, line: u32, ty: Ty, kind: ExprKind) -> Expr {
        Expr { id: self.id(), line, ty, kind }
    }

    fn mk_stmt(&mut self, line: u32, kind: StmtKind) -> Stmt {
        Stmt { id: self.id(), line, kind }
    }

    fn fresh(&mut self) -> Ty {
        self.tvars.push(None);
        Ty::Var(self.tvars.len() - 1)
    }

    /// Resolve type variables (deep).
    fn norm(&self, t: &Ty) -> Ty {
        match t {
            Ty::Var(v) => match &self.tvars[*v] {
                Some(t) => self.norm(t),
                None => t.clone(),
            },
            Ty::List(a) => Ty::list(self.norm(a)),
            Ty::Set(a) => Ty::set(self.norm(a)),
            Ty::Dict(k, v, d) => Ty::Dict(Box::new(self.norm(k)), Box::new(self.norm(v)), *d),
            Ty::Tuple(ts) => Ty::Tuple(ts.iter().map(|t| self.norm(t)).collect()),
            Ty::Optional(a) => Ty::opt(self.norm(a)),
            Ty::Deque(a) => Ty::Deque(Box::new(self.norm(a))),
            Ty::Func(ps, r) => Ty::Func(ps.iter().map(|t| self.norm(t)).collect(), Box::new(self.norm(r))),
            _ => t.clone(),
        }
    }

    fn unify(&mut self, a: &Ty, b: &Ty) -> bool {
        let (a, b) = (self.norm(a), self.norm(b));
        match (&a, &b) {
            (Ty::Var(x), Ty::Var(y)) if x == y => true,
            (Ty::Var(x), _) => {
                self.tvars[*x] = Some(b.clone());
                true
            }
            (_, Ty::Var(y)) => {
                self.tvars[*y] = Some(a.clone());
                true
            }
            (Ty::List(x), Ty::List(y)) | (Ty::Set(x), Ty::Set(y)) | (Ty::Optional(x), Ty::Optional(y)) => self.unify(x, y),
            (Ty::Dict(k1, v1, d1), Ty::Dict(k2, v2, d2)) => d1 == d2 && self.unify(k1, k2) && self.unify(v1, v2),
            (Ty::Tuple(x), Ty::Tuple(y)) => {
                x.len() == y.len() && x.iter().zip(y.iter()).all(|(p, q)| self.unify(p, q))
            }
            (Ty::Deque(x), Ty::Deque(y)) => self.unify(x, y),
            (Ty::Func(p1, r1), Ty::Func(p2, r2)) => {
                p1.len() == p2.len() && p1.iter().zip(p2.iter()).all(|(p, q)| self.unify(p, q)) && self.unify(r1, r2)
            }
            _ => a == b,
        }
    }

    fn tn(&self, t: &Ty) -> String {
        format!("{}", TyName(&self.norm(t), &self.p))
    }

    /// Convert `e` to type `want` (int → float promotion, else unification).
    fn coerce(&mut self, e: Expr, want: &Ty, what: &str) -> Res<Expr> {
        let (have, w) = (self.norm(&e.ty), self.norm(want));
        if have == Ty::Int && w == Ty::Float {
            let line = e.line;
            return Ok(self.mk(line, Ty::Float, ExprKind::ToFloat(Box::new(e))));
        }
        if self.unify(&have, &w) {
            return Ok(e);
        }
        // subclass → base class
        if let (Ty::Class(x) | Ty::Obj(x), Ty::Class(a)) = (&have, &w) {
            if self.p.is_sub(*x, *a) {
                let line = e.line;
                return Ok(self.mk(line, w.clone(), ExprKind::Upcast(Box::new(e))));
            }
        }
        // T → T | None
        if let (Ty::Optional(inner), false) = (&w, matches!(have, Ty::Optional(_) | Ty::Var(_))) {
            let line = e.line;
            let x = self.coerce(e, inner, what)?;
            return Ok(self.mk(line, w.clone(), ExprKind::SomeOf(Box::new(x))));
        }
        err(e.line, format!("{} must be {}, not {}", what, self.tn(&w), self.tn(&have)))
    }

    // ---------------------------------------------------------------- module

    fn module(&mut self, body: &[AStmt]) -> Res<()> {
        let mut stub_imports: Vec<(String, Vec<String>, u32)> = Vec::new();
        let mut main_body: Vec<&AStmt> = Vec::new();
        let mut fdefs: Vec<&ast::StmtFunctionDef> = Vec::new();
        let mut cdefs: Vec<&ast::StmtClassDef> = Vec::new();
        let mut counts = HashMap::new();
        assigned_names(body, &mut counts);
        let mut const_defs = Vec::new();
        for s in body {
            match s {
                AStmt::Import(i) => {
                    for a in &i.names {
                        let n = a.name.id.as_str();
                        if !(stdlib::MODULES.contains(&n) || ["dataclasses", "typing"].contains(&n)) || a.asname.is_some() {
                            return err(self.line(s), format!("module `{}` is not supported", n))
                                .map_err(|e| e.hint(format!("supported: {}, dataclasses, typing", stdlib::MODULES.join(", "))));
                        }
                        self.imports.insert(n.to_string());
                    }
                }
                AStmt::ImportFrom(i) => {
                    let m = i.module.as_ref().map(|m| m.id.as_str()).unwrap_or("");
                    match m {
                        "dataclasses" | "typing" | "__future__" => {
                            for a in &i.names {
                                self.imports.insert(a.name.id.to_string());
                            }
                        }
                        m if stdlib::MODULES.contains(&m) => {
                            for a in &i.names {
                                let local = a.asname.as_ref().unwrap_or(&a.name).id.to_string();
                                self.from_names.insert(local, format!("{}.{}", m, a.name.id));
                            }
                        }
                        m if m.starts_with("rust.") => {
                            stub_imports.push((m[5..].to_string(), i.names.iter().map(|a| a.name.id.to_string()).collect::<Vec<_>>(), self.line(s)));
                        }
                        // ownership annotations
                        "rust" => {
                            for a in &i.names {
                                let n = a.name.id.as_str();
                                if !["Ref", "Mut", "Own", "inline", "shared"].contains(&n) {
                                    return err(self.line(s), format!("`rust.{}` is not available", n));
                                }
                                self.imports.insert(n.to_string());
                            }
                        }
                        _ => return err(self.line(s), format!("module `{}` is not supported", m)),
                    }
                }
                AStmt::FunctionDef(d) => fdefs.push(d),
                AStmt::ClassDef(d) => {
                    if !self.protocol(d)? {
                        cdefs.push(d)
                    }
                }
                // `T = TypeVar("T")`
                AStmt::Assign(a)
                    if matches!(&*a.value, AExpr::Call(c) if matches!(&*c.func, AExpr::Name(n) if n.id.as_str() == "TypeVar")) =>
                {
                    let [AExpr::Name(n)] = &a.targets[..] else { return err(self.line(s), "unsupported TypeVar") };
                    self.typevars.insert(n.id.to_string());
                }
                AStmt::If(i) if is_main_guard(&i.test) && i.elif_else_clauses.is_empty() => {
                    main_body.extend(i.body.iter())
                }
                _ if is_docstring(s) => {}
                AStmt::Assign(a)
                    if a.targets.len() == 1
                        && matches!(&a.targets[0], AExpr::Name(n) if counts.get(n.id.as_str()) == Some(&1))
                        && is_const_expr(&a.value, &self.consts)
                        && !matches!(*a.value, AExpr::NoneLiteral(_)) =>
                {
                    let AExpr::Name(n) = &a.targets[0] else { unreachable!() };
                    self.consts.insert(n.id.to_string(), const_defs.len());
                    const_defs.push((n.id.to_string(), None, &*a.value, self.line(s)));
                }
                AStmt::AnnAssign(a)
                    if a.value.is_some()
                        && matches!(&*a.annotation, AExpr::Name(t) if ["int", "float", "bool", "str"].contains(&t.id.as_str()))
                        && matches!(&*a.target, AExpr::Name(n) if counts.get(n.id.as_str()) == Some(&1))
                        && is_const_expr(a.value.as_ref().unwrap(), &self.consts) =>
                {
                    let AExpr::Name(n) = &*a.target else { unreachable!() };
                    self.consts.insert(n.id.to_string(), const_defs.len());
                    const_defs.push((n.id.to_string(), Some(&*a.annotation), a.value.as_deref().unwrap(), self.line(s)));
                }
                _ => main_body.push(s),
            }
        }
        // `re` runs on the regex crate: a feature of pyrt
        if self.imports.contains("re") || self.from_names.values().any(|q| q.starts_with("re.")) {
            self.p.crates.push("pyrt/re".into());
        }
        for n in counts.keys() {
            if !self.consts.contains_key(n) {
                self.module_vars.insert(n.clone());
            }
        }
        // names
        for d in &cdefs {
            let id = self.p.classes.len();
            if self.classes.insert(d.name.id.to_string(), id).is_some() {
                return err(self.line(*d), format!("class `{}` is defined twice", d.name.id));
            }
            self.p.classes.push(Class {
                name: d.name.id.to_string(),
                line: self.line(*d),
                fields: vec![],
                dataclass: false,
                frozen: false,
                init: None,
                methods: vec![],
                base: None,
                subs: vec![],
                dispatch: vec![],
                exc: None,
                ext: None,
                shared: false,
            });
        }
        for d in &cdefs {
            let cid = self.classes[d.name.id.as_str()];
            let line = self.line(*d);
            let Some(args) = &d.arguments else { continue };
            if !args.keywords.is_empty() || args.args.len() > 1 {
                return err(line, "only single inheritance is supported");
            }
            let Some(b) = args.args.first() else { continue };
            let AExpr::Name(n) = b else { return err(line, "unsupported base class") };
            let b = n.id.as_str();
            if b == "object" {
                continue;
            }
            if EXCEPTIONS.contains(&b) {
                self.p.classes[cid].exc = Some(b.to_string());
                continue;
            }
            let Some(&bc) = self.classes.get(b) else { return err(line, format!("unknown base class `{}`", b)) };
            if bc >= cid {
                return err(line, format!("define `{}` before its subclasses", b));
            }
            self.p.classes[cid].base = Some(bc);
            self.p.classes[cid].exc = self.p.classes[bc].exc.clone();
            self.p.classes[bc].subs.push(cid);
        }
        // `main` holds module-level code; constants are lowered in its context
        self.p.main = self.new_func("main", 0, None);
        self.enter(self.p.main);
        for (name, ann, value, line) in const_defs {
            let want = match ann {
                Some(a) => Some(self.ann(a)?),
                None => None,
            };
            let mut e = self.expr(value, want.as_ref())?;
            if let Some(w) = &want {
                e = self.coerce(e, w, "constant")?;
            }
            if !matches!(self.norm(&e.ty), Ty::Int | Ty::Float | Ty::Bool | Ty::Str) {
                return err(line, "constants must be int, float, bool or str");
            }
            self.p.consts.push(Const { name, value: e });
        }
        for (krate, names, line) in stub_imports {
            self.load_stub(&krate, &names, line)?;
        }
        for d in &fdefs {
            if self.funcs.contains_key(d.name.id.as_str()) || self.classes.contains_key(d.name.id.as_str()) {
                return err(self.line(*d), format!("`{}` is defined twice", d.name.id));
            }
            if self.is_template(d) {
                let leaked: &'static ast::StmtFunctionDef = Box::leak(Box::new((*d).clone()));
                self.templates.insert(d.name.id.to_string(), leaked);
                continue;
            }
            let id = self.signature(d, None)?;
            self.funcs.insert(d.name.id.to_string(), id);
        }
        let mut bodies: Vec<(FuncId, &ast::StmtFunctionDef)> = Vec::new();
        let mut asts: HashMap<ClassId, Vec<(String, &ast::StmtFunctionDef, ClassId)>> = HashMap::new();
        for d in &cdefs {
            self.class_sig(d, &mut bodies, &mut asts)?;
        }
        self.dispatchers()?;
        // classes in order: inherited fields, then __init__ (it defines fields)
        for d in &cdefs {
            let cid = self.classes[d.name.id.as_str()];
            if let Some(b) = self.p.classes[cid].base {
                let mut fs = self.p.classes[b].fields.clone();
                for f in std::mem::take(&mut self.p.classes[cid].fields) {
                    if fs.iter().any(|g| g.name == f.name) {
                        return err(self.line(*d), format!("field `{}` is already defined in a base class", f.name));
                    }
                    fs.push(f);
                }
                self.p.classes[cid].fields = fs;
            }
            if let Some(init) = self.p.classes[cid].init {
                let ast = bodies.iter().find(|(f, _)| *f == init).unwrap().1;
                self.body(init, &ast.body)?;
            }
        }
        for &(fid, d) in &bodies {
            if !self.p.funcs[fid].is_init {
                self.body(fid, &d.body)?;
            }
        }
        for d in &fdefs {
            let Some(&fid) = self.funcs.get(d.name.id.as_str()) else { continue };
            self.body(fid, &d.body)?;
        }
        let main = self.p.main;
        self.enter(main);
        let mut stmts = Vec::new();
        for s in main_body {
            self.stmt(s, &mut stmts)?;
        }
        self.p.funcs[main].body = stmts;
        self.lower_instances()?;
        Ok(())
    }

    /// `from rust.<crate> import A, b`: declarations of `<crate>.pyi`.
    fn load_stub(&mut self, krate: &str, names: &[String], line: u32) -> Res<()> {
        let file = format!("{}.pyi", krate);
        let Some(path) = self.stub_dirs.iter().map(|d| d.join(&file)).find(|p| p.exists()) else {
            return err(line, format!("no stub for crate `{}`", krate))
                .map_err(|e| e.hint(format!("write stubs/{} (see `py2rs stub {}`)", file, krate)));
        };
        let text = std::fs::read_to_string(&path).map_err(|e| Error::new(line, e.to_string()))?;
        let dep = text
            .lines()
            .find_map(|l| l.trim().strip_prefix("# crate:"))
            .map(|l| l.trim().to_string())
            .unwrap_or_else(|| format!("{} = \"*\"", krate));
        if !self.p.crates.contains(&dep) {
            self.p.crates.push(dep);
        }
        let module = ruff_python_parser::parse_module(&text)
            .map_err(|e| Error::new(line, format!("{}: {}", path.display(), e.error)))?
            .into_syntax();
        // line numbers in errors refer to the stub
        let saved_lines = std::mem::replace(&mut self.lines, LineIndex::from_source_text(&text));
        self.in_stub = Some(krate.to_string());
        let r = self.stub_items(krate, &module.body, names);
        self.in_stub = None;
        self.lines = saved_lines;
        r.map_err(|e| Error::new(line, format!("in {}: line {}: {}", path.display(), e.line, e.msg)))
    }

    fn stub_items(&mut self, krate: &str, body: &[AStmt], names: &[String]) -> Res<()> {
        for s in body {
            match s {
                AStmt::ClassDef(d) => {
                    let name = d.name.id.to_string();
                    let mut path = format!("{}::{}", krate, name);
                    for dec in &d.decorator_list {
                        if let AExpr::Call(c) = &dec.expression {
                            if let (AExpr::Name(n), Some(AExpr::StringLiteral(l))) = (&*c.func, c.arguments.args.first()) {
                                if n.id.as_str() == "rust" {
                                    path = l.value.to_str().to_string();
                                }
                            }
                        }
                    }
                    let cid = self.p.classes.len();
                    self.p.classes.push(Class {
                        name: name.clone(),
                        line: self.line(d),
                        fields: vec![],
                        dataclass: false,
                        frozen: false,
                        init: None,
                        methods: vec![],
                        base: None,
                        subs: vec![],
                        dispatch: vec![],
                        exc: None,
                        ext: Some(path),
                        shared: false,
                    });
                    // all classes of the stub are known while it is read
                    self.classes.insert(name.clone(), cid);
                }
                AStmt::FunctionDef(_) | AStmt::ImportFrom(_) | AStmt::Import(_) | AStmt::Pass(_) => {}
                _ if is_docstring(s) => {}
                _ => return err(self.line(s), "stubs contain classes and functions"),
            }
        }
        for s in body {
            match s {
                AStmt::ClassDef(d) => {
                    let cid = self.p.classes.iter().rposition(|c| c.name == d.name.id.as_str()).unwrap();
                    for m in &d.body {
                        let AStmt::FunctionDef(f) = m else { continue };
                        let fid = self.signature(f, Some(cid))?;
                        let name = f.name.id.to_string();
                        if name == "__init__" {
                            self.p.funcs[fid].is_init = true;
                            self.p.classes[cid].init = Some(fid);
                        }
                        self.p.classes[cid].methods.push((name, fid));
                    }
                }
                AStmt::FunctionDef(f) => {
                    let fid = self.signature(f, None)?;
                    let name = f.name.id.to_string();
                    if names.contains(&name) {
                        self.funcs.insert(name, fid);
                    }
                }
                _ => {}
            }
        }
        for n in names {
            if !self.classes.contains_key(n) && !self.funcs.contains_key(n) {
                return err(0, format!("`{}` is not declared in the stub of `{}`", n, krate));
            }
        }
        // only the imported names are visible to the program
        for s in body {
            if let AStmt::ClassDef(d) = s {
                let n = d.name.id.to_string();
                if !names.contains(&n) {
                    self.classes.remove(&n);
                }
            }
        }
        Ok(())
    }

    fn new_func(&mut self, name: &str, line: u32, class: Option<ClassId>) -> FuncId {
        self.p.funcs.push(Func {
            name: name.to_string(),
            line,
            class,
            self_var: None,
            params: vec![],
            ret: Ty::None,
            body: vec![],
            vars: vec![],
            is_init: false,
            is_static: false,
            cache: false,
            property: false,
            cls: None,
            captures: vec![],
            varargs: None,
            is_gen: false,
            shadows: vec![],
            dispatch: vec![],
            def_class: None,
            ext: None,
        });
        self.p.funcs.len() - 1
    }

    fn enter(&mut self, f: FuncId) {
        self.cur = f;
        let mut scope = HashMap::new();
        for (i, v) in self.p.funcs[f].vars.iter().enumerate() {
            if v.kind != VarKind::Bound {
                scope.insert(v.name.clone(), i);
            }
        }
        self.scopes = vec![scope];
        self.active.clear();
        self.loops = 0;
        self.narrow.clear();
        self.handlers.clear();
    }

    fn new_var(&mut self, name: &str, ty: Ty, kind: VarKind) -> VarId {
        let vars = &mut self.p.funcs[self.cur].vars;
        vars.push(Var { name: name.to_string(), ty, kind });
        let id = vars.len() - 1;
        self.scopes.last_mut().unwrap().insert(name.to_string(), id);
        id
    }

    fn var(&self, v: VarId) -> &Var {
        &self.p.funcs[self.cur].vars[v]
    }

    fn lookup(&self, name: &str) -> Option<VarId> {
        self.scopes.iter().rev().find_map(|s| s.get(name).copied())
    }

    fn signature(&mut self, d: &ast::StmtFunctionDef, class: Option<ClassId>) -> Res<FuncId> {
        let line = self.line(d);
        if d.is_async {
            return err(line, "async functions are not supported");
        }
        if d.type_params.is_some() && self.type_env.is_empty() {
            return err(line, "generic functions are not supported");
        }
        let mut is_static = false;
        let mut cache = false;
        let mut property = false;
        let mut classmethod = false;
        let mut ext = self.in_stub.as_ref().map(|_| Extern::default());
        for dec in &d.decorator_list {
            if let Some(c) = self.cache_decorator(&dec.expression, class.is_some(), self.line(dec))? {
                cache = c;
                continue;
            }
            match &dec.expression {
                AExpr::Name(n) if n.id.as_str() == "staticmethod" && class.is_some() => is_static = true,
                AExpr::Name(n) if n.id.as_str() == "property" && class.is_some() => property = true,
                AExpr::Name(n) if n.id.as_str() == "classmethod" && class.is_some() => {
                    is_static = true;
                    classmethod = true;
                }
                AExpr::Attribute(a) if class.is_some() && matches!(a.attr.id.as_str(), "setter" | "deleter") => {
                    return err(self.line(dec), "property setters are not supported")
                        .map_err(|e| e.hint("use a method: set_x(self, v)"));
                }
                // stubs: @rust("path or {self}.template({0})"), @mut
                AExpr::Name(n) if n.id.as_str() == "mut" && ext.is_some() => ext.as_mut().unwrap().self_mut = true,
                AExpr::Call(c) if ext.is_some() && matches!(&*c.func, AExpr::Name(n) if n.id.as_str() == "rust") => {
                    let Some(AExpr::StringLiteral(l)) = c.arguments.args.first() else {
                        return err(self.line(dec), "@rust needs a string");
                    };
                    ext.as_mut().unwrap().rust = l.value.to_str().to_string();
                }
                _ => return err(self.line(dec), "decorators are not supported here")
                    .map_err(|e| e.hint("supported: @dataclass on classes, @staticmethod / @classmethod / @property on methods, @functools.cache on functions")),
            }
        }
        let fid = self.new_func(d.name.id.as_str(), line, class);
        self.p.funcs[fid].cache = cache;
        self.enter(fid);
        let ps = &d.parameters;
        if ps.kwarg.is_some() {
            return err(line, "**kwargs is not supported");
        }
        // `*args: T` is a `list[T]` parameter between the positional and keyword-only ones
        let star = ps.vararg.as_ref().map(|v| ast::ParameterWithDefault {
            node_index: Default::default(),
            range: v.range,
            parameter: (**v).clone(),
            default: None,
        });
        let mut all: Vec<&ast::ParameterWithDefault> = ps.posonlyargs.iter().chain(ps.args.iter()).collect();
        let star_at = all.len();
        all.extend(star.iter());
        all.extend(ps.kwonlyargs.iter());
        let mut first = true;
        for (pi, p) in all.into_iter().enumerate() {
            let is_star = star.is_some() && pi == star_at;
            let name = p.parameter.name.id.as_str();
            if first && classmethod {
                first = false;
                self.p.funcs[fid].cls = Some(name.to_string());
                continue;
            }
            if first && class.is_some() && !is_static {
                first = false;
                if name != "self" {
                    return err(line, "the first parameter of a method must be `self`");
                }
                let c = class.unwrap();
                let st = if self.p.classes[c].subs.is_empty() { Ty::Class(c) } else { Ty::Obj(c) };
                let v = self.new_var("self", st, VarKind::SelfParam);
                self.p.funcs[fid].self_var = Some(v);
                continue;
            }
            first = false;
            let Some(a) = &p.parameter.annotation else {
                return err(self.line(&p.parameter), format!("parameter `{}` needs a type annotation", name));
            };
            // Ref[T] / Mut[T] / Own[T]
            let (a, force): (&AExpr, Option<Force>) = match &**a {
                AExpr::Subscript(sub) => match &*sub.value {
                    AExpr::Name(n) if self.imports.contains(n.id.as_str()) && ["Ref", "Mut", "Own"].contains(&n.id.as_str()) => {
                        let f = match n.id.as_str() {
                            "Ref" => Force::Ref,
                            "Mut" => Force::Mut,
                            _ => Force::Own,
                        };
                        (&*sub.slice, Some(f))
                    }
                    _ => (&**a, None),
                },
                _ => (&**a, None),
            };
            let width = if self.in_stub.is_some() { width_of(a) } else { None };
            let ty = self.ann(a)?;
            let ty = if is_star {
                self.p.funcs[fid].varargs = Some(self.p.funcs[fid].params.len());
                Ty::list(ty)
            } else {
                ty
            };
            if force.is_some() && ty.is_copy() {
                return err(self.line(&p.parameter), "Ref / Mut / Own are for lists, dicts, sets, str and objects");
            }
            let default = match &p.default {
                Some(e) => {
                    let ex = self.expr(e, Some(&ty))?;
                    let ex = self.coerce(ex, &ty, "default value")?;
                    if !is_const_expr(e, &self.consts) {
                        return err(ex.line, "default values must be constants (numbers, strings, bools)")
                            .map_err(|e| e.hint("Python evaluates defaults once; use None-free constants"));
                    }
                    Some(ex)
                }
                None => None,
            };
            let v = self.new_var(name, ty, VarKind::Param);
            self.p.funcs[fid].params.push(Param { var: v, default, force, width });
        }
        if class.is_some() && !is_static && self.p.funcs[fid].self_var.is_none() {
            return err(line, "methods need a `self` parameter");
        }
        self.p.funcs[fid].ret = match &d.returns {
            // stubs: Result[T] raises ValueError on Err
            Some(r) if self.in_stub.is_some() => {
                let inner: &AExpr = match &**r {
                    AExpr::Subscript(sub) if matches!(&*sub.value, AExpr::Name(n) if n.id.as_str() == "Result") => {
                        ext.as_mut().unwrap().result = true;
                        &sub.slice
                    }
                    x => x,
                };
                ext.as_mut().unwrap().ret_width = width_of(inner);
                self.ann(inner)?
            }
            Some(r) => {
                let t = self.ann(r)?;
                if yields(&d.body) {
                    if !matches!(t, Ty::Gen(_)) {
                        return err(line, "a generator returns Iterator[T]");
                    }
                    if class.is_some() {
                        return err(line, "generator methods are not supported").map_err(|e| e.hint("use a function taking the object"));
                    }
                    self.p.funcs[fid].is_gen = true;
                }
                t
            }
            None => Ty::None,
        };
        if let Some(mut x) = ext {
            if x.rust.is_empty() {
                let ty_name = class.and_then(|c| self.p.classes[c].ext.clone()).map(|p| p.rsplit("::").next().unwrap().to_string());
                x.rust = match (ty_name, d.name.id.as_str()) {
                    (Some(t), "__init__") => format!("{}::new", t),
                    (Some(t), n) if is_static => format!("{}::{}", t, n),
                    (Some(_), n) => n.to_string(),
                    (None, n) => format!("{}::{}", self.in_stub.as_ref().unwrap(), n),
                };
            }
            self.p.funcs[fid].ext = Some(x);
        }
        self.p.funcs[fid].is_static = is_static;
        if property {
            if !self.p.funcs[fid].params.is_empty() || self.p.funcs[fid].ret == Ty::None {
                return err(line, "a property takes only self and returns a value");
            }
            self.p.funcs[fid].property = true;
        }
        if cache {
            for q in self.p.funcs[fid].params.clone() {
                let t = self.p.funcs[fid].vars[q.var].ty.clone();
                self.hashable(&t, line).map_err(|e| Error::new(line, format!("@cache: {}", e.msg)))?;
            }
        }
        Ok(fid)
    }

    fn class_sig<'a>(
        &mut self,
        d: &'a ast::StmtClassDef,
        bodies: &mut Vec<(FuncId, &'a ast::StmtFunctionDef)>,
        asts: &mut HashMap<ClassId, Vec<(String, &'a ast::StmtFunctionDef, ClassId)>>,
    ) -> Res<()> {
        let cid = self.classes[d.name.id.as_str()];
        if self.p.classes[cid].exc.is_some() {
            if d.body.iter().all(|s| matches!(s, AStmt::Pass(_)) || is_docstring(s)) {
                return Ok(());
            }
            // an exception with fields: a class whose `super().__init__(m)` sets `msg_`
            let leaked: &'a ast::StmtClassDef = Box::leak(Box::new(self.exc_class(d)?));
            return self.class_sig_(leaked, cid, bodies, asts);
        }
        self.class_sig_(d, cid, bodies, asts)
    }

    fn class_sig_<'a>(
        &mut self,
        d: &'a ast::StmtClassDef,
        cid: ClassId,
        bodies: &mut Vec<(FuncId, &'a ast::StmtFunctionDef)>,
        asts: &mut HashMap<ClassId, Vec<(String, &'a ast::StmtFunctionDef, ClassId)>>,
    ) -> Res<()> {
        for dec in &d.decorator_list {
            let (name, kws) = match &dec.expression {
                AExpr::Name(n) => (n.id.as_str(), None),
                AExpr::Call(c) => match &*c.func {
                    AExpr::Name(n) => (n.id.as_str(), Some(&c.arguments)),
                    _ => ("", None),
                },
                _ => ("", None),
            };
            if name == "shared" && self.imports.contains("shared") {
                let c = &self.p.classes[cid];
                if c.base.is_some() || !c.subs.is_empty() {
                    return err(self.line(dec), "@shared classes cannot be in a class hierarchy");
                }
                self.p.classes[cid].shared = true;
                continue;
            }
            if name != "dataclass" {
                return err(self.line(dec), "only @dataclass / @shared are supported on classes");
            }
            self.p.classes[cid].dataclass = true;
            if let Some(a) = kws {
                for k in &a.keywords {
                    match (k.arg.as_ref().map(|i| i.id.as_str()), &k.value) {
                        (Some("frozen"), AExpr::BooleanLiteral(b)) => self.p.classes[cid].frozen = b.value,
                        (Some("eq"), AExpr::BooleanLiteral(b)) if b.value => {}
                        _ => return err(self.line(dec), "unsupported dataclass option"),
                    }
                }
            }
        }
        let main = self.p.main;
        for s in &d.body {
            // `count = 0` / `X: ClassVar[int] = 0`: a class attribute
            let attr = match s {
                AStmt::Assign(a) if a.targets.len() == 1 => match &a.targets[0] {
                    AExpr::Name(n) => Some((n.id.as_str(), None, &*a.value)),
                    _ => None,
                },
                AStmt::AnnAssign(a) => match (&*a.target, &*a.annotation, &a.value) {
                    (AExpr::Name(n), AExpr::Subscript(sub), Some(v)) if matches!(&*sub.value, AExpr::Name(c) if c.id.as_str() == "ClassVar") => {
                        Some((n.id.as_str(), Some(&*sub.slice), &**v))
                    }
                    _ => None,
                },
                _ => None,
            };
            if let Some((name, ann, value)) = attr {
                let line = self.line(s);
                if !is_const_expr(value, &self.consts) {
                    return err(line, "class attributes must be constants (literals)")
                        .map_err(|e| e.hint("a shared mutable class attribute is global state; keep it in an object"));
                }
                self.enter(main);
                let want = match ann {
                    Some(a) => Some(self.ann(a)?),
                    None => None,
                };
                let mut e = self.expr(value, want.as_ref())?;
                if let Some(w) = &want {
                    e = self.coerce(e, w, "class attribute")?;
                }
                if !matches!(self.norm(&e.ty), Ty::Int | Ty::Float | Ty::Bool | Ty::Str) {
                    return err(line, "class attributes must be int, float, bool or str");
                }
                let cname = format!("{}_{}", snake_name(&d.name.id).to_uppercase(), name.to_uppercase());
                self.class_attrs.insert((cid, name.to_string()), self.p.consts.len());
                self.p.consts.push(Const { name: cname, value: e });
                continue;
            }
            match s {
                AStmt::AnnAssign(a) => {
                    let AExpr::Name(n) = &*a.target else {
                        return err(self.line(s), "unsupported class attribute");
                    };
                    self.enter(main);
                    let ty = self.ann(&a.annotation)?;
                    let factory = match a.value.as_deref() {
                        Some(AExpr::Call(c)) if matches!(&*c.func, AExpr::Name(n) if n.id.as_str() == "field") => {
                            match c.arguments.keywords.first().map(|k| (k.arg.as_ref().map(|i| i.id.as_str()), &k.value)) {
                                Some((Some("default_factory"), AExpr::Name(f))) if c.arguments.keywords.len() == 1 && c.arguments.args.is_empty() => {
                                    Some(f.id.to_string())
                                }
                                _ => return err(self.line(s), "only field(default_factory=list / dict / set) is supported"),
                            }
                        }
                        _ => None,
                    };
                    let default = match &a.value {
                        // a fresh empty container for every object
                        Some(_) if factory.is_some() => {
                            let l = self.line(s);
                            let k = match (factory.as_deref(), self.norm(&ty)) {
                                (Some("list"), Ty::List(_)) => ExprKind::List(vec![]),
                                (Some("dict"), Ty::Dict(..)) => ExprKind::Dict(vec![]),
                                (Some("set"), Ty::Set(_)) => ExprKind::Set(vec![]),
                                _ => return err(l, "default_factory must match the field type (list, dict or set)"),
                            };
                            Some(self.mk(l, ty.clone(), k))
                        }
                        Some(v) => {
                            if !is_const_expr(v, &self.consts) {
                                return err(self.line(s), "field defaults must be constants");
                            }
                            let e = self.expr(v, Some(&ty))?;
                            Some(self.coerce(e, &ty, "field default")?)
                        }
                        None => None,
                    };
                    self.p.classes[cid].fields.push(Field { name: n.id.to_string(), ty, default });
                }
                AStmt::FunctionDef(f) => {
                    let fid = self.signature(f, Some(cid))?;
                    let name = f.name.id.as_str();
                    if self.p.classes[cid].method(name).is_some() {
                        return err(self.line(f), format!("method `{}` is defined twice", name));
                    }
                    if name == "__init__" {
                        if self.p.classes[cid].dataclass {
                            return err(self.line(f), "a dataclass cannot define __init__");
                        }
                        if self.p.funcs[fid].ret != Ty::None {
                            return err(self.line(f), "__init__ must return None");
                        }
                        self.p.funcs[fid].is_init = true;
                        self.p.classes[cid].init = Some(fid);
                    } else if name.starts_with("__") && !["__str__", "__repr__"].contains(&name) {
                        if !dunder::SUPPORTED.contains(&name) {
                            return err(self.line(f), format!("special method `{}` is not supported", name));
                        }
                        self.check_dunder(cid, fid, self.line(f))?;
                    }
                    self.p.classes[cid].methods.push((name.to_string(), fid));
                    bodies.push((fid, f));
                }
                AStmt::Pass(_) => {}
                _ if is_docstring(s) => {}
                _ => return err(self.line(s), "unsupported statement in class body"),
            }
        }
        // methods of the base class, compiled again for this class (so that
        // `self.m()` calls this class's version); entries: (name, ast, class it is written in)
        let mut map: Vec<(String, &'a ast::StmtFunctionDef, ClassId)> = match self.p.classes[cid].base {
            Some(b) => asts.get(&b).cloned().unwrap_or_default(),
            None => vec![],
        };
        let own: Vec<(String, &'a ast::StmtFunctionDef)> = d
            .body
            .iter()
            .filter_map(|s| if let AStmt::FunctionDef(f) = s { Some((f.name.id.to_string(), f)) } else { None })
            .collect();
        for (n, _) in &own {
            if let Some(fid) = self.p.classes[cid].method(n) {
                self.p.funcs[fid].def_class = Some(cid);
            }
        }
        let mut work: Vec<(&'a ast::StmtFunctionDef, ClassId)> = own.iter().map(|(_, f)| (*f, cid)).collect();
        if self.p.classes[cid].base.is_some() {
            for (name, f, def) in &map {
                if own.iter().any(|(n, _)| n == name) {
                    continue;
                }
                if name == "__init__" && self.p.classes[cid].dataclass {
                    return err(self.line(d), "a dataclass cannot inherit an __init__");
                }
                let fid = self.signature(f, Some(cid))?;
                self.p.funcs[fid].def_class = Some(*def);
                if name == "__init__" {
                    self.p.funcs[fid].is_init = true;
                    self.p.classes[cid].init = Some(fid);
                }
                self.p.classes[cid].methods.push((name.clone(), fid));
                bodies.push((fid, f));
                work.push((f, *def));
            }
        }
        // `super().m(...)` in a method written in class D: D's base's m, compiled
        // for this class as `<base>_<m>`
        while let Some((f, def)) = work.pop() {
            let mut used = vec![];
            super_calls(&f.body, &mut used);
            for m in used {
                if m == "__init__" {
                    continue;
                }
                let Some(b) = self.p.classes[def].base else { continue };
                let name = format!("{}_{}", snake_name(&self.p.classes[b].name), m);
                if self.p.classes[cid].method(&name).is_some() {
                    continue;
                }
                let Some((_, bf, bdef)) = asts.get(&b).and_then(|v| v.iter().find(|(n, _, _)| *n == m)).cloned() else {
                    return err(self.line(d), format!("`{}` has no method `{}`", self.p.classes[b].name, m));
                };
                let fid = self.signature(bf, Some(cid))?;
                self.p.funcs[fid].name = name.clone();
                self.p.funcs[fid].def_class = Some(bdef);
                self.p.classes[cid].methods.push((name, fid));
                bodies.push((fid, bf));
                work.push((bf, bdef));
            }
        }
        for (n, f) in own {
            map.retain(|(m, _, _)| *m != n);
            map.push((n, f, cid));
        }
        asts.insert(cid, map);
        Ok(())
    }

    /// For each class with subclasses, one method per name that calls the
    /// implementation of the actual class.
    fn dispatchers(&mut self) -> Res<()> {
        for c in 0..self.p.classes.len() {
            if self.p.classes[c].subs.is_empty() || self.p.classes[c].exc.is_some() {
                continue;
            }
            let methods = self.p.classes[c].methods.clone();
            for (name, fid) in methods {
                if name == "__init__" || self.p.funcs[fid].def_class.is_some_and(|d| !self.p.is_sub(c, d) && d != c) || name.contains('_') && self.p.classes.iter().any(|k| name.starts_with(&format!("{}_", snake_name(&k.name)))) || self.p.funcs[fid].is_static {
                    continue;
                }
                let proto = self.p.funcs[fid].clone();
                let sig = |f: &Func| -> (Vec<Ty>, Ty) {
                    (f.params.iter().map(|q| f.vars[q.var].ty.clone()).collect(), f.ret.clone())
                };
                let want = sig(&proto);
                let mut impls = vec![];
                for x in self.p.subtree(c) {
                    let Some(xf) = self.p.classes[x].method(&name) else { continue };
                    if sig(&self.p.funcs[xf]) != want {
                        return err(self.p.funcs[xf].line, format!(
                            "{}.{} must take and return the same types as in {}",
                            self.p.classes[x].name, name, self.p.classes[c].name));
                    }
                    impls.push((x, xf));
                }
                let mut f = proto;
                f.body = vec![];
                f.dispatch = impls;
                let mut vars = vec![];
                let mut params = vec![];
                let orig = self.p.funcs[fid].clone();
                let sv = Var { name: "self".into(), ty: Ty::Class(c), kind: VarKind::SelfParam };
                vars.push(sv);
                for q in &orig.params {
                    vars.push(orig.vars[q.var].clone());
                    params.push(Param { var: vars.len() - 1, default: q.default.clone(), force: q.force, width: q.width.clone() });
                }
                f.vars = vars;
                f.params = params;
                f.self_var = Some(0);
                let d = self.p.funcs.len();
                self.p.funcs.push(f);
                self.p.classes[c].dispatch.push((name, d));
            }
        }
        Ok(())
    }

    fn body(&mut self, fid: FuncId, body: &[AStmt]) -> Res<()> {
        self.enter(fid);
        // `cls` in a classmethod names the class it is compiled for
        let cls = match (&self.p.funcs[fid].cls, self.p.funcs[fid].class) {
            (Some(n), Some(c)) => Some((n.clone(), self.classes.insert(n.clone(), c))),
            _ => None,
        };
        let mut names = HashMap::new();
        assigned_names(body, &mut names);
        let mut names: HashSet<String> = names.into_keys().collect();
        names.extend(self.p.funcs[fid].vars.iter().filter(|v| matches!(v.kind, VarKind::Param | VarKind::SelfParam)).map(|v| v.name.clone()));
        self.enclosing.push(names);
        self.local_funcs.push(vec![]);
        let mut out = Vec::new();
        let mut res = Ok(());
        for s in body {
            if is_docstring(s) {
                continue;
            }
            res = self.stmt(s, &mut out);
            if res.is_err() {
                break;
            }
        }
        self.enclosing.pop();
        for (n, old) in self.local_funcs.pop().unwrap_or_default().into_iter().rev() {
            match old {
                Some(o) => self.funcs.insert(n, o),
                None => self.funcs.remove(&n),
            };
        }
        if let Some((n, old)) = cls {
            match old {
                Some(o) => self.classes.insert(n, o),
                None => self.classes.remove(&n),
            };
        }
        res?;
        self.p.funcs[fid].body = out;
        Ok(())
    }

    // ------------------------------------------------------------ annotations

    fn ann(&mut self, e: &AExpr) -> Res<Ty> {
        let line = self.line(e);
        if let Some(n) = self.local_name(e) {
            return self.ann(&n);
        }
        // in an instance of a generic function: its type variables and unions
        if let AExpr::Name(n) = e {
            if let Some(t) = self.type_env.get(n.id.as_str()) {
                return Ok(t.clone());
            }
            if self.typevars.contains(n.id.as_str()) || self.protocols.contains_key(n.id.as_str()) {
                return err(line, format!("`{}` can only be used in the parameters of a function", n.id));
            }
        }
        if generic::is_union_ann(e) {
            return match self.type_env.get(&format!("U:{}", self.src[e.range()].trim())) {
                Some(t) => Ok(t.clone()),
                None => err(line, "union types are only supported as function parameters")
                    .map_err(|e| e.hint("each call is compiled for the type of its argument")),
            };
        }
        match e {
            AExpr::Name(n) => match n.id.as_str() {
                "int" => Ok(Ty::Int),
                "float" => Ok(Ty::Float),
                "bool" => Ok(Ty::Bool),
                "str" => Ok(Ty::Str),
                "None" => Ok(Ty::None),
                w if self.in_stub.is_some() && WIDTHS.contains(&w) => Ok(if w == "f32" { Ty::Float } else { Ty::Int }),
                "list" | "List" | "dict" | "Dict" | "set" | "Set" | "tuple" | "Tuple" => {
                    err(line, format!("`{}` needs element types, e.g. list[int]", n.id))
                }
                "Any" | "object" => err(line, "`Any` / `object` is outside the typed subset")
                    .map_err(|e| e.hint("use a concrete type")),
                c => match self.classes.get(c) {
                    Some(id) => Ok(Ty::Class(*id)),
                    None => err(line, format!("unknown type `{}`", c)),
                },
            },
            AExpr::NoneLiteral(_) => Ok(Ty::None),
            AExpr::StringLiteral(s) => {
                let text = s.value.to_str();
                match ruff_python_parser::parse_expression(text) {
                    Ok(p) => {
                        let inner = p.into_expr();
                        self.ann(&inner).map_err(|e| Error::new(line, e.msg))
                    }
                    Err(_) => err(line, format!("unknown type `{}`", text)),
                }
            }
            AExpr::Subscript(s) => {
                let args: Vec<&AExpr> = match &*s.slice {
                    AExpr::Tuple(t) => t.elts.iter().collect(),
                    x => vec![x],
                };
                if let Some(q) = self.std_name(&s.value) {
                    return self.std_ann(&q, &args, line);
                }
                let AExpr::Name(n) = &*s.value else { return err(line, "unsupported type annotation") };
                match (n.id.as_str(), args.len()) {
                    ("list" | "List", 1) => Ok(Ty::list(self.ann(args[0])?)),
                    ("set" | "Set", 1) => Ok(Ty::set(self.ann(args[0])?)),
                    ("dict" | "Dict", 2) => Ok(Ty::dict(self.ann(args[0])?, self.ann(args[1])?)),
                    ("tuple" | "Tuple", _) => {
                        if args.iter().any(|a| matches!(a, AExpr::EllipsisLiteral(_))) {
                            return err(line, "variable-length tuples are not supported; use list[T]");
                        }
                        Ok(Ty::Tuple(args.iter().map(|a| self.ann(a)).collect::<Res<_>>()?))
                    }
                    ("Optional", 1) => self.optional(args[0], line),
                    ("Iterator" | "Iterable" | "Generator", _) if !args.is_empty() => {
                        if n.id.as_str() == "Generator" && args[1..].iter().any(|a| !matches!(a, AExpr::NoneLiteral(_))) {
                            return err(line, "Generator[T, None, None]: send() and return values are not supported");
                        }
                        Ok(Ty::Gen(Box::new(self.ann(args[0])?)))
                    }
                    ("Callable", 2) => {
                        let AExpr::List(ps) = args[0] else {
                            return err(line, "Callable[[A, B], R] needs the parameter types in a list");
                        };
                        let ps = ps.elts.iter().map(|a| self.ann(a)).collect::<Res<Vec<_>>>()?;
                        let r = self.ann(args[1])?;
                        Ok(Ty::Func(ps, Box::new(r)))
                    }
                    ("Union", 2) => {
                        let (a, b) = (args[0], args[1]);
                        if matches!(b, AExpr::NoneLiteral(_)) {
                            self.optional(a, line)
                        } else if matches!(a, AExpr::NoneLiteral(_)) {
                            self.optional(b, line)
                        } else {
                            err(line, "union types other than `X | None` are not supported")
                        }
                    }
                    _ => err(line, "unsupported type annotation"),
                }
            }
            AExpr::BinOp(b) if b.op == ast::Operator::BitOr => {
                if matches!(*b.right, AExpr::NoneLiteral(_)) {
                    self.optional(&b.left, line)
                } else if matches!(*b.left, AExpr::NoneLiteral(_)) {
                    self.optional(&b.right, line)
                } else {
                    err(line, "union types other than `X | None` are not supported")
                }
            }
            _ => err(line, "unsupported type annotation"),
        }
    }

    fn optional(&mut self, a: &AExpr, line: u32) -> Res<Ty> {
        let t = self.ann(a)?;
        if matches!(t, Ty::Optional(_) | Ty::None) {
            return err(line, "nested Optional types are not supported");
        }
        Ok(Ty::opt(t))
    }

    // ------------------------------------------------------------- statements

    fn block(&mut self, body: &[AStmt]) -> Res<Vec<Stmt>> {
        let mut out = Vec::new();
        let depth = self.narrow.len();
        for s in body {
            self.stmt(s, &mut out)?;
        }
        self.narrow.truncate(depth);
        Ok(out)
    }

    /// `P is not None` (true) / `P is None` (false) for a place P, and the rest
    /// of an `and` chain.
    fn narrowing<'a>(&mut self, test: &'a AExpr) -> Option<(&'a AExpr, bool, Vec<&'a AExpr>, Option<ClassId>)> {
        fn is_place(e: &AExpr) -> bool {
            match e {
                AExpr::Name(_) => true,
                AExpr::Attribute(a) => is_place(&a.value),
                _ => false,
            }
        }
        match test {
            AExpr::Compare(c) if c.ops.len() == 1 && matches!(c.operands[1], AExpr::NoneLiteral(_)) && is_place(&c.operands[0]) => {
                match c.ops[0] {
                    ast::CmpOp::IsNot | ast::CmpOp::NotEq => Some((&c.operands[0], true, vec![], None)),
                    ast::CmpOp::Is | ast::CmpOp::Eq => Some((&c.operands[0], false, vec![], None)),
                    _ => None,
                }
            }
            AExpr::Call(c) if matches!(&*c.func, AExpr::Name(n) if n.id.as_str() == "isinstance" && self.lookup("isinstance").is_none())
                && c.arguments.args.len() == 2 && is_place(&c.arguments.args[0]) =>
            {
                let d = self.class_arg(&c.arguments.args[1], 0).ok()?;
                Some((&c.arguments.args[0], true, vec![], Some(d)))
            }
            AExpr::UnaryOp(u) if u.op == ast::UnaryOp::Not && matches!(&*u.operand, AExpr::Call(_)) => {
                let (p, pos, rest, d) = self.narrowing(&u.operand)?;
                if !pos || !rest.is_empty() || d.is_none() {
                    return None;
                }
                Some((p, false, rest, d))
            }
            AExpr::Name(_) | AExpr::Attribute(_) if is_place(test) => {
                let e = self.expr(test, None).ok()?;
                match self.norm(&e.ty) {
                    Ty::Optional(t) if matches!(*t, Ty::Class(_) | Ty::Match(_) | Ty::Pattern(_)) => Some((test, true, vec![], None)),
                    _ => None,
                }
            }
            AExpr::UnaryOp(u) if u.op == ast::UnaryOp::Not && is_place(&u.operand) => {
                let e = self.expr(&u.operand, None).ok()?;
                match self.norm(&e.ty) {
                    Ty::Optional(t) if matches!(*t, Ty::Class(_) | Ty::Match(_) | Ty::Pattern(_)) => Some((&u.operand, false, vec![], None)),
                    _ => None,
                }
            }
            AExpr::BoolOp(b) if b.op == ast::BoolOp::And => {
                let (p, pos, _, d) = self.narrowing(&b.values[0])?;
                if !pos {
                    return None;
                }
                Some((p, true, b.values[1..].iter().collect(), d))
            }
            _ => None,
        }
    }

    /// Lower the place of a narrowing and create the variable for its value.
    fn narrow_var(&mut self, place: &AExpr, cls: Option<ClassId>) -> Res<Option<(Expr, VarId)>> {
        let pe = self.expr(place, None)?;
        let inner = match cls {
            None => {
                let Ty::Optional(inner) = self.norm(&pe.ty) else {
                    return err(pe.line, format!("this is {}, never None", self.tn(&pe.ty)));
                };
                inner
            }
            Some(d) => {
                // only a value that may be several classes, narrowed to one without subclasses
                let Ty::Class(c) = self.norm(&pe.ty) else { return Ok(None) };
                if self.p.classes[c].subs.is_empty() || d == c || !self.p.is_sub(d, c) {
                    return Ok(None);
                }
                Box::new(Ty::Class(d))
            }
        };
        let Some(key) = place_key(&pe) else { return Ok(None) };
        let name = match place {
            AExpr::Name(n) => n.id.to_string(),
            AExpr::Attribute(a) => {
                let n = a.attr.id.to_string();
                if self.lookup(&n).is_some() { format!("{}_", n) } else { n }
            }
            _ => unreachable!(),
        };
        let vars = &mut self.p.funcs[self.cur].vars;
        vars.push(Var { name, ty: *inner, kind: VarKind::Narrow });
        let v = vars.len() - 1;
        let _ = key;
        Ok(Some((pe, v)))
    }

    fn push_narrow(&mut self, pe: &Expr, v: VarId) {
        if let Some(k) = place_key(pe) {
            self.narrow.push((k, v));
        }
    }

    /// An assignment to a place ends the narrowings of it and below it.
    fn unnarrow(&mut self, t: &Expr) {
        if let Some((r, path)) = place_key(t) {
            self.narrow.retain(|((r2, p2), _)| !(*r2 == r && p2.len() >= path.len() && p2[..path.len()] == path[..]));
        }
    }

    fn narrowed(&self, e: &Expr) -> Option<VarId> {
        let k = place_key(e)?;
        self.narrow.iter().rev().find(|(k2, _)| *k2 == k).map(|(_, v)| *v)
    }

    fn stmt(&mut self, s: &AStmt, out: &mut Vec<Stmt>) -> Res<()> {
        let line = self.line(s);
        let kind = match s {
            AStmt::Expr(e) => {
                if matches!(*e.value, AExpr::StringLiteral(_) | AExpr::EllipsisLiteral(_)) {
                    return Ok(());
                }
                match &*e.value {
                    AExpr::Yield(y) => {
                        let Ty::Gen(t) = self.p.funcs[self.cur].ret.clone() else {
                            return err(line, "`yield` outside a generator");
                        };
                        let Some(v) = &y.value else { return err(line, "`yield` needs a value") };
                        let x = self.expr(v, Some(&t))?;
                        let x = self.coerce(x, &t, "yielded value")?;
                        let ye = self.mk(line, Ty::None, ExprKind::Builtin("yield", vec![x]));
                        out.push(self.stmt_(line, StmtKind::Expr(ye)));
                        return Ok(());
                    }
                    AExpr::YieldFrom(y) => {
                        // `for v in it: yield v`
                        let Ty::Gen(t) = self.p.funcs[self.cur].ret.clone() else {
                            return err(line, "`yield from` outside a generator");
                        };
                        let t = *t;
                        let (iter, item) = self.iter(&y.value)?;
                        if !self.unify(&item, &t) {
                            return err(line, "`yield from` items have the wrong type");
                        }
                        let vars = &mut self.p.funcs[self.cur].vars;
                        vars.push(Var { name: "item".into(), ty: t.clone(), kind: VarKind::Bound });
                        let v = vars.len() - 1;
                        self.active.insert(v);
                        let x = self.mk(line, t, ExprKind::Var(v));
                        let ye = self.mk(line, Ty::None, ExprKind::Builtin("yield", vec![x]));
                        let body = vec![self.stmt_(line, StmtKind::Expr(ye))];
                        self.active.remove(&v);
                        out.push(self.stmt_(line, StmtKind::For(Target::Var(v), iter, body, vec![])));
                        return Ok(());
                    }
                    AExpr::Call(_) => {}
                    _ => return err(line, "expression statement has no effect"),
                }
                let ex = self.expr(&e.value, None)?;
                StmtKind::Expr(ex)
            }
            AStmt::Pass(_) => StmtKind::Pass,
            AStmt::Break(_) => StmtKind::Break,
            AStmt::Continue(_) => StmtKind::Continue,
            AStmt::Return(r) => {
                let ret = self.p.funcs[self.cur].ret.clone();
                if self.cur == self.p.main {
                    return err(line, "`return` outside a function");
                }
                match &r.value {
                    None if ret == Ty::None || matches!(ret, Ty::Gen(_)) => StmtKind::Return(None),
                    Some(_) if matches!(ret, Ty::Gen(_)) => return err(line, "a generator cannot return a value"),
                    None if matches!(ret, Ty::Optional(_)) => {
                        let n = self.mk(line, ret.clone(), ExprKind::NoneVal);
                        StmtKind::Return(Some(n))
                    }
                    None => return err(line, format!("missing return value ({})", self.tn(&ret))),
                    Some(v) => {
                        if ret == Ty::None {
                            return err(line, "function returns a value but has no return annotation")
                                .map_err(|e| e.hint("add `-> type` to the def"));
                        }
                        let e = self.expr(v, Some(&ret))?;
                        StmtKind::Return(Some(self.coerce(e, &ret, "return value")?))
                    }
                }
            }
            AStmt::If(i) => {
                let st = self.lower_if(&i.test, &i.body, &i.elif_else_clauses, line, true, out)?;
                if let Some(st) = st {
                    out.push(st);
                }
                return Ok(());
            }
            AStmt::While(w) => {
                let depth = self.narrow.len();
                let cond = match self.narrowing(&w.test) {
                    Some((place, true, rest, cls)) => match self.narrow_var(place, cls)? {
                        Some((pe, v)) => self.narrow_cond(pe, v, &rest, line)?,
                        None => self.cond(&w.test)?,
                    },
                    _ => self.cond(&w.test)?,
                };
                self.loops += 1;
                let body = self.block(&w.body)?;
                self.loops -= 1;
                self.narrow.truncate(depth);
                let orelse = self.block(&w.orelse)?;
                StmtKind::While(cond, body, orelse)
            }
            AStmt::For(f) => {
                if f.is_async {
                    return err(line, "async for is not supported");
                }
                let (iter, item) = self.iter(&f.iter)?;
                self.scopes.push(HashMap::new());
                let target = self.bind_target(&f.target, &item)?;
                let mut vars = vec![];
                target.vars(&mut vars);
                // loop variables are function-scoped in Python: keep them visible
                // in the function scope while the loop runs
                let inner = self.scopes.pop().unwrap();
                let saved: Vec<(String, Option<VarId>)> = inner
                    .iter()
                    .map(|(n, v)| (n.clone(), self.scopes[0].insert(n.clone(), *v)))
                    .collect();
                self.active.extend(vars.iter().copied());
                // `for k in d` / `for k, v in d.items()`: k is a key of d
                let key_of = match (&iter, &target) {
                    (Iter::Over(d) | Iter::Keys(d), Target::Var(k)) => Some((*k, d)),
                    (Iter::Items(d), Target::Tuple(ts)) => match ts.first() {
                        Some(Target::Var(k)) => Some((*k, d)),
                        _ => None,
                    },
                    _ => None,
                };
                if let Some((k, Expr { kind: ExprKind::Var(d), ty: Ty::Dict(..), .. })) = key_of {
                    self.loop_keys.insert(k, *d);
                }
                self.loops += 1;
                let body = self.block(&f.body)?;
                self.loops -= 1;
                for v in &vars {
                    self.active.remove(v);
                    self.loop_keys.remove(v);
                }
                let orelse = self.block(&f.orelse)?;
                for (n, old) in saved {
                    match old {
                        Some(o) if self.var(o).kind != VarKind::Bound => {
                            self.scopes[0].insert(n, o);
                        }
                        _ => {}
                    }
                }
                StmtKind::For(target, iter, body, orelse)
            }
            AStmt::Assign(a) => {
                if a.targets.len() != 1 {
                    return err(line, "chained assignment (`a = b = ...`) is not supported")
                        .map_err(|e| e.hint("assign each name separately"));
                }
                return self.assign(&a.targets[0], None, Some(&a.value), line, out);
            }
            AStmt::AnnAssign(a) => {
                let ty = self.ann(&a.annotation)?;
                return self.assign(&a.target, Some(ty), a.value.as_deref(), line, out);
            }
            AStmt::AugAssign(a) => self.aug_assign(a, line)?,
            AStmt::Raise(r) => {
                if r.cause.is_some() {
                    return err(line, "`raise ... from` is not supported");
                }
                let Some(exc) = &r.exc else {
                    return match self.handlers.last() {
                        Some(v) => {
                            out.push(self.stmt_(line, StmtKind::Reraise(*v)));
                            Ok(())
                        }
                        None => err(line, "bare `raise` outside an except clause"),
                    };
                };
                // an exception with fields: raised as an object
                let cname = match &**exc {
                    AExpr::Name(n) => Some(n.id.as_str()),
                    AExpr::Call(c) => match &*c.func {
                        AExpr::Name(n) => Some(n.id.as_str()),
                        _ => None,
                    },
                    _ => None,
                };
                if let Some(&c) = cname.and_then(|n| self.classes.get(n)) {
                    if self.p.classes[c].exc.is_some() && self.p.classes[c].init.is_some() {
                        let AExpr::Call(call) = &**exc else {
                            return err(line, format!("`{}` needs its arguments", cname.unwrap()));
                        };
                        let obj = self.construct_(c, &call.arguments, line)?;
                        out.push(self.stmt_(line, StmtKind::Raise(cname.unwrap().to_string(), Some(obj))));
                        return Ok(());
                    }
                }
                let (name, arg) = match &**exc {
                    AExpr::Name(n) => (n.id.to_string(), None),
                    AExpr::Call(c) => match &*c.func {
                        AExpr::Name(n) if c.arguments.args.len() <= 1 && c.arguments.keywords.is_empty() => {
                            (n.id.to_string(), c.arguments.args.first())
                        }
                        _ => return err(line, "unsupported raise"),
                    },
                    _ => return err(line, "unsupported raise"),
                };
                if !self.is_exception(&name) {
                    return err(line, format!("unknown exception `{}`", name));
                }

                // str(e) is str(arg); for KeyError it is repr(arg)
                let key = crate::exc::chain(&self.p, &name).iter().any(|k| k == "KeyError");
                let msg = match arg {
                    Some(a) => {
                        let e = self.expr(a, None)?;
                        self.printable(&e.ty, line)?;
                        let l = e.line;
                        if key {
                            Some(self.mk(l, Ty::Str, ExprKind::Builtin("repr", vec![e])))
                        } else if self.norm(&e.ty) == Ty::Str {
                            Some(e)
                        } else {
                            Some(self.mk(l, Ty::Str, ExprKind::Builtin("str", vec![e])))
                        }
                    }
                    None => None,
                };
                StmtKind::Raise(name, msg)
            }
            AStmt::Assert(a) => {
                // `assert a is not None and b is not None`: one let-else each
                if let (AExpr::BoolOp(b), None) = (&*a.test, &a.msg) {
                    let all = b.op == ast::BoolOp::And
                        && b.values.iter().all(|v| matches!(self.narrowing(v), Some((_, true, ref r, None)) if r.is_empty()));
                    if all {
                        for v in &b.values {
                            let Some((place, _, _, _)) = self.narrowing(v) else { unreachable!() };
                            let Some((pe, nv)) = self.narrow_var(place, None)? else {
                                return err(line, "unsupported assert");
                            };
                            let raise = self.stmt_(line, StmtKind::Raise("AssertionError".into(), None));
                            self.push_narrow(&pe, nv);
                            out.push(self.stmt_(line, StmtKind::LetSome(nv, pe, vec![raise])));
                        }
                        return Ok(());
                    }
                }
                if let Some((place, true, rest, None)) = self.narrowing(&a.test) {
                    if rest.is_empty() && a.msg.is_none() {
                        if let Some((pe, v)) = self.narrow_var(place, None)? {
                            let raise = self.stmt_(line, StmtKind::Raise("AssertionError".into(), None));
                            self.push_narrow(&pe, v);
                            out.push(self.stmt_(line, StmtKind::LetSome(v, pe, vec![raise])));
                            return Ok(());
                        }
                    }
                }
                let c = self.cond(&a.test)?;
                let m = match &a.msg {
                    Some(m) => {
                        let e = self.expr(m, Some(&Ty::Str))?;
                        Some(self.coerce(e, &Ty::Str, "assert message")?)
                    }
                    None => None,
                };
                StmtKind::Assert(c, m)
            }
            AStmt::Delete(d) => {
                for t in &d.targets {
                    let AExpr::Subscript(_) = t else {
                        return err(line, "only `del xs[i]` / `del d[k]` are supported");
                    };
                    let e = self.expr(t, None)?;
                    if matches!(e.kind, ExprKind::Slice(..)) {
                        return err(line, "deleting a slice is not supported");
                    }
                    if self.norm(&self.base_ty(&e)) == Ty::Str {
                        return err(line, "strings are immutable");
                    }
                    out.push(self.stmt_(line, StmtKind::Del(e)));
                }
                return Ok(());
            }
            AStmt::Nonlocal(n) => {
                let f = &self.p.funcs[self.cur];
                let ok = n.names.iter().all(|x| {
                    f.params.iter().any(|p| f.vars[p.var].name == x.id.as_str() && p.force == Some(Force::Mut))
                });
                if !ok {
                    return err(line, "`nonlocal` names a variable of an enclosing function");
                }
                return Ok(());
            }
            AStmt::Global(_) => {
                return err(line, "`global` / `nonlocal` are outside the subset")
                    .map_err(|e| e.hint("pass the value as a parameter and return the new value"))
            }
            AStmt::FunctionDef(d) => {
                self.nested_def(d, line)?;
                return Ok(());
            }
            AStmt::ClassDef(_) => return err(line, "classes must be defined at module level"),
            AStmt::Try(t) => {
                if t.is_star {
                    return err(line, "`except*` is not supported");
                }
                let body = self.block(&t.body)?;
                let mut hs = vec![];
                for h in &t.handlers {
                    let ast::ExceptHandler::ExceptHandler(h) = h;
                    let hl = self.line(h);
                    let kinds = match &h.type_ {
                        None => vec!["BaseException".to_string()],
                        Some(e) => {
                            let names: Vec<&AExpr> = match &**e {
                                AExpr::Tuple(t) => t.elts.iter().collect(),
                                x => vec![x],
                            };
                            let mut ks = vec![];
                            for n in names {
                                let AExpr::Name(n) = n else { return err(hl, "except needs exception class names") };
                                if !self.is_exception(n.id.as_str()) {
                                    return err(hl, format!("unknown exception `{}`", n.id));
                                }
                                ks.push(n.id.to_string());
                            }
                            ks
                        }
                    };
                    self.scopes.push(HashMap::new());
                    let name = h.name.as_ref().map(|n| n.id.to_string()).unwrap_or_else(|| "caught".into());
                    let v = if h.name.is_some() {
                        self.new_var(&name, Ty::Exc, VarKind::Bound)
                    } else {
                        let vars = &mut self.p.funcs[self.cur].vars;
                        vars.push(Var { name, ty: Ty::Exc, kind: VarKind::Bound });
                        vars.len() - 1
                    };
                    self.active.insert(v);
                    self.handlers.push(v);
                    // `except C as e` of an exception with fields: `e.field`
                    if let [k] = &kinds[..] {
                        if let Some(&c) = self.classes.get(k.as_str()) {
                            if self.p.classes[c].init.is_some() {
                                self.exc_vars.insert(v, c);
                            }
                        }
                    }
                    let b = self.block(&h.body)?;
                    self.handlers.pop();
                    self.active.remove(&v);
                    self.scopes.pop();
                    hs.push(Handler { kinds, var: v, body: b });
                }
                let orelse = self.block(&t.orelse)?;
                let fin = self.block(&t.finalbody)?;
                StmtKind::Try(body, hs, orelse, fin)
            }
            AStmt::With(w) => {
                if w.is_async {
                    return err(line, "`async with` is not supported");
                }
                // `with open(p) as f:` → `f = open(p)`, the body, `f.close()`
                let mut files = vec![];
                for it in &w.items {
                    let Some(AExpr::Name(n)) = it.optional_vars.as_deref() else {
                        return err(line, "write `with open(...) as f:`");
                    };
                    self.assign(it.optional_vars.as_deref().unwrap(), None, Some(&it.context_expr), line, out)?;
                    let v = self.lookup(n.id.as_str()).unwrap();
                    if self.norm(&self.var(v).ty) != Ty::File {
                        return err(line, "only `with open(...)` is supported");
                    }
                    files.push(v);
                }
                for s in &w.body {
                    self.stmt(s, out)?;
                }
                for v in files.into_iter().rev() {
                    let f = self.mk(line, Ty::File, ExprKind::Var(v));
                    let c = self.mk(line, Ty::None, ExprKind::Builtin("file.close", vec![f]));
                    out.push(self.stmt_(line, StmtKind::Expr(c)));
                }
                return Ok(());
            }
            AStmt::Match(m) => {
                for s in self.lower_match(m, line)? {
                    self.stmt(&s, out)?;
                }
                return Ok(());
            }
            AStmt::Import(_) | AStmt::ImportFrom(_) => return err(line, "imports must be at module level"),
            _ => return err(line, "unsupported statement"),
        };
        out.push(self.stmt_(line, kind));
        Ok(())
    }

    /// `IsSome(P) and rest...` with the narrowing active from here on.
    fn narrow_cond(&mut self, pe: Expr, v: VarId, rest: &[&AExpr], line: u32) -> Res<Expr> {
        self.push_narrow(&pe, v);
        let is = match (self.norm(&pe.ty), self.var(v).ty.clone()) {
            (Ty::Class(_), Ty::Class(d)) => self.mk(line, Ty::Bool, ExprKind::IsInst(Box::new(pe), d, Some(v))),
            _ => self.mk(line, Ty::Bool, ExprKind::IsSome(Box::new(pe), true, Some(v))),
        };
        if rest.is_empty() {
            return Ok(is);
        }
        let mut vs = vec![is];
        for r in rest {
            let x = self.cond(r)?;
            vs.push(x);
        }
        Ok(self.mk(line, Ty::Bool, ExprKind::And(vs)))
    }

    /// `if` / `elif` / `else`; every test may narrow. With `top`, a diverging
    /// `if x is None:` without else becomes a let-else (pushed to `out`).
    fn lower_if(
        &mut self,
        test: &AExpr,
        body: &[AStmt],
        clauses: &[ast::ElifElseClause],
        line: u32,
        top: bool,
        out: &mut Vec<Stmt>,
    ) -> Res<Option<Stmt>> {
        // `isinstance(x, int)` with x's type known: only the branch that runs
        if let Some(v) = self.static_isinstance(test)? {
            if v {
                let b = self.block(body)?;
                out.extend(b);
                return Ok(None);
            }
            return match clauses.split_first() {
                None => Ok(None),
                Some((c, rest)) => match &c.test {
                    Some(t) => self.lower_if(t, &c.body, rest, line, top, out),
                    None => {
                        let b = self.block(&c.body)?;
                        out.extend(b);
                        Ok(None)
                    }
                },
            };
        }
        let depth = self.narrow.len();
        if let Some((place, pos, rest, cls)) = self.narrowing(test) {
            if let Some((pe, v)) = self.narrow_var(place, cls)? {
                if pos {
                    let cond = self.narrow_cond(pe, v, &rest, line)?;
                    let then = self.block(body)?;
                    self.narrow.truncate(depth);
                    let els = self.lower_clauses(clauses)?;
                    return Ok(Some(self.stmt_(line, StmtKind::If(cond, then, els))));
                }
                let diverges = matches!(
                    body.last(),
                    Some(AStmt::Return(_) | AStmt::Raise(_) | AStmt::Break(_) | AStmt::Continue(_))
                );
                if top && clauses.is_empty() && diverges && cls.is_none() {
                    // `if x is None: return` → let-else, x narrowed for the rest of the block
                    let b = self.block(body)?;
                    self.push_narrow(&pe, v);
                    out.push(self.stmt_(line, StmtKind::LetSome(v, pe, b)));
                    return Ok(None);
                }
                // `if x is None: A else: B` → `if let Some(x) = .. { B } else { A }`
                let then = self.block(body)?;
                let cond = self.narrow_cond(pe, v, &[], line)?;
                let els = self.lower_clauses(clauses)?;
                self.narrow.truncate(depth);
                return Ok(Some(self.stmt_(line, StmtKind::If(cond, els, then))));
            }
        }
        let cond = self.cond(test)?;
        let then = self.block(body)?;
        let els = self.lower_clauses(clauses)?;
        Ok(Some(self.stmt_(line, StmtKind::If(cond, then, els))))
    }

    fn lower_clauses(&mut self, clauses: &[ast::ElifElseClause]) -> Res<Vec<Stmt>> {
        match clauses.first() {
            None => Ok(vec![]),
            Some(c) => match &c.test {
                None => self.block(&c.body),
                Some(t) => {
                    let line = self.line(c);
                    let mut dummy = vec![];
                    let st = self.lower_if(t, &c.body, &clauses[1..], line, false, &mut dummy)?;
                    Ok(st.into_iter().collect())
                }
            },
        }
    }

    fn stmt_(&mut self, line: u32, kind: StmtKind) -> Stmt {
        self.mk_stmt(line, kind)
    }

    fn base_ty(&self, e: &Expr) -> Ty {
        match &e.kind {
            ExprKind::Index(b, _) | ExprKind::Slice(b, ..) => b.ty.clone(),
            _ => e.ty.clone(),
        }
    }

    fn assign(
        &mut self,
        target: &AExpr,
        ann: Option<Ty>,
        value: Option<&AExpr>,
        line: u32,
        out: &mut Vec<Stmt>,
    ) -> Res<()> {
        // `xs[i], xs[j] = xs[j], xs[i]` → swap
        if let (AExpr::Tuple(t), Some(AExpr::Tuple(v))) = (target, value) {
            if let Some(s) = self.swap(&t.elts, &v.elts, line)? {
                out.push(s);
                return Ok(());
            }
        }
        // `obj[k] = v` → `obj.__setitem__(k, v)`
        if let (AExpr::Subscript(s), Some(value)) = (target, value) {
            let base = self.expr(&s.value, None)?;
            if let Some(f) = self.find_method(&base.ty, "__setitem__") {
                let pts: Vec<Ty> = self.p.funcs[f].params.iter().map(|p| self.p.funcs[f].vars[p.var].ty.clone()).collect();
                let k = self.expr(&s.slice, pts.first())?;
                let v = self.expr(value, pts.get(1))?;
                let e = self.call_dunder(base, f, vec![k, v], line)?;
                out.push(self.stmt_(line, StmtKind::Expr(e)));
                return Ok(());
            }
        }
        let Some(value) = value else {
            // `x: int` declares only
            let AExpr::Name(n) = target else { return err(line, "annotation without a value") };
            if self.lookup(n.id.as_str()).is_none() {
                self.new_var(n.id.as_str(), ann.unwrap(), VarKind::Local);
            }
            return Ok(());
        };
        let want = match (&ann, target) {
            (Some(t), _) => Some(t.clone()),
            (None, AExpr::Name(n)) => self.lookup(n.id.as_str()).map(|v| self.var(v).ty.clone()),
            _ => None,
        };
        let mut val = self.expr(value, want.as_ref())?;
        if let Some(w) = &ann {
            val = self.coerce(val, w, "value")?;
        }
        let ty = val.ty.clone();
        let t = self.target(target, &ty, ann.is_some(), &mut val, line)?;
        out.push(self.stmt_(line, StmtKind::Assign(t, val)));
        Ok(())
    }

    fn swap(&mut self, t: &[AExpr], v: &[AExpr], line: u32) -> Res<Option<Stmt>> {
        use ruff_python_ast::comparable::ComparableExpr;
        if t.len() != 2 || v.len() != 2 {
            return Ok(None);
        }
        let same = |a: &AExpr, b: &AExpr| ComparableExpr::from(a) == ComparableExpr::from(b);
        let (AExpr::Subscript(a), AExpr::Subscript(b)) = (&t[0], &t[1]) else { return Ok(None) };
        if !same(&a.value, &b.value) || !same(&t[0], &v[1]) || !same(&t[1], &v[0]) {
            return Ok(None);
        }
        let xs = self.expr(&a.value, None)?;
        if !matches!(self.norm(&xs.ty), Ty::List(_)) {
            return Ok(None);
        }
        let i = self.expr(&a.slice, Some(&Ty::Int))?;
        let j = self.expr(&b.slice, Some(&Ty::Int))?;
        if self.norm(&i.ty) != Ty::Int || self.norm(&j.ty) != Ty::Int {
            return Ok(None);
        }
        let e = self.mk(line, Ty::None, ExprKind::Builtin("list.swap", vec![xs, i, j]));
        Ok(Some(self.stmt_(line, StmtKind::Expr(e))))
    }

    /// Lower an assignment target, creating variables as needed.
    fn target(&mut self, t: &AExpr, ty: &Ty, declared: bool, val: &mut Expr, line: u32) -> Res<Target> {
        match t {
            AExpr::Name(n) => {
                let name = n.id.as_str();
                if self.consts.contains_key(name) && self.lookup(name).is_none() {
                    return err(line, format!("cannot assign to constant `{}`", name));
                }
                let found = self.lookup(name).filter(|v| !(self.var(*v).kind == VarKind::Bound && !self.active.contains(v)));
                match found {
                    Some(v) => {
                        if declared {
                            return err(line, format!("`{}` is already defined", name));
                        }
                        if self.var(v).kind == VarKind::SelfParam {
                            return err(line, "cannot assign to `self`");
                        }
                        let vt = self.var(v).ty.clone();
                        let place = self.mk(line, vt.clone(), ExprKind::Var(v));
                        self.unnarrow(&place);
                        let e = std::mem::replace(val, self.mk(line, Ty::None, ExprKind::NoneLit));
                        *val = self.coerce(e, &vt, &format!("`{}`", name)).map_err(|e| {
                            e.hint(format!("`{}` was declared as {}; a variable keeps one type", name, self.tn(&vt)))
                        })?;
                        Ok(Target::Var(v))
                    }
                    None => {
                        if self.norm(ty) == Ty::None {
                            return err(line, "cannot assign None (the function returns nothing)");
                        }
                        Ok(Target::Var(self.new_var(name, ty.clone(), VarKind::Local)))
                    }
                }
            }
            AExpr::Tuple(_) | AExpr::List(_) => {
                let elts = match t {
                    AExpr::Tuple(t) => &t.elts,
                    AExpr::List(l) => &l.elts,
                    _ => unreachable!(),
                };
                // `a, b = xs` (a list): checked at run time like Python
                if let Ty::List(et) = self.norm(ty) {
                    let n = elts.len();
                    let tt = Ty::Tuple(vec![(*et).clone(); n]);
                    let e = std::mem::replace(val, self.mk(line, Ty::None, ExprKind::NoneLit));
                    let k = self.mk(line, Ty::Int, ExprKind::Int(n as i64));
                    *val = self.mk(line, tt.clone(), ExprKind::Builtin("unpack", vec![e, k]));
                    let mut dummy = self.mk(line, Ty::None, ExprKind::NoneLit);
                    return self.target(t, &tt, declared, &mut dummy, line);
                }
                let Ty::Tuple(ts) = self.norm(ty) else {
                    return err(line, "only tuples can be unpacked");
                };
                if ts.len() != elts.len() {
                    return err(line, format!("cannot unpack {} values into {}", ts.len(), elts.len()));
                }
                let mut out = Vec::new();
                for (e, t) in elts.iter().zip(ts.iter()) {
                    let mut dummy = self.mk(line, t.clone(), ExprKind::NoneLit);
                    out.push(self.target(e, t, false, &mut dummy, line)?);
                    if !matches!(dummy.kind, ExprKind::NoneLit) {
                        return err(line, "unpacking needs matching element types");
                    }
                }
                Ok(Target::Tuple(out))
            }
            AExpr::Subscript(s) => {
                let base = self.expr(&s.value, None)?;
                let bt = self.norm(&base.ty);
                if matches!(*s.slice, AExpr::Slice(_)) {
                    return err(line, "slice assignment is not supported");
                }
                let e = std::mem::replace(val, self.mk(line, Ty::None, ExprKind::NoneLit));
                match &bt {
                    Ty::List(et) => {
                        let i = self.expr(&s.slice, Some(&Ty::Int))?;
                        let i = self.coerce(i, &Ty::Int, "list index")?;
                        *val = self.coerce(e, et, "list element")?;
                        let et = (**et).clone();
                        Ok(Target::Place(self.mk(line, et, ExprKind::Index(Box::new(base), Box::new(i)))))
                    }
                    Ty::Dict(kt, vt, _) => {
                        let k = self.expr(&s.slice, Some(kt))?;
                        let k = self.coerce(k, kt, "dict key")?;
                        *val = self.coerce(e, vt, "dict value")?;
                        let vt = (**vt).clone();
                        Ok(Target::Place(self.mk(line, vt, ExprKind::Index(Box::new(base), Box::new(k)))))
                    }
                    Ty::Str => err(line, "strings are immutable"),
                    Ty::Tuple(_) => err(line, "tuples are immutable"),
                    _ => err(line, format!("cannot assign into {}", self.tn(&bt))),
                }
            }
            AExpr::Attribute(a) => {
                let obj = self.expr(&a.value, None)?;
                let (Ty::Class(c) | Ty::Obj(c)) = self.norm(&obj.ty) else {
                    return err(line, "attribute assignment needs a class instance");
                };
                let name = a.attr.id.as_str();
                let f = self.cur;
                let in_init = self.p.funcs[f].is_init
                    && matches!(obj.kind, ExprKind::Var(v) if Some(v) == self.p.funcs[f].self_var);
                let e = std::mem::replace(val, self.mk(line, Ty::None, ExprKind::NoneLit));
                let fty = match self.p.classes[c].field(name) {
                    Some(fd) => fd.ty.clone(),
                    None if in_init => {
                        let t = self.norm(ty);
                        if matches!(t, Ty::None) {
                            return err(line, "cannot store None in a field");
                        }
                        self.p.classes[c].fields.push(Field { name: name.to_string(), ty: t.clone(), default: None });
                        t
                    }
                    None => {
                        return err(line, format!("`{}` has no field `{}`", self.p.classes[c].name, name))
                            .map_err(|e| e.hint("fields are declared in the class body or in __init__"))
                    }
                };
                if self.p.classes[c].frozen {
                    return err(line, format!("`{}` is frozen", self.p.classes[c].name));
                }
                *val = self.coerce(e, &fty, &format!("field `{}`", name))?;
                let pl = self.mk(line, fty, ExprKind::Field(Box::new(obj), name.to_string()));
                self.unnarrow(&pl);
                Ok(Target::Place(pl))
            }
            _ => err(line, "unsupported assignment target"),
        }
    }

    fn aug_assign(&mut self, a: &ast::StmtAugAssign, line: u32) -> Res<StmtKind> {
        let op = binop(a.op).ok_or_else(|| Error::new(line, "unsupported operator"))?;
        let target = self.expr(&a.target, None)?;
        let tt = self.norm(&target.ty);
        let rhs = self.expr(&a.value, Some(&tt))?;
        // type of `t op rhs` must be the target's type
        let probe = self.binary(op, target.clone(), rhs, line)?;
        if let ExprKind::Method(..) = probe.kind {
            // an object's operator: `t = t.__op__(rhs)` (Python rebinds the target)
            let rt = self.norm(&probe.ty);
            if rt != tt {
                return err(line, format!("`{}=` would change the type from {} to {}", opstr(op), self.tn(&tt), self.tn(&rt)));
            }
            let mut val = probe;
            let t = self.target(&a.target, &tt, false, &mut val, line)?;
            return Ok(StmtKind::Assign(t, val));
        }
        let rt = self.norm(&probe.ty);
        if rt != tt {
            return err(line, format!("`{}=` would change the type from {} to {}", opstr(op), self.tn(&tt), self.tn(&rt)));
        }
        let ExprKind::Bin(_, _, rhs) = probe.kind else {
            return err(line, "unsupported augmented assignment");
        };
        let rhs = *rhs;
        match &target.kind {
            ExprKind::Var(v) => {
                if self.var(*v).kind == VarKind::SelfParam {
                    return err(line, "cannot assign to `self`");
                }
                if matches!(tt, Ty::List(_) | Ty::Set(_) | Ty::Dict(..)) {
                    Ok(StmtKind::AugAssign(target, op, rhs))
                } else {
                    Ok(StmtKind::AugVar(*v, op, rhs))
                }
            }
            ExprKind::Index(b, _) if matches!(self.norm(&b.ty), Ty::Str | Ty::Tuple(_)) => err(line, "immutable"),
            ExprKind::Index(..) | ExprKind::Field(..) => {
                if let ExprKind::Field(o, _) = &target.kind {
                    if let Ty::Class(c) = self.norm(&o.ty) {
                        if self.p.classes[c].frozen {
                            return err(line, format!("`{}` is frozen", self.p.classes[c].name));
                        }
                    }
                }
                Ok(StmtKind::AugAssign(target, op, rhs))
            }
            ExprKind::Const(_) => err(line, "cannot assign to a constant"),
            _ => err(line, "unsupported augmented assignment target"),
        }
    }

    // ------------------------------------------------------------ expressions

    /// A condition: any value is converted by its truth value.
    fn cond(&mut self, e: &AExpr) -> Res<Expr> {
        let line = self.line(e);
        // in a condition only the truth value of `a and b` matters
        match e {
            AExpr::BoolOp(b) => {
                let mut vs = vec![];
                for v in &b.values {
                    vs.push(self.cond(v)?);
                }
                let k = if b.op == ast::BoolOp::And { ExprKind::And(vs) } else { ExprKind::Or(vs) };
                Ok(self.mk(line, Ty::Bool, k))
            }
            AExpr::UnaryOp(u) if u.op == ast::UnaryOp::Not => {
                let c = self.cond(&u.operand)?;
                Ok(self.mk(line, Ty::Bool, ExprKind::Unary(UnOp::Not, Box::new(c))))
            }
            _ => {
                let ex = self.expr(e, None)?;
                self.truth(ex)
            }
        }
    }

    fn truth(&mut self, e: Expr) -> Res<Expr> {
        let line = e.line;
        let e = match self.truth_dunder(e, line)? {
            Ok(x) => return Ok(x),
            Err(e) => e,
        };
        let t = self.norm(&e.ty);
        match t {
            Ty::Bool => Ok(e),
            Ty::Int | Ty::Float | Ty::Str | Ty::List(_) | Ty::Dict(..) | Ty::Set(_) | Ty::Deque(_) => {
                let line = e.line;
                Ok(self.mk(line, Ty::Bool, ExprKind::Builtin("truth", vec![e])))
            }
            Ty::Optional(inner) if matches!(*inner, Ty::Class(_) | Ty::Match(_) | Ty::Pattern(_)) => {
                let line = e.line;
                Ok(self.mk(line, Ty::Bool, ExprKind::IsSome(Box::new(e), true, None)))
            }
            Ty::Optional(_) => err(e.line, "the truth value of an Optional number/str also tests 0 / \"\"")
                .map_err(|e| e.hint("write `x is not None`")),
            _ => err(e.line, format!("{} has no truth value here", self.tn(&t))),
        }
    }

    fn expr(&mut self, e: &AExpr, want: Option<&Ty>) -> Res<Expr> {
        let line = self.line(e);
        use ExprKind as K;
        Ok(match e {
            AExpr::NumberLiteral(n) => match &n.value {
                ast::Number::Int(i) => match i.as_i64() {
                    Some(v) => {
                        if want.map(|w| self.norm(w)) == Some(Ty::Float) {
                            self.mk(line, Ty::Float, K::Float(v as f64))
                        } else {
                            self.mk(line, Ty::Int, K::Int(v))
                        }
                    }
                    None => return err(line, "integer literal does not fit in 64 bits")
                        .map_err(|e| e.hint("py2rs maps int to i64")),
                },
                ast::Number::Float(f) => self.mk(line, Ty::Float, K::Float(*f)),
                ast::Number::Complex { .. } => return err(line, "complex numbers are not supported"),
            },
            AExpr::BooleanLiteral(b) => self.mk(line, Ty::Bool, K::Bool(b.value)),
            AExpr::StringLiteral(s) => self.mk(line, Ty::Str, K::Str(s.value.to_str().to_string())),
            AExpr::NoneLiteral(_) => {
                let t = match want.map(|w| self.norm(w)) {
                    Some(t @ Ty::Optional(_)) => t,
                    _ => {
                        let v = self.fresh();
                        Ty::opt(v)
                    }
                };
                self.mk(line, t, K::NoneVal)
            }
            AExpr::FString(f) => self.fstring(f, line)?,
            AExpr::Name(n) => self.name(n.id.as_str(), line)?,
            AExpr::BinOp(b) => {
                let op = binop(b.op).ok_or_else(|| Error::new(line, "unsupported operator `@`"))?;
                let l = self.expr(&b.left, None)?;
                let r = self.expr(&b.right, None)?;
                self.binary(op, l, r, line)?
            }
            AExpr::UnaryOp(u) => match u.op {
                ast::UnaryOp::Not => {
                    let c = self.cond(&u.operand)?;
                    self.mk(line, Ty::Bool, K::Unary(UnOp::Not, Box::new(c)))
                }
                op => {
                    let x = self.expr(&u.operand, want)?;
                    let m = match op {
                        ast::UnaryOp::USub => "__neg__",
                        ast::UnaryOp::UAdd => "__pos__",
                        _ => "__invert__",
                    };
                    let x = match self.unary_dunder(x, m, line)? {
                        Ok(e) => return Ok(e),
                        Err(x) => x,
                    };
                    let t = self.norm(&x.ty);
                    let (uo, ok) = match op {
                        ast::UnaryOp::USub => (UnOp::Neg, t.is_num()),
                        ast::UnaryOp::UAdd => (UnOp::Pos, t.is_num()),
                        _ => (UnOp::Invert, t == Ty::Int),
                    };
                    if !ok {
                        return err(line, format!("bad operand type for unary operator: {}", self.tn(&t)));
                    }
                    // fold negative literals
                    match (uo, &x.kind) {
                        (UnOp::Neg, K::Int(v)) => self.mk(line, Ty::Int, K::Int(-v)),
                        (UnOp::Neg, K::Float(v)) => self.mk(line, Ty::Float, K::Float(-v)),
                        _ => self.mk(line, t, K::Unary(uo, Box::new(x))),
                    }
                }
            },
            AExpr::BoolOp(b) => {
                let mut vs = Vec::new();
                for v in &b.values {
                    let x = self.expr(v, None)?;
                    if self.norm(&x.ty) != Ty::Bool {
                        return err(line, format!("`{}` needs bool operands, not {}",
                            if b.op == ast::BoolOp::And { "and" } else { "or" }, self.tn(&x.ty)))
                            .map_err(|e| e.hint("Python returns an operand here; write an explicit condition or if-expression"));
                    }
                    vs.push(x);
                }
                self.mk(line, Ty::Bool, if b.op == ast::BoolOp::And { K::And(vs) } else { K::Or(vs) })
            }
            AExpr::Compare(c) => self.compare(c, line)?,
            AExpr::If(i) => {
                if let Some((place, pos, rest, cls)) = self.narrowing(&i.test) {
                    if pos || rest.is_empty() {
                        if let Some((pe, v)) = self.narrow_var(place, cls)? {
                            let depth = self.narrow.len();
                            let c = self.narrow_cond(pe, v, &rest, line)?;
                            // the narrowed branch first
                            let (yes, no) = if pos { (&i.body, &i.orelse) } else { (&i.orelse, &i.body) };
                            let a = self.expr(yes, want)?;
                            self.narrow.truncate(depth);
                            let b = self.expr(no, want)?;
                            let (a, b) = self.join(a, b, line)?;
                            let t = a.ty.clone();
                            return Ok(self.mk(line, t, K::IfExp(Box::new(c), Box::new(a), Box::new(b))));
                        }
                    }
                }
                let c = self.cond(&i.test)?;
                let a = self.expr(&i.body, want)?;
                let b = self.expr(&i.orelse, want)?;
                let (a, b) = self.join(a, b, line)?;
                let t = a.ty.clone();
                self.mk(line, t, K::IfExp(Box::new(c), Box::new(a), Box::new(b)))
            }
            AExpr::List(l) => {
                let et = match want.map(|w| self.norm(w)) {
                    Some(Ty::List(t)) => *t,
                    _ => self.fresh(),
                };
                let mut v = Vec::new();
                for x in &l.elts {
                    if let AExpr::Starred(_) = x {
                        return err(line, "`*` in list displays is not supported");
                    }
                    let ex = self.expr(x, Some(&et))?;
                    v.push(self.coerce(ex, &et, "list element")?);
                }
                self.mk(line, Ty::list(et), K::List(v))
            }
            AExpr::Set(l) => {
                let et = match want.map(|w| self.norm(w)) {
                    Some(Ty::Set(t)) => *t,
                    _ => self.fresh(),
                };
                let mut v = Vec::new();
                for x in &l.elts {
                    let ex = self.expr(x, Some(&et))?;
                    v.push(self.coerce(ex, &et, "set element")?);
                }
                self.hashable(&et, line)?;
                self.mk(line, Ty::set(et), K::Set(v))
            }
            AExpr::Dict(d) => {
                let (kt, vt) = match want.map(|w| self.norm(w)) {
                    Some(Ty::Dict(k, v, _)) => (*k, *v),
                    _ => (self.fresh(), self.fresh()),
                };
                let mut v = Vec::new();
                for it in &d.items {
                    let Some(k) = &it.key else { return err(line, "`**` in dict displays is not supported") };
                    let ke = self.expr(k, Some(&kt))?;
                    let ke = self.coerce(ke, &kt, "dict key")?;
                    let ve = self.expr(&it.value, Some(&vt))?;
                    let ve = self.coerce(ve, &vt, "dict value")?;
                    v.push((ke, ve));
                }
                self.hashable(&kt, line)?;
                self.mk(line, Ty::dict(kt, vt), K::Dict(v))
            }
            AExpr::Tuple(t) => {
                let wants: Vec<Option<Ty>> = match want.map(|w| self.norm(w)) {
                    Some(Ty::Tuple(ts)) if ts.len() == t.elts.len() => ts.into_iter().map(Some).collect(),
                    _ => vec![None; t.elts.len()],
                };
                let mut v = Vec::new();
                for (x, w) in t.elts.iter().zip(wants) {
                    let ex = self.expr(x, w.as_ref())?;
                    v.push(match &w {
                        Some(w) => self.coerce(ex, w, "tuple element")?,
                        None => ex,
                    });
                }
                if v.is_empty() {
                    return err(line, "empty tuples are not supported");
                }
                let ts = v.iter().map(|e| e.ty.clone()).collect();
                self.mk(line, Ty::Tuple(ts), K::Tuple(v))
            }
            AExpr::ListComp(c) => self.comp(CompKind::List, &c.elt, None, &c.generators, line)?,
            AExpr::SetComp(c) => self.comp(CompKind::Set, &c.elt, None, &c.generators, line)?,
            AExpr::DictComp(c) => {
                let Some(k) = &c.key else { return err(line, "unsupported dict comprehension") };
                self.comp(CompKind::Dict, k, Some(&c.value), &c.generators, line)?
            }
            AExpr::Generator(_) => {
                return err(line, "generator expressions are only supported as arguments of sum/min/max/any/all/list/set/sorted/join")
            }
            AExpr::Attribute(a) => {
                if let Some(n) = self.local_name(e) {
                    return self.expr(&n, want);
                }
                if let Some(q) = self.std_name(e) {
                    return self.std_value(&q, line);
                }
                // `C.attr`: a class attribute
                if let AExpr::Name(n) = &*a.value {
                    if self.lookup(n.id.as_str()).is_none() {
                        if let Some(&c) = self.classes.get(n.id.as_str()) {
                            return match self.class_attr(c, a.attr.id.as_str()) {
                                Some(k) => {
                                    let t = self.p.consts[k].value.ty.clone();
                                    Ok(self.mk(line, t, K::Const(k)))
                                }
                                None => err(line, format!("class `{}` has no attribute `{}`", n.id, a.attr.id)),
                            };
                        }
                    }
                }
                let obj = self.expr(&a.value, None)?;
                if let (Ty::Exc, ExprKind::Var(v)) = (self.norm(&obj.ty), &obj.kind) {
                    if let Some(&c) = self.exc_vars.get(v) {
                        let name = a.attr.id.as_str();
                        return match self.p.classes[c].field(name).filter(|_| name != "msg_") {
                            Some(f) => {
                                let t = f.ty.clone();
                                let n = self.mk(line, Ty::Str, K::Str(name.to_string()));
                                let cn = self.mk(line, Ty::Str, K::Str(self.p.classes[c].name.clone()));
                                Ok(self.mk(line, t, K::Builtin("exc.field", vec![obj, cn, n])))
                            }
                            None => err(line, format!("`{}` has no field `{}`", self.p.classes[c].name, name)),
                        };
                    }
                }
                match self.norm(&obj.ty) {
                    Ty::Class(c) | Ty::Obj(c) => {
                        let name = a.attr.id.as_str();
                        match self.p.classes[c].field(name) {
                            Some(f) => {
                                let t = f.ty.clone();
                                let fe = self.mk(line, t, K::Field(Box::new(obj), name.to_string()));
                                match self.narrowed(&fe) {
                                    Some(v) => {
                                        let t = self.var(v).ty.clone();
                                        self.mk(line, t, K::Var(v))
                                    }
                                    None => fe,
                                }
                            }
                            None => {
                                if let Some(f) = self.find_method(&obj.ty, name).filter(|f| self.p.funcs[*f].property) {
                                    return self.call_dunder(obj, f, vec![], line);
                                }
                                if let Some(k) = self.class_attr(c, name) {
                                    let t = self.p.consts[k].value.ty.clone();
                                    return Ok(self.mk(line, t, K::Const(k)));
                                }
                                return err(line, format!("`{}` has no field `{}`", self.p.classes[c].name, name));
                            }
                        }
                    }
                    t => return err(line, format!("{} has no attribute `{}`", self.tn(&t), a.attr.id)),
                }
            }
            AExpr::Subscript(s) => self.subscript(s, line)?,
            AExpr::Call(c) => self.call(c, want, line)?,
            AExpr::Lambda(_) => match want.map(|w| self.norm(w)) {
                Some(Ty::Func(ps, r)) => {
                    let l = self.lambda(e, &ps)?;
                    if !self.unify(&l.ty, &r) && *r != Ty::None {
                        return err(line, format!("the lambda returns {}, not {}", self.tn(&l.ty), self.tn(&r)));
                    }
                    let mut l = l;
                    l.ty = Ty::Func(ps, r);
                    l
                }
                _ => return err(line, "a lambda needs a known type here: pass it to a Callable parameter or annotate the variable")
                    .map_err(|e| e.hint("f: Callable[[int], int] = lambda x: x + 1")),
            },
            AExpr::Named(_) => return err(line, "`:=` is not supported"),
            AExpr::Starred(_) => return err(line, "`*` unpacking is not supported"),
            AExpr::Yield(_) | AExpr::YieldFrom(_) => return err(line, "`yield` is only supported as a statement"),
            AExpr::Await(_) => return err(line, "`await` is not supported"),
            AExpr::BytesLiteral(_) => return err(line, "bytes are not supported yet"),
            AExpr::Slice(_) => return err(line, "unexpected slice"),
            _ => return err(line, "unsupported expression"),
        })
    }

    /// The element type of an empty literal that a builtin needs now: int.
    fn settle(&mut self, t: &Ty) {
        if matches!(self.norm(t), Ty::Var(_)) {
            self.unify(t, &Ty::Int);
        }
    }

    fn hashable(&self, t: &Ty, line: u32) -> Res<()> {
        match self.norm(t) {
            Ty::List(_) | Ty::Set(_) | Ty::Dict(..) | Ty::Deque(_) => err(line, "unhashable type: lists, sets and dicts cannot be keys"),
            Ty::Float => err(line, "float keys are not supported"),
            Ty::Tuple(ts) => ts.iter().try_for_each(|t| self.hashable(t, line)),
            Ty::Class(c) => {
                let cl = &self.p.classes[c];
                if cl.dataclass && cl.frozen && cl.subs.is_empty() {
                    cl.fields.iter().try_for_each(|f| self.hashable(&f.ty, line))
                } else {
                    err(line, "only frozen dataclasses (without subclasses) can be keys")
                }
            }
            _ => Ok(()),
        }
    }

    fn name(&mut self, n: &str, line: u32) -> Res<Expr> {
        if let Some(v) = self.lookup(n) {
            let var = self.var(v);
            if var.kind == VarKind::Bound && !self.active.contains(&v) {
                return err(line, format!("loop variable `{}` is used after its loop", n))
                    .map_err(|e| e.hint("copy it into another variable inside the loop"));
            }
            let t = var.ty.clone();
            let e = self.mk(line, t, ExprKind::Var(v));
            if let Some(nv) = self.narrowed(&e) {
                let t = self.var(nv).ty.clone();
                return Ok(self.mk(line, t, ExprKind::Var(nv)));
            }
            return Ok(e);
        }
        if let Some(&c) = self.consts.get(n) {
            if c < self.p.consts.len() {
                let t = self.p.consts[c].value.ty.clone();
                return Ok(self.mk(line, t, ExprKind::Const(c)));
            }
        }
        if let Some(&fid) = self.funcs.get(n) {
            return self.func_value(fid, line);
        }
        if self.funcs.contains_key(n) || self.classes.contains_key(n) {
            return err(line, format!("`{}` cannot be used as a value", n))
                .map_err(|e| e.hint("functions and classes are not first-class values in the subset"));
        }
        if let Some(q) = self.from_names.get(n).cloned() {
            if !self.funcs.contains_key(n) && !self.classes.contains_key(n) {
                return self.std_value(&q, line);
            }
        }
        if self.module_vars.contains(n) && self.cur != self.p.main {
            return err(line, format!("module-level variable `{}` is used inside a function", n))
                .map_err(|e| e.hint("pass it as a parameter, or make it a constant (assigned once, literal value)"));
        }
        err(line, format!("name `{}` is not defined", n))
    }

    /// Unify the types of two branches (int/float promotion).
    fn join(&mut self, a: Expr, b: Expr, line: u32) -> Res<(Expr, Expr)> {
        let (ta, tb) = (self.norm(&a.ty), self.norm(&b.ty));
        if ta == Ty::Int && tb == Ty::Float {
            return Ok((self.coerce(a, &Ty::Float, "")?, b));
        }
        if ta == Ty::Float && tb == Ty::Int {
            let b = self.coerce(b, &Ty::Float, "")?;
            return Ok((a, b));
        }
        // two classes: their nearest common base class
        if let (Ty::Class(x) | Ty::Obj(x), Ty::Class(y) | Ty::Obj(y)) = (&ta, &tb) {
            let mut c = Some(*x);
            while let Some(k) = c {
                if self.p.is_sub(*y, k) {
                    let t = Ty::Class(k);
                    let a = self.coerce(a, &t, "value")?;
                    let b = self.coerce(b, &t, "value")?;
                    return Ok((a, b));
                }
                c = self.p.classes[k].base;
            }
        }
        if !self.unify(&ta, &tb) {
            return err(line, format!("branches have different types: {} and {}", self.tn(&ta), self.tn(&tb)));
        }
        Ok((a, b))
    }

    fn binary(&mut self, op: BinOp, l: Expr, r: Expr, line: u32) -> Res<Expr> {
        use BinOp::*;
        let (l, r) = match self.binary_dunder(op, l, r, line)? {
            Ok(e) => return Ok(e),
            Err(x) => x,
        };
        // an unknown element type (of an empty literal) takes the other operand's
        match (self.norm(&l.ty), self.norm(&r.ty)) {
            (Ty::Var(_), Ty::Var(_)) => {
                self.unify(&l.ty, &Ty::Int);
                self.unify(&r.ty, &Ty::Int);
            }
            (Ty::Var(_), t @ (Ty::Int | Ty::Float | Ty::Str)) | (t @ (Ty::Int | Ty::Float | Ty::Str), Ty::Var(_)) => {
                self.unify(&l.ty, &t);
                self.unify(&r.ty, &t);
            }
            _ => {}
        }
        let (lt, rt) = (self.norm(&l.ty), self.norm(&r.ty));
        let bad = |s: &Self| {
            err(line, format!("unsupported operand types for {}: {} and {}", opstr(op), s.tn(&lt), s.tn(&rt)))
        };
        let ty = match (&lt, &rt) {
            (Ty::Int, Ty::Int) => match op {
                Div => Ty::Float,
                Pow => {
                    if let ExprKind::Int(v) = r.kind {
                        if v < 0 {
                            return err(line, "int ** negative int gives a float; write a float base");
                        }
                    }
                    Ty::Int
                }
                _ => Ty::Int,
            },
            (Ty::Int | Ty::Float, Ty::Int | Ty::Float) => {
                if matches!(op, Shl | Shr | BitAnd | BitOr | BitXor) {
                    return bad(self);
                }
                let l = self.coerce(l, &Ty::Float, "")?;
                let r = self.coerce(r, &Ty::Float, "")?;
                return Ok(self.mk(line, Ty::Float, ExprKind::Bin(op, Box::new(l), Box::new(r))));
            }
            (Ty::Bool, Ty::Bool) if matches!(op, BitAnd | BitOr | BitXor) => Ty::Bool,
            (Ty::Str, Ty::Str) if op == Add => Ty::Str,
            (Ty::Str, Ty::Int) if op == Mul => Ty::Str,
            (Ty::Int, Ty::Str) if op == Mul => {
                return Ok(self.mk(line, Ty::Str, ExprKind::Bin(op, Box::new(r), Box::new(l))));
            }
            (Ty::List(_), Ty::List(_)) if op == Add => {
                if !self.unify(&lt, &rt) {
                    return bad(self);
                }
                lt.clone()
            }
            (Ty::List(_), Ty::Int) if op == Mul => lt.clone(),
            (Ty::Set(_), Ty::Set(_)) if matches!(op, BitOr | BitAnd | Sub | BitXor) => {
                if !self.unify(&lt, &rt) {
                    return bad(self);
                }
                lt.clone()
            }
            _ => return bad(self),
        };
        Ok(self.mk(line, ty, ExprKind::Bin(op, Box::new(l), Box::new(r))))
    }

    fn compare(&mut self, c: &ast::ExprCompare, line: u32) -> Res<Expr> {
        // `x is None`, `x is not None` (also with == / !=)
        if c.ops.len() == 1 {
            let none = |e: &AExpr| matches!(e, AExpr::NoneLiteral(_));
            let (a, b) = (&c.operands[0], &c.operands[1]);
            if none(a) || none(b) {
                let x = if none(a) { b } else { a };
                let some = match c.ops[0] {
                    ast::CmpOp::Is | ast::CmpOp::Eq => false,
                    ast::CmpOp::IsNot | ast::CmpOp::NotEq => true,
                    _ => return err(line, "None can only be compared with `is` / `is not`"),
                };
                let e = self.expr(x, None)?;
                if !matches!(self.norm(&e.ty), Ty::Optional(_)) {
                    return err(line, format!("this is {}, never None", self.tn(&e.ty)));
                }
                return Ok(self.mk(line, Ty::Bool, ExprKind::IsSome(Box::new(e), some, None)));
            }
        }
        let first = self.expr(&c.operands[0], None)?;
        // `a is b` for @shared objects: the same object
        if c.ops.len() == 1 && matches!(c.ops[0], ast::CmpOp::Is | ast::CmpOp::IsNot) {
            let b = self.expr(&c.operands[1], None)?;
            let shared = |s: &Self, t: &Ty| matches!(s.norm(t), Ty::Class(k) if s.p.classes[k].shared);
            if !shared(self, &first.ty) || !shared(self, &b.ty) {
                return err(line, "`is` compares @shared objects (or a value with None)")
                    .map_err(|e| e.hint("use =="));
            }
            let e = self.mk(line, Ty::Bool, ExprKind::Builtin("is", vec![first, b]));
            return Ok(if c.ops[0] == ast::CmpOp::IsNot { self.mk(line, Ty::Bool, ExprKind::Unary(UnOp::Not, Box::new(e))) } else { e });
        }
        if c.ops.len() == 1 && (self.is_obj(&first.ty) || matches!(c.ops[0], ast::CmpOp::In | ast::CmpOp::NotIn)) {
            let b = self.expr(&c.operands[1], None)?;
            let op = match c.ops[0] {
                ast::CmpOp::Eq => Some(CmpOp::Eq),
                ast::CmpOp::NotEq => Some(CmpOp::Ne),
                ast::CmpOp::Lt => Some(CmpOp::Lt),
                ast::CmpOp::LtE => Some(CmpOp::Le),
                ast::CmpOp::Gt => Some(CmpOp::Gt),
                ast::CmpOp::GtE => Some(CmpOp::Ge),
                ast::CmpOp::In => Some(CmpOp::In),
                ast::CmpOp::NotIn => Some(CmpOp::NotIn),
                _ => None,
            };
            if let Some(op) = op {
                if self.is_obj(&first.ty) || self.is_obj(&b.ty) {
                    if let Ok(e) = self.compare_dunder(op, first.clone(), b, line)? {
                        return Ok(e);
                    }
                }
            }
        }
        let mut prev_ty = self.norm(&first.ty);
        let mut first = first;
        let mut rest = Vec::new();
        for (op, rhs) in c.ops.iter().zip(c.operands[1..].iter()) {
            let op = match op {
                ast::CmpOp::Eq => CmpOp::Eq,
                ast::CmpOp::NotEq => CmpOp::Ne,
                ast::CmpOp::Lt => CmpOp::Lt,
                ast::CmpOp::LtE => CmpOp::Le,
                ast::CmpOp::Gt => CmpOp::Gt,
                ast::CmpOp::GtE => CmpOp::Ge,
                ast::CmpOp::In => CmpOp::In,
                ast::CmpOp::NotIn => CmpOp::NotIn,
                ast::CmpOp::Is | ast::CmpOp::IsNot => {
                    return err(line, "`is` is only supported with None (no object identity)")
                        .map_err(|e| e.hint("use `==` to compare values"))
                }
            };
            let r = self.expr(rhs, Some(&prev_ty))?;
            let rt = self.norm(&r.ty);
            let r = match op {
                CmpOp::In | CmpOp::NotIn => {
                    let elem = match &rt {
                        Ty::List(t) | Ty::Set(t) | Ty::Dict(t, _, _) | Ty::Deque(t) => (**t).clone(),
                        Ty::Str => Ty::Str,
                        Ty::Tuple(ts) if ts.windows(2).all(|w| w[0] == w[1]) => ts[0].clone(),
                        _ => return err(line, format!("`in` needs a list, set, dict or str, not {}", self.tn(&rt))),
                    };
                    if rest.is_empty() {
                        first = self.coerce(first, &elem, "`in` operand")?;
                    } else if self.norm(&prev_ty) != self.norm(&elem) {
                        return err(line, "`in` operand has the wrong type");
                    }
                    r
                }
                _ => {
                    let lt = prev_ty.clone();
                    if lt.is_num() && rt.is_num() {
                        if lt != rt {
                            if rest.is_empty() && lt == Ty::Int {
                                first = self.coerce(first, &Ty::Float, "")?;
                            }
                            self.coerce(r, &Ty::Float, "")?
                        } else {
                            r
                        }
                    } else {
                        // `x == y` with x: T | None and y: T compares as Optionals
                        if matches!(op, CmpOp::Eq | CmpOp::Ne) && !self.unify(&lt, &rt) {
                            if let Ty::Optional(inner) = &lt {
                                if self.unify(inner, &rt) {
                                    let r = self.coerce(r, &lt, "operand")?;
                                    prev_ty = lt.clone();
                                    rest.push((op, r));
                                    continue;
                                }
                            }
                            if let Ty::Optional(inner) = &rt {
                                if self.unify(inner, &lt) && rest.is_empty() {
                                    first = self.coerce(first, &rt, "operand")?;
                                    prev_ty = rt.clone();
                                    rest.push((op, r));
                                    continue;
                                }
                            }
                        }
                        if !self.unify(&lt, &rt) {
                            return err(line, format!("cannot compare {} with {}", self.tn(&lt), self.tn(&rt)));
                        }
                        if !matches!(op, CmpOp::Eq | CmpOp::Ne) {
                            self.orderable(&lt, line)?;
                        } else if let Ty::Class(c) = self.norm(&lt) {
                            if !self.p.classes[c].dataclass {
                                return err(line, "`==` on a plain class compares identity; not supported")
                                    .map_err(|e| e.hint("use a @dataclass"));
                            }
                        }
                        r
                    }
                }
            };
            prev_ty = self.norm(&r.ty);
            rest.push((op, r));
        }
        Ok(self.mk(line, Ty::Bool, ExprKind::Cmp(Box::new(first), rest)))
    }

    fn orderable(&self, t: &Ty, line: u32) -> Res<()> {
        match self.norm(t) {
            Ty::Int | Ty::Float | Ty::Str | Ty::Bool => Ok(()),
            Ty::Class(c) if self.p.classes[c].method("__lt__").is_some() => Ok(()),
            Ty::List(e) => self.orderable(&e, line),
            Ty::Tuple(ts) => ts.iter().try_for_each(|t| self.orderable(t, line)),
            // elements of an empty literal: settled (as int) at the end
            Ty::Var(_) => Ok(()),
            t => err(line, format!("{} values cannot be ordered", self.tn(&t))),
        }
    }

    fn subscript(&mut self, s: &ast::ExprSubscript, line: u32) -> Res<Expr> {
        let base = self.expr(&s.value, None)?;
        if let Some(f) = self.find_method(&base.ty, "__getitem__") {
            let pt = self.p.funcs[f].params.first().map(|p| self.p.funcs[f].vars[p.var].ty.clone());
            let k = self.expr(&s.slice, pt.as_ref())?;
            return self.call_dunder(base, f, vec![k], line);
        }
        let bt = self.norm(&base.ty);
        if let AExpr::Slice(sl) = &*s.slice {
            if !matches!(bt, Ty::List(_) | Ty::Str) {
                return err(line, format!("cannot slice {}", self.tn(&bt)));
            }
            let part = |x: &Option<Box<AExpr>>, f: &mut Self| -> Res<Option<Box<Expr>>> {
                match x {
                    Some(e) => {
                        let ex = f.expr(e, Some(&Ty::Int))?;
                        Ok(Some(Box::new(f.coerce(ex, &Ty::Int, "slice index")?)))
                    }
                    None => Ok(None),
                }
            };
            let lo = part(&sl.lower, self)?;
            let hi = part(&sl.upper, self)?;
            let st = part(&sl.step, self)?;
            if let Some(s) = &st {
                if matches!(s.kind, ExprKind::Int(0)) {
                    return err(line, "slice step cannot be zero");
                }
            }
            return Ok(self.mk(line, bt, ExprKind::Slice(Box::new(base), lo, hi, st)));
        }
        match &bt {
            Ty::List(et) | Ty::Deque(et) => {
                let i = self.expr(&s.slice, Some(&Ty::Int))?;
                let i = self.coerce(i, &Ty::Int, "list index")?;
                let et = (**et).clone();
                Ok(self.mk(line, et, ExprKind::Index(Box::new(base), Box::new(i))))
            }
            Ty::Str => {
                let i = self.expr(&s.slice, Some(&Ty::Int))?;
                let i = self.coerce(i, &Ty::Int, "string index")?;
                Ok(self.mk(line, Ty::Str, ExprKind::Index(Box::new(base), Box::new(i))))
            }
            Ty::Dict(kt, vt, dk) => {
                let k = self.expr(&s.slice, Some(kt))?;
                let k = self.coerce(k, kt, "dict key")?;
                let vt = (**vt).clone();
                let present = match (&base.kind, &k.kind) {
                    (ExprKind::Var(d), ExprKind::Var(kv)) => self.loop_keys.get(kv) == Some(d),
                    _ => false,
                };
                let e = self.mk(line, vt, ExprKind::Index(Box::new(base), Box::new(k)));
                if present && *dk == DK::Default {
                    self.p.present.insert(e.id);
                }
                Ok(e)
            }
            Ty::Tuple(ts) => {
                let i = self.expr(&s.slice, None)?;
                let ExprKind::Int(v) = i.kind else {
                    return err(line, "tuple index must be an integer literal");
                };
                let n = ts.len() as i64;
                let k = if v < 0 { v + n } else { v };
                if k < 0 || k >= n {
                    return err(line, "tuple index out of range");
                }
                let t = ts[k as usize].clone();
                Ok(self.mk(line, t, ExprKind::TupleField(Box::new(base), k as usize)))
            }
            Ty::Var(_) => err(line, "the element type is not known yet here; add a type annotation"),
            _ => err(line, format!("{} is not subscriptable", self.tn(&bt))),
        }
    }

    fn fstring(&mut self, f: &ast::ExprFString, line: u32) -> Res<Expr> {
        let mut parts = Vec::new();
        for p in f.value.iter() {
            match p {
                ast::FStringPartRef::Literal(s) => parts.push(FPart::Lit(s.value.to_string())),
                ast::FStringPartRef::FString(fs) => {
                    for el in fs.elements.iter() {
                        match el {
                            ast::InterpolatedStringElement::Literal(l) => parts.push(FPart::Lit(l.value.to_string())),
                            ast::InterpolatedStringElement::Interpolation(i) => {
                                if i.debug_text.is_some() {
                                    return err(line, "`{x=}` in f-strings is not supported");
                                }
                                let e = self.expr(&i.expression, None)?;
                                let conv = match i.conversion {
                                    ast::ConversionFlag::Repr => Some('r'),
                                    ast::ConversionFlag::Str | ast::ConversionFlag::None => None,
                                    ast::ConversionFlag::Ascii => return err(line, "`!a` is not supported"),
                                };
                                let mut spec = String::new();
                                if let Some(s) = &i.format_spec {
                                    for se in s.elements.iter() {
                                        match se {
                                            ast::InterpolatedStringElement::Literal(l) => spec += &l.value,
                                            _ => return err(line, "nested fields in format specs are not supported"),
                                        }
                                    }
                                }
                                if !spec.is_empty() {
                                    self.settle(&e.ty);
                                }
                                let t = if conv.is_some() || i.conversion == ast::ConversionFlag::Str { Ty::Str } else { self.norm(&e.ty) };
                                let conv = if i.conversion == ast::ConversionFlag::Str && !spec.is_empty() { Some('s') } else { conv };
                                if !spec.is_empty() {
                                    check_spec(&spec, &t).map_err(|m| Error::new(line, m))?;
                                }
                                parts.push(FPart::Val(e, conv, spec));
                            }
                        }
                    }
                }
            }
        }
        Ok(self.mk(line, Ty::Str, ExprKind::FStr(parts)))
    }

    // -------------------------------------------------------- comprehensions

    /// An iterable in `for` / comprehension / builtin argument position.
    fn iter(&mut self, e: &AExpr) -> Res<(Iter, Ty)> {
        let line = self.line(e);
        if let AExpr::Generator(g) = e {
            let c = self.comp(CompKind::Gen, &g.elt, None, &g.generators, line)?;
            let t = c.ty.clone();
            return Ok((Iter::Gen(c), t));
        }
        if let AExpr::Call(c) = e {
            let args = &c.arguments;
            if let AExpr::Name(n) = &*c.func {
                let builtin = self.lookup(n.id.as_str()).is_none() && !self.funcs.contains_key(n.id.as_str());
                if builtin && !args.keywords.iter().any(|k| k.arg.is_none()) {
                    match n.id.as_str() {
                        "range" => {
                            if !args.keywords.is_empty() || args.args.is_empty() || args.args.len() > 3 {
                                return err(line, "range() takes 1 to 3 arguments");
                            }
                            let mut v = Vec::new();
                            for a in args.args.iter() {
                                let x = self.expr(a, Some(&Ty::Int))?;
                                v.push(self.coerce(x, &Ty::Int, "range() argument")?);
                            }
                            if let Some(Expr { kind: ExprKind::Int(0), .. }) = v.get(2) {
                                return err(line, "range() step must not be zero");
                            }
                            let it = match v.len() {
                                1 => {
                                    let zero = self.mk(line, Ty::Int, ExprKind::Int(0));
                                    Iter::Range(zero, v.pop().unwrap(), None)
                                }
                                2 => {
                                    let b = v.pop().unwrap();
                                    Iter::Range(v.pop().unwrap(), b, None)
                                }
                                _ => {
                                    let s = v.pop().unwrap();
                                    let b = v.pop().unwrap();
                                    Iter::Range(v.pop().unwrap(), b, Some(s))
                                }
                            };
                            return Ok((it, Ty::Int));
                        }
                        "enumerate" => {
                            if args.args.is_empty() || args.args.len() > 2 {
                                return err(line, "enumerate() takes 1 or 2 arguments");
                            }
                            let (inner, t) = self.iter(&args.args[0])?;
                            let mut start = match args.args.get(1) {
                                Some(s) => Some(self.expr(s, Some(&Ty::Int))?),
                                None => None,
                            };
                            for k in args.keywords.iter() {
                                if k.arg.as_ref().map(|a| a.id.as_str()) == Some("start") {
                                    start = Some(self.expr(&k.value, Some(&Ty::Int))?);
                                } else {
                                    return err(line, "unexpected keyword argument");
                                }
                            }
                            if let Some(s) = start.take() {
                                start = Some(self.coerce(s, &Ty::Int, "enumerate start")?);
                            }
                            return Ok((Iter::Enumerate(Box::new(inner), start), Ty::Tuple(vec![Ty::Int, t])));
                        }
                        "zip" => {
                            if args.args.len() < 2 || !args.keywords.is_empty() {
                                return err(line, "zip() needs at least two arguments (no strict=)");
                            }
                            let mut its = Vec::new();
                            let mut ts = Vec::new();
                            for a in args.args.iter() {
                                let (i, t) = self.iter(a)?;
                                its.push(i);
                                ts.push(t);
                            }
                            return Ok((Iter::Zip(its), Ty::Tuple(ts)));
                        }
                        "reversed" => {
                            if args.args.len() != 1 || !args.keywords.is_empty() {
                                return err(line, "reversed() takes one argument");
                            }
                            let (inner, t) = self.iter(&args.args[0])?;
                            match &inner {
                                Iter::Range(..) => {}
                                Iter::Over(x) if matches!(self.norm(&x.ty), Ty::List(_) | Ty::Str) => {}
                                
                                _ => return err(line, "reversed() needs a list, str, range or enumerate"),
                            }
                            return Ok((Iter::Reversed(Box::new(inner)), t));
                        }
                        _ => {}
                    }
                }
            }
            if let AExpr::Attribute(a) = &*c.func {
                let m = a.attr.id.as_str();
                if matches!(m, "items" | "keys" | "values") && args.args.is_empty() && args.keywords.is_empty() {
                    let d = self.expr(&a.value, None)?;
                    if let Ty::Dict(k, v, _) = self.norm(&d.ty) {
                        return Ok(match m {
                            "items" => (Iter::Items(d), Ty::Tuple(vec![*k, *v])),
                            "keys" => (Iter::Keys(d), *k),
                            _ => (Iter::Values(d), *v),
                        });
                    }
                }
            }
        }
        if self.std_name(e).as_deref() == Some("sys.stdin") {
            let x = self.mk(line, Ty::list(Ty::Str), ExprKind::Builtin("sys.stdin.readlines", vec![]));
            return Ok((Iter::Over(x), Ty::Str));
        }
        let x = self.expr(e, None)?;
        if self.norm(&x.ty) == Ty::File {
            let l = self.mk(line, Ty::list(Ty::Str), ExprKind::Builtin("file.readlines", vec![x]));
            return Ok((Iter::Over(l), Ty::Str));
        }
        let t = match self.norm(&x.ty) {
            Ty::List(t) | Ty::Set(t) | Ty::Deque(t) | Ty::Gen(t) => *t,
            Ty::Dict(k, _, _) => *k,
            Ty::Str => Ty::Str,
            Ty::Tuple(ts) => {
                if ts.windows(2).all(|w| w[0] == w[1]) {
                    return err(line, "iterate over a list instead of a tuple");
                }
                return err(line, "cannot iterate over a tuple with mixed element types");
            }
            Ty::Var(_) => return err(line, "the type is not known yet here; add a type annotation"),
            t => return err(line, format!("{} is not iterable", self.tn(&t))),
        };
        Ok((Iter::Over(x), t))
    }

    /// Bind a for / comprehension target to fresh loop variables.
    fn bind_target(&mut self, t: &AExpr, ty: &Ty) -> Res<Target> {
        let line = self.line(t);
        match t {
            AExpr::Name(n) => {
                let name = n.id.as_str();
                let mut shadow = None;
                if let Some(v) = self.scopes[0].get(name).copied() {
                    if self.var(v).kind != VarKind::Bound {
                        if matches!(self.var(v).kind, VarKind::Param | VarKind::SelfParam) {
                            return err(line, format!("loop variable `{}` is also a parameter", name))
                                .map_err(|e| e.hint("use a different name for the loop variable"));
                        }
                        shadow = Some(v);
                    } else if self.active.contains(&v) {
                        return err(line, format!("`{}` is already the variable of an enclosing loop", name));
                    }
                }
                let b = self.new_var(name, ty.clone(), VarKind::Bound);
                if let Some(l) = shadow {
                    self.p.funcs[self.cur].shadows.push((b, l));
                }
                Ok(Target::Var(b))
            }
            AExpr::Tuple(_) | AExpr::List(_) => {
                let elts = match t {
                    AExpr::Tuple(t) => &t.elts,
                    AExpr::List(l) => &l.elts,
                    _ => unreachable!(),
                };
                let Ty::Tuple(ts) = self.norm(ty) else {
                    return err(line, "only tuples can be unpacked");
                };
                if ts.len() != elts.len() {
                    return err(line, format!("cannot unpack {} values into {}", ts.len(), elts.len()));
                }
                let mut out = Vec::new();
                for (e, t) in elts.iter().zip(ts.iter()) {
                    out.push(self.bind_target(e, t)?);
                }
                Ok(Target::Tuple(out))
            }
            _ => err(line, "loop targets must be names or tuples of names"),
        }
    }

    fn comp(
        &mut self,
        kind: CompKind,
        elt: &AExpr,
        val: Option<&AExpr>,
        gens: &[ast::Comprehension],
        line: u32,
    ) -> Res<Expr> {
        self.scopes.push(HashMap::new());
        let mut cgens = Vec::new();
        let mut bound = Vec::new();
        for g in gens {
            if g.is_async {
                return err(line, "async comprehensions are not supported");
            }
            let (iter, item) = self.iter(&g.iter)?;
            self.scopes.push(HashMap::new());
            let target = self.bind_comp_target(&g.target, &item)?;
            let mut vars = vec![];
            target.vars(&mut vars);
            self.active.extend(vars.iter().copied());
            bound.extend(vars);
            let mut conds = Vec::new();
            for c in &g.ifs {
                let cl = self.line(c);
                if let Some((place, true, rest, cls)) = self.narrowing(c) {
                    if let Some((pe, v)) = self.narrow_var(place, cls)? {
                        conds.push(self.narrow_cond(pe, v, &rest, cl)?);
                        continue;
                    }
                }
                conds.push(self.cond(c)?);
            }
            cgens.push(CompFor { target, iter, conds });
        }
        let depth0 = self.narrow.len();
        let e = self.expr(elt, None)?;
        let v = match val {
            Some(v) => Some(self.expr(v, None)?),
            None => None,
        };
        let _ = depth0;
        for _ in gens {
            self.scopes.pop();
        }
        self.scopes.pop();
        for b in bound {
            self.active.remove(&b);
        }
        let ty = match kind {
            CompKind::List => Ty::list(e.ty.clone()),
            CompKind::Set => {
                self.hashable(&e.ty, line)?;
                Ty::set(e.ty.clone())
            }
            CompKind::Dict => {
                self.hashable(&e.ty, line)?;
                Ty::dict(e.ty.clone(), v.as_ref().unwrap().ty.clone())
            }
            CompKind::Gen => e.ty.clone(),
        };
        Ok(self.mk(line, ty, ExprKind::Comp(Box::new(Comp { kind, elt: e, val: v, gens: cgens }))))
    }

    /// Comprehension variables live in their own scope (Python 3).
    fn bind_comp_target(&mut self, t: &AExpr, ty: &Ty) -> Res<Target> {
        let line = self.line(t);
        match t {
            AExpr::Name(n) => {
                let vars = &mut self.p.funcs[self.cur].vars;
                vars.push(Var { name: n.id.to_string(), ty: ty.clone(), kind: VarKind::Bound });
                let id = vars.len() - 1;
                self.scopes.last_mut().unwrap().insert(n.id.to_string(), id);
                Ok(Target::Var(id))
            }
            AExpr::Tuple(_) | AExpr::List(_) => {
                let elts = match t {
                    AExpr::Tuple(t) => &t.elts,
                    AExpr::List(l) => &l.elts,
                    _ => unreachable!(),
                };
                let Ty::Tuple(ts) = self.norm(ty) else { return err(line, "only tuples can be unpacked") };
                if ts.len() != elts.len() {
                    return err(line, format!("cannot unpack {} values into {}", ts.len(), elts.len()));
                }
                let mut out = Vec::new();
                for (e, t) in elts.iter().zip(ts.iter()) {
                    out.push(self.bind_comp_target(e, t)?);
                }
                Ok(Target::Tuple(out))
            }
            _ => err(line, "comprehension targets must be names or tuples of names"),
        }
    }

    fn lambda(&mut self, e: &AExpr, params: &[Ty]) -> Res<Expr> {
        let line = self.line(e);
        // key=len, key=abs, key=str.lower, ...: a lambda calling it
        let builtin = match e {
            AExpr::Name(n) if self.lookup(n.id.as_str()).is_none() && ["len", "abs", "str", "int", "float"].contains(&n.id.as_str()) => Some((None, n.id.to_string())),
            AExpr::Attribute(a) if matches!(&*a.value, AExpr::Name(n) if n.id.as_str() == "str") => Some((Some("str"), a.attr.id.to_string())),
            _ => None,
        };
        if let (Some((recv, f)), [t]) = (builtin, params) {
            self.scopes.push(HashMap::new());
            let vars = &mut self.p.funcs[self.cur].vars;
            vars.push(Var { name: "x".into(), ty: t.clone(), kind: VarKind::Bound });
            let v = vars.len() - 1;
            self.scopes.last_mut().unwrap().insert("\u{0}x".into(), v);
            self.active.insert(v);
            let xe = self.mk(line, t.clone(), ExprKind::Var(v));
            let body = match recv {
                None => {
                    let fake = ast::Arguments { range: Default::default(), node_index: Default::default(), args: Default::default(), keywords: Default::default() };
                    let _ = fake;
                    match f.as_str() {
                        "len" => self.mk(line, Ty::Int, ExprKind::Builtin("len", vec![xe])),
                        "abs" => {
                            let t2 = self.norm(t);
                            self.mk(line, t2, ExprKind::Builtin("abs", vec![xe]))
                        }
                        "str" => self.mk(line, Ty::Str, ExprKind::Builtin("str", vec![xe])),
                        "float" => self.coerce(xe, &Ty::Float, "key")?,
                        _ => xe,
                    }
                }
                Some(_) => {
                    if self.norm(t) != Ty::Str {
                        return err(line, "str methods as key= need str items");
                    }
                    let n: &'static str = match f.as_str() {
                        "lower" => "str.lower",
                        "upper" => "str.upper",
                        "casefold" => "str.casefold",
                        "strip" => "str.strip",
                        _ => return err(line, format!("key=str.{} is not supported", f)),
                    };
                    self.mk(line, Ty::Str, ExprKind::Builtin(n, vec![xe]))
                }
            };
            self.scopes.pop();
            self.active.remove(&v);
            let bt = body.ty.clone();
            return Ok(self.mk(line, bt, ExprKind::Lambda(vec![v], Box::new(body))));
        }
        let AExpr::Lambda(l) = e else { return err(line, "key= needs a lambda or a function like len") };
        let names: Vec<&str> = match &l.parameters {
            Some(p) => {
                if p.vararg.is_some() || p.kwarg.is_some() || !p.kwonlyargs.is_empty() {
                    return err(line, "unsupported lambda parameters");
                }
                p.posonlyargs.iter().chain(p.args.iter()).map(|a| a.parameter.name.id.as_str()).collect()
            }
            None => vec![],
        };
        if names.len() != params.len() {
            return err(line, format!("the lambda must take {} argument(s)", params.len()));
        }
        self.scopes.push(HashMap::new());
        let mut vs = Vec::new();
        for (n, t) in names.iter().zip(params) {
            let vars = &mut self.p.funcs[self.cur].vars;
            vars.push(Var { name: n.to_string(), ty: t.clone(), kind: VarKind::Bound });
            let id = vars.len() - 1;
            self.scopes.last_mut().unwrap().insert(n.to_string(), id);
            self.active.insert(id);
            vs.push(id);
        }
        let body = self.expr(&l.body, None)?;
        self.scopes.pop();
        for v in &vs {
            self.active.remove(v);
        }
        let t = body.ty.clone();
        Ok(self.mk(line, t, ExprKind::Lambda(vs, Box::new(body))))
    }

    // ------------------------------------------------------------------ calls

    fn call(&mut self, c: &ast::ExprCall, want: Option<&Ty>, line: u32) -> Res<Expr> {
        let args = &c.arguments;
        // inline("rust code") / inline[T]("rust expression"): raw Rust
        let inline_ty = match &*c.func {
            AExpr::Name(n) if n.id.as_str() == "inline" && self.imports.contains("inline") => Some(Ty::None),
            AExpr::Subscript(sub) if matches!(&*sub.value, AExpr::Name(n) if n.id.as_str() == "inline" && self.imports.contains("inline")) => {
                Some(self.ann(&sub.slice)?)
            }
            _ => None,
        };
        if let Some(t) = inline_ty {
            let [AExpr::StringLiteral(code)] = &args.args[..] else {
                return err(line, "inline() takes one string of Rust code");
            };
            let code = code.value.to_str().to_string();
            // the variables it names count as used (and possibly modified)
            let mut v = vec![self.mk(line, Ty::Str, ExprKind::Str(code.clone()))];
            let words: Vec<String> = code
                .split(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
                .filter(|w| !w.is_empty())
                .map(String::from)
                .collect();
            let mut seen = HashSet::new();
            for w in words {
                if let Some(var) = self.lookup(&w) {
                    if seen.insert(var) {
                        let ty = self.var(var).ty.clone();
                        v.push(self.mk(line, ty, ExprKind::Var(var)));
                    }
                }
            }
            let name: &'static str = if t == Ty::None { "inline" } else { "inline_expr" };
            return Ok(self.mk(line, t, ExprKind::Builtin(name, v)));
        }
        if args.args.iter().any(|a| matches!(a, AExpr::Starred(_))) || args.keywords.iter().any(|k| k.arg.is_none()) {
            return err(line, "`*args` / `**kwargs` in calls are not supported");
        }
        match &*c.func {
            AExpr::Name(n) => {
                let name = n.id.as_str();
                if let Some(v) = self.lookup(name) {
                    if let Ty::Func(ps, r) = self.norm(&self.var(v).ty) {
                        return self.call_value(v, &ps, &r, args, line);
                    }
                    return err(line, format!("`{}` is not callable", name));
                }
                if let Some(&fid) = self.funcs.get(name) {
                    let a = self.call_args(fid, args, false, line)?;
                    let t = self.p.funcs[fid].ret.clone();
                    return Ok(self.mk(line, t, ExprKind::Call(fid, a)));
                }
                if let Some(&cid) = self.classes.get(name) {
                    return self.construct(cid, args, line);
                }
                if self.templates.contains_key(name) {
                    return self.instantiate(name, args, line);
                }
                if let Some(q) = self.std_name(&c.func) {
                    return self.std_call(&q, args, want, line);
                }
                self.builtin(name, args, want, line)
            }
            AExpr::Attribute(a) if matches!(&*a.value, AExpr::Call(sc) if matches!(&*sc.func, AExpr::Name(n) if n.id.as_str() == "super")) => {
                let m = a.attr.id.as_str();
                let f = self.cur;
                let (Some(c), Some(sv)) = (self.p.funcs[f].class, self.p.funcs[f].self_var) else {
                    return err(line, "super() outside a method");
                };
                // relative to the class the method is written in
                let def = self.p.funcs[f].def_class.unwrap_or(c);
                let Some(b) = self.p.classes[def].base else { return err(line, "super() in a class without a base class") };
                if m == "__init__" {
                    if !self.p.funcs[f].is_init {
                        return err(line, "super().__init__() only inside __init__");
                    }
                    let Some(binit) = self.p.classes[b].init else {
                        return err(line, format!("`{}` has no __init__", self.p.classes[b].name));
                    };
                    let a = self.call_args(binit, args, false, line)?;
                    return Ok(self.mk(line, Ty::None, ExprKind::SuperInit(binit, a)));
                }
                let Some(fid) = self.p.classes[c].method(&format!("{}_{}", snake_name(&self.p.classes[b].name), m)) else {
                    return err(line, format!("`{}` has no method `{}`", self.p.classes[b].name, m));
                };
                if self.p.funcs[f].is_init {
                    return err(line, "methods cannot be called on `self` inside __init__");
                }
                let a = self.call_args(fid, args, true, line)?;
                let t = self.p.funcs[fid].ret.clone();
                let st = self.var(sv).ty.clone();
                let recv = self.mk(line, st, ExprKind::Var(sv));
                Ok(self.mk(line, t, ExprKind::Method(Box::new(recv), fid, a)))
            }
            AExpr::Attribute(a) if self.local_name(&c.func).is_some() => {
                let mut c2 = c.clone();
                c2.func = Box::new(self.local_name(&c.func).unwrap());
                self.call(&c2, want, line)
            }
            AExpr::Attribute(a) => {
                let m = a.attr.id.as_str();
                if let Some(q) = self.std_name(&c.func) {
                    return self.std_call(&q, args, want, line);
                }
                if let AExpr::Name(n) = &*a.value {
                    if self.lookup(n.id.as_str()).is_none() {
                        if let Some(&cid) = self.classes.get(n.id.as_str()) {
                            let Some(fid) = self.p.classes[cid].method(m) else {
                                return err(line, format!("`{}` has no method `{}`", n.id, m));
                            };
                            if !self.p.funcs[fid].is_static {
                                return err(line, "call methods on an instance");
                            }
                            let a = self.call_args(fid, args, false, line)?;
                            let t = self.p.funcs[fid].ret.clone();
                            return Ok(self.mk(line, t, ExprKind::Call(fid, a)));
                        }
                    }
                    if n.id.as_str() == "str" || n.id.as_str() == "dict" || n.id.as_str() == "list" {
                        if self.lookup(n.id.as_str()).is_none() {
                            return err(line, format!("`{}.{}` is not supported", n.id, m));
                        }
                    }
                }
                let recv = self.expr(&a.value, None)?;
                self.method(recv, m, args, want, line)
            }
            _ => err(line, "unsupported call"),
        }
    }

    fn construct(&mut self, cid: ClassId, args: &ast::Arguments, line: u32) -> Res<Expr> {
        if self.p.classes[cid].exc.is_some() {
            return err(line, "exceptions can only be raised");
        }
        let e = self.construct_(cid, args, line)?;
        if self.p.classes[cid].subs.is_empty() {
            return Ok(e);
        }
        // an object of a class with subclasses: exactly this class, as the enum
        let mut e = e;
        e.ty = Ty::Obj(cid);
        Ok(self.mk(line, Ty::Class(cid), ExprKind::Upcast(Box::new(e))))
    }

    fn construct_(&mut self, cid: ClassId, args: &ast::Arguments, line: u32) -> Res<Expr> {
        if let Some(init) = self.p.classes[cid].init {
            let a = self.call_args(init, args, false, line)?;
            return Ok(self.mk(line, Ty::Class(cid), ExprKind::New(cid, a)));
        }
        if !self.p.classes[cid].dataclass {
            if args.args.is_empty() && args.keywords.is_empty() && self.p.classes[cid].fields.is_empty() {
                return Ok(self.mk(line, Ty::Class(cid), ExprKind::New(cid, vec![])));
            }
            return err(line, format!("`{}` has no __init__", self.p.classes[cid].name));
        }
        let fields: Vec<(String, Ty, Option<Expr>)> = self.p.classes[cid]
            .fields
            .iter()
            .map(|f| (f.name.clone(), f.ty.clone(), f.default.clone()))
            .collect();
        let mut slots: Vec<Option<Expr>> = vec![None; fields.len()];
        if args.args.len() > fields.len() {
            return err(line, "too many arguments");
        }
        for (i, a) in args.args.iter().enumerate() {
            let e = self.expr(a, Some(&fields[i].1))?;
            slots[i] = Some(self.coerce(e, &fields[i].1, &format!("field `{}`", fields[i].0))?);
        }
        for k in args.keywords.iter() {
            let n = k.arg.as_ref().unwrap().id.as_str();
            let Some(i) = fields.iter().position(|f| f.0 == n) else {
                return err(line, format!("unexpected keyword argument `{}`", n));
            };
            if slots[i].is_some() {
                return err(line, format!("field `{}` given twice", n));
            }
            let e = self.expr(&k.value, Some(&fields[i].1))?;
            slots[i] = Some(self.coerce(e, &fields[i].1, &format!("field `{}`", n))?);
        }
        let mut out = Vec::new();
        for (i, s) in slots.into_iter().enumerate() {
            match s {
                Some(e) => out.push(e),
                None => match &fields[i].2 {
                    Some(d) => {
                        let mut d = d.clone();
                        d.id = self.id();
                        out.push(d)
                    }
                    None => return err(line, format!("missing field `{}`", fields[i].0)),
                },
            }
        }
        Ok(self.mk(line, Ty::Class(cid), ExprKind::New(cid, out)))
    }

    /// Arguments of a user function in parameter order, defaults filled in.
    fn call_args(&mut self, fid: FuncId, args: &ast::Arguments, _method: bool, line: u32) -> Res<Vec<Expr>> {
        let params: Vec<(String, Ty, Option<Expr>)> = {
            let f = &self.p.funcs[fid];
            f.params[..f.params.len() - f.captures.len()]
                .iter()
                .map(|p| (f.vars[p.var].name.clone(), f.vars[p.var].ty.clone(), p.default.clone()))
                .collect()
        };
        let fname = self.p.funcs[fid].name.clone();
        if args.args.len() > params.len() && self.p.funcs[fid].varargs.is_none() {
            return err(line, format!("{}() takes {} arguments but {} were given", fname, params.len(), args.args.len()));
        }
        let mut slots: Vec<Option<Expr>> = vec![None; params.len()];
        let star = self.p.funcs[fid].varargs;
        for (i, a) in args.args.iter().enumerate() {
            if let Some(si) = star {
                if i >= si {
                    // extra positional arguments go into the `*args` list
                    let Ty::List(et) = self.norm(&params[si].1) else { unreachable!() };
                    let e = self.expr(a, Some(&et))?;
                    let e = self.coerce(e, &et, &format!("argument `{}`", params[si].0))?;
                    match &mut slots[si] {
                        Some(Expr { kind: ExprKind::List(v), .. }) => v.push(e),
                        _ => slots[si] = Some(self.mk(line, params[si].1.clone(), ExprKind::List(vec![e]))),
                    }
                    continue;
                }
            }
            let e = self.expr(a, Some(&params[i].1))?;
            slots[i] = Some(self.coerce(e, &params[i].1, &format!("argument `{}`", params[i].0))?);
        }
        if let Some(si) = star {
            if slots[si].is_none() {
                slots[si] = Some(self.mk(line, params[si].1.clone(), ExprKind::List(vec![])));
            }
        }
        for k in args.keywords.iter() {
            let n = k.arg.as_ref().unwrap().id.as_str();
            let Some(i) = params.iter().position(|p| p.0 == n) else {
                return err(line, format!("{}() got an unexpected keyword argument `{}`", fname, n));
            };
            if slots[i].is_some() {
                return err(line, format!("{}() got multiple values for `{}`", fname, n));
            }
            let e = self.expr(&k.value, Some(&params[i].1))?;
            slots[i] = Some(self.coerce(e, &params[i].1, &format!("argument `{}`", n))?);
        }
        let mut out = Vec::new();
        for (i, s) in slots.into_iter().enumerate() {
            match s {
                Some(e) => out.push(e),
                None => match &params[i].2 {
                    Some(d) => {
                        let mut d = d.clone();
                        d.id = self.id();
                        out.push(d)
                    }
                    None => return err(line, format!("{}() is missing argument `{}`", fname, params[i].0)),
                },
            }
        }
        out.extend(self.capture_args(fid, line)?);
        Ok(out)
    }

    fn kw<'a>(&self, args: &'a ast::Arguments, allowed: &[&str], line: u32) -> Res<HashMap<&'a str, &'a AExpr>> {
        let mut m = HashMap::new();
        for k in args.keywords.iter() {
            let n = k.arg.as_ref().unwrap().id.as_str();
            if !allowed.contains(&n) {
                return err(line, format!("unexpected keyword argument `{}`", n));
            }
            m.insert(n, &k.value);
        }
        Ok(m)
    }

    fn nargs(&self, args: &ast::Arguments, lo: usize, hi: usize, name: &str, line: u32) -> Res<()> {
        let n = args.args.len();
        if n < lo || n > hi {
            return err(line, if lo == hi {
                format!("{}() takes {} argument(s)", name, lo)
            } else {
                format!("{}() takes {} to {} arguments", name, lo, hi)
            });
        }
        Ok(())
    }

    fn arg(&mut self, args: &ast::Arguments, i: usize, t: &Ty, what: &str) -> Res<Expr> {
        let e = self.expr(&args.args[i], Some(t))?;
        self.coerce(e, t, what)
    }

    /// `key=` and `reverse=` of sorted / list.sort
    fn sort_opts(&mut self, kws: &HashMap<&str, &AExpr>, et: &Ty, line: u32) -> Res<(Expr, Expr)> {
        let key = match kws.get("key") {
            Some(k) => {
                let l = self.lambda(k, &[et.clone()])?;
                self.orderable(&l.ty, line)?;
                l
            }
            None => {
                self.orderable(et, line)?;
                self.mk(line, Ty::None, ExprKind::NoneLit)
            }
        };
        let rev = match kws.get("reverse") {
            Some(r) => {
                let e = self.expr(r, Some(&Ty::Bool))?;
                let e = self.coerce(e, &Ty::Bool, "reverse")?;
                if !matches!(e.kind, ExprKind::Bool(_)) {
                    return err(line, "reverse= must be True or False");
                }
                e
            }
            None => self.mk(line, Ty::Bool, ExprKind::Bool(false)),
        };
        Ok((key, rev))
    }

    fn iter_arg(&mut self, a: &AExpr) -> Res<(Expr, Ty)> {
        let line = self.line(a);
        let (it, t) = self.iter(a)?;
        Ok((self.mk(line, t.clone(), ExprKind::Iter(Box::new(it))), t))
    }

    fn builtin(&mut self, name: &str, args: &ast::Arguments, want: Option<&Ty>, line: u32) -> Res<Expr> {
        use ExprKind as K;
        let b = |s: &mut Self, n: &'static str, t: Ty, v: Vec<Expr>| Ok(s.mk(line, t, K::Builtin(n, v)));
        match name {
            "print" => {
                let kws = self.kw(args, &["sep", "end"], line)?;
                let mut v = Vec::new();
                for k in ["sep", "end"] {
                    v.push(match kws.get(k) {
                        Some(e) => {
                            let x = self.expr(e, Some(&Ty::Str))?;
                            self.coerce(x, &Ty::Str, k)?
                        }
                        None => self.mk(line, Ty::Str, K::Str(if k == "sep" { " " } else { "\n" }.into())),
                    });
                }
                for a in args.args.iter() {
                    let x = self.expr(a, None)?;
                    self.printable(&x.ty, line)?;
                    v.push(x);
                }
                b(self, "print", Ty::None, v)
            }
            "len" => {
                self.nargs(args, 1, 1, name, line)?;
                let x = self.expr(&args.args[0], None)?;
                let x = match self.unary_dunder(x, "__len__", line)? {
                    Ok(e) => return Ok(e),
                    Err(x) => x,
                };
                match self.norm(&x.ty) {
                    Ty::List(_) | Ty::Str | Ty::Dict(..) | Ty::Set(_) | Ty::Tuple(_) | Ty::Deque(_) => b(self, "len", Ty::Int, vec![x]),
                    t => err(line, format!("{} has no len()", self.tn(&t))),
                }
            }
            "abs" => {
                self.nargs(args, 1, 1, name, line)?;
                let x = self.expr(&args.args[0], want)?;
                let x = match self.unary_dunder(x, "__abs__", line)? {
                    Ok(e) => return Ok(e),
                    Err(x) => x,
                };
                self.settle(&x.ty);
                let t = self.norm(&x.ty);
                if !t.is_num() {
                    return err(line, format!("bad operand type for abs(): {}", self.tn(&t)));
                }
                b(self, "abs", t, vec![x])
            }
            "min" | "max" => {
                let kws = self.kw(args, &["key", "default"], line)?;
                if args.args.is_empty() {
                    return err(line, format!("{}() needs arguments", name));
                }
                if args.args.len() == 1 {
                    let (it, t) = self.iter_arg(&args.args[0])?;
                    let key = match kws.get("key") {
                        Some(k) => {
                            let l = self.lambda(k, &[t.clone()])?;
                            self.orderable(&l.ty, line)?;
                            l
                        }
                        None => {
                            self.orderable(&t, line)?;
                            self.mk(line, Ty::None, K::NoneLit)
                        }
                    };
                    if let Some(d) = kws.get("default") {
                        if kws.contains_key("key") {
                            return err(line, "key= together with default= is not supported");
                        }
                        let dv = self.expr(d, Some(&t))?;
                        let dv = self.coerce(dv, &t, "default")?;
                        return b(self, if name == "min" { "min_or" } else { "max_or" }, t, vec![it, dv]);
                    }
                    return b(self, if name == "min" { "min_iter" } else { "max_iter" }, t, vec![it, key]);
                }
                if !kws.is_empty() {
                    return err(line, "key= is only supported with one iterable argument");
                }
                let mut v = Vec::new();
                for a in args.args.iter() {
                    let x = self.expr(a, want)?;
                    self.settle(&x.ty);
                    v.push(x);
                }
                let float = v.iter().any(|e| self.norm(&e.ty) == Ty::Float);
                if float && v.iter().any(|e| self.norm(&e.ty) == Ty::Int) {
                    return err(line, format!("{}() of an int and a float returns either type", name))
                        .map_err(|e| e.hint("convert the ints with float()"));
                }
                let t = self.norm(&v[0].ty);
                let mut out = Vec::new();
                for e in v {
                    out.push(self.coerce(e, &t, &format!("{}() argument", name))?);
                }
                self.orderable(&t, line)?;
                b(self, if name == "min" { "min" } else { "max" }, t, out)
            }
            "sum" => {
                self.nargs(args, 1, 1, name, line)?;
                self.kw(args, &[], line)?;
                let (it, t) = self.iter_arg(&args.args[0])?;
                if matches!(self.norm(&t), Ty::Var(_)) {
                    self.unify(&t, &Ty::Int);
                }
                if !self.norm(&t).is_num() {
                    return err(line, format!("sum() of {} is not supported", self.tn(&t)));
                }
                b(self, "sum", self.norm(&t), vec![it])
            }
            "any" | "all" => {
                self.nargs(args, 1, 1, name, line)?;
                let (it, t) = self.iter_arg(&args.args[0])?;
                let tt = self.norm(&t);
                if !matches!(tt, Ty::Bool | Ty::Int | Ty::Float | Ty::Str | Ty::List(_)) {
                    return err(line, format!("{}() of {} is not supported", name, self.tn(&tt)));
                }
                b(self, if name == "any" { "any" } else { "all" }, Ty::Bool, vec![it])
            }
            "sorted" => {
                self.nargs(args, 1, 1, name, line)?;
                let kws = self.kw(args, &["key", "reverse"], line)?;
                let (it, t) = self.iter_arg(&args.args[0])?;
                let (key, rev) = self.sort_opts(&kws, &t, line)?;
                b(self, "sorted", Ty::list(t), vec![it, key, rev])
            }
            "list" | "set" => {
                self.nargs(args, 0, 1, name, line)?;
                self.kw(args, &[], line)?;
                if args.args.is_empty() {
                    let t = match (name, want.map(|w| self.norm(w))) {
                        ("list", Some(Ty::List(t))) | ("set", Some(Ty::Set(t))) => *t,
                        _ => self.fresh(),
                    };
                    return Ok(if name == "list" {
                        self.mk(line, Ty::list(t), K::List(vec![]))
                    } else {
                        self.mk(line, Ty::set(t), K::Set(vec![]))
                    });
                }
                let (it, t) = self.iter_arg(&args.args[0])?;
                if name == "list" {
                    b(self, "list", Ty::list(t), vec![it])
                } else {
                    self.hashable(&t, line)?;
                    b(self, "set", Ty::set(t), vec![it])
                }
            }
            "dict" => {
                self.nargs(args, 0, 0, name, line)?;
                self.kw(args, &[], line)?;
                let (k, v) = match want.map(|w| self.norm(w)) {
                    Some(Ty::Dict(k, v, _)) => (*k, *v),
                    _ => (self.fresh(), self.fresh()),
                };
                Ok(self.mk(line, Ty::dict(k, v), K::Dict(vec![])))
            }
            "str" | "repr" => {
                self.nargs(args, 1, 1, name, line)?;
                let x = self.expr(&args.args[0], None)?;
                self.printable(&x.ty, line)?;
                b(self, if name == "str" { "str" } else { "repr" }, Ty::Str, vec![x])
            }
            "int" if args.args.len() == 2 => {
                let x = self.arg(args, 0, &Ty::Str, "int() argument")?;
                let base = self.arg(args, 1, &Ty::Int, "int() base")?;
                b(self, "int_base", Ty::Int, vec![x, base])
            }
            "int" => {
                self.nargs(args, 1, 1, name, line)?;
                let x = self.expr(&args.args[0], None)?;
                self.settle(&x.ty);
                match self.norm(&x.ty) {
                    Ty::Int => Ok(x),
                    Ty::Float | Ty::Str | Ty::Bool => b(self, "int", Ty::Int, vec![x]),
                    t => err(line, format!("int() of {} is not supported", self.tn(&t))),
                }
            }
            "float" => {
                self.nargs(args, 1, 1, name, line)?;
                let x = self.expr(&args.args[0], None)?;
                self.settle(&x.ty);
                match self.norm(&x.ty) {
                    Ty::Float => Ok(x),
                    Ty::Int => self.coerce(x, &Ty::Float, ""),
                    Ty::Str => b(self, "float", Ty::Float, vec![x]),
                    t => err(line, format!("float() of {} is not supported", self.tn(&t))),
                }
            }
            "bool" => {
                self.nargs(args, 1, 1, name, line)?;
                self.cond(&args.args[0])
            }
            "round" => {
                self.nargs(args, 1, 2, name, line)?;
                // round(int, n) is an int
                let probe = self.expr(&args.args[0], None)?;
                if self.norm(&probe.ty) == Ty::Int {
                    if args.args.len() == 1 {
                        return Ok(probe);
                    }
                    let n = self.arg(args, 1, &Ty::Int, "round() digits")?;
                    return b(self, "round_int", Ty::Int, vec![probe, n]);
                }
                let x = self.arg(args, 0, &Ty::Float, "round() argument")?;
                if args.args.len() == 2 {
                    let n = self.arg(args, 1, &Ty::Int, "round() digits")?;
                    b(self, "round2", Ty::Float, vec![x, n])
                } else {
                    b(self, "round", Ty::Int, vec![x])
                }
            }
            "divmod" => {
                self.nargs(args, 2, 2, name, line)?;
                let x = self.arg(args, 0, &Ty::Int, "divmod() argument")?;
                let y = self.arg(args, 1, &Ty::Int, "divmod() argument")?;
                b(self, "divmod", Ty::Tuple(vec![Ty::Int, Ty::Int]), vec![x, y])
            }
            "pow" => {
                self.nargs(args, 3, 3, name, line)?;
                let x = self.arg(args, 0, &Ty::Int, "pow() argument")?;
                let y = self.arg(args, 1, &Ty::Int, "pow() argument")?;
                let m = self.arg(args, 2, &Ty::Int, "pow() argument")?;
                b(self, "powmod", Ty::Int, vec![x, y, m])
            }
            "bin" | "hex" | "oct" => {
                self.nargs(args, 1, 1, name, line)?;
                let x = self.arg(args, 0, &Ty::Int, &format!("{}() argument", name))?;
                let n: &'static str = match name {
                    "bin" => "bin",
                    "hex" => "hex",
                    _ => "oct",
                };
                b(self, n, Ty::Str, vec![x])
            }
            "ord" => {
                self.nargs(args, 1, 1, name, line)?;
                let x = self.arg(args, 0, &Ty::Str, "ord() argument")?;
                b(self, "ord", Ty::Int, vec![x])
            }
            "chr" => {
                self.nargs(args, 1, 1, name, line)?;
                let x = self.arg(args, 0, &Ty::Int, "chr() argument")?;
                b(self, "chr", Ty::Str, vec![x])
            }
            "input" => {
                self.nargs(args, 0, 1, name, line)?;
                let mut v = Vec::new();
                if !args.args.is_empty() {
                    v.push(self.arg(args, 0, &Ty::Str, "input() prompt")?);
                }
                b(self, "input", Ty::Str, v)
            }
            "range" | "enumerate" | "zip" | "reversed" => {
                err(line, format!("{}() can only be used in a for loop or as an argument of list()/sum()/...", name))
            }
            "isinstance" => {
                self.nargs(args, 2, 2, name, line)?;
                let x = self.expr(&args.args[0], None)?;
                if self.norm(&x.ty) == Ty::Exc {
                    let AExpr::Name(n) = &args.args[1] else { return err(line, "isinstance() needs one class") };
                    if !self.is_exception(n.id.as_str()) {
                        return err(line, format!("`{}` is not an exception class", n.id));
                    }
                    let k = self.mk(line, Ty::Str, ExprKind::Str(n.id.to_string()));
                    return b(self, "exc.is", Ty::Bool, vec![x, k]);
                }
                let d = self.class_arg(&args.args[1], line)?;
                self.isinstance(x, d, None, line)
            }
            "open" => self.open_call(args, line),
            "next" => {
                self.kw(args, &[], line)?;
                self.nargs(args, 1, 2, "next", line)?;
                let g = self.expr(&args.args[0], None)?;
                let Ty::Gen(t) = self.norm(&g.ty) else {
                    return err(line, "next() needs a generator");
                };
                let mut v = vec![g];
                if args.args.len() == 2 {
                    v.push(self.arg(args, 1, &t, "default")?);
                }
                b(self, "gen.next", *t, v)
            }
            "type" | "id" | "hash" | "getattr" | "setattr" | "hasattr" | "eval" | "exec"
            | "globals" | "locals" | "vars" | "iter" | "map" | "filter" | "super" => {
                err(line, format!("{}() is outside the typed subset", name))
            }
            _ if EXCEPTIONS.contains(&name) => err(line, "exceptions can only be raised"),
            _ => err(line, format!("name `{}` is not defined", name)),
        }
    }

    fn is_exception(&self, n: &str) -> bool {
        EXCEPTIONS.contains(&n) || self.classes.get(n).map(|c| self.p.classes[*c].exc.is_some()).unwrap_or(false)
    }

    fn class_arg(&self, a: &AExpr, line: u32) -> Res<ClassId> {
        match a {
            AExpr::Name(n) => match self.classes.get(n.id.as_str()) {
                Some(c) if self.p.classes[*c].exc.is_none() => Ok(*c),
                _ => err(line, format!("`{}` is not a class of this program", n.id)),
            },
            _ => err(line, "isinstance() needs one class (no tuples)"),
        }
    }

    /// `isinstance(x, d)`: constant when the static type decides it.
    fn isinstance(&mut self, x: Expr, d: ClassId, var: Option<VarId>, line: u32) -> Res<Expr> {
        let (Ty::Class(c) | Ty::Obj(c)) = self.norm(&x.ty) else {
            return err(line, format!("isinstance() of {} is outside the typed subset", self.tn(&x.ty)));
        };
        let exact = matches!(self.norm(&x.ty), Ty::Obj(_)) || self.p.classes[c].subs.is_empty();
        if self.p.is_sub(c, d) {
            return Ok(self.mk(line, Ty::Bool, ExprKind::Bool(true)));
        }
        if exact || !self.p.is_sub(d, c) {
            return Ok(self.mk(line, Ty::Bool, ExprKind::Bool(false)));
        }
        Ok(self.mk(line, Ty::Bool, ExprKind::IsInst(Box::new(x), d, var)))
    }

    fn printable(&self, t: &Ty, line: u32) -> Res<()> {
        match self.norm(t) {
            Ty::Class(c) | Ty::Obj(c) => {
                let cl = &self.p.classes[c];
                let all = self.p.subtree(c).iter().all(|x| {
                    let k = &self.p.classes[*x];
                    k.dataclass || k.method("__str__").is_some() || k.method("__repr__").is_some()
                });
                if all {
                    Ok(())
                } else {
                    err(line, format!("printing a `{}` shows its address; define __str__ or use @dataclass", cl.name))
                }
            }
            Ty::List(e) | Ty::Set(e) | Ty::Deque(e) => self.printable(&e, line),
            Ty::Dict(k, v, _) => {
                self.printable(&k, line)?;
                self.printable(&v, line)
            }
            Ty::Tuple(ts) => ts.iter().try_for_each(|t| self.printable(t, line)),
            Ty::None => err(line, "cannot print None"),
            Ty::Gen(_) => err(line, "printing a generator shows its address; print list(...)"),
            Ty::Exc => Ok(()),
            Ty::Optional(t) => self.printable(&t, line),
            _ => Ok(()),
        }
    }

    fn math_const(&mut self, name: &str, line: u32) -> Res<Expr> {
        let n: &'static str = match name {
            "pi" => "math.pi",
            "e" => "math.e",
            "inf" => "math.inf",
            "tau" => "math.tau",
            _ => return err(line, format!("math.{} is not supported", name)),
        };
        Ok(self.mk(line, Ty::Float, ExprKind::Builtin(n, vec![])))
    }

    fn math(&mut self, name: &str, args: &ast::Arguments, line: u32) -> Res<Expr> {
        self.kw(args, &[], line)?;
        let f1: &[&str] = &["sqrt", "exp", "sin", "cos", "tan", "asin", "acos", "atan", "fabs", "log2", "log10", "sinh", "cosh", "tanh"];
        let (n, ps, rt): (&'static str, Vec<Ty>, Ty) = match name {
            n if f1.contains(&n) => (leak(format!("math.{}", n)), vec![Ty::Float], Ty::Float),
            "log" => {
                if args.args.len() == 2 {
                    ("math.log2args", vec![Ty::Float, Ty::Float], Ty::Float)
                } else {
                    ("math.log", vec![Ty::Float], Ty::Float)
                }
            }
            "atan2" | "hypot" | "pow" | "fmod" | "copysign" => (leak(format!("math.{}", name)), vec![Ty::Float, Ty::Float], Ty::Float),
            "radians" | "degrees" => (leak(format!("math.{}", name)), vec![Ty::Float], Ty::Float),
            "isclose" => ("math.isclose", vec![Ty::Float, Ty::Float], Ty::Bool),
            "floor" | "ceil" | "trunc" => (leak(format!("math.{}", name)), vec![Ty::Float], Ty::Int),
            "gcd" | "lcm" => (leak(format!("math.{}", name)), vec![Ty::Int, Ty::Int], Ty::Int),
            "isqrt" | "factorial" => (leak(format!("math.{}", name)), vec![Ty::Int], Ty::Int),
            "comb" | "perm" => (leak(format!("math.{}", name)), vec![Ty::Int, Ty::Int], Ty::Int),
            "isnan" | "isinf" | "isfinite" => (leak(format!("math.{}", name)), vec![Ty::Float], Ty::Bool),
            _ => return err(line, format!("math.{} is not supported", name)),
        };
        self.nargs(args, ps.len(), ps.len(), &format!("math.{}", name), line)?;
        let mut v = Vec::new();
        for (i, t) in ps.iter().enumerate() {
            v.push(self.arg(args, i, t, &format!("math.{}() argument", name))?);
        }
        Ok(self.mk(line, rt, ExprKind::Builtin(n, v)))
    }

    fn method(&mut self, recv: Expr, m: &str, args: &ast::Arguments, want: Option<&Ty>, line: u32) -> Res<Expr> {
        use ExprKind as K;
        let rt = self.norm(&recv.ty);
        let b = |s: &mut Self, n: &'static str, t: Ty, mut v: Vec<Expr>, recv: Expr| {
            v.insert(0, recv);
            Ok(s.mk(line, t, K::Builtin(n, v)))
        };
        let _ = want;
        let recv = match self.std_method(recv, m, args, line)? {
            Ok(e) => return Ok(e),
            Err(r) => r,
        };
        match &rt {
            Ty::Class(c) | Ty::Obj(c) => {
                let c = *c;
                // a value that may be a subclass calls through the dispatcher
                let disp = match &rt {
                    Ty::Class(_) if !self.p.classes[c].subs.is_empty() => {
                        self.p.classes[c].dispatch.iter().find(|(n, _)| n == m).map(|(_, f)| *f)
                    }
                    _ => None,
                };
                let Some(fid) = disp.or_else(|| self.p.classes[c].method(m)) else {
                    return err(line, format!("`{}` has no method `{}`", self.p.classes[c].name, m));
                };
                if m == "__init__" {
                    return err(line, "__init__ cannot be called directly");
                }
                if self.p.funcs[fid].property {
                    return err(line, format!("`{}` is a property: read it without ()", m));
                }
                let a = self.call_args(fid, args, true, line)?;
                let t = self.p.funcs[fid].ret.clone();
                if self.p.funcs[fid].is_static {
                    return Ok(self.mk(line, t, K::Call(fid, a)));
                }
                if self.p.funcs[self.cur].is_init {
                    if let K::Var(v) = recv.kind {
                        if Some(v) == self.p.funcs[self.cur].self_var {
                            return err(line, "methods cannot be called on `self` inside __init__")
                                .map_err(|e| e.hint("call it after the object is constructed"));
                        }
                    }
                }
                Ok(self.mk(line, t, K::Method(Box::new(recv), fid, a)))
            }
            Ty::List(et) => {
                let et = (**et).clone();
                match m {
                    "append" => {
                        self.nargs(args, 1, 1, m, line)?;
                        let x = self.arg(args, 0, &et, "appended value")?;
                        b(self, "list.append", Ty::None, vec![x], recv)
                    }
                    "extend" => {
                        self.nargs(args, 1, 1, m, line)?;
                        let (it, t) = self.iter_arg(&args.args[0])?;
                        if !self.unify(&t, &et) {
                            return err(line, "extend() with a different element type");
                        }
                        b(self, "list.extend", Ty::None, vec![it], recv)
                    }
                    "insert" => {
                        self.nargs(args, 2, 2, m, line)?;
                        let i = self.arg(args, 0, &Ty::Int, "index")?;
                        let x = self.arg(args, 1, &et, "inserted value")?;
                        b(self, "list.insert", Ty::None, vec![i, x], recv)
                    }
                    "pop" => {
                        self.nargs(args, 0, 1, m, line)?;
                        if args.args.is_empty() {
                            b(self, "list.pop", et, vec![], recv)
                        } else {
                            let i = self.arg(args, 0, &Ty::Int, "index")?;
                            b(self, "list.pop_at", et, vec![i], recv)
                        }
                    }
                    "remove" | "index" | "count" => {
                        self.nargs(args, 1, 1, m, line)?;
                        let x = self.arg(args, 0, &et, "value")?;
                        let (n, t): (&'static str, Ty) = match m {
                            "remove" => ("list.remove", Ty::None),
                            "index" => ("list.index", Ty::Int),
                            _ => ("list.count", Ty::Int),
                        };
                        b(self, n, t, vec![x], recv)
                    }
                    "sort" => {
                        self.nargs(args, 0, 0, m, line)?;
                        let kws = self.kw(args, &["key", "reverse"], line)?;
                        let (key, rev) = self.sort_opts(&kws, &et, line)?;
                        b(self, "list.sort", Ty::None, vec![key, rev], recv)
                    }
                    "reverse" | "clear" | "copy" => {
                        self.nargs(args, 0, 0, m, line)?;
                        let (n, t): (&'static str, Ty) = match m {
                            "reverse" => ("list.reverse", Ty::None),
                            "clear" => ("list.clear", Ty::None),
                            _ => ("list.copy", rt.clone()),
                        };
                        b(self, n, t, vec![], recv)
                    }
                    _ => err(line, format!("list has no method `{}`", m)),
                }
            }
            Ty::Dict(kt, vt, _) => {
                let (kt, vt) = ((**kt).clone(), (**vt).clone());
                match m {
                    "get" => {
                        self.nargs(args, 1, 2, m, line)?;
                        if args.args.len() == 1 {
                            let k = self.arg(args, 0, &kt, "key")?;
                            return b(self, "dict.get_opt", Ty::opt(vt), vec![k], recv);
                        }
                        let k = self.arg(args, 0, &kt, "key")?;
                        let d = self.arg(args, 1, &vt, "default")?;
                        b(self, "dict.get", vt, vec![k, d], recv)
                    }
                    "pop" => {
                        self.nargs(args, 1, 2, m, line)?;
                        let k = self.arg(args, 0, &kt, "key")?;
                        if args.args.len() == 2 {
                            let d = self.arg(args, 1, &vt, "default")?;
                            b(self, "dict.pop_default", vt, vec![k, d], recv)
                        } else {
                            b(self, "dict.pop", vt, vec![k], recv)
                        }
                    }
                    "setdefault" => {
                        self.nargs(args, 2, 2, m, line)?;
                        let k = self.arg(args, 0, &kt, "key")?;
                        let d = self.arg(args, 1, &vt, "default")?;
                        b(self, "dict.setdefault", vt, vec![k, d], recv)
                    }
                    "copy" | "clear" => {
                        self.nargs(args, 0, 0, m, line)?;
                        if m == "copy" {
                            b(self, "dict.copy", rt.clone(), vec![], recv)
                        } else {
                            b(self, "dict.clear", Ty::None, vec![], recv)
                        }
                    }
                    "update" => {
                        self.nargs(args, 1, 1, m, line)?;
                        let o = self.arg(args, 0, &rt, "argument")?;
                        b(self, "dict.update", Ty::None, vec![o], recv)
                    }
                    "items" | "keys" | "values" => {
                        let (it, t) = self.iter_value_from(recv, m, line)?;
                        let _ = t;
                        let lt = match m {
                            "items" => Ty::list(Ty::Tuple(vec![kt, vt])),
                            "keys" => Ty::list(kt),
                            _ => Ty::list(vt),
                        };
                        Ok(self.mk(line, lt, K::Builtin("list", vec![it])))
                    }
                    _ => err(line, format!("dict has no method `{}`", m)),
                }
            }
            Ty::Set(et) => {
                let et = (**et).clone();
                match m {
                    "add" | "remove" | "discard" => {
                        self.nargs(args, 1, 1, m, line)?;
                        let x = self.arg(args, 0, &et, "value")?;
                        let n: &'static str = match m {
                            "add" => "set.add",
                            "remove" => "set.remove",
                            _ => "set.discard",
                        };
                        b(self, n, Ty::None, vec![x], recv)
                    }
                    "union" | "intersection" | "difference" | "symmetric_difference" => {
                        self.nargs(args, 1, 1, m, line)?;
                        let o = self.arg(args, 0, &rt, "argument")?;
                        let op = match m {
                            "union" => BinOp::BitOr,
                            "intersection" => BinOp::BitAnd,
                            "difference" => BinOp::Sub,
                            _ => BinOp::BitXor,
                        };
                        Ok(self.mk(line, rt.clone(), K::Bin(op, Box::new(recv), Box::new(o))))
                    }
                    "issubset" | "issuperset" | "isdisjoint" => {
                        self.nargs(args, 1, 1, m, line)?;
                        let o = self.arg(args, 0, &rt, "argument")?;
                        let n: &'static str = match m {
                            "issubset" => "set.issubset",
                            "issuperset" => "set.issuperset",
                            _ => "set.isdisjoint",
                        };
                        b(self, n, Ty::Bool, vec![o], recv)
                    }
                    "copy" => b(self, "set.copy", rt.clone(), vec![], recv),
                    "clear" => b(self, "set.clear", Ty::None, vec![], recv),
                    _ => err(line, format!("set has no method `{}`", m)),
                }
            }
            Ty::Str => self.str_method(recv, m, args, line),
            Ty::Var(_) => err(line, "the type is not known yet here; add a type annotation"),
            t => err(line, format!("{} has no method `{}`", self.tn(t), m)),
        }
    }

    fn iter_value_from(&mut self, d: Expr, m: &str, line: u32) -> Res<(Expr, Ty)> {
        let Ty::Dict(k, v, _) = self.norm(&d.ty) else { unreachable!() };
        let (it, t) = match m {
            "items" => (Iter::Items(d), Ty::Tuple(vec![*k, *v])),
            "keys" => (Iter::Keys(d), *k),
            _ => (Iter::Values(d), *v),
        };
        Ok((self.mk(line, t.clone(), ExprKind::Iter(Box::new(it))), t))
    }

    fn str_method(&mut self, recv: Expr, m: &str, args: &ast::Arguments, line: u32) -> Res<Expr> {
        // split(maxsplit=n), split(sep, n), rsplit(sep, n)
        if matches!(m, "split" | "rsplit") {
            let kws = self.kw(args, &["sep", "maxsplit"], line)?;
            if !kws.is_empty() || args.args.len() == 2 {
                let sep: Option<&AExpr> = args.args.first().or(kws.get("sep").copied());
                let max = match args.args.get(1).or(kws.get("maxsplit").copied()) {
                    Some(a) => {
                        let e = self.expr(a, Some(&Ty::Int))?;
                        self.coerce(e, &Ty::Int, "maxsplit")?
                    }
                    _ => self.mk(line, Ty::Int, ExprKind::Int(-1)),
                };
                let ls = Ty::list(Ty::Str);
                return match sep {
                    Some(AExpr::NoneLiteral(_)) | None => Ok(self.mk(line, ls, ExprKind::Builtin("str.split_ws_max", vec![recv, max]))),
                    Some(a) => {
                        let e = self.expr(a, Some(&Ty::Str))?;
                        let e = self.coerce(e, &Ty::Str, "separator")?;
                        let f: &'static str = if m == "split" { "str.split_max" } else { "str.rsplit_max" };
                        Ok(self.mk(line, ls, ExprKind::Builtin(f, vec![recv, e, max])))
                    }
                };
            }
        }
        self.kw(args, &[], line)?;
        let n = args.args.len();
        let s = Ty::Str;
        let (name, ps, rt): (&'static str, Vec<Ty>, Ty) = match (m, n) {
            ("upper" | "lower" | "title" | "capitalize" | "swapcase" | "casefold", 0) => (leak(format!("str.{}", m)), vec![], s.clone()),
            ("strip" | "lstrip" | "rstrip", 0) => (leak(format!("str.{}", m)), vec![], s.clone()),
            ("strip" | "lstrip" | "rstrip", 1) => (leak(format!("str.{}_chars", m)), vec![s.clone()], s.clone()),
            ("split", 0) => ("str.split_ws", vec![], Ty::list(s.clone())),
            ("split", 1) => ("str.split", vec![s.clone()], Ty::list(s.clone())),
            ("rsplit", 0) => ("str.split_ws", vec![], Ty::list(s.clone())),
            ("splitlines", 0) => ("str.splitlines", vec![], Ty::list(s.clone())),
            ("replace", 2) => ("str.replace", vec![s.clone(), s.clone()], s.clone()),
            ("replace", 3) => ("str.replace_n", vec![s.clone(), s.clone(), Ty::Int], s.clone()),
            ("rsplit", 1) => ("str.rsplit_all", vec![s.clone()], Ty::list(s.clone())),
            ("find" | "count", 2) => (leak(format!("str.{}_from", m)), vec![s.clone(), Ty::Int], Ty::Int),
            ("partition" | "rpartition", 1) => (leak(format!("str.{}", m)), vec![s.clone()], Ty::Tuple(vec![s.clone(), s.clone(), s.clone()])),
            ("removeprefix" | "removesuffix", 1) => (leak(format!("str.{}", m)), vec![s.clone()], s.clone()),
            ("center" | "ljust" | "rjust", 2) => (leak(format!("str.{}_fill", m)), vec![Ty::Int, s.clone()], s.clone()),
            ("isascii", 0) => ("str.isascii", vec![], Ty::Bool),
            ("startswith" | "endswith", 1) => (leak(format!("str.{}", m)), vec![s.clone()], Ty::Bool),
            ("find" | "rfind" | "index" | "rindex" | "count", 1) => (leak(format!("str.{}", m)), vec![s.clone()], Ty::Int),
            ("isdigit" | "isalpha" | "isalnum" | "isspace" | "isupper" | "islower" | "isdecimal" | "isnumeric", 0) => {
                (leak(format!("str.{}", m)), vec![], Ty::Bool)
            }
            ("zfill", 1) => ("str.zfill", vec![Ty::Int], s.clone()),
            ("center" | "ljust" | "rjust", 1) => (leak(format!("str.{}", m)), vec![Ty::Int], s.clone()),
            ("join", 1) => {
                let (it, t) = self.iter_arg(&args.args[0])?;
                if self.norm(&t) != Ty::Str {
                    return err(line, format!("join() needs strings, not {}", self.tn(&t)));
                }
                return Ok(self.mk(line, s, ExprKind::Builtin("str.join", vec![recv, it])));
            }
            _ => return err(line, format!("str.{}() with {} argument(s) is not supported", m, n)),
        };
        let mut v = vec![recv];
        for (i, t) in ps.iter().enumerate() {
            v.push(self.arg(args, i, t, &format!("str.{}() argument", m))?);
        }
        Ok(self.mk(line, rt, ExprKind::Builtin(name, v)))
    }

    // ----------------------------------------------------------------- finish

    /// Resolve all type variables; unresolved ones are errors.
    fn finish(&mut self) -> Res<()> {
        // nothing constrains the elements of an empty literal: any type is right
        for t in &mut self.tvars {
            if t.is_none() {
                *t = Some(Ty::Int);
            }
        }
        let mut p = std::mem::take(&mut self.p);
        let r = (|| {
            for f in &mut p.funcs {
                for v in &mut f.vars {
                    v.ty = self.norm(&v.ty);
                    if has_var(&v.ty) {
                        return err(f.line, format!("cannot infer the type of `{}` in {}()", v.name, f.name))
                            .map_err(|e| e.hint("annotate it, e.g. `xs: list[int] = []`"));
                    }
                }
                for s in &mut f.body {
                    self.fix_stmt(s)?;
                }
                for prm in &mut f.params {
                    if let Some(d) = &mut prm.default {
                        self.fix_expr(d)?;
                    }
                }
            }
            for c in &mut p.classes {
                for fd in &mut c.fields {
                    fd.ty = self.norm(&fd.ty);
                    if let Some(d) = &mut fd.default {
                        self.fix_expr(d)?;
                    }
                }
            }
            for c in &mut p.consts {
                self.fix_expr(&mut c.value)?;
            }
            Ok(())
        })();
        self.p = p;
        r
    }

    fn fix_stmt(&self, s: &mut Stmt) -> Res<()> {
        let mut r = Ok(());
        visit_stmt_exprs_mut(s, &mut |e| {
            if r.is_ok() {
                r = self.fix_expr(e);
            }
        });
        r
    }

    fn fix_expr(&self, e: &mut Expr) -> Res<()> {
        e.ty = self.norm(&e.ty);
        if has_var(&e.ty) {
            return err(e.line, "cannot infer the element type of this empty container")
                .map_err(|e| e.hint("annotate the variable, e.g. `xs: list[int] = []`"));
        }
        let mut r = Ok(());
        children_mut(e, &mut |c| {
            if r.is_ok() {
                r = self.fix_expr(c);
            }
        });
        r
    }
}

fn leak(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn has_var(t: &Ty) -> bool {
    match t {
        Ty::Var(_) => true,
        Ty::List(a) | Ty::Set(a) => has_var(a),
        Ty::Dict(k, v, _) => has_var(k) || has_var(v),
        Ty::Tuple(ts) => ts.iter().any(has_var),
        Ty::Optional(t) => has_var(t),
        _ => false,
    }
}

pub fn binop(op: ast::Operator) -> Option<BinOp> {
    use ast::Operator as O;
    Some(match op {
        O::Add => BinOp::Add,
        O::Sub => BinOp::Sub,
        O::Mult => BinOp::Mul,
        O::Div => BinOp::Div,
        O::FloorDiv => BinOp::FloorDiv,
        O::Mod => BinOp::Mod,
        O::Pow => BinOp::Pow,
        O::LShift => BinOp::Shl,
        O::RShift => BinOp::Shr,
        O::BitAnd => BinOp::BitAnd,
        O::BitOr => BinOp::BitOr,
        O::BitXor => BinOp::BitXor,
        O::MatMult => return None,
    })
}

pub fn opstr(op: BinOp) -> &'static str {
    use BinOp::*;
    match op {
        Add => "+",
        Sub => "-",
        Mul => "*",
        Div => "/",
        FloorDiv => "//",
        Mod => "%",
        Pow => "**",
        Shl => "<<",
        Shr => ">>",
        BitAnd => "&",
        BitOr => "|",
        BitXor => "^",
    }
}

/// Validate a format spec against the value type (subset of the mini-language).
fn check_spec(spec: &str, t: &Ty) -> Result<(), String> {
    let s = pyrt::spec::Spec::parse(spec).ok_or_else(|| format!("unsupported format spec `{}`", spec))?;
    let ok = match (s.ty, t) {
        (None, Ty::Int | Ty::Float | Ty::Str) => true,
        (Some('d' | 'x' | 'X' | 'b' | 'o' | 'c'), Ty::Int) => true,
        (Some('f' | 'F' | 'e' | 'E' | 'g' | 'G' | '%'), Ty::Int | Ty::Float) => true,
        (Some('s'), Ty::Str) => true,
        _ => false,
    };
    if !ok {
        return Err(format!("format spec `{}` does not fit this value", spec));
    }
    if s.grouping.is_some() && !matches!(t, Ty::Int | Ty::Float) {
        return Err("thousands separators need a number".into());
    }
    Ok(())
}

// ------------------------------------------------------------ mutable visits

pub fn children_mut(e: &mut Expr, f: &mut dyn FnMut(&mut Expr)) {
    use ExprKind::*;
    match &mut e.kind {
        Int(_) | Float(_) | Bool(_) | Str(_) | NoneLit | NoneVal | Var(_) | Const(_) => {}
        Bin(_, a, b) | Index(a, b) => {
            f(a);
            f(b)
        }
        Unary(_, a) | ToFloat(a) | Field(a, _) | TupleField(a, _) | Lambda(_, a) | SomeOf(a) | IsSome(a, _, _)
        | Upcast(a) | IsInst(a, _, _) => f(a),
        Cmp(a, rest) => {
            f(a);
            rest.iter_mut().for_each(|(_, e)| f(e))
        }
        And(v) | Or(v) | List(v) | Tuple(v) | Set(v) | Call(_, v) | New(_, v) | Builtin(_, v) | SuperInit(_, v) => {
            v.iter_mut().for_each(|e| f(e))
        }
        IfExp(a, b, c) => {
            f(a);
            f(b);
            f(c)
        }
        Method(r, _, v) => {
            f(r);
            v.iter_mut().for_each(|e| f(e))
        }
        Slice(a, b, c, d) => {
            f(a);
            for x in [b, c, d].into_iter().flatten() {
                f(x)
            }
        }
        Dict(v) => v.iter_mut().for_each(|(k, x)| {
            f(k);
            f(x)
        }),
        Comp(c) => {
            for g in &mut c.gens {
                iter_exprs_mut(&mut g.iter, f);
                target_exprs_mut(&mut g.target, f);
                g.conds.iter_mut().for_each(|e| f(e));
            }
            f(&mut c.elt);
            if let Some(v) = &mut c.val {
                f(v)
            }
        }
        FStr(parts) => parts.iter_mut().for_each(|p| {
            if let FPart::Val(e, _, _) = p {
                f(e)
            }
        }),
        Iter(it) => iter_exprs_mut(it, f),
    }
}

fn iter_exprs_mut(it: &mut Iter, f: &mut dyn FnMut(&mut Expr)) {
    match it {
        Iter::Range(a, b, c) => {
            f(a);
            f(b);
            if let Some(c) = c {
                f(c)
            }
        }
        Iter::Over(e) | Iter::Keys(e) | Iter::Values(e) | Iter::Items(e) | Iter::Gen(e) => f(e),
        Iter::Enumerate(i, s) => {
            iter_exprs_mut(i, f);
            if let Some(s) = s {
                f(s)
            }
        }
        Iter::Zip(v) => v.iter_mut().for_each(|i| iter_exprs_mut(i, f)),
        Iter::Reversed(i) => iter_exprs_mut(i, f),
    }
}

fn target_exprs_mut(t: &mut Target, f: &mut dyn FnMut(&mut Expr)) {
    match t {
        Target::Var(_) => {}
        Target::Place(e) => f(e),
        Target::Tuple(v) => v.iter_mut().for_each(|t| target_exprs_mut(t, f)),
    }
}

fn visit_stmt_exprs_mut(s: &mut Stmt, f: &mut dyn FnMut(&mut Expr)) {
    match &mut s.kind {
        StmtKind::Expr(e) | StmtKind::Del(e) => f(e),
        StmtKind::Assign(t, e) => {
            target_exprs_mut(t, f);
            f(e)
        }
        StmtKind::AugAssign(t, _, e) => {
            f(t);
            f(e)
        }
        StmtKind::AugVar(_, _, e) => f(e),
        StmtKind::If(c, a, b) | StmtKind::While(c, a, b) => {
            f(c);
            a.iter_mut().chain(b.iter_mut()).for_each(|s| visit_stmt_exprs_mut(s, f))
        }
        StmtKind::For(t, it, a, b) => {
            target_exprs_mut(t, f);
            iter_exprs_mut(it, f);
            a.iter_mut().chain(b.iter_mut()).for_each(|s| visit_stmt_exprs_mut(s, f))
        }
        StmtKind::Return(Some(e)) => f(e),
        StmtKind::Raise(_, Some(e)) => f(e),
        StmtKind::LetSome(_, e, b) => {
            f(e);
            b.iter_mut().for_each(|s| visit_stmt_exprs_mut(s, f))
        }
        StmtKind::Try(b, hs, e, fi) => {
            b.iter_mut().chain(e.iter_mut()).chain(fi.iter_mut()).for_each(|s| visit_stmt_exprs_mut(s, f));
            for h in hs {
                h.body.iter_mut().for_each(|s| visit_stmt_exprs_mut(s, f));
            }
        }
        StmtKind::Assert(c, m) => {
            f(c);
            if let Some(m) = m {
                f(m)
            }
        }
        _ => {}
    }
}
