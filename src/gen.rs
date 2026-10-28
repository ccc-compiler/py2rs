//! Typed IR → Rust source.

use std::collections::{HashMap, HashSet};

use crate::diag::{err, Res};
use crate::ir::*;
use crate::own::{uses, Mode, Own};

const KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in",
    "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "static", "struct", "super", "trait",
    "true", "type", "unsafe", "use", "where", "while", "async", "await", "dyn", "abstract", "become", "box", "do",
    "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "gen",
];

pub fn ident(n: &str) -> String {
    if KEYWORDS.contains(&n) {
        format!("r#{}", n)
    } else if n == "Self" || n == "self" {
        format!("{}_", n)
    } else {
        n.to_string()
    }
}

fn rust_str(s: &str) -> String {
    let mut o = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => o += "\\\"",
            '\\' => o += "\\\\",
            '\n' => o += "\\n",
            '\r' => o += "\\r",
            '\t' => o += "\\t",
            '\0' => o += "\\0",
            c if (c as u32) < 0x20 || c as u32 == 0x7f => o += &format!("\\u{{{:x}}}", c as u32),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// Text inside a format string.
fn fmt_text(s: &str) -> String {
    let q = rust_str(s);
    q[1..q.len() - 1].replace('{', "{{").replace('}', "}}")
}

fn float_lit(v: f64) -> String {
    if v.is_nan() {
        "f64::NAN".into()
    } else if v.is_infinite() {
        if v > 0.0 { "f64::INFINITY".into() } else { "f64::NEG_INFINITY".into() }
    } else {
        let s = format!("{:?}", v);
        if s.contains('.') || s.contains('e') || s.contains("inf") {
            s
        } else {
            s + ".0"
        }
    }
}

pub fn rty(t: &Ty, p: &Program) -> String {
    match t {
        Ty::Int => "i64".into(),
        Ty::Float => "f64".into(),
        Ty::Bool => "bool".into(),
        Ty::Str => "String".into(),
        Ty::None => "()".into(),
        Ty::List(e) => format!("Vec<{}>", rty(e, p)),
        Ty::Set(e) => format!("IndexSet<{}>", rty(e, p)),
        Ty::Dict(k, v) => format!("IndexMap<{}, {}>", rty(k, p), rty(v, p)),
        Ty::Tuple(ts) => {
            let v: Vec<String> = ts.iter().map(|t| rty(t, p)).collect();
            if v.len() == 1 {
                format!("({},)", v[0])
            } else {
                format!("({})", v.join(", "))
            }
        }
        Ty::Class(c) => p.classes[*c].name.clone(),
        Ty::Var(_) => "_".into(),
    }
}

/// Ordered by `Ord` (no floats inside)?
fn total_order(t: &Ty) -> bool {
    match t {
        Ty::Int | Ty::Bool | Ty::Str => true,
        Ty::List(e) => total_order(e),
        Ty::Tuple(ts) => ts.iter().all(total_order),
        _ => false,
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Kind {
    /// an owned temporary
    Val,
    /// an owned place (variable, field, element)
    Place,
    /// `&T`
    Ref,
    /// `&mut T`
    Mut,
}

/// Generated expression.
#[derive(Clone, Debug)]
struct R {
    code: String,
    kind: Kind,
    prec: u8,
}

const ATOM: u8 = 100;
const UNARY: u8 = 90;
const CAST: u8 = 85;
const MUL: u8 = 80;
const ADD: u8 = 70;
const SHIFT: u8 = 60;
const BAND: u8 = 55;
const BXOR: u8 = 50;
const BOR: u8 = 45;
const CMP: u8 = 40;
const AND: u8 = 30;
const OR: u8 = 20;
const RANGE: u8 = 10;
const LOW: u8 = 5;

fn r(code: impl Into<String>, kind: Kind, prec: u8) -> R {
    R { code: code.into(), kind, prec }
}

/// A number as the receiver of a method call (`5_i64.pow(2)`).
fn recv(x: &R) -> String {
    let c = x.code.trim_start_matches('-');
    let lit = !c.is_empty() && c.chars().all(|ch| ch.is_ascii_digit() || ch == '.' || ch == 'e' || ch == '_') && c.chars().next().unwrap().is_ascii_digit();
    if lit && !x.code.contains("_i64") && !x.code.contains("_f64") {
        let suf = if c.contains('.') || c.contains('e') { "_f64" } else { "_i64" };
        if x.code.starts_with('-') {
            format!("({}{})", x.code, suf)
        } else {
            format!("{}{}", x.code, suf)
        }
    } else {
        par(x, ATOM)
    }
}

fn par(x: &R, min: u8) -> String {
    if x.prec < min {
        format!("({})", x.code)
    } else {
        x.code.clone()
    }
}

/// How a loop yields its items.
#[derive(Clone, Debug)]
enum Item {
    Val,
    Ref,
    Mut,
    Tup(Vec<Item>),
    /// zip of n iterators: nested pairs ((a, b), c)
    Zip(Vec<Item>),
}

pub fn generate(p: &Program, o: &Own) -> Res<String> {
    let mut out = String::new();
    let uses_map = p_uses_collections(p);
    if uses_map.0 || uses_map.1 {
        let mut v = vec![];
        if uses_map.0 {
            v.push("IndexMap");
        }
        if uses_map.1 {
            v.push("IndexSet");
        }
        out += &format!("use pyrt::{{{}}};\n\n", v.join(", "));
    }
    for c in &p.consts {
        let mut g = FGen::new(p, o, p.main);
        let v = g.ex(&c.value)?;
        let (t, code) = match c.value.ty {
            Ty::Str => ("&str".to_string(), v.code),
            _ => (rty(&c.value.ty, p), v.code),
        };
        if c.name.chars().any(|ch| ch.is_lowercase()) {
            out += "#[allow(non_upper_case_globals)]\n";
        }
        out += &format!("const {}: {} = {};\n", ident(&c.name), t, code);
    }
    if !p.consts.is_empty() {
        out += "\n";
    }
    for (ci, c) in p.classes.iter().enumerate() {
        out += &gen_class(p, o, ci, c)?;
    }
    for (fid, f) in p.funcs.iter().enumerate() {
        if f.class.is_some() || fid == p.main {
            continue;
        }
        let mut g = FGen::new(p, o, fid);
        out += &g.func("")?;
        out += "\n";
    }
    let mut g = FGen::new(p, o, p.main);
    out += &g.func("")?;
    Ok(out)
}

fn p_uses_collections(p: &Program) -> (bool, bool) {
    fn ty(t: &Ty, m: &mut (bool, bool)) {
        match t {
            Ty::Dict(k, v) => {
                m.0 = true;
                ty(k, m);
                ty(v, m)
            }
            Ty::Set(e) => {
                m.1 = true;
                ty(e, m)
            }
            Ty::List(e) => ty(e, m),
            Ty::Tuple(ts) => ts.iter().for_each(|t| ty(t, m)),
            _ => {}
        }
    }
    fn ex(e: &Expr, m: &mut (bool, bool)) {
        ty(&e.ty, m);
        e.children(&mut |c| ex(c, m));
    }
    fn st(s: &Stmt, m: &mut (bool, bool)) {
        let mut f = |e: &Expr| ex(e, m);
        stmt_exprs(s, &mut f);
        for b in sub_blocks(s) {
            b.iter().for_each(|s| st(s, m));
        }
    }
    let mut m = (false, false);
    for f in &p.funcs {
        f.vars.iter().for_each(|v| ty(&v.ty, &mut m));
        ty(&f.ret, &mut m);
        f.body.iter().for_each(|s| st(s, &mut m));
    }
    for c in &p.classes {
        c.fields.iter().for_each(|f| ty(&f.ty, &mut m));
    }
    m
}

fn sub_blocks(s: &Stmt) -> Vec<&Vec<Stmt>> {
    match &s.kind {
        StmtKind::If(_, a, b) | StmtKind::While(_, a, b) | StmtKind::For(_, _, a, b) => vec![a, b],
        _ => vec![],
    }
}

fn stmt_exprs(s: &Stmt, f: &mut dyn FnMut(&Expr)) {
    match &s.kind {
        StmtKind::Expr(e) | StmtKind::Del(e) => f(e),
        StmtKind::Assign(t, e) => {
            t.exprs(f);
            if let Target::Place(p) = t {
                f(p)
            }
            f(e)
        }
        StmtKind::AugAssign(t, _, e) => {
            f(t);
            f(e)
        }
        StmtKind::AugVar(_, _, e) => f(e),
        StmtKind::If(c, ..) | StmtKind::While(c, ..) => f(c),
        StmtKind::For(_, it, ..) => it.exprs(f),
        StmtKind::Return(Some(e)) | StmtKind::Raise(_, Some(e)) => f(e),
        StmtKind::Assert(c, m) => {
            f(c);
            if let Some(m) = m {
                f(m)
            }
        }
        _ => {}
    }
}

fn gen_class(p: &Program, o: &Own, ci: ClassId, c: &Class) -> Res<String> {
    let mut out = String::new();
    let derives = if c.dataclass { "Clone, Debug, PartialEq" } else { "Clone, Debug" };
    out += &format!("#[derive({})]\npub struct {} {{\n", derives, c.name);
    for f in &c.fields {
        out += &format!("    pub {}: {},\n", ident(&f.name), rty(&f.ty, p));
    }
    out += "}\n\n";
    let methods: Vec<FuncId> = c.methods.iter().map(|(_, f)| *f).collect();
    if !methods.is_empty() {
        out += &format!("impl {} {{\n", c.name);
        let mut first = true;
        for fid in &methods {
            if !first {
                out += "\n";
            }
            first = false;
            let mut g = FGen::new(p, o, *fid);
            let code = g.func("    ")?;
            out += &code;
        }
        out += "}\n\n";
    }
    let has_str = c.method("__str__").is_some();
    let has_repr = c.method("__repr__").is_some();
    if c.dataclass || has_str || has_repr {
        out += &format!("impl pyrt::Py for {} {{\n", c.name);
        out += "    fn fmt_repr(&self, out: &mut String) {\n";
        if has_repr {
            out += "        out.push_str(&self.__repr__());\n";
        } else if c.dataclass {
            out += &format!("        out.push_str(\"{}(\");\n", c.name);
            for (i, f) in c.fields.iter().enumerate() {
                let sep = if i > 0 { ", " } else { "" };
                out += &format!("        out.push_str(\"{}{}=\");\n", sep, f.name);
                out += &format!("        self.{}.fmt_repr(out);\n", ident(&f.name));
            }
            out += "        out.push(')');\n";
        } else {
            out += "        out.push_str(&self.__str__());\n";
        }
        out += "    }\n";
        if has_str {
            out += "    fn fmt_str(&self, out: &mut String) {\n        out.push_str(&self.__str__());\n    }\n";
        }
        out += "}\n\n";
    }
    let _ = ci;
    Ok(out)
}

/// Where a local variable is declared.
#[derive(Clone, Debug, PartialEq)]
enum Decl {
    /// `let` at this assignment statement
    At(u32),
    /// `let x: T;` before this statement
    Before(u32),
}

struct FGen<'a> {
    p: &'a Program,
    o: &'a Own,
    fid: FuncId,
    f: &'a Func,
    kinds: HashMap<VarId, Kind>,
    decl: HashMap<VarId, Decl>,
    declared: HashSet<VarId>,
    muts: HashSet<VarId>,
    nonneg: HashSet<VarId>,
    labels: u32,
    /// label of each enclosing loop that has an `else`
    loop_labels: Vec<Option<String>>,
    /// fields of `self` held in locals inside __init__
    init_fields: HashMap<String, String>,
    tmp: u32,
    /// statements to emit before the current one
    pre: Vec<String>,
    /// variables read anywhere in the function
    used: HashSet<VarId>,
}

impl<'a> FGen<'a> {
    fn new(p: &'a Program, o: &'a Own, fid: FuncId) -> Self {
        FGen {
            p,
            o,
            fid,
            f: &p.funcs[fid],
            kinds: HashMap::new(),
            decl: HashMap::new(),
            declared: HashSet::new(),
            muts: HashSet::new(),
            nonneg: HashSet::new(),
            labels: 0,
            loop_labels: vec![],
            init_fields: HashMap::new(),
            tmp: 0,
            pre: vec![],
            used: {
                let mut u = vec![];
                fn st(s: &Stmt, u: &mut Vec<VarId>) {
                    stmt_exprs(s, &mut |e| uses(e, u));
                    if let StmtKind::AugVar(v, ..) = s.kind {
                        u.push(v);
                    }
                    for b in sub_blocks(s) {
                        b.iter().for_each(|x| st(x, u));
                    }
                }
                p.funcs[fid].body.iter().for_each(|s| st(s, &mut u));
                u.into_iter().collect()
            },
        }
    }

    fn name(&self, v: VarId) -> String {
        let var = &self.f.vars[v];
        if var.kind == VarKind::SelfParam {
            return "self".into();
        }
        if var.kind == VarKind::Bound && (var.name == "_" || !self.used.contains(&v)) {
            return "_".into();
        }
        ident(&var.name)
    }

    fn ty(&self, t: &Ty) -> String {
        rty(t, self.p)
    }

    // ------------------------------------------------------------ functions

    fn func(&mut self, ind: &str) -> Res<String> {
        let f = self.f;
        let is_main = self.fid == self.p.main;
        let name = if is_main {
            "main".to_string()
        } else if f.name == "main" && f.class.is_none() {
            "py_main".to_string()
        } else if f.is_init {
            "new".to_string()
        } else {
            ident(&f.name)
        };
        let mut params = vec![];
        if let Some(sv) = f.self_var {
            if !f.is_init {
                if self.o.self_mut[self.fid] {
                    params.push("&mut self".to_string());
                    self.kinds.insert(sv, Kind::Mut);
                } else {
                    params.push("&self".to_string());
                    self.kinds.insert(sv, Kind::Ref);
                }
            }
        }
        self.plan_decls();
        for (i, q) in f.params.iter().enumerate() {
            let v = &f.vars[q.var];
            let mode = self.o.params[self.fid][i];
            let mutable = self.muts.contains(&q.var) || self.o.mut_vars[self.fid].contains(&q.var);
            let (t, k) = match mode {
                Mode::Copy => (self.ty(&v.ty), Kind::Place),
                Mode::Owned => (self.ty(&v.ty), Kind::Place),
                Mode::Ref => (
                    match &v.ty {
                        Ty::Str => "&str".to_string(),
                        Ty::List(e) => format!("&[{}]", self.ty(e)),
                        t => format!("&{}", self.ty(t)),
                    },
                    Kind::Ref,
                ),
                Mode::Mut => (format!("&mut {}", self.ty(&v.ty)), Kind::Mut),
            };
            let m = if mutable && matches!(mode, Mode::Copy | Mode::Owned) { "mut " } else { "" };
            params.push(format!("{}{}: {}", m, self.name(q.var), t));
            self.kinds.insert(q.var, k);
            self.declared.insert(q.var);
        }
        let ret = if f.is_init {
            " -> Self".to_string()
        } else if f.ret == Ty::None {
            String::new()
        } else {
            format!(" -> {}", self.ty(&f.ret))
        };
        let vis = if f.class.is_some() { "pub " } else { "" };
        let mut out = format!("{}{}fn {}({}){} {{\n", ind, vis, name, params.join(", "), ret);
        let inner = format!("{}    ", ind);
        if is_main {
            out += &format!("{}pyrt::init();\n", inner);
        }
        if f.is_init {
            self.init_prologue(&inner, &mut out)?;
        }
        let body = &f.body;
        let n = body.len();
        for (i, s) in body.iter().enumerate() {
            // a final `return x` becomes the tail expression
            if i + 1 == n && !f.is_init {
                if let StmtKind::Return(Some(e)) = &s.kind {
                    self.pre_decls(s.id, &inner, &mut out);
                    let v = self.owned(e)?;
                    out += &format!("{}{}\n", inner, v);
                    continue;
                }
            }
            self.stmt(s, &inner, &mut out)?;
        }
        if f.is_init {
            let c = &self.p.classes[f.class.unwrap()];
            let fs: Vec<String> = c
                .fields
                .iter()
                .map(|fd| {
                    let local = &self.init_fields[&fd.name];
                    if *local == ident(&fd.name) {
                        local.clone()
                    } else {
                        format!("{}: {}", ident(&fd.name), local)
                    }
                })
                .collect();
            out += &format!("{}Self {{ {} }}\n", inner, fs.join(", "));
        } else if f.ret != Ty::None && !always_exits(body) {
            return err(f.line, format!("{}() can reach its end without returning a value", f.name))
                .map_err(|e| e.hint("add a return (Python would return None)"));
        }
        out += &format!("{}}}\n", ind);
        Ok(out)
    }

    /// In `__init__`, every field is a local until `Self { .. }` is built.
    fn init_prologue(&mut self, ind: &str, out: &mut String) -> Res<()> {
        let c = &self.p.classes[self.f.class.unwrap()];
        let names: HashSet<&str> = self.f.vars.iter().map(|v| v.name.as_str()).collect();
        for fd in &c.fields {
            let local = if names.contains(fd.name.as_str()) {
                format!("self_{}", fd.name)
            } else {
                ident(&fd.name)
            };
            self.init_fields.insert(fd.name.clone(), local);
        }
        // fields assigned more than once, in nested blocks or with a class default
        let mut counts: HashMap<String, (u32, bool)> = HashMap::new();
        fn walk(s: &[Stmt], depth: u32, sv: VarId, c: &mut HashMap<String, (u32, bool)>) {
            for st in s {
                if let StmtKind::Assign(Target::Place(Expr { kind: ExprKind::Field(o, f), .. }), _)
                | StmtKind::AugAssign(Expr { kind: ExprKind::Field(o, f), .. }, _, _) = &st.kind
                {
                    if matches!(o.kind, ExprKind::Var(v) if v == sv) {
                        let e = c.entry(f.clone()).or_insert((0, false));
                        e.0 += 1;
                        if depth > 0 || matches!(st.kind, StmtKind::AugAssign(..)) {
                            e.1 = true;
                        }
                    }
                }
                for b in sub_blocks(st) {
                    walk(b, depth + 1, sv, c);
                }
            }
        }
        let sv = self.f.self_var.unwrap();
        walk(&self.f.body, 0, sv, &mut counts);
        let mutated = self.field_mutated_in_init();
        for fd in &c.fields {
            let local = self.init_fields[&fd.name].clone();
            let (n, nested) = counts.get(&fd.name).copied().unwrap_or((0, false));
            if n == 0 {
                match &fd.default {
                    Some(d) => {
                        let v = self.owned(d)?;
                        out.push_str(&format!("{}let {} = {};\n", ind, local, v));
                    }
                    None => {
                        return err(self.f.line, format!("__init__ does not set field `{}`", fd.name));
                    }
                }
            } else if n > 1 || nested || mutated.contains(&fd.name) {
                let m = if n > 1 || nested || mutated.contains(&fd.name) { "mut " } else { "" };
                out.push_str(&format!("{}let {}{}: {};\n", ind, m, local, self.ty(&fd.ty)));
                self.init_fields_declared(&fd.name);
            }
        }
        Ok(())
    }

    fn init_fields_declared(&mut self, f: &str) {
        self.init_fields.insert(format!("#declared:{}", f), String::new());
    }

    fn field_mutated_in_init(&self) -> HashSet<String> {
        let sv = self.f.self_var.unwrap();
        let mut out = HashSet::new();
        fn base_field(e: &Expr, sv: VarId) -> Option<String> {
            match &e.kind {
                ExprKind::Field(o, f) if matches!(o.kind, ExprKind::Var(v) if v == sv) => Some(f.clone()),
                ExprKind::Field(o, _) | ExprKind::Index(o, _) | ExprKind::TupleField(o, _) => base_field(o, sv),
                _ => None,
            }
        }
        fn ex(e: &Expr, sv: VarId, out: &mut HashSet<String>) {
            if let ExprKind::Builtin(n, a) = &e.kind {
                if n.starts_with("list.") || n.starts_with("dict.") || n.starts_with("set.") {
                    if let Some(f) = base_field(&a[0], sv) {
                        out.insert(f);
                    }
                }
            }
            if let ExprKind::Method(r, _, _) = &e.kind {
                if let Some(f) = base_field(r, sv) {
                    out.insert(f);
                }
            }
            e.children(&mut |c| ex(c, sv, out));
        }
        fn st(s: &Stmt, sv: VarId, out: &mut HashSet<String>) {
            match &s.kind {
                StmtKind::Assign(Target::Place(p), _) | StmtKind::AugAssign(p, _, _) => {
                    if let ExprKind::Field(..) = p.kind {
                        if let ExprKind::Field(o, _) = &p.kind {
                            if let Some(f) = base_field(o, sv) {
                                out.insert(f);
                            }
                        }
                    } else if let Some(f) = base_field(p, sv) {
                        out.insert(f);
                    }
                }
                _ => {}
            }
            stmt_exprs(s, &mut |e| ex(e, sv, out));
            for b in sub_blocks(s) {
                b.iter().for_each(|s| st(s, sv, out));
            }
        }
        self.f.body.iter().for_each(|s| st(s, sv, &mut out));
        out
    }

    // ---------------------------------------------------------- declarations

    /// Decide where each local is declared and whether it needs `mut`.
    fn plan_decls(&mut self) {
        // block tree: (parent block, enclosing loop stmt)
        struct B {
            loop_stmt: Option<u32>,
        }
        let mut blocks: Vec<B> = vec![B { loop_stmt: None }];
        // var → list of (block, index of child stmt in that block, stmt)
        let mut occ: HashMap<VarId, Vec<Vec<(usize, usize, u32)>>> = HashMap::new();
        let mut assigns: HashMap<VarId, Vec<Vec<(usize, usize, u32)>>> = HashMap::new();
        fn walk(
            s: &[Stmt],
            blk: usize,
            path: &mut Vec<(usize, usize, u32)>,
            blocks: &mut Vec<B>,
            occ: &mut HashMap<VarId, Vec<Vec<(usize, usize, u32)>>>,
            assigns: &mut HashMap<VarId, Vec<Vec<(usize, usize, u32)>>>,
        ) {
            for (i, st) in s.iter().enumerate() {
                path.push((blk, i, st.id));
                let mut vs = vec![];
                stmt_exprs(st, &mut |e| uses(e, &mut vs));
                match &st.kind {
                    StmtKind::Assign(t, _) => {
                        let mut d = vec![];
                        t.vars(&mut d);
                        for v in &d {
                            assigns.entry(*v).or_default().push(path.clone());
                        }
                        vs.extend(d);
                    }
                    StmtKind::AugVar(v, _, _) => {
                        vs.push(*v);
                        assigns.entry(*v).or_default().push(path.clone());
                    }
                    _ => {}
                }
                for v in vs {
                    occ.entry(v).or_default().push(path.clone());
                }
                let is_loop = matches!(st.kind, StmtKind::While(..) | StmtKind::For(..));
                for (bi, b) in sub_blocks(st).into_iter().enumerate() {
                    let _ = blk;
                    blocks.push(B { loop_stmt: if is_loop && bi == 0 { Some(st.id) } else { None } });
                    let nb = blocks.len() - 1;
                    walk(b, nb, path, blocks, occ, assigns);
                }
                path.pop();
            }
        }
        let mut path = vec![];
        walk(&self.f.body, 0, &mut path, &mut blocks, &mut occ, &mut assigns);
        for (v, paths) in &occ {
            let var = &self.f.vars[*v];
            if var.kind != VarKind::Local {
                continue;
            }
            // deepest common prefix of all occurrence paths
            let mut common = paths[0].len();
            for p in paths {
                let mut k = 0;
                while k < common && k < p.len() && p[k] == paths[0][k] {
                    k += 1;
                }
                common = k;
            }
            // the declaring block is the block of path element `common`
            // (or of the last common statement's sub-block)
            let mut level = common; // statements paths[0][..common] are shared
            loop {
                let blk = if level < paths[0].len() { paths[0][level].0 } else { paths[0][level - 1].0 };
                // a variable read before written in a loop body lives across iterations
                let lp = blocks[blk].loop_stmt;
                match lp {
                    Some(ls) if self.o.loop_live.get(&ls).map(|l| l.contains(v)).unwrap_or(false) && level > 0 => {
                        level -= 1;
                        // move to the block containing the loop statement
                        while level > 0 && paths[0][level].2 != ls {
                            level -= 1;
                        }
                        if paths[0][level].2 == ls {
                            // declare before the loop statement (at this level)
                        }
                        continue;
                    }
                    _ => {}
                }
                break;
            }
            let level = level.min(paths[0].len() - 1);
            let first = paths.iter().map(|p| p[level].1).min().unwrap();
            let stmt_id = paths.iter().find(|p| p[level].1 == first).unwrap()[level].2;
            // `let` at the assignment if it is the first occurrence, at this level, a plain assignment
            let at_assign = paths.iter().all(|p| p.len() > level && (p[level].1 > first || p.len() == level + 1 || p[level].1 == first))
                && self.is_plain_assign_of(stmt_id, *v)
                && paths.iter().filter(|p| p[level].1 == first).all(|p| p.len() == level + 1);
            self.decl.insert(*v, if at_assign { Decl::At(stmt_id) } else { Decl::Before(stmt_id) });
            // mut: a path may assign twice (branches of an `if` are exclusive)
            let sid = match &self.decl[v] {
                Decl::At(s) | Decl::Before(s) => *s,
            };
            if let Some(blk) = block_of(&self.f.body, sid) {
                if assign_count(blk, *v) > 1 {
                    self.muts.insert(*v);
                }
            }
        }
        // a variable assigned once through a hoisted declaration needs no mut;
        // modified objects always do
        for v in &self.o.mut_vars[self.fid] {
            self.muts.insert(*v);
        }
        // rebound parameters
        for (v, n) in assigns.iter() {
            if self.f.vars[*v].kind == VarKind::Param && !n.is_empty() {
                self.muts.insert(*v);
            }
        }
    }

    fn is_plain_assign_of(&self, sid: u32, v: VarId) -> bool {
        fn find(s: &[Stmt], id: u32) -> Option<&Stmt> {
            for st in s {
                if st.id == id {
                    return Some(st);
                }
                for b in sub_blocks(st) {
                    if let Some(x) = find(b, id) {
                        return Some(x);
                    }
                }
            }
            None
        }
        match find(&self.f.body, sid).map(|s| &s.kind) {
            Some(StmtKind::Assign(Target::Var(w), _)) => *w == v,
            Some(StmtKind::Assign(Target::Tuple(ts), _)) => {
                ts.iter().all(|t| matches!(t, Target::Var(_)))
                    && ts.iter().any(|t| matches!(t, Target::Var(w) if *w == v))
            }
            _ => false,
        }
    }

    fn pre_decls(&mut self, sid: u32, ind: &str, out: &mut String) {
        let mut vs: Vec<VarId> = self
            .decl
            .iter()
            .filter(|(_, d)| **d == Decl::Before(sid))
            .map(|(v, _)| *v)
            .collect();
        vs.sort();
        for v in vs {
            if self.declared.insert(v) {
                let m = if self.muts.contains(&v) { "mut " } else { "" };
                out.push_str(&format!("{}let {}{}: {};\n", ind, m, self.name(v), self.ty(&self.f.vars[v].ty)));
                self.kinds.insert(v, Kind::Place);
            }
        }
    }

    // ------------------------------------------------------------ statements

    fn block(&mut self, s: &[Stmt], ind: &str, out: &mut String) -> Res<()> {
        for st in s {
            self.stmt(st, ind, out)?;
        }
        Ok(())
    }

    fn label(&mut self) -> String {
        self.labels += 1;
        if self.labels == 1 {
            "'done".into()
        } else {
            format!("'done{}", self.labels)
        }
    }

    fn stmt(&mut self, s: &Stmt, ind: &str, out: &mut String) -> Res<()> {
        self.pre_decls(s.id, ind, out);
        let inner = format!("{}    ", ind);
        match &s.kind {
            StmtKind::Expr(e) => {
                let v = self.ex(e)?;
                for p in std::mem::take(&mut self.pre) {
                    out.push_str(&format!("{}{}\n", ind, p));
                }
                out.push_str(&format!("{}{};\n", ind, v.code));
            }
            StmtKind::Pass => {}
            StmtKind::Break => match self.loop_labels.last() {
                Some(Some(l)) => out.push_str(&format!("{}break {};\n", ind, l)),
                _ => out.push_str(&format!("{}break;\n", ind)),
            },
            StmtKind::Continue => out.push_str(&format!("{}continue;\n", ind)),
            StmtKind::Return(None) => out.push_str(&format!("{}return;\n", ind)),
            StmtKind::Return(Some(e)) => {
                let v = self.owned(e)?;
                out.push_str(&format!("{}return {};\n", ind, v));
            }
            StmtKind::Assign(t, v) => self.assign(s, t, v, ind, out)?,
            StmtKind::AugAssign(t, op, v) => self.aug_assign(t, *op, v, ind, out)?,
            StmtKind::AugVar(v, op, e) => {
                let name = self.name(*v);
                let t = self.f.vars[*v].ty.clone();
                let target = R { code: name.clone(), kind: Kind::Place, prec: ATOM };
                let line = self.aug_code(&target, &t, *op, e)?;
                out.push_str(&format!("{}{}\n", ind, line));
            }
            StmtKind::If(c, a, b) => {
                let cv = self.cond(c)?;
                out.push_str(&format!("{}if {} {{\n", ind, cv));
                self.block(a, &inner, out)?;
                let mut els = b;
                loop {
                    if els.is_empty() {
                        out.push_str(&format!("{}}}\n", ind));
                        break;
                    }
                    if els.len() == 1 {
                        if let StmtKind::If(c2, a2, b2) = &els[0].kind {
                            if self.no_decls(els[0].id) {
                                let cv = self.cond(c2)?;
                                out.push_str(&format!("{}}} else if {} {{\n", ind, cv));
                                self.block(a2, &inner, out)?;
                                els = b2;
                                continue;
                            }
                        }
                    }
                    out.push_str(&format!("{}}} else {{\n", ind));
                    self.block(els, &inner, out)?;
                    out.push_str(&format!("{}}}\n", ind));
                    break;
                }
            }
            StmtKind::While(c, body, orelse) => {
                let forever = matches!(c.kind, ExprKind::Bool(true));
                let label = if orelse.is_empty() || !has_break(body) { None } else { Some(self.label()) };
                let (ind2, inner2) = match &label {
                    Some(l) => {
                        out.push_str(&format!("{}{}: {{\n", ind, l));
                        (inner.clone(), format!("{}    ", inner))
                    }
                    None => (ind.to_string(), inner.clone()),
                };
                if forever {
                    out.push_str(&format!("{}loop {{\n", ind2));
                } else {
                    let cv = self.cond(c)?;
                    out.push_str(&format!("{}while {} {{\n", ind2, cv));
                }
                let mut b = String::new();
                self.loop_labels.push(label.clone());
                self.block(body, &inner2, &mut b)?;
                self.loop_labels.pop();
                out.push_str(&b);
                out.push_str(&format!("{}}}\n", ind2));
                self.block(orelse, &ind2, out)?;
                if label.is_some() {
                    out.push_str(&format!("{}}}\n", ind));
                }
            }
            StmtKind::For(t, it, body, orelse) => {
                let label = if orelse.is_empty() || !has_break(body) { None } else { Some(self.label()) };
                let (ind2, inner2) = match &label {
                    Some(l) => {
                        out.push_str(&format!("{}{}: {{\n", ind, l));
                        (inner.clone(), format!("{}    ", inner))
                    }
                    None => (ind.to_string(), inner.clone()),
                };
                let by_mut = self.o.iter_mut.contains(&s.id);
                let (code, item) = self.iter_code(it, by_mut, true)?;
                self.mark_nonneg(t, it);
                let pat = self.pat(t, &item)?;
                out.push_str(&format!("{}for {} in {} {{\n", ind2, pat, code));
                let mut b = String::new();
                self.rebind_loop_vars(t, &inner2, &mut b);
                self.loop_labels.push(label.clone());
                self.block(body, &inner2, &mut b)?;
                self.loop_labels.pop();
                out.push_str(&b);
                out.push_str(&format!("{}}}\n", ind2));
                self.block(orelse, &ind2, out)?;
                if label.is_some() {
                    out.push_str(&format!("{}}}\n", ind));
                }
            }
            StmtKind::Raise(name, msg) => {
                let m = match msg {
                    Some(m) => self.borrow_str(m)?,
                    None => "\"\"".into(),
                };
                out.push_str(&format!("{}pyrt::raise({}, {});\n", ind, rust_str(name), m));
            }
            StmtKind::Assert(c, m) => {
                let cv = self.cond(c)?;
                match m {
                    Some(m) => {
                        let mv = self.display(m)?;
                        out.push_str(&format!("{}assert!({}, \"AssertionError: {{}}\", {});\n", ind, cv, mv));
                    }
                    None => out.push_str(&format!("{}assert!({});\n", ind, cv)),
                }
            }
            StmtKind::Del(e) => {
                let ExprKind::Index(b, k) = &e.kind else { unreachable!() };
                let base = self.ex(b)?;
                match &b.ty {
                    Ty::List(_) => {
                        let i = self.index_code(&base, k)?;
                        out.push_str(&format!("{}{}.remove({});\n", ind, par(&base, ATOM), i));
                    }
                    _ => {
                        let kb = self.borrow(k)?;
                        out.push_str(&format!("{}pyrt::dict_pop({}, {});\n", ind, self.mut_ref_of(&base), kb));
                    }
                }
            }
        }
        Ok(())
    }

    fn no_decls(&self, sid: u32) -> bool {
        !self.decl.values().any(|d| *d == Decl::Before(sid))
    }

    /// Loop variables assigned in the body get a mutable copy.
    fn rebind_loop_vars(&mut self, t: &Target, ind: &str, out: &mut String) {
        let mut vs = vec![];
        t.vars(&mut vs);
        for v in vs {
            if self.o.rebound[self.fid].contains(&v) {
                let name = self.name(v);
                let k = self.kinds.get(&v).copied().unwrap_or(Kind::Place);
                let ty = &self.f.vars[v].ty;
                let val = match k {
                    Kind::Ref | Kind::Mut if ty.is_copy() => format!("*{}", name),
                    Kind::Ref | Kind::Mut => format!("{}.clone()", name),
                    _ => name.clone(),
                };
                out.push_str(&format!("{}let mut {} = {};\n", ind, name, val));
                self.kinds.insert(v, Kind::Place);
                self.nonneg.remove(&v);
            }
        }
    }

    fn mark_nonneg(&mut self, t: &Target, it: &Iter) {
        match (t, it) {
            (Target::Var(v), Iter::Range(a, _, step)) => {
                let a_ok = matches!(a.kind, ExprKind::Int(x) if x >= 0) || self.is_nonneg(a);
                let s_ok = match step {
                    None => true,
                    Some(s) => matches!(s.kind, ExprKind::Int(x) if x > 0),
                };
                if a_ok && s_ok && !self.o.rebound[self.fid].contains(v) {
                    self.nonneg.insert(*v);
                }
            }
            (Target::Var(v), Iter::Reversed(i)) => {
                if let Iter::Range(a, _, None) = &**i {
                    if (matches!(a.kind, ExprKind::Int(x) if x >= 0) || self.is_nonneg(a)) && !self.o.rebound[self.fid].contains(v) {
                        self.nonneg.insert(*v);
                    }
                }
            }
            (Target::Tuple(ts), Iter::Enumerate(_, start)) => {
                let ok = match start {
                    None => true,
                    Some(s) => matches!(s.kind, ExprKind::Int(x) if x >= 0),
                };
                if let (true, Target::Var(v)) = (ok, &ts[0]) {
                    if !self.o.rebound[self.fid].contains(v) {
                        self.nonneg.insert(*v);
                    }
                }
            }
            _ => {}
        }
    }

    fn is_nonneg(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Int(x) => *x >= 0,
            ExprKind::Var(v) => self.nonneg.contains(v),
            ExprKind::Builtin("len", _) => true,
            ExprKind::Bin(BinOp::Add | BinOp::Mul, a, b) => self.is_nonneg(a) && self.is_nonneg(b),
            _ => false,
        }
    }

    fn assign(&mut self, s: &Stmt, t: &Target, v: &Expr, ind: &str, out: &mut String) -> Res<()> {
        match t {
            Target::Var(w) => {
                let val = self.owned(v)?;
                let name = self.name(*w);
                if self.decl.get(w) == Some(&Decl::At(s.id)) && !self.declared.contains(w) {
                    self.declared.insert(*w);
                    self.kinds.insert(*w, Kind::Place);
                    let m = if self.muts.contains(w) { "mut " } else { "" };
                    let ann = if needs_annotation(v) { format!(": {}", self.ty(&v.ty)) } else { String::new() };
                    out.push_str(&format!("{}let {}{}{} = {};\n", ind, m, name, ann, val));
                } else {
                    out.push_str(&format!("{}{} = {};\n", ind, name, val));
                }
            }
            Target::Place(pl) => self.store(pl, v, ind, out)?,
            Target::Tuple(ts) => {
                let all_vars = ts.iter().all(|t| matches!(t, Target::Var(_)));
                let vars: Vec<VarId> = ts.iter().filter_map(|t| if let Target::Var(v) = t { Some(*v) } else { None }).collect();
                let all_new = all_vars && vars.iter().all(|w| self.decl.get(w) == Some(&Decl::At(s.id)) && !self.declared.contains(w));
                let all_old = all_vars && vars.iter().all(|w| self.declared.contains(w));
                let val = self.owned(v)?;
                if all_new {
                    let pats: Vec<String> = vars
                        .iter()
                        .map(|w| {
                            let m = if self.muts.contains(w) { "mut " } else { "" };
                            format!("{}{}", m, self.name(*w))
                        })
                        .collect();
                    for w in &vars {
                        self.declared.insert(*w);
                        self.kinds.insert(*w, Kind::Place);
                    }
                    out.push_str(&format!("{}let ({}) = {};\n", ind, pats.join(", "), val));
                } else if all_old {
                    let names: Vec<String> = vars.iter().map(|w| self.name(*w)).collect();
                    out.push_str(&format!("{}({}) = {};\n", ind, names.join(", "), val));
                } else {
                    self.tmp += 1;
                    let tmp = format!("t{}", self.tmp);
                    out.push_str(&format!("{}let {} = {};\n", ind, tmp, val));
                    self.assign_parts(s, t, &tmp, ind, out)?;
                }
            }
        }
        Ok(())
    }

    fn assign_parts(&mut self, s: &Stmt, t: &Target, src: &str, ind: &str, out: &mut String) -> Res<()> {
        let Target::Tuple(ts) = t else { unreachable!() };
        for (i, t) in ts.iter().enumerate() {
            let part = format!("{}.{}", src, i);
            match t {
                Target::Var(w) => {
                    let name = self.name(*w);
                    if self.decl.get(w) == Some(&Decl::At(s.id)) && !self.declared.contains(w) {
                        self.declared.insert(*w);
                        self.kinds.insert(*w, Kind::Place);
                        let m = if self.muts.contains(w) { "mut " } else { "" };
                        out.push_str(&format!("{}let {}{} = {};\n", ind, m, name, part));
                    } else {
                        out.push_str(&format!("{}{} = {};\n", ind, name, part));
                    }
                }
                Target::Place(pl) => {
                    let code = R { code: part, kind: Kind::Val, prec: ATOM };
                    self.store_code(pl, code, ind, out)?;
                }
                Target::Tuple(_) => self.assign_parts(s, t, &part, ind, out)?,
            }
        }
        Ok(())
    }

    /// `xs[i] = v`, `d[k] = v`, `obj.f = v`
    fn store(&mut self, pl: &Expr, v: &Expr, ind: &str, out: &mut String) -> Res<()> {
        let val = self.owned(v)?;
        self.store_code(pl, R { code: val, kind: Kind::Val, prec: LOW }, ind, out)
    }

    fn store_code(&mut self, pl: &Expr, val: R, ind: &str, out: &mut String) -> Res<()> {
        match &pl.kind {
            ExprKind::Index(b, k) => {
                let base = self.ex(b)?;
                match &b.ty {
                    Ty::Dict(..) => {
                        let key = self.owned(k)?;
                        out.push_str(&format!("{}{}.insert({}, {});\n", ind, par(&base, ATOM), key, val.code));
                    }
                    _ => {
                        let i = self.index_code(&base, k)?;
                        out.push_str(&format!("{}{}[{}] = {};\n", ind, par(&base, ATOM), i, val.code));
                    }
                }
            }
            ExprKind::Field(o, f) => {
                if let Some(local) = self.init_field(o, f) {
                    let declared = self.init_fields.contains_key(&format!("#declared:{}", f));
                    if declared {
                        out.push_str(&format!("{}{} = {};\n", ind, local, val.code));
                    } else {
                        out.push_str(&format!("{}let {} = {};\n", ind, local, val.code));
                        self.init_fields_declared(f);
                    }
                    return Ok(());
                }
                let obj = self.ex(o)?;
                out.push_str(&format!("{}{}.{} = {};\n", ind, par(&obj, ATOM), ident(f), val.code));
            }
            _ => return err(pl.line, "unsupported assignment target"),
        }
        Ok(())
    }

    fn init_field(&self, o: &Expr, f: &str) -> Option<String> {
        if !self.f.is_init {
            return None;
        }
        match o.kind {
            ExprKind::Var(v) if Some(v) == self.f.self_var => self.init_fields.get(f).cloned(),
            _ => None,
        }
    }

    fn aug_assign(&mut self, t: &Expr, op: BinOp, v: &Expr, ind: &str, out: &mut String) -> Res<()> {
        // containers: `xs += ys`
        match (&t.ty, op) {
            (Ty::List(_), BinOp::Add) => {
                let target = self.ex(t)?;
                let src = self.extend_arg(v)?;
                out.push_str(&format!("{}{}.extend({});\n", ind, par(&target, ATOM), src));
                return Ok(());
            }
            (Ty::List(_), BinOp::Mul) => {
                let target = self.ex(t)?;
                let n = self.val(v)?;
                let tb = self.borrow_r(&target);
                out.push_str(&format!("{}{} = pyrt::repeat({}, {});\n", ind, target.code, tb, n.code));
                return Ok(());
            }
            (Ty::Set(_), BinOp::BitOr) => {
                let target = self.ex(t)?;
                let src = self.extend_arg(v)?;
                out.push_str(&format!("{}{}.extend({});\n", ind, par(&target, ATOM), src));
                return Ok(());
            }
            (Ty::Set(_), _) => {
                let target = self.ex(t)?;
                let o = self.borrow(v)?;
                let sym = crate::front::opstr(op);
                out.push_str(&format!("{}{} = &{} {} {};\n", ind, target.code, par(&target, UNARY), sym, o));
                return Ok(());
            }
            _ => {}
        }
        let target = match &t.kind {
            ExprKind::Field(o, f) if self.init_field(o, f).is_some() => {
                R { code: self.init_field(o, f).unwrap(), kind: Kind::Place, prec: ATOM }
            }
            ExprKind::Index(b, k) if matches!(b.ty, Ty::Dict(..)) => {
                let base = self.ex(b)?;
                let kb = self.borrow(k)?;
                R { code: format!("{}[{}]", par(&base, ATOM), kb), kind: Kind::Place, prec: ATOM }
            }
            _ => self.ex(t)?,
        };
        let line = self.aug_code(&target, &t.ty, op, v)?;
        out.push_str(&format!("{}{}\n", ind, line));
        Ok(())
    }

    /// `x op= v` for a numeric / string target.
    fn aug_code(&mut self, target: &R, ty: &Ty, op: BinOp, v: &Expr) -> Res<String> {
        let tc = &target.code;
        match ty {
            Ty::Str => match op {
                BinOp::Add => {
                    let b = self.borrow_str(v)?;
                    Ok(format!("{} += {};", tc, b))
                }
                _ => {
                    let n = self.val(v)?;
                    Ok(format!("{} = pyrt::repeat_str(&{}, {});", tc, tc, n.code))
                }
            },
            Ty::Int | Ty::Float | Ty::Bool => {
                let rhs = self.val(v)?;
                let native = match (ty, op) {
                    (Ty::Int, BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor) => true,
                    (Ty::Float, BinOp::Add | BinOp::Sub | BinOp::Mul) => true,
                    (Ty::Float, BinOp::Div) => self.nonzero_lit(v),
                    (Ty::Bool, BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor) => true,
                    _ => false,
                };
                if native {
                    Ok(format!("{} {}= {};", tc, crate::front::opstr(op), rhs.code))
                } else {
                    let cur = R { code: tc.clone(), kind: Kind::Place, prec: ATOM };
                    let e = self.arith(op, ty, &cur, ty, &rhs, v)?;
                    Ok(format!("{} = {};", tc, e.code))
                }
            }
            _ => err(v.line, "unsupported augmented assignment"),
        }
    }

    /// Argument of `extend` (an iterable whose items are stored).
    fn extend_arg(&mut self, v: &Expr) -> Res<String> {
        if let ExprKind::Iter(it) = &v.kind {
            if let Iter::Over(x) = &**it {
                return self.extend_arg(x);
            }
            let (code, item) = self.iter_code(it, false, false)?;
            return Ok(match item {
                Item::Ref => {
                    if v.ty.is_copy() {
                        format!("{}.copied()", code)
                    } else {
                        format!("{}.cloned()", code)
                    }
                }
                _ => code,
            });
        }
        let x = self.ex(v)?;
        if self.o.moves.contains(&v.id) || x.kind == Kind::Val {
            return Ok(x.code);
        }
        let elem_copy = match &v.ty {
            Ty::List(e) | Ty::Set(e) => e.is_copy(),
            _ => false,
        };
        Ok(format!("{}.iter(){}", par(&x, ATOM), if elem_copy { ".copied()" } else { ".cloned()" }))
    }

    // ----------------------------------------------------------- expressions

    /// A condition (bool).
    fn cond(&mut self, e: &Expr) -> Res<String> {
        let v = self.val(e)?;
        Ok(v.code)
    }

    fn kind_of(&self, v: VarId) -> Kind {
        self.kinds.get(&v).copied().unwrap_or(Kind::Place)
    }

    /// Value of a Copy-typed expression.
    fn val(&mut self, e: &Expr) -> Res<R> {
        let x = self.ex(e)?;
        Ok(self.val_r(x))
    }

    fn val_r(&self, x: R) -> R {
        match x.kind {
            Kind::Ref | Kind::Mut => r(format!("*{}", par(&x, UNARY)), Kind::Val, UNARY),
            _ => x,
        }
    }

    /// An owned value of any type.
    fn owned(&mut self, e: &Expr) -> Res<String> {
        if let ExprKind::IfExp(c, a, b) = &e.kind {
            let cv = self.cond(c)?;
            let av = self.owned(a)?;
            let bv = self.owned(b)?;
            return Ok(format!("if {} {{ {} }} else {{ {} }}", cv, av, bv));
        }
        let x = self.ex(e)?;
        Ok(self.owned_r(&x, &e.ty, self.o.moves.contains(&e.id)))
    }

    fn owned_r(&self, x: &R, t: &Ty, moved: bool) -> String {
        if t.is_copy() {
            return self.val_r(x.clone()).code;
        }
        match x.kind {
            Kind::Val => x.code.clone(),
            Kind::Place if moved => x.code.clone(),
            Kind::Place => format!("{}.clone()", par(x, ATOM)),
            Kind::Ref | Kind::Mut => match t {
                Ty::Str => format!("{}.to_string()", par(x, ATOM)),
                Ty::List(_) => format!("{}.to_vec()", par(x, ATOM)),
                _ => format!("{}.clone()", par(x, ATOM)),
            },
        }
    }

    /// `&T` for a function argument.
    fn borrow(&mut self, e: &Expr) -> Res<String> {
        let x = self.ex(e)?;
        Ok(self.borrow_r(&x))
    }

    fn borrow_r(&self, x: &R) -> String {
        match x.kind {
            Kind::Ref | Kind::Mut => x.code.clone(),
            _ => format!("&{}", par(x, UNARY)),
        }
    }

    /// A `&str` for a string expression.
    fn borrow_str(&mut self, e: &Expr) -> Res<String> {
        let x = self.ex(e)?;
        Ok(match x.kind {
            Kind::Ref | Kind::Mut => x.code,
            _ => format!("&{}", par(&x, UNARY)),
        })
    }

    fn mut_ref_of(&self, x: &R) -> String {
        match x.kind {
            Kind::Mut => x.code.clone(),
            _ => format!("&mut {}", par(x, UNARY)),
        }
    }

    /// Argument for `{}` in a format string.
    fn display(&mut self, e: &Expr) -> Res<String> {
        let x = self.ex(e)?;
        Ok(match &e.ty {
            Ty::Int => self.val_r(x).code,
            Ty::Str => x.code,
            _ => format!("pyrt::s({})", self.borrow_r(&x)),
        })
    }

    fn ex(&mut self, e: &Expr) -> Res<R> {
        use ExprKind as K;
        Ok(match &e.kind {
            K::Int(v) => {
                if *v < 0 {
                    r(v.to_string(), Kind::Val, UNARY)
                } else {
                    r(v.to_string(), Kind::Val, ATOM)
                }
            }
            K::Float(v) => {
                let s = float_lit(*v);
                let p = if s.starts_with('-') { UNARY } else { ATOM };
                r(s, Kind::Val, p)
            }
            K::Bool(b) => r(b.to_string(), Kind::Val, ATOM),
            K::Str(s) => r(rust_str(s), Kind::Ref, ATOM),
            K::NoneLit => r("()", Kind::Val, ATOM),
            K::Var(v) => r(self.name(*v), self.kind_of(*v), ATOM),
            K::Const(c) => {
                let cst = &self.p.consts[*c];
                let k = if cst.value.ty == Ty::Str { Kind::Ref } else { Kind::Val };
                r(ident(&cst.name), k, ATOM)
            }
            K::ToFloat(x) => {
                if let K::Int(v) = x.kind {
                    r(float_lit(v as f64), Kind::Val, if v < 0 { UNARY } else { ATOM })
                } else {
                    let v = self.val(x)?;
                    r(format!("{} as f64", par(&v, CAST)), Kind::Val, CAST)
                }
            }
            K::Bin(op, a, b) => self.binary(*op, a, b, e)?,
            K::Unary(op, x) => {
                let v = self.val(x)?;
                match op {
                    UnOp::Neg => r(format!("-{}", par(&v, UNARY)), Kind::Val, UNARY),
                    UnOp::Pos => v,
                    UnOp::Invert | UnOp::Not => r(format!("!{}", par(&v, UNARY)), Kind::Val, UNARY),
                }
            }
            K::Cmp(first, rest) => self.compare(first, rest)?,
            K::And(v) | K::Or(v) => {
                let (sym, p) = if matches!(e.kind, K::And(_)) { ("&&", AND) } else { ("||", OR) };
                let mut parts = vec![];
                for x in v {
                    let c = self.val(x)?;
                    parts.push(par(&c, p + 1));
                }
                r(parts.join(&format!(" {} ", sym)), Kind::Val, p)
            }
            K::IfExp(c, a, b) => {
                let cv = self.cond(c)?;
                let (av, bv) = if e.ty.is_copy() {
                    (self.val(a)?.code, self.val(b)?.code)
                } else {
                    (self.owned(a)?, self.owned(b)?)
                };
                r(format!("if {} {{ {} }} else {{ {} }}", cv, av, bv), Kind::Val, LOW)
            }
            K::Call(fid, args) => {
                let f = &self.p.funcs[*fid];
                let name = match f.class {
                    Some(c) => format!("{}::{}", self.p.classes[c].name, ident(&f.name)),
                    None if f.name == "main" => "py_main".into(),
                    None => ident(&f.name),
                };
                let a = self.args(*fid, args)?;
                r(format!("{}({})", name, a), Kind::Val, ATOM)
            }
            K::Method(recv, fid, args) => {
                let rv = self.ex(recv)?;
                let a = self.args(*fid, args)?;
                let name = ident(&self.p.funcs[*fid].name);
                r(format!("{}.{}({})", par(&rv, ATOM), name, a), Kind::Val, ATOM)
            }
            K::New(c, args) => {
                let cl = &self.p.classes[*c];
                if let Some(init) = cl.init {
                    let a = self.args(init, args)?;
                    r(format!("{}::new({})", cl.name, a), Kind::Val, ATOM)
                } else {
                    let mut fs = vec![];
                    for (fd, a) in cl.fields.iter().zip(args.iter()) {
                        let v = self.owned(a)?;
                        let n = ident(&fd.name);
                        if v == n {
                            fs.push(n);
                        } else {
                            fs.push(format!("{}: {}", n, v));
                        }
                    }
                    if fs.is_empty() {
                        r(format!("{} {{}}", cl.name), Kind::Val, ATOM)
                    } else {
                        r(format!("{} {{ {} }}", cl.name, fs.join(", ")), Kind::Val, ATOM)
                    }
                }
            }
            K::Field(o, f) => {
                if let Some(local) = self.init_field(o, f) {
                    return Ok(r(local, Kind::Place, ATOM));
                }
                let ov = self.ex(o)?;
                r(format!("{}.{}", par(&ov, ATOM), ident(f)), Kind::Place, ATOM)
            }
            K::TupleField(o, i) => {
                let ov = self.ex(o)?;
                r(format!("{}.{}", par(&ov, ATOM), i), Kind::Place, ATOM)
            }
            K::Index(b, k) => {
                let base = self.ex(b)?;
                match &b.ty {
                    Ty::Str => {
                        let i = self.val(k)?;
                        r(format!("pyrt::str_at({}, {})", self.borrow_r(&base), i.code), Kind::Val, ATOM)
                    }
                    Ty::Dict(..) => {
                        let kb = self.borrow(k)?;
                        r(format!("{}[{}]", par(&base, ATOM), kb), Kind::Place, ATOM)
                    }
                    _ => {
                        let i = self.index_code(&base, k)?;
                        r(format!("{}[{}]", par(&base, ATOM), i), Kind::Place, ATOM)
                    }
                }
            }
            K::Slice(b, lo, hi, st) => {
                let base = self.ex(b)?;
                let opt = |x: &Option<Box<Expr>>, g: &mut Self| -> Res<String> {
                    Ok(match x {
                        Some(e) => format!("Some({})", g.val(e)?.code),
                        None => "None".into(),
                    })
                };
                let (l, h, s) = (opt(lo, self)?, opt(hi, self)?, opt(st, self)?);
                let f = if b.ty == Ty::Str { "pyrt::str_slice" } else { "pyrt::slice" };
                r(format!("{}({}, {}, {}, {})", f, self.borrow_r(&base), l, h, s), Kind::Val, ATOM)
            }
            K::List(v) => {
                if v.is_empty() {
                    r("Vec::new()", Kind::Val, ATOM)
                } else {
                    let mut xs = vec![];
                    for x in v {
                        xs.push(self.owned(x)?);
                    }
                    r(format!("vec![{}]", xs.join(", ")), Kind::Val, ATOM)
                }
            }
            K::Tuple(v) => {
                let mut xs = vec![];
                for x in v {
                    xs.push(self.owned(x)?);
                }
                if xs.len() == 1 {
                    r(format!("({},)", xs[0]), Kind::Val, ATOM)
                } else {
                    r(format!("({})", xs.join(", ")), Kind::Val, ATOM)
                }
            }
            K::Set(v) => {
                if v.is_empty() {
                    r("IndexSet::new()", Kind::Val, ATOM)
                } else {
                    let mut xs = vec![];
                    for x in v {
                        xs.push(self.owned(x)?);
                    }
                    r(format!("IndexSet::from([{}])", xs.join(", ")), Kind::Val, ATOM)
                }
            }
            K::Dict(v) => {
                if v.is_empty() {
                    r("IndexMap::new()", Kind::Val, ATOM)
                } else {
                    let mut xs = vec![];
                    for (k, x) in v {
                        let kk = self.owned(k)?;
                        let vv = self.owned(x)?;
                        xs.push(format!("({}, {})", kk, vv));
                    }
                    r(format!("IndexMap::from([{}])", xs.join(", ")), Kind::Val, ATOM)
                }
            }
            K::Comp(c) => self.comp(c, &e.ty)?,
            K::FStr(parts) => self.fstring(parts)?,
            K::Builtin(name, args) => self.builtin(name, args, e)?,
            K::Lambda(..) => return err(e.line, "lambda outside sort/min/max"),
            K::Iter(it) => {
                let (code, item) = self.iter_code(it, false, false)?;
                let code = match item {
                    Item::Ref if e.ty.is_copy() => format!("{}.copied()", code),
                    Item::Ref => format!("{}.cloned()", code),
                    _ => code,
                };
                r(format!("{}.collect::<Vec<{}>>()", code, self.ty(&e.ty)), Kind::Val, ATOM)
            }
        })
    }

    /// Rust index for a list index expression.
    fn index_code(&mut self, base: &R, k: &Expr) -> Res<String> {
        if let ExprKind::Int(v) = k.kind {
            if v >= 0 {
                return Ok(v.to_string());
            }
            if base.prec == ATOM && !base.code.contains('(') {
                return Ok(format!("{}.len() - {}", base.code, -v));
            }
        }
        let i = self.val(k)?;
        if self.is_nonneg(k) {
            return Ok(format!("{} as usize", par(&i, CAST)));
        }
        Ok(format!("pyrt::idx({}, {}.len())", i.code, par(base, ATOM)))
    }

    fn nonzero_lit(&self, e: &Expr) -> bool {
        match &e.kind {
            ExprKind::Int(v) => *v != 0,
            ExprKind::Float(v) => *v != 0.0,
            ExprKind::ToFloat(x) => self.nonzero_lit(x),
            _ => false,
        }
    }

    fn binary(&mut self, op: BinOp, a: &Expr, b: &Expr, e: &Expr) -> Res<R> {
        match (&a.ty, &b.ty) {
            (Ty::Str, Ty::Str) => {
                // a + b
                let (x, y) = (self.ex(a)?, self.ex(b)?);
                return Ok(r(format!("format!(\"{{}}{{}}\", {}, {})", x.code, y.code), Kind::Val, ATOM));
            }
            (Ty::Str, Ty::Int) => {
                let s = self.borrow_str(a)?;
                let n = self.val(b)?;
                return Ok(r(format!("pyrt::repeat_str({}, {})", s, n.code), Kind::Val, ATOM));
            }
            (Ty::List(_), Ty::List(_)) => {
                let x = self.borrow(a)?;
                let y = self.borrow(b)?;
                return Ok(r(format!("pyrt::concat({}, {})", x, y), Kind::Val, ATOM));
            }
            (Ty::List(et), Ty::Int) => {
                let n = self.val(b)?;
                if let ExprKind::List(items) = &a.kind {
                    if items.len() == 1 && et.is_copy() {
                        let x = self.owned(&items[0])?;
                        let cnt = if let ExprKind::Int(k) = b.kind {
                            k.max(0).to_string()
                        } else if self.is_nonneg(b) {
                            format!("{} as usize", par(&n, CAST))
                        } else {
                            format!("{}.max(0) as usize", par(&n, ATOM))
                        };
                        return Ok(r(format!("vec![{}; {}]", x, cnt), Kind::Val, ATOM));
                    }
                }
                let x = self.borrow(a)?;
                return Ok(r(format!("pyrt::repeat({}, {})", x, n.code), Kind::Val, ATOM));
            }
            (Ty::Set(_), Ty::Set(_)) => {
                let x = self.ex(a)?;
                let y = self.ex(b)?;
                let sym = crate::front::opstr(op);
                return Ok(r(format!("{} {} {}", self.borrow_r(&x), sym, self.borrow_r(&y)), Kind::Val, BOR));
            }
            _ => {}
        }
        let x = self.val(a)?;
        let y = self.val(b)?;
        let x = if all_literal(a) && all_literal(b) && a.ty == Ty::Int {
            if let ExprKind::Int(v) = a.kind {
                r(format!("{}_i64", v), Kind::Val, if v < 0 { UNARY } else { ATOM })
            } else {
                x
            }
        } else {
            x
        };
        self.arith(op, &a.ty, &x, &b.ty, &y, b).map(|mut v| {
            if e.ty == Ty::Bool && v.prec > BAND {
                v.prec = BOR;
            }
            v
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn arith(&mut self, op: BinOp, ta: &Ty, x: &R, _tb: &Ty, y: &R, b: &Expr) -> Res<R> {
        use BinOp::*;
        let int = *ta == Ty::Int;
        let bin = |sym: &str, p: u8| r(format!("{} {} {}", par(x, p), sym, par(y, p + 1)), Kind::Val, p);
        let posl = matches!(b.kind, ExprKind::Int(v) if v > 0);
        Ok(match op {
            Add => bin("+", ADD),
            Sub => bin("-", ADD),
            Mul => bin("*", MUL),
            BitAnd => bin("&", BAND),
            BitOr => bin("|", BOR),
            BitXor => bin("^", BXOR),
            Div if int => {
                if self.nonzero_lit(b) {
                    r(format!("{} as f64 / {}", par(x, CAST), float_lit(match b.kind { ExprKind::Int(v) => v as f64, _ => 1.0 })), Kind::Val, MUL)
                } else {
                    r(format!("pyrt::div({} as f64, {} as f64)", par(x, CAST), par(y, CAST)), Kind::Val, ATOM)
                }
            }
            Div => {
                if self.nonzero_lit(b) {
                    bin("/", MUL)
                } else {
                    r(format!("pyrt::div({}, {})", x.code, y.code), Kind::Val, ATOM)
                }
            }
            FloorDiv if int => {
                if posl {
                    r(format!("{}.div_euclid({})", recv(x), y.code), Kind::Val, ATOM)
                } else {
                    r(format!("pyrt::floordiv({}, {})", x.code, y.code), Kind::Val, ATOM)
                }
            }
            FloorDiv => r(format!("pyrt::ffloordiv({}, {})", x.code, y.code), Kind::Val, ATOM),
            Mod if int => {
                if posl {
                    r(format!("{}.rem_euclid({})", recv(x), y.code), Kind::Val, ATOM)
                } else {
                    r(format!("pyrt::modulo({}, {})", x.code, y.code), Kind::Val, ATOM)
                }
            }
            Mod => r(format!("pyrt::fmod({}, {})", x.code, y.code), Kind::Val, ATOM),
            Pow if int => {
                if let ExprKind::Int(v) = b.kind {
                    if v >= 0 && v <= u32::MAX as i64 {
                        let base = recv(x);
                        return Ok(r(format!("{}.pow({})", base, v), Kind::Val, ATOM));
                    }
                }
                r(format!("pyrt::ipow({}, {})", x.code, y.code), Kind::Val, ATOM)
            }
            Pow => {
                if matches!(b.kind, ExprKind::Float(v) if v > 0.0) || matches!(&b.kind, ExprKind::ToFloat(i) if matches!(i.kind, ExprKind::Int(v) if v > 0)) {
                    r(format!("{}.powf({})", recv(x), y.code), Kind::Val, ATOM)
                } else {
                    r(format!("pyrt::fpow({}, {})", x.code, y.code), Kind::Val, ATOM)
                }
            }
            Shl => r(format!("pyrt::shl({}, {})", x.code, y.code), Kind::Val, ATOM),
            Shr => {
                if matches!(b.kind, ExprKind::Int(v) if (0..64).contains(&v)) {
                    bin(">>", SHIFT)
                } else {
                    r(format!("pyrt::shr({}, {})", x.code, y.code), Kind::Val, ATOM)
                }
            }
        })
    }

    fn compare(&mut self, first: &Expr, rest: &[(CmpOp, Expr)]) -> Res<R> {
        let mut parts = vec![];
        let mut left = first;
        for (i, (op, right)) in rest.iter().enumerate() {
            if i > 0 && !is_simple(left) {
                return err(left.line, "chained comparisons need a simple middle operand");
            }
            parts.push(self.cmp1(*op, left, right)?);
            left = right;
        }
        if parts.len() == 1 {
            return Ok(parts.pop().unwrap());
        }
        Ok(r(parts.iter().map(|p| par(p, AND + 1)).collect::<Vec<_>>().join(" && "), Kind::Val, AND))
    }

    fn cmp1(&mut self, op: CmpOp, a: &Expr, b: &Expr) -> Res<R> {
        match op {
            CmpOp::In | CmpOp::NotIn => {
                let neg = if op == CmpOp::NotIn { "!" } else { "" };
                let c = self.ex(b)?;
                let code = match &b.ty {
                    Ty::Str => {
                        let s = self.borrow_str(a)?;
                        format!("{}.contains({})", par(&c, ATOM), s)
                    }
                    Ty::List(et) => {
                        if **et == Ty::Str {
                            let x = self.ex(a)?;
                            if x.kind == Kind::Place {
                                format!("{}.contains(&{})", par(&c, ATOM), x.code)
                            } else {
                                format!("{}.iter().any(|e| e == {})", par(&c, ATOM), x.code)
                            }
                        } else {
                            let x = self.borrow(a)?;
                            format!("{}.contains({})", par(&c, ATOM), x)
                        }
                    }
                    Ty::Set(_) => {
                        let x = self.borrow(a)?;
                        format!("{}.contains({})", par(&c, ATOM), x)
                    }
                    _ => {
                        let x = self.borrow(a)?;
                        format!("{}.contains_key({})", par(&c, ATOM), x)
                    }
                };
                Ok(r(format!("{}{}", neg, code), Kind::Val, if neg.is_empty() { ATOM } else { UNARY }))
            }
            _ => {
                let sym = match op {
                    CmpOp::Eq => "==",
                    CmpOp::Ne => "!=",
                    CmpOp::Lt => "<",
                    CmpOp::Le => "<=",
                    CmpOp::Gt => ">",
                    CmpOp::Ge => ">=",
                    _ => unreachable!(),
                };
                if a.ty.is_copy() {
                    let x = self.val(a)?;
                    let y = self.val(b)?;
                    return Ok(r(format!("{} {} {}", par(&x, CMP + 1), sym, par(&y, CMP + 1)), Kind::Val, CMP));
                }
                let x = self.ex(a)?;
                let y = self.ex(b)?;
                let refy = |k: Kind| matches!(k, Kind::Ref | Kind::Mut);
                let (xs, ys) = match (&a.ty, refy(x.kind), refy(y.kind)) {
                    // String vs &str and Vec vs slice compare through their unsized forms
                    (Ty::Str | Ty::List(_), rx, ry) if rx != ry && !(matches!(op, CmpOp::Eq | CmpOp::Ne) && a.ty == Ty::Str) => {
                        (format!("{}[..]", par(&x, ATOM)), format!("{}[..]", par(&y, ATOM)))
                    }
                    (_, true, false) if a.ty != Ty::Str => (format!("*{}", par(&x, UNARY)), par(&y, CMP + 1)),
                    (_, false, true) if a.ty != Ty::Str => (par(&x, CMP + 1), format!("*{}", par(&y, UNARY))),
                    _ => (par(&x, CMP + 1), par(&y, CMP + 1)),
                };
                if !matches!(op, CmpOp::Eq | CmpOp::Ne) && !total_order(&a.ty) {
                    let ord = match op {
                        CmpOp::Lt => "Less",
                        CmpOp::Gt => "Greater",
                        CmpOp::Le => "Greater",
                        _ => "Less",
                    };
                    let neg = matches!(op, CmpOp::Le | CmpOp::Ge);
                    let c = format!("pyrt::PyOrd::py_cmp(&{}, &{}) {} std::cmp::Ordering::{}", xs, ys, if neg { "!=" } else { "==" }, ord);
                    return Ok(r(c, Kind::Val, CMP));
                }
                Ok(r(format!("{} {} {}", xs, sym, ys), Kind::Val, CMP))
            }
        }
    }

    /// Arguments of a user function call, by parameter mode.
    fn args(&mut self, fid: FuncId, args: &[Expr]) -> Res<String> {
        let mut out = vec![];
        for (i, a) in args.iter().enumerate() {
            let s = match self.o.params[fid][i] {
                Mode::Copy => self.val(a)?.code,
                Mode::Owned => self.owned(a)?,
                Mode::Ref => self.borrow(a)?,
                Mode::Mut => {
                    let x = self.ex(a)?;
                    self.mut_ref_of(&x)
                }
            };
            out.push(s);
        }
        Ok(out.join(", "))
    }

    /// Format-string text and arguments showing `str(e)`.
    fn fmt_piece(&mut self, e: &Expr) -> Res<(String, Vec<String>)> {
        match &e.kind {
            ExprKind::Str(s) => Ok((fmt_text(s), vec![])),
            ExprKind::FStr(parts) => self.fstring_parts(parts),
            _ => Ok(("{}".into(), vec![self.display(e)?])),
        }
    }

    fn fstring(&mut self, parts: &[FPart]) -> Res<R> {
        let (fmt, args) = self.fstring_parts(parts)?;
        if args.is_empty() {
            return Ok(r(format!("{}.to_string()", rust_str(&unfmt(&fmt))), Kind::Val, ATOM));
        }
        Ok(r(format!("format!(\"{}\", {})", fmt, args.join(", ")), Kind::Val, ATOM))
    }

    fn fstring_parts(&mut self, parts: &[FPart]) -> Res<(String, Vec<String>)> {
        let mut fmt = String::new();
        let mut args = vec![];
        for p in parts {
            match p {
                FPart::Lit(s) => fmt += &fmt_text(s),
                FPart::Val(e, conv, spec) => {
                    if *conv == Some('r') {
                        let x = self.ex(e)?;
                        let b = self.borrow_r(&x);
                        if spec.is_empty() {
                            fmt += "{}";
                            args.push(format!("pyrt::r({})", b));
                        } else {
                            fmt += "{}";
                            args.push(format!("pyrt::fmt(&pyrt::repr({}), {})", b, rust_str(spec)));
                        }
                        continue;
                    }
                    if spec.is_empty() {
                        fmt += "{}";
                        args.push(self.display(e)?);
                        continue;
                    }
                    match rust_spec(spec, &e.ty) {
                        Some((rs, as_float, py_str)) => {
                            fmt += &format!("{{:{}}}", rs);
                            let x = self.ex(e)?;
                            args.push(if py_str {
                                format!("pyrt::s({})", self.borrow_r(&x))
                            } else if as_float {
                                format!("{} as f64", par(&self.val_r(x), CAST))
                            } else if e.ty.is_copy() {
                                self.val_r(x).code
                            } else {
                                x.code
                            });
                        }
                        None => {
                            fmt += "{}";
                            let x = self.ex(e)?;
                            args.push(format!("pyrt::fmt({}, {})", self.borrow_r(&x), rust_str(spec)));
                        }
                    }
                }
            }
        }
        Ok((fmt, args))
    }

    // ------------------------------------------------------------ iteration

    /// Iterator expression and how it yields items.
    /// `for_loop`: may use the `&xs` / `&mut xs` forms.
    fn iter_code(&mut self, it: &Iter, by_mut: bool, for_loop: bool) -> Res<(String, Item)> {
        Ok(match it {
            Iter::Range(a, b, step) => {
                let lit = matches!(a.kind, ExprKind::Int(_)) && matches!(b.kind, ExprKind::Int(_));
                let av = self.val(a)?;
                let bv = self.val(b)?;
                let bcode = if lit { format!("{}_i64", bv.code) } else { par(&bv, RANGE + 1) };
                let range = format!("{}..{}", par(&av, RANGE + 1), bcode);
                match step {
                    None => (if for_loop { range } else { format!("({})", range) }, Item::Val),
                    Some(s) => match s.kind {
                        ExprKind::Int(k) if k > 0 => (format!("({}).step_by({})", range, k), Item::Val),
                        ExprKind::Int(k) if k < 0 => {
                            let lo = match b.kind {
                                ExprKind::Int(v) => {
                                    let x = v + 1;
                                    if lit { format!("{}_i64", x) } else { x.to_string() }
                                }
                                _ => format!("{} + 1", par(&bv, ADD)),
                            };
                            let r = format!("({}..={}).rev()", lo, par(&av, RANGE + 1));
                            if k == -1 {
                                (r, Item::Val)
                            } else {
                                (format!("{}.step_by({})", r, -k), Item::Val)
                            }
                        }
                        _ => {
                            let sv = self.val(s)?;
                            (format!("pyrt::range({}, {}, {})", av.code, bv.code, sv.code), Item::Val)
                        }
                    },
                }
            }
            Iter::Over(x) => {
                let c = self.ex(x)?;
                match &x.ty {
                    Ty::Str => (format!("{}.chars().map(String::from)", par(&c, ATOM)), Item::Val),
                    Ty::Dict(..) => (format!("{}.keys()", par(&c, ATOM)), Item::Ref),
                    _ if c.kind == Kind::Val => {
                        if for_loop {
                            (c.code, Item::Val)
                        } else {
                            (format!("{}.into_iter()", par(&c, ATOM)), Item::Val)
                        }
                    }
                    _ if by_mut => {
                        if for_loop && c.kind == Kind::Place {
                            (format!("&mut {}", par(&c, UNARY)), Item::Mut)
                        } else {
                            (format!("{}.iter_mut()", par(&c, ATOM)), Item::Mut)
                        }
                    }
                    _ => {
                        if for_loop && c.kind == Kind::Place {
                            (format!("&{}", par(&c, UNARY)), Item::Ref)
                        } else {
                            (format!("{}.iter()", par(&c, ATOM)), Item::Ref)
                        }
                    }
                }
            }
            Iter::Keys(x) => {
                let c = self.ex(x)?;
                (format!("{}.keys()", par(&c, ATOM)), Item::Ref)
            }
            Iter::Values(x) => {
                let c = self.ex(x)?;
                if by_mut {
                    (format!("{}.values_mut()", par(&c, ATOM)), Item::Mut)
                } else {
                    (format!("{}.values()", par(&c, ATOM)), Item::Ref)
                }
            }
            Iter::Items(x) => {
                let c = self.ex(x)?;
                if by_mut {
                    (format!("{}.iter_mut()", par(&c, ATOM)), Item::Tup(vec![Item::Ref, Item::Mut]))
                } else if for_loop && c.kind == Kind::Place {
                    (format!("&{}", par(&c, UNARY)), Item::Tup(vec![Item::Ref, Item::Ref]))
                } else {
                    (format!("{}.iter()", par(&c, ATOM)), Item::Tup(vec![Item::Ref, Item::Ref]))
                }
            }
            Iter::Enumerate(inner, start) => {
                let (ic, item) = self.iter_code(inner, by_mut, false)?;
                let s = match start {
                    Some(s) => {
                        let v = self.val(s)?;
                        if let ExprKind::Int(_) = s.kind {
                            format!("{}_i64", v.code)
                        } else {
                            par(&v, RANGE + 1)
                        }
                    }
                    None => "0_i64".into(),
                };
                (format!("({}..).zip({})", s, ic), Item::Tup(vec![Item::Val, item]))
            }
            Iter::Zip(its) => {
                let mut code = String::new();
                let mut items = vec![];
                for (i, x) in its.iter().enumerate() {
                    let (c, item) = self.iter_code(x, false, false)?;
                    if i == 0 {
                        code = c;
                    } else {
                        code = format!("{}.zip({})", code, c);
                    }
                    items.push(item);
                }
                (code, Item::Zip(items))
            }
            Iter::Reversed(inner) => {
                let (c, item) = self.iter_code(inner, by_mut, false)?;
                let c = if c.starts_with('(') || c.ends_with(')') { c } else { format!("({})", c) };
                (format!("{}.rev()", c), item)
            }
            Iter::Gen(e) => {
                let ExprKind::Comp(c) = &e.kind else { unreachable!() };
                (self.comp_iter(c)?, Item::Val)
            }
        })
    }

    /// Pattern binding a loop target to an item; sets variable kinds.
    fn pat(&mut self, t: &Target, item: &Item) -> Res<String> {
        Ok(match (t, item) {
            (Target::Var(v), Item::Val) => {
                self.kinds.insert(*v, Kind::Place);
                self.name(*v)
            }
            (Target::Var(v), Item::Ref) => {
                if self.f.vars[*v].ty.is_copy() {
                    self.kinds.insert(*v, Kind::Place);
                    format!("&{}", self.name(*v))
                } else {
                    self.kinds.insert(*v, Kind::Ref);
                    self.name(*v)
                }
            }
            (Target::Var(v), Item::Mut) => {
                self.kinds.insert(*v, Kind::Mut);
                self.name(*v)
            }
            (Target::Tuple(ts), Item::Tup(items)) if ts.len() == items.len() => {
                let mut ps = vec![];
                for (t, i) in ts.iter().zip(items.iter()) {
                    ps.push(self.pat(t, i)?);
                }
                format!("({})", ps.join(", "))
            }
            (Target::Tuple(ts), Item::Zip(items)) if ts.len() == items.len() => {
                let mut p = self.pat(&ts[0], &items[0])?;
                for (t, i) in ts.iter().zip(items.iter()).skip(1) {
                    let q = self.pat(t, i)?;
                    p = format!("({}, {})", p, q);
                }
                p
            }
            (Target::Tuple(ts), Item::Ref) => {
                let mut leaves = vec![];
                t.vars(&mut leaves);
                let all_copy = leaves.iter().all(|v| self.f.vars[*v].ty.is_copy());
                if all_copy {
                    let p = self.pat_tuple(ts, Kind::Place);
                    format!("&{}", p)
                } else {
                    self.pat_tuple(ts, Kind::Ref)
                }
            }
            (Target::Tuple(ts), Item::Mut) => self.pat_tuple(ts, Kind::Mut),
            (Target::Tuple(ts), Item::Val) => self.pat_tuple(ts, Kind::Place),
            (Target::Var(v), _) => {
                return err(self.f.line, format!("unpack the items: `for a, b in ...` instead of `{}`", self.f.vars[*v].name))
            }
            _ => return err(self.f.line, "loop target does not match the items"),
        })
    }

    fn pat_tuple(&mut self, ts: &[Target], k: Kind) -> String {
        let mut ps = vec![];
        for t in ts {
            match t {
                Target::Var(v) => {
                    self.kinds.insert(*v, k);
                    ps.push(self.name(*v));
                }
                Target::Tuple(inner) => ps.push(self.pat_tuple(inner, k)),
                Target::Place(_) => ps.push("_".into()),
            }
        }
        format!("({})", ps.join(", "))
    }

    /// Can `&PAT` match a reference to the item (all bindings Copy)?
    fn item_copyable(&self, t: &Target, item: &Item) -> bool {
        match (t, item) {
            (Target::Var(v), Item::Val) => self.f.vars[*v].ty.is_copy(),
            (_, Item::Ref) => true,
            (_, Item::Mut) => false,
            (Target::Tuple(ts), Item::Tup(is)) | (Target::Tuple(ts), Item::Zip(is)) => {
                ts.iter().zip(is.iter()).all(|(t, i)| self.item_copyable(t, i))
            }
            (Target::Tuple(ts), Item::Val) => {
                let mut leaves = vec![];
                t.vars(&mut leaves);
                let _ = ts;
                leaves.iter().all(|v| self.f.vars[*v].ty.is_copy())
            }
            _ => false,
        }
    }

    /// The iterator of a single-`for` comprehension, before collecting.
    fn comp_iter(&mut self, c: &Comp) -> Res<String> {
        if c.gens.len() != 1 {
            // nested loops: build a Vec first
            let v = self.comp_block(c, &Ty::list(c.elt.ty.clone()))?;
            return Ok(format!("{}.into_iter()", par(&v, ATOM)));
        }
        let g = &c.gens[0];
        let (code, item) = self.iter_code(&g.iter, false, false)?;
        let mut code = code;
        if !g.conds.is_empty() {
            if self.item_copyable(&g.target, &item) {
                let p = self.pat(&g.target, &item)?;
                let mut cs = vec![];
                for cnd in &g.conds {
                    cs.push(self.cond(cnd)?);
                }
                let fp = if p.starts_with('&') || p.starts_with('(') || !p.contains('&') {
                    format!("&{}", p)
                } else {
                    format!("&{}", p)
                };
                code = format!("{}.filter(|{}| {})", code, fp, cs.join(" && "));
            } else {
                let p = self.pat(&g.target, &item)?;
                let mut cs = vec![];
                for cnd in &g.conds {
                    cs.push(self.cond(cnd)?);
                }
                let elt = self.comp_elt(c)?;
                return Ok(format!("{}.filter_map(|{}| if {} {{ Some({}) }} else {{ None }})", code, p, cs.join(" && "), elt));
            }
        }
        let p = self.pat(&g.target, &item)?;
        // `[x for x in xs]`
        if let (ExprKind::Var(v), Target::Var(w), None) = (&c.elt.kind, &g.target, &c.val) {
            if v == w {
                return Ok(match item {
                    Item::Ref if c.elt.ty.is_copy() => format!("{}.copied()", code),
                    Item::Ref => format!("{}.cloned()", code),
                    _ => code,
                });
            }
        }
        let elt = self.comp_elt(c)?;
        Ok(format!("{}.map(|{}| {})", code, p, elt))
    }

    fn comp_elt(&mut self, c: &Comp) -> Res<String> {
        let k = self.owned(&c.elt)?;
        Ok(match &c.val {
            Some(v) => format!("({}, {})", k, self.owned(v)?),
            None => k,
        })
    }

    fn comp(&mut self, c: &Comp, ty: &Ty) -> Res<R> {
        // `[[0] * m for _ in range(n)]` → vec![vec![0; m]; n]
        if c.kind == CompKind::List && c.gens.len() == 1 && c.gens[0].conds.is_empty() {
            if let (Iter::Range(a, n, None), Target::Var(v)) = (&c.gens[0].iter, &c.gens[0].target) {
                let mut u = vec![];
                uses(&c.elt, &mut u);
                if matches!(a.kind, ExprKind::Int(0)) && !u.contains(v) && pure(&c.elt) {
                    let e = self.owned(&c.elt)?;
                    let nv = self.val(n)?;
                    let cnt = if let ExprKind::Int(k) = n.kind {
                        k.max(0).to_string()
                    } else if self.is_nonneg(n) {
                        format!("{} as usize", par(&nv, CAST))
                    } else {
                        format!("{}.max(0) as usize", par(&nv, ATOM))
                    };
                    return Ok(r(format!("vec![{}; {}]", e, cnt), Kind::Val, ATOM));
                }
            }
        }
        if c.gens.len() != 1 {
            return self.comp_block(c, ty);
        }
        let it = self.comp_iter(c)?;
        Ok(r(format!("{}.collect::<{}>()", it, self.ty(ty)), Kind::Val, ATOM))
    }

    /// Nested comprehension as a block with loops.
    fn comp_block(&mut self, c: &Comp, ty: &Ty) -> Res<R> {
        let acc = "acc";
        let mut s = format!("{{ let mut {} = {}::new(); ", acc, match ty {
            Ty::Set(_) => "IndexSet",
            Ty::Dict(..) => "IndexMap",
            _ => "Vec",
        });
        let mut close = String::new();
        for g in &c.gens {
            let (code, item) = self.iter_code(&g.iter, false, true)?;
            let p = self.pat(&g.target, &item)?;
            s += &format!("for {} in {} {{ ", p, code);
            close += "} ";
            for cnd in &g.conds {
                let cv = self.cond(cnd)?;
                s += &format!("if {} {{ ", cv);
                close += "} ";
            }
        }
        let k = self.owned(&c.elt)?;
        let push = match (ty, &c.val) {
            (Ty::Dict(..), Some(v)) => {
                let vv = self.owned(v)?;
                format!("{}.insert({}, {}); ", acc, k, vv)
            }
            (Ty::Set(_), _) => format!("{}.insert({}); ", acc, k),
            _ => format!("{}.push({}); ", acc, k),
        };
        s += &push;
        s += &close;
        s += &format!("{} }}", acc);
        Ok(r(s, Kind::Val, ATOM))
    }

    /// Iterator over owned or borrowed items for sum/min/max/...
    fn iter_arg(&mut self, e: &Expr) -> Res<(String, Item)> {
        let ExprKind::Iter(it) = &e.kind else { return err(e.line, "expected an iterable") };
        self.iter_code(it, false, false)
    }

    /// Iterator yielding values (Copy items copied).
    fn iter_values(&mut self, e: &Expr) -> Res<String> {
        let (code, item) = self.iter_arg(e)?;
        Ok(match item {
            Item::Ref if e.ty.is_copy() => format!("{}.copied()", code),
            Item::Ref => format!("{}.cloned()", code),
            _ => code,
        })
    }

    /// Lambda body with its parameter bound to an item reference.
    fn lambda(&mut self, l: &Expr, item_is_ref: bool) -> Res<(String, String)> {
        let ExprKind::Lambda(ps, body) = &l.kind else { unreachable!() };
        let v = ps[0];
        let ty = self.f.vars[v].ty.clone();
        let pat = if ty.is_copy() {
            self.kinds.insert(v, Kind::Place);
            if item_is_ref {
                format!("&&{}", self.name(v))
            } else {
                format!("&{}", self.name(v))
            }
        } else {
            self.kinds.insert(v, Kind::Ref);
            if item_is_ref {
                format!("&{}", self.name(v))
            } else {
                self.name(v)
            }
        };
        let b = self.owned(body)?;
        Ok((pat, b))
    }

    // -------------------------------------------------------------- builtins

    fn builtin(&mut self, name: &str, a: &[Expr], e: &Expr) -> Res<R> {
        let v = |c: String| r(c, Kind::Val, ATOM);
        Ok(match name {
            "print" => {
                let (sep, end) = (&a[0], &a[1]);
                let lit = |x: &Expr| if let ExprKind::Str(s) = &x.kind { Some(s.clone()) } else { None };
                match (lit(sep), lit(end)) {
                    (Some(sp), Some(en)) => {
                        let mut fmt = String::new();
                        let mut args = vec![];
                        // Python evaluates the arguments in order and prints objects
                        // afterwards: values before a modifying call go to temporaries
                        let last_mut = a[2..].iter().rposition(|x| self.modifies(x));
                        for (i, x) in a[2..].iter().enumerate() {
                            if i > 0 {
                                fmt += &fmt_text(&sp);
                            }
                            if let Some(lm) = last_mut {
                                let fresh = x.ty.is_copy() || !matches!(x.kind, ExprKind::Var(_) | ExprKind::Field(..) | ExprKind::Index(..));
                                if i <= lm && fresh && a.len() > 3 {
                                    self.tmp += 1;
                                    let t = format!("t{}", self.tmp);
                                    let val = if x.ty.is_copy() { self.val(x)?.code } else { self.owned(x)? };
                                    self.pre.push(format!("let {} = {};", t, val));
                                    fmt += "{}";
                                    args.push(match &x.ty {
                                        Ty::Int | Ty::Str => t,
                                        _ => format!("pyrt::s(&{})", t),
                                    });
                                    continue;
                                }
                            }
                            let (f, xs) = self.fmt_piece(x)?;
                            fmt += &f;
                            args.extend(xs);
                        }
                        let mac = if en == "\n" { "println" } else { "print" };
                        if en != "\n" {
                            fmt += &fmt_text(&en);
                        }
                        if args.is_empty() {
                            if fmt.is_empty() {
                                v(format!("{}!()", mac))
                            } else {
                                v(format!("{}!(\"{}\")", mac, fmt))
                            }
                        } else {
                            v(format!("{}!(\"{}\", {})", mac, fmt, args.join(", ")))
                        }
                    }
                    _ => {
                        let sp = self.ex(sep)?;
                        let en = self.ex(end)?;
                        let mut fmt = String::new();
                        let mut all = vec![];
                        for (i, x) in a[2..].iter().enumerate() {
                            if i > 0 {
                                fmt += "{}";
                                all.push(sp.code.clone());
                            }
                            fmt += "{}";
                            all.push(self.display(x)?);
                        }
                        fmt += "{}";
                        all.push(en.code);
                        v(format!("print!(\"{}\", {})", fmt, all.join(", ")))
                    }
                }
            }
            "truth" => {
                let x = self.ex(&a[0])?;
                match &a[0].ty {
                    Ty::Int => r(format!("{} != 0", par(&self.val_r(x), CMP + 1)), Kind::Val, CMP),
                    Ty::Float => r(format!("{} != 0.0", par(&self.val_r(x), CMP + 1)), Kind::Val, CMP),
                    _ => r(format!("!{}.is_empty()", par(&x, ATOM)), Kind::Val, UNARY),
                }
            }
            "len" => {
                let x = self.ex(&a[0])?;
                match &a[0].ty {
                    Ty::Str => r(format!("{}.chars().count() as i64", par(&x, ATOM)), Kind::Val, CAST),
                    Ty::Tuple(ts) => v(ts.len().to_string()),
                    _ => r(format!("{}.len() as i64", par(&x, ATOM)), Kind::Val, CAST),
                }
            }
            "abs" => {
                let x = self.val(&a[0])?;
                v(format!("{}.abs()", recv(&x)))
            }
            "min" | "max" => {
                let mut acc = self.val(&a[0])?;
                for x in &a[1..] {
                    let y = self.val(x)?;
                    acc = if e.ty == Ty::Float {
                        v(format!("pyrt::f{}({}, {})", name, acc.code, y.code))
                    } else if e.ty == Ty::Int {
                        v(format!("{}.{}({})", recv(&acc), name, y.code))
                    } else {
                        return err(e.line, "min()/max() of several arguments needs numbers");
                    };
                }
                acc
            }
            "min_iter" | "max_iter" => {
                let short = &name[..3];
                let (code, item) = self.iter_arg(&a[0])?;
                let is_ref = matches!(item, Item::Ref);
                let code = if is_ref && e.ty.is_copy() { format!("{}.copied()", code) } else { code };
                let kind = if is_ref && !e.ty.is_copy() { Kind::Ref } else { Kind::Val };
                if let ExprKind::Lambda(..) = a[1].kind {
                    let (p, b) = self.lambda(&a[1], is_ref && !e.ty.is_copy())?;
                    r(format!("pyrt::{}_by_key({}, |{}| {})", short, code, p, b), kind, ATOM)
                } else {
                    r(format!("pyrt::{}({})", short, code), kind, ATOM)
                }
            }
            "sum" => {
                let code = self.iter_values(&a[0])?;
                if e.ty == Ty::Float {
                    v(format!("pyrt::sum_f64({})", code))
                } else {
                    v(format!("{}.sum::<i64>()", code))
                }
            }
            "any" | "all" => {
                let ExprKind::Iter(it) = &a[0].kind else { unreachable!() };
                if let Iter::Gen(g) = &**it {
                    let ExprKind::Comp(c) = &g.kind else { unreachable!() };
                    if c.gens.len() == 1 && c.gens[0].conds.is_empty() {
                        let (code, item) = self.iter_code(&c.gens[0].iter, false, false)?;
                        let p = self.pat(&c.gens[0].target, &item)?;
                        let t = self.truth_code(&c.elt)?;
                        return Ok(v(format!("{}.{}(|{}| {})", code, name, p, t)));
                    }
                }
                let (code, item) = self.iter_arg(&a[0])?;
                let t = &a[0].ty;
                let x = R { code: "x".into(), kind: if matches!(item, Item::Ref) { Kind::Ref } else { Kind::Place }, prec: ATOM };
                let test = truth_of(t, &self.val_r(x.clone()), &x);
                v(format!("{}.{}(|x| {})", code, name, test))
            }
            "sorted" => {
                let (key, rev) = (&a[1], matches!(a[2].kind, ExprKind::Bool(true)));
                let et = match &e.ty {
                    Ty::List(t) => (**t).clone(),
                    _ => unreachable!(),
                };
                let src = self.iter_values(&a[0])?;
                if let ExprKind::Lambda(..) = key.kind {
                    let (p, b) = self.lambda(key, false)?;
                    let f = if rev { "sorted_by_key_rev" } else { "sorted_by_key" };
                    v(format!("pyrt::{}({}, |{}| {})", f, src, p, b))
                } else if rev {
                    v(format!("pyrt::sorted_rev({})", src))
                } else {
                    let _ = et;
                    v(format!("pyrt::sorted({})", src))
                }
            }
            "list" => {
                if let ExprKind::Iter(it) = &a[0].kind {
                    if let Iter::Over(x) = &**it {
                        if matches!(x.ty, Ty::List(_)) {
                            let c = self.ex(x)?;
                            return Ok(v(self.owned_r(&c, &x.ty, self.o.moves.contains(&x.id))));
                        }
                    }
                }
                let code = self.iter_values(&a[0])?;
                v(format!("{}.collect::<{}>()", code, self.ty(&e.ty)))
            }
            "set" => {
                let code = self.iter_values(&a[0])?;
                v(format!("{}.collect::<{}>()", code, self.ty(&e.ty)))
            }
            "str" => match &a[0].ty {
                Ty::Int => {
                    let x = self.val(&a[0])?;
                    v(format!("{}.to_string()", recv(&x)))
                }
                Ty::Str => {
                    let x = self.owned(&a[0])?;
                    r(x, Kind::Val, ATOM)
                }
                _ => {
                    let b = self.borrow(&a[0])?;
                    v(format!("pyrt::str({})", b))
                }
            },
            "repr" => {
                let b = self.borrow(&a[0])?;
                v(format!("pyrt::repr({})", b))
            }
            "int" => match &a[0].ty {
                Ty::Str => v(format!("pyrt::int_str({})", self.borrow_str(&a[0])?)),
                Ty::Float => v(format!("pyrt::int_float({})", self.val(&a[0])?.code)),
                _ => {
                    let x = self.val(&a[0])?;
                    r(format!("{} as i64", par(&x, CAST)), Kind::Val, CAST)
                }
            },
            "float" => v(format!("pyrt::float_str({})", self.borrow_str(&a[0])?)),
            "round" => v(format!("pyrt::round({})", self.val(&a[0])?.code)),
            "round2" => {
                let x = self.val(&a[0])?;
                let n = self.val(&a[1])?;
                v(format!("pyrt::round2({}, {})", x.code, n.code))
            }
            "divmod" => {
                let x = self.val(&a[0])?;
                let y = self.val(&a[1])?;
                v(format!("(pyrt::floordiv({0}, {1}), pyrt::modulo({0}, {1}))", x.code, y.code))
            }
            "powmod" => {
                let (x, y, m) = (self.val(&a[0])?, self.val(&a[1])?, self.val(&a[2])?);
                v(format!("pyrt::powmod({}, {}, {})", x.code, y.code, m.code))
            }
            "ord" => v(format!("pyrt::ord({})", self.borrow_str(&a[0])?)),
            "chr" => v(format!("pyrt::chr({})", self.val(&a[0])?.code)),
            "input" => {
                if a.is_empty() {
                    v("pyrt::input(\"\")".into())
                } else {
                    v(format!("pyrt::input({})", self.borrow_str(&a[0])?))
                }
            }
            n if n.starts_with("math.") => self.math(&n[5..], a)?,
            n if n.starts_with("list.") => self.list_method(&n[5..], a, e)?,
            n if n.starts_with("dict.") => self.dict_method(&n[5..], a, e)?,
            n if n.starts_with("set.") => self.set_method(&n[4..], a)?,
            n if n.starts_with("str.") => self.str_method(&n[4..], a)?,
            _ => return err(e.line, format!("internal: unknown builtin {}", name)),
        })
    }

    /// Does evaluating `e` modify some object?
    fn modifies(&self, e: &Expr) -> bool {
        let mut m = false;
        fn go(g: &FGen, e: &Expr, m: &mut bool) {
            match &e.kind {
                ExprKind::Builtin(n, _) if n.starts_with("list.") || n.starts_with("dict.") || n.starts_with("set.") => {
                    if !matches!(*n, "list.copy" | "list.index" | "list.count" | "dict.get" | "dict.copy" | "set.copy"
                        | "set.issubset" | "set.issuperset" | "set.isdisjoint") {
                        *m = true;
                    }
                }
                ExprKind::Method(_, f, _) if g.o.self_mut[*f] => *m = true,
                ExprKind::Call(f, _) if g.o.params[*f].contains(&Mode::Mut) => *m = true,
                _ => {}
            }
            e.children(&mut |c| go(g, c, m));
        }
        go(self, e, &mut m);
        m
    }

    fn truth_code(&mut self, e: &Expr) -> Res<String> {
        let x = self.ex(e)?;
        Ok(truth_of(&e.ty, &self.val_r(x.clone()), &x))
    }

    fn math(&mut self, n: &str, a: &[Expr]) -> Res<R> {
        let mut vs = vec![];
        for x in a {
            vs.push(self.val(x)?);
        }
        let v = |c: String| r(c, Kind::Val, ATOM);
        let m = |vs: &[R], f: &str| format!("{}.{}()", recv(&vs[0]), f);
        Ok(match n {
            "pi" => r("std::f64::consts::PI", Kind::Val, ATOM),
            "e" => r("std::f64::consts::E", Kind::Val, ATOM),
            "tau" => r("std::f64::consts::TAU", Kind::Val, ATOM),
            "inf" => r("f64::INFINITY", Kind::Val, ATOM),
            "sin" | "cos" | "tan" | "atan" | "tanh" => v(m(&vs, n)),
            "fabs" => v(m(&vs, "abs")),
            "isnan" => v(m(&vs, "is_nan")),
            "isinf" => v(m(&vs, "is_infinite")),
            "isfinite" => v(m(&vs, "is_finite")),
            "atan2" => v(format!("{}.atan2({})", recv(&vs[0]), vs[1].code)),
            "hypot" => v(format!("{}.hypot({})", recv(&vs[0]), vs[1].code)),
            "copysign" => v(format!("{}.copysign({})", recv(&vs[0]), vs[1].code)),
            "log2args" => v(format!("pyrt::math::log_base({}, {})", vs[0].code, vs[1].code)),
            _ => v(format!("pyrt::math::{}({})", n, vs.iter().map(|x| x.code.clone()).collect::<Vec<_>>().join(", "))),
        })
    }

    fn list_method(&mut self, m: &str, a: &[Expr], e: &Expr) -> Res<R> {
        let recv = self.ex(&a[0])?;
        let rc = par(&recv, ATOM);
        let v = |c: String| r(c, Kind::Val, ATOM);
        let et = match &a[0].ty {
            Ty::List(t) => (**t).clone(),
            _ => unreachable!(),
        };
        Ok(match m {
            "append" => v(format!("{}.push({})", rc, self.owned(&a[1])?)),
            "extend" => v(format!("{}.extend({})", rc, self.extend_arg(&a[1])?)),
            "insert" => {
                let x = self.owned(&a[2])?;
                if let ExprKind::Int(0) = a[1].kind {
                    v(format!("{}.insert(0, {})", rc, x))
                } else {
                    let i = self.val(&a[1])?;
                    v(format!("pyrt::insert({}, {}, {})", self.mut_ref_of(&recv), i.code, x))
                }
            }
            "pop" => v(format!("{}.pop().expect(\"IndexError: pop from empty list\")", rc)),
            "pop_at" => {
                if let ExprKind::Int(0) = a[1].kind {
                    v(format!("pyrt::pop_at({}, 0)", self.mut_ref_of(&recv)))
                } else {
                    let i = self.val(&a[1])?;
                    v(format!("pyrt::pop_at({}, {})", self.mut_ref_of(&recv), i.code))
                }
            }
            "remove" => {
                let x = self.owned(&a[1])?;
                v(format!("pyrt::remove({}, &{})", self.mut_ref_of(&recv), x))
            }
            "index" => {
                let x = self.owned(&a[1])?;
                v(format!("pyrt::index({}, &{})", self.borrow_r(&recv), x))
            }
            "count" => {
                let x = self.owned(&a[1])?;
                v(format!("pyrt::count({}, &{})", self.borrow_r(&recv), x))
            }
            "reverse" => v(format!("{}.reverse()", rc)),
            "clear" => v(format!("{}.clear()", rc)),
            "copy" => v(self.owned_r(&recv, &a[0].ty, false)),
            "swap" => {
                let i = self.index_code(&recv, &a[1])?;
                let j = self.index_code(&recv, &a[2])?;
                v(format!("{}.swap({}, {})", rc, i, j))
            }
            "sort" => {
                let rev = matches!(a[2].kind, ExprKind::Bool(true));
                if let ExprKind::Lambda(_, body) = &a[1].kind {
                    let kt = body.ty.clone();
                    let (p, b) = self.lambda(&a[1], false)?;
                    if total_order(&kt) && !rev && kt.is_copy() {
                        v(format!("{}.sort_by_key(|{}| {})", rc, p, b))
                    } else {
                        let f = if rev { "sort_by_key_rev" } else { "sort_by_key" };
                        v(format!("pyrt::{}({}, |{}| {})", f, self.mut_ref_of(&recv), p, b))
                    }
                } else if total_order(&et) {
                    if rev {
                        v(format!("{}.sort_by(|a, b| b.cmp(a))", rc))
                    } else {
                        v(format!("{}.sort()", rc))
                    }
                } else {
                    let f = if rev { "sort_rev" } else { "sort" };
                    v(format!("pyrt::{}({})", f, self.mut_ref_of(&recv)))
                }
            }
            _ => return err(e.line, format!("internal: list.{}", m)),
        })
    }

    fn dict_method(&mut self, m: &str, a: &[Expr], e: &Expr) -> Res<R> {
        let recv = self.ex(&a[0])?;
        let rc = par(&recv, ATOM);
        let v = |c: String| r(c, Kind::Val, ATOM);
        Ok(match m {
            "get" => {
                let k = self.borrow(&a[1])?;
                let d = self.owned(&a[2])?;
                let how = if e.ty.is_copy() { "copied" } else { "cloned" };
                v(format!("{}.get({}).{}().unwrap_or({})", rc, k, how, d))
            }
            "pop" => {
                let k = self.borrow(&a[1])?;
                v(format!("pyrt::dict_pop({}, {})", self.mut_ref_of(&recv), k))
            }
            "pop_default" => {
                let k = self.borrow(&a[1])?;
                let d = self.owned(&a[2])?;
                v(format!("{}.shift_remove({}).unwrap_or({})", rc, k, d))
            }
            "setdefault" => {
                let k = self.owned(&a[1])?;
                let d = self.owned(&a[2])?;
                r(format!("{}.entry({}).or_insert({})", rc, k, d), Kind::Mut, ATOM)
            }
            "copy" => v(self.owned_r(&recv, &a[0].ty, false)),
            "clear" => v(format!("{}.clear()", rc)),
            "update" => {
                let o = self.borrow(&a[1])?;
                v(format!("pyrt::update({}, {})", self.mut_ref_of(&recv), o))
            }
            _ => return err(e.line, format!("internal: dict.{}", m)),
        })
    }

    fn set_method(&mut self, m: &str, a: &[Expr]) -> Res<R> {
        let recv = self.ex(&a[0])?;
        let rc = par(&recv, ATOM);
        let v = |c: String| r(c, Kind::Val, ATOM);
        Ok(match m {
            "add" => v(format!("{}.insert({})", rc, self.owned(&a[1])?)),
            "remove" => {
                let x = self.borrow(&a[1])?;
                v(format!("pyrt::set_remove({}, {})", self.mut_ref_of(&recv), x))
            }
            "discard" => {
                let x = self.borrow(&a[1])?;
                v(format!("{}.shift_remove({})", rc, x))
            }
            "issubset" => v(format!("{}.is_subset({})", rc, self.borrow(&a[1])?)),
            "issuperset" => v(format!("{}.is_superset({})", rc, self.borrow(&a[1])?)),
            "isdisjoint" => v(format!("{}.is_disjoint({})", rc, self.borrow(&a[1])?)),
            "copy" => v(self.owned_r(&recv, &a[0].ty, false)),
            "clear" => v(format!("{}.clear()", rc)),
            _ => unreachable!(),
        })
    }

    fn str_method(&mut self, m: &str, a: &[Expr]) -> Res<R> {
        let recv = self.ex(&a[0])?;
        let rc = par(&recv, ATOM);
        let sb = self.borrow_r(&recv);
        let v = |c: String| r(c, Kind::Val, ATOM);
        let arg = |g: &mut Self, i: usize| -> Res<String> {
            if a[i].ty == Ty::Str {
                g.borrow_str(&a[i])
            } else {
                Ok(g.val(&a[i])?.code)
            }
        };
        Ok(match m {
            "upper" => v(format!("{}.to_uppercase()", rc)),
            "lower" => v(format!("{}.to_lowercase()", rc)),
            "casefold" => v(format!("{}.to_lowercase()", rc)),
            "replace" => {
                let (x, y) = (arg(self, 1)?, arg(self, 2)?);
                v(format!("{}.replace({}, {})", rc, x, y))
            }
            "startswith" => v(format!("{}.starts_with({})", rc, arg(self, 1)?)),
            "endswith" => v(format!("{}.ends_with({})", rc, arg(self, 1)?)),
            "split" => v(format!("pyrt::split({}, {})", sb, arg(self, 1)?)),
            "join" => {
                let sep = sb;
                if let ExprKind::Iter(it) = &a[1].kind {
                    if let Iter::Over(x) = &**it {
                        if matches!(x.ty, Ty::List(_)) {
                            let c = self.ex(x)?;
                            return Ok(v(format!("{}.join({})", par(&c, ATOM), sep)));
                        }
                    }
                }
                let code = self.iter_values(&a[1])?;
                v(format!("{}.collect::<Vec<String>>().join({})", code, sep))
            }
            "find" | "rfind" | "count" => {
                let f = if m == "count" { "count_str" } else { m };
                v(format!("pyrt::{}({}, {})", f, sb, arg(self, 1)?))
            }
            "index" | "rindex" => v(format!("pyrt::{}_str({}, {})", m, sb, arg(self, 1)?)),
            "split_ws" | "splitlines" | "strip" | "lstrip" | "rstrip" | "isdigit" | "isdecimal" | "isnumeric"
            | "isalpha" | "isalnum" | "isspace" | "isupper" | "islower" | "title" | "capitalize" | "swapcase" => {
                let f = match m {
                    "isdecimal" | "isnumeric" => "isdigit",
                    x => x,
                };
                v(format!("pyrt::{}({})", f, sb))
            }
            _ => {
                // one-argument helpers: strip_chars, zfill, center, ljust, rjust
                let x = arg(self, 1)?;
                v(format!("pyrt::{}({}, {})", m, sb, x))
            }
        })
    }
}

fn truth_of(t: &Ty, val: &R, x: &R) -> String {
    match t {
        Ty::Bool => val.code.clone(),
        Ty::Int => format!("{} != 0", par(val, CMP + 1)),
        Ty::Float => format!("{} != 0.0", par(val, CMP + 1)),
        _ => format!("!{}.is_empty()", par(x, ATOM)),
    }
}

fn unfmt(s: &str) -> String {
    // the literal text of a format string without arguments
    s.replace("{{", "{").replace("}}", "}").replace("\\n", "\n").replace("\\t", "\t").replace("\\\"", "\"").replace("\\\\", "\\")
}

/// The statement list that directly contains statement `id`.
fn block_of(s: &[Stmt], id: u32) -> Option<&[Stmt]> {
    if s.iter().any(|x| x.id == id) {
        return Some(s);
    }
    for x in s {
        for b in sub_blocks(x) {
            if let Some(r) = block_of(b, id) {
                return Some(r);
            }
        }
    }
    None
}

/// How many times `v` may be assigned along one path (2 = repeatedly).
fn assign_count(s: &[Stmt], v: VarId) -> u32 {
    let mut n = 0;
    for x in s {
        n += match &x.kind {
            StmtKind::Assign(t, _) => {
                let mut d = vec![];
                t.vars(&mut d);
                d.contains(&v) as u32
            }
            StmtKind::AugVar(w, ..) => (*w == v) as u32,
            StmtKind::If(_, a, b) => assign_count(a, v).max(assign_count(b, v)),
            StmtKind::While(_, a, b) | StmtKind::For(_, _, a, b) => {
                (if assign_count(a, v) > 0 { 2 } else { 0 }) + assign_count(b, v)
            }
            _ => 0,
        };
    }
    n
}

/// Expression without variables or calls (safe to evaluate repeatedly).
fn pure(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Bool(_) | ExprKind::Str(_) | ExprKind::Const(_) | ExprKind::Var(_) => true,
        ExprKind::Bin(_, a, b) => pure(a) && pure(b),
        ExprKind::Unary(_, a) | ExprKind::ToFloat(a) => pure(a),
        ExprKind::List(v) | ExprKind::Tuple(v) => v.iter().all(pure),
        _ => false,
    }
}

fn all_literal(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Int(_) => true,
        ExprKind::Bin(_, a, b) => all_literal(a) && all_literal(b),
        ExprKind::Unary(_, a) => all_literal(a),
        _ => false,
    }
}

fn is_simple(e: &Expr) -> bool {
    matches!(e.kind, ExprKind::Var(_) | ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Const(_) | ExprKind::Str(_))
}

/// `let x = ...` needs a type when inference would not find the Python one.
fn needs_annotation(v: &Expr) -> bool {
    match &v.kind {
        ExprKind::Int(_) => true,
        ExprKind::List(x) | ExprKind::Set(x) => x.is_empty(),
        ExprKind::Dict(x) => x.is_empty(),
        ExprKind::Bin(..) | ExprKind::Unary(..) => all_literal(v),
        _ => false,
    }
}

/// Every path through `s` ends in return / raise / an endless loop.
fn always_exits(s: &[Stmt]) -> bool {
    match s.last().map(|x| &x.kind) {
        Some(StmtKind::Return(_)) | Some(StmtKind::Raise(..)) => true,
        Some(StmtKind::If(_, a, b)) => always_exits(a) && always_exits(b),
        Some(StmtKind::While(c, body, _)) => matches!(c.kind, ExprKind::Bool(true)) && !has_break(body),
        _ => false,
    }
}

fn has_break(s: &[Stmt]) -> bool {
    s.iter().any(|x| match &x.kind {
        StmtKind::Break => true,
        StmtKind::If(_, a, b) => has_break(a) || has_break(b),
        _ => false,
    })
}

/// Python format spec → Rust format spec when they agree.
/// Returns (rust spec, value needs `as f64`, value needs pyrt::s).
fn rust_spec(spec: &str, t: &Ty) -> Option<(String, bool, bool)> {
    let sp = pyrt::spec::Spec::parse(spec)?;
    if sp.grouping.is_some() || sp.align == Some('=') || sp.alt {
        return None;
    }
    if matches!(sp.fill, Some('{' | '}')) {
        return None;
    }
    let mut s = String::new();
    if let Some(f) = sp.fill {
        s.push(f);
    }
    if let Some(a) = sp.align {
        s.push(a);
    }
    match (t, sp.ty) {
        (Ty::Int, None | Some('d')) => {
            if let Some(sg) = sp.sign {
                if sg != '+' && sg != '-' {
                    return None;
                }
                if sg == '+' {
                    s.push('+');
                }
            }
            if sp.zero {
                s.push('0');
            }
            if let Some(w) = sp.width {
                s += &w.to_string();
            }
            if sp.precision.is_some() {
                return None;
            }
            Some((s, false, false))
        }
        (Ty::Float | Ty::Int, Some('f')) => {
            if let Some(sg) = sp.sign {
                if sg == ' ' {
                    return None;
                }
                if sg == '+' {
                    s.push('+');
                }
            }
            if sp.zero {
                s.push('0');
            }
            if let Some(w) = sp.width {
                s += &w.to_string();
            }
            s += &format!(".{}", sp.precision.unwrap_or(6));
            Some((s, *t == Ty::Int, false))
        }
        (Ty::Float, None) if sp.precision.is_none() && sp.sign.is_none() && !sp.zero => {
            if let Some(w) = sp.width {
                s += &w.to_string();
            }
            Some((s, false, true))
        }
        (Ty::Str, None | Some('s')) => {
            if sp.sign.is_some() || sp.zero {
                return None;
            }
            if let Some(w) = sp.width {
                s += &w.to_string();
            }
            if let Some(p) = sp.precision {
                s += &format!(".{}", p);
            }
            Some((s, false, false))
        }
        _ => None,
    }
}
