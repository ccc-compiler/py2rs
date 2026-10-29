//! Typed IR → Rust source.

use std::collections::{HashMap, HashSet};

use crate::diag::{err, Res};
use crate::ir::*;
use crate::exc::{self, Exc};
use crate::own::{uses, Mode, Own};

mod stdlib;

const KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in",
    "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "static", "struct", "super", "trait",
    "true", "type", "unsafe", "use", "where", "while", "async", "await", "dyn", "abstract", "become", "box", "do",
    "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "gen",
];

pub fn ident(n: &str) -> String {
    // operator methods: `__add__` → `op_add`
    if n.len() > 4 && n.starts_with("__") && n.ends_with("__") && !matches!(n, "__init__" | "__str__" | "__repr__") {
        return format!("op_{}", &n[2..n.len() - 2]);
    }
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
        Ty::Dict(k, v, DK::Plain) => format!("IndexMap<{}, {}>", rty(k, p), rty(v, p)),
        Ty::Dict(k, v, DK::Default) => format!("pyrt::DefaultDict<{}, {}>", rty(k, p), rty(v, p)),
        Ty::Dict(k, _, DK::Counter) => format!("pyrt::Counter<{}>", rty(k, p)),
        Ty::Deque(t) => format!("VecDeque<{}>", rty(t, p)),
        Ty::Pattern(_) => "pyrt::re::Pattern".to_string(),
        Ty::Match(_) => "pyrt::re::Match".to_string(),
        Ty::File => "pyrt::files::File".to_string(),
        Ty::Gen(t) => format!("pyrt::Gen<{}>", rty(t, p)),
        Ty::Func(ps, ret) => {
            let ps: Vec<String> = ps.iter().map(|t| fn_arg_ty(t, p)).collect();
            let r = if **ret == Ty::None { String::new() } else { format!(" -> {}", rty(ret, p)) };
            format!("impl Fn({}){}", ps.join(", "), r)
        }
        Ty::Tuple(ts) => {
            let v: Vec<String> = ts.iter().map(|t| rty(t, p)).collect();
            if v.len() == 1 {
                format!("({},)", v[0])
            } else {
                format!("({})", v.join(", "))
            }
        }
        Ty::Class(c) => match &p.classes[*c].ext {
            Some(path) => path.rsplit("::").next().unwrap().to_string(),
            None => p.classes[*c].name.clone(),
        },
        Ty::Obj(c) => struct_name(p, *c),
        Ty::Exc => "PyErr".into(),
        Ty::Optional(t) => match &**t {
            Ty::Class(c) if recursive(p, *c) => format!("Option<Box<{}>>", p.classes[*c].name),
            t => format!("Option<{}>", rty(t, p)),
        },
        Ty::Var(_) => "_".into(),
    }
}

/// Can it be a set member / dict key (Eq + Hash)?
fn hashable_ty(t: &Ty, p: &Program) -> bool {
    match t {
        Ty::Int | Ty::Bool | Ty::Str => true,
        Ty::Tuple(ts) => ts.iter().all(|t| hashable_ty(t, p)),
        Ty::Optional(t) => hashable_ty(t, p),
        Ty::Class(c) => {
            let k = &p.classes[*c];
            k.dataclass && k.frozen && k.subs.is_empty() && k.fields.iter().all(|f| hashable_ty(&f.ty, p))
        }
        _ => false,
    }
}

/// The struct of a class's own objects (a class with subclasses is an enum).
pub fn struct_name(p: &Program, c: ClassId) -> String {
    if p.classes[c].subs.is_empty() {
        p.classes[c].name.clone()
    } else {
        format!("{}Obj", p.classes[c].name)
    }
}

/// Classes that are constructed somewhere.
fn instantiated(p: &Program) -> HashSet<ClassId> {
    let mut out = HashSet::new();
    fn ex(e: &Expr, out: &mut HashSet<ClassId>) {
        if let ExprKind::New(c, _) = e.kind {
            out.insert(c);
        }
        e.children(&mut |c| ex(c, out));
    }
    fn st(s: &Stmt, out: &mut HashSet<ClassId>) {
        stmt_exprs(s, &mut |e| ex(e, out));
        for b in s.blocks() {
            b.iter().for_each(|x| st(x, out));
        }
    }
    for f in &p.funcs {
        f.body.iter().for_each(|s| st(s, &mut out));
    }
    out
}

/// Fields read / written through a value that may be any subclass.
fn enum_fields(p: &Program) -> (HashSet<(ClassId, String)>, HashSet<(ClassId, String)>) {
    let mut rd = HashSet::new();
    let mut wr = HashSet::new();
    let key = |e: &Expr| -> Option<(ClassId, String)> {
        if let ExprKind::Field(o, f) = &e.kind {
            if let Ty::Class(c) = o.ty {
                if !p.classes[c].subs.is_empty() {
                    return Some((c, f.clone()));
                }
            }
        }
        None
    };
    fn ex(p: &Program, e: &Expr, key: &dyn Fn(&Expr) -> Option<(ClassId, String)>, rd: &mut HashSet<(ClassId, String)>, wr: &mut HashSet<(ClassId, String)>) {
        if let Some(k) = key(e) {
            rd.insert(k);
        }
        match &e.kind {
            ExprKind::Builtin(n, a) if mutating_builtin(n) => {
                if let Some(k) = key(&a[0]) {
                    wr.insert(k);
                }
            }
            ExprKind::Method(r, f, _) if !p.funcs[*f].dispatch.is_empty() || true => {
                if let Some(k) = key(r) {
                    wr.insert(k);
                }
            }
            _ => {}
        }
        e.children(&mut |c| ex(p, c, key, rd, wr));
    }
    fn st(p: &Program, s: &Stmt, key: &dyn Fn(&Expr) -> Option<(ClassId, String)>, rd: &mut HashSet<(ClassId, String)>, wr: &mut HashSet<(ClassId, String)>) {
        match &s.kind {
            StmtKind::Assign(Target::Place(t), _) | StmtKind::AugAssign(t, _, _) => {
                if let Some(k) = key(t) {
                    wr.insert(k);
                }
            }
            _ => {}
        }
        stmt_exprs(s, &mut |e| ex(p, e, key, rd, wr));
        for b in s.blocks() {
            b.iter().for_each(|x| st(p, x, key, rd, wr));
        }
    }
    for f in &p.funcs {
        for s in &f.body {
            st(p, s, &key, &mut rd, &mut wr);
        }
    }
    (rd, wr)
}

pub fn mutating_builtin(n: &str) -> bool {
    matches!(
        n,
        "list.append" | "list.extend" | "list.insert" | "list.pop" | "list.pop_at" | "list.remove" | "list.sort"
            | "list.reverse" | "list.clear" | "list.swap" | "dict.pop" | "dict.pop_default" | "dict.setdefault"
            | "dict.clear" | "dict.update" | "set.add" | "set.remove" | "set.discard" | "set.clear"
    ) || stdlib::mutating(n)
}

/// Prefixes of the standard library builtins (`gen::stdlib`).
const STD: &[&str] = &[
    "re.",
    "file.",
    "os.",
    "open",
    "collections.", "deque.", "counter.", "heapq.", "bisect.", "itertools.", "functools.", "random.", "sys.", "time.",
    "json.",
];

/// A class that contains itself (through Optional fields) needs a Box.
pub fn recursive(p: &Program, c: ClassId) -> bool {
    fn reach(p: &Program, t: &Ty, target: ClassId, seen: &mut HashSet<ClassId>) -> bool {
        match t {
            Ty::Class(d) | Ty::Obj(d) => {
                // a value of d may be any subclass of d
                p.subtree(*d).into_iter().any(|x| {
                    if x == target {
                        return true;
                    }
                    if !seen.insert(x) {
                        return false;
                    }
                    p.classes[x].fields.iter().any(|f| reach(p, &f.ty, target, seen))
                })
            }
            Ty::Optional(x) => reach(p, x, target, seen),
            Ty::Tuple(ts) => ts.iter().any(|x| reach(p, x, target, seen)),
            _ => false,
        }
    }
    let mut seen = HashSet::new();
    p.subtree(c).into_iter().any(|x| p.classes[x].fields.iter().any(|f| reach(p, &f.ty, c, &mut seen)))
}

/// A field holding a class that contains itself is a `Box`.
pub fn boxed_field(p: &Program, t: &Ty) -> bool {
    matches!(t, Ty::Class(d) if recursive(p, *d))
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
    /// a reference to a `Box<T>` (the value of an `Option<Box<T>>`)
    boxed: bool,
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
    R { code: code.into(), kind, prec, boxed: false }
}

/// A number as the receiver of a method call (`5_i64.pow(2)`).
fn recv(x: &R) -> String {
    // arithmetic of literals only: give it a type
    let only_lits = x.code.chars().all(|c| c.is_ascii_digit() || " ._+-*/()e".contains(c));
    if only_lits && x.code.contains(|c: char| " +*/()".contains(c)) {
        let t = if x.code.contains('.') || x.code.contains('e') { "f64" } else { "i64" };
        return format!("(({}) as {})", x.code, t);
    }
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

thread_local! {
    /// some call site uses `.or_raise()`
    static OR_RAISE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// some code uses PyErr
    static PYERR: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// user exception classes raised as PyErr
    static EXC_USED: std::cell::RefCell<HashSet<String>> = std::cell::RefCell::new(HashSet::new());
    /// copies the generated code makes: (function, line, what)
    static CLONES: std::cell::RefCell<Vec<(FuncId, u32, String)>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// The copies (`.clone()`, `.to_vec()`, `.to_string()`) made by the last `generate`.
pub fn clones() -> Vec<(FuncId, u32, String)> {
    CLONES.with(|c| c.borrow().clone())
}

pub fn generate(p: &Program, o: &Own, x: &Exc) -> Res<String> {
    OR_RAISE.with(|c| c.set(false));
    PYERR.with(|c| c.set(false));
    CLONES.with(|c| c.borrow_mut().clear());
    EXC_USED.with(|c| c.borrow_mut().clear());
    let mut out = String::new();
    // constants something refers to (others would be dead code)
    let mut used_consts = HashSet::new();
    {
        fn ex(e: &Expr, u: &mut HashSet<usize>) {
            if let ExprKind::Const(c) = e.kind {
                u.insert(c);
            }
            e.children(&mut |x| ex(x, u));
        }
        fn st(s: &Stmt, u: &mut HashSet<usize>) {
            stmt_exprs(s, &mut |e| ex(e, u));
            s.blocks().iter().for_each(|b| b.iter().for_each(|x| st(x, u)));
        }
        p.funcs.iter().for_each(|f| f.body.iter().for_each(|s| st(s, &mut used_consts)));
        p.classes.iter().for_each(|c| c.fields.iter().filter_map(|f| f.default.as_ref()).for_each(|e| ex(e, &mut used_consts)));
        p.funcs.iter().for_each(|f| f.params.iter().filter_map(|q| q.default.as_ref()).for_each(|e| ex(e, &mut used_consts)));
        // constants used by constants
        loop {
            let n = used_consts.len();
            for (i, c) in p.consts.iter().enumerate() {
                if used_consts.contains(&i) {
                    ex(&c.value, &mut used_consts);
                }
            }
            if used_consts.len() == n {
                break;
            }
        }
    }
    for (ci, c) in p.consts.iter().enumerate() {
        if !used_consts.contains(&ci) {
            continue;
        }
        let mut g = FGen::new(p, o, x, p.main);
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
    let inst = instantiated(p);
    let fields = enum_fields(p);
    for (ci, c) in p.classes.iter().enumerate() {
        if (c.exc.is_some() && c.init.is_none()) || c.ext.is_some() {
            continue;
        }
        out += &gen_class(p, o, x, ci, c, &inst, &fields)?;
    }
    for (fid, f) in p.funcs.iter().enumerate() {
        if f.class.is_some() || fid == p.main || f.ext.is_some() {
            continue;
        }
        let mut g = FGen::new(p, o, x, fid);
        let code = g.func("")?;
        if f.cache {
            out += &stdlib::cache_wrapper(p, o, x, fid, &code)?;
        } else {
            out += &code;
        }
        out += "\n";
    }
    let mut g = FGen::new(p, o, x, p.main);
    let main = g.func("")?;
    if p.funcs.iter().any(|f| f.body.iter().any(|s| stmt_has_builtin(s, "sys.setrecursionlimit"))) {
        // deep recursion: the program runs on a thread with a large stack
        out += &main.replacen("fn main() {", "fn run() {", 1);
        out += "\nfn main() {\n    pyrt::with_stack(run);\n}\n";
    } else {
        out += &main;
    }
    let uses_map = p_uses_collections(p);
    let mut v = vec![];
    if uses_map.0 {
        v.push("IndexMap");
    }
    if uses_map.1 {
        v.push("IndexSet");
    }
    if uses_map.2 {
        v.push("VecDeque");
    }
    // printing goes through pyrt's buffered output
    if out.contains("print!(") {
        v.push("print");
    }
    if out.contains(" println!(") || out.contains("\tprintln!(") || out.contains("{println!(") {
        v.push("println");
    }
    if OR_RAISE.with(|c| c.get()) {
        v.push("OrRaise");
    }
    if PYERR.with(|c| c.get()) {
        v.push("PyErr");
    }
    let mut head = match v.len() {
        0 => String::new(),
        1 => format!("use pyrt::{};\n\n", v[0]),
        _ => format!("use pyrt::{{{}}};\n\n", v.join(", ")),
    };
    // crates (read by the project writer) and their types
    let mut crate_head = String::new();
    for c in &p.crates {
        crate_head += &format!("// py2rs-crate: {}\n", c);
    }
    let mut uses_ext: Vec<String> = p.classes.iter().filter_map(|c| c.ext.clone()).filter(|x| x.contains("::")).collect();
    uses_ext.sort();
    uses_ext.dedup();
    for u in &uses_ext {
        crate_head += &format!("use {};\n", u);
    }
    if !crate_head.is_empty() {
        head = format!("{}{}", crate_head, if head.is_empty() { "\n".to_string() } else { head });
    }
    // exception classes raised as values: their name and bases, most specific first
    let used = EXC_USED.with(|c| c.borrow().clone());
    let mut any = false;
    for c in &p.classes {
        if c.exc.is_some() && used.contains(&c.name) {
            let ch: Vec<String> = exc::chain(p, &c.name).iter().map(|k| format!("{:?}", k)).collect();
            head += &format!("const {}: &[&str] = &[{}];\n", exc_const(&c.name), ch.join(", "));
            any = true;
        }
    }
    if any {
        head += "\n";
    }
    if !all_reachable(p) {
        head = format!("// some definitions are not used by this program (a module of definitions)\n#![allow(dead_code)]\n\n{}", head);
    }
    Ok(head + &out)
}

/// Is every function called (directly or through others) from the program?
fn all_reachable(p: &Program) -> bool {
    let mut seen = vec![false; p.funcs.len()];
    let mut work = vec![p.main];
    seen[p.main] = true;
    // special methods are called by Rust traits (printing, ==, sorting)
    for c in &p.classes {
        for (n, f) in &c.methods {
            if matches!(n.as_str(), "__str__" | "__repr__" | "__eq__" | "__lt__") || c.exc.is_some() {
                seen[*f] = true;
                work.push(*f);
            }
        }
    }
    while let Some(f) = work.pop() {
        let mut found = vec![];
        fn ex(p: &Program, e: &Expr, found: &mut Vec<FuncId>) {
            match &e.kind {
                ExprKind::Call(f, _) | ExprKind::Method(_, f, _) | ExprKind::SuperInit(f, _) => found.push(*f),
                ExprKind::New(c, _) => found.extend(p.classes[*c].init),
                _ => {}
            }
            e.children(&mut |c| ex(p, c, found));
        }
        fn st(p: &Program, s: &Stmt, found: &mut Vec<FuncId>) {
            stmt_exprs(s, &mut |e| ex(p, e, found));
            if let StmtKind::Raise(_, Some(Expr { kind: ExprKind::New(c, _), .. })) = &s.kind {
                found.extend(p.classes[*c].init);
            }
            s.blocks().iter().for_each(|b| b.iter().for_each(|x| st(p, x, found)));
        }
        p.funcs[f].body.iter().for_each(|s| st(p, s, &mut found));
        found.extend(p.funcs[f].dispatch.iter().map(|(_, x)| *x));
        for g in found {
            if !seen[g] {
                seen[g] = true;
                work.push(g);
            }
        }
    }
    p.funcs.iter().enumerate().all(|(i, f)| seen[i] || f.ext.is_some() || (f.class.is_some() && !f.dispatch.is_empty()))
}

/// The constant naming an exception class and its bases.
fn exc_const(name: &str) -> String {
    if crate::front::EXCEPTIONS.contains(&name) {
        format!("pyrt::{}", snake(name).to_uppercase())
    } else {
        snake(name).to_uppercase()
    }
}

fn p_uses_collections(p: &Program) -> (bool, bool, bool) {
    fn ty(t: &Ty, m: &mut (bool, bool, bool)) {
        match t {
            Ty::Deque(e) => {
                m.2 = true;
                ty(e, m)
            }
            Ty::Dict(k, v, d) => {
                m.0 |= *d == DK::Plain;
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
    fn ex(e: &Expr, m: &mut (bool, bool, bool)) {
        ty(&e.ty, m);
        e.children(&mut |c| ex(c, m));
    }
    fn st(s: &Stmt, m: &mut (bool, bool, bool)) {
        let mut f = |e: &Expr| ex(e, m);
        stmt_exprs(s, &mut f);
        for b in sub_blocks(s) {
            b.iter().for_each(|s| st(s, m));
        }
    }
    let mut m = (false, false, false);
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
    s.blocks()
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
        StmtKind::If(c, ..) | StmtKind::While(c, ..) | StmtKind::LetSome(_, c, _) => f(c),
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

fn gen_class(
    p: &Program,
    o: &Own,
    x: &Exc,
    ci: ClassId,
    c: &Class,
    inst: &HashSet<ClassId>,
    fields: &(HashSet<(ClassId, String)>, HashSet<(ClassId, String)>),
) -> Res<String> {
    let mut out = String::new();
    let in_tree = c.base.is_some() || !c.subs.is_empty();
    if !c.subs.is_empty() {
        out += &gen_enum(p, o, x, ci, c, inst, fields)?;
    }
    // classes of a hierarchy that are never constructed need no struct
    if in_tree && !inst.contains(&ci) {
        return Ok(out);
    }
    let sname = struct_name(p, ci);
    let hashable = c.dataclass && c.frozen && c.subs.is_empty() && c.fields.iter().all(|f| hashable_ty(&f.ty, p));
    let user_eq = c.method("__eq__");
    let derives = if user_eq.is_some() {
        "Clone, Debug"
    } else if hashable {
        "Clone, Debug, PartialEq, Eq, Hash"
    } else if c.dataclass {
        "Clone, Debug, PartialEq"
    } else {
        "Clone, Debug"
    };
    if c.shared {
        out += &shared_struct(p, c, &sname);
    } else {
        out += &format!("#[derive({})]\npub struct {} {{\n", derives, sname);
        for f in &c.fields {
            let t = rty(&f.ty, p);
            let t = if boxed_field(p, &f.ty) { format!("Box<{}>", t) } else { t };
            out += &format!("    pub {}: {},\n", ident(&f.name), t);
        }
        out += "}\n\n";
    }
    // `==` (also in `in`, `list.index`, ...) and sorting use the class's own methods
    let other = |fid: FuncId| if o.params[fid].first() == Some(&Mode::Owned) { "other.clone()" } else { "other" };
    if let Some(f) = user_eq {
        out += &format!(
            "impl PartialEq for {0} {{\n    fn eq(&self, other: &Self) -> bool {{\n        self.op_eq({1})\n    }}\n}}\n\n",
            sname,
            other(f)
        );
    }
    if let Some(f) = c.method("__lt__") {
        let o2 = other(f);
        let back = if o2 == "other" { "self" } else { "self.clone()" };
        out += &format!(
            "impl pyrt::PyOrd for {0} {{\n    fn py_cmp(&self, other: &Self) -> std::cmp::Ordering {{\n        \
             if self.op_lt({1}) {{\n            std::cmp::Ordering::Less\n        }} else if other.op_lt({2}) {{\n            \
             std::cmp::Ordering::Greater\n        }} else {{\n            std::cmp::Ordering::Equal\n        }}\n    }}\n}}\n\n",
            sname, o2, back
        );
    }
    let methods: Vec<FuncId> = c.methods.iter().map(|(_, f)| *f).collect();
    if !methods.is_empty() {
        out += &format!("impl {} {{\n", sname);
        let mut first = true;
        for fid in &methods {
            if !first {
                out += "\n";
            }
            first = false;
            let mut g = FGen::new(p, o, x, *fid);
            let code = g.func("    ")?;
            out += &code;
        }
        out += "}\n\n";
    }
    let has_str = c.method("__str__").is_some();
    let has_repr = c.method("__repr__").is_some();
    if c.dataclass || has_str || has_repr {
        out += &format!("impl pyrt::Py for {} {{\n", sname);
        out += "    fn fmt_repr(&self, out: &mut String) {\n";
        if has_repr {
            out += "        out.push_str(&self.__repr__());\n";
        } else if c.dataclass {
            out += &format!("        out.push_str(\"{}(\");\n", c.name);
            for (i, f) in c.fields.iter().enumerate() {
                let sep = if i > 0 { ", " } else { "" };
                out += &format!("        out.push_str(\"{}{}=\");\n", sep, f.name);
                if c.shared {
                    out += &format!("        self.{}().fmt_repr(out);\n", ident(&f.name));
                } else {
                    out += &format!("        self.{}.fmt_repr(out);\n", ident(&f.name));
                }
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
    Ok(out)
}

/// A @shared class: `C(Rc<RefCell<CData>>)` with accessors; cloning `C` shares the object.
fn shared_struct(p: &Program, c: &Class, sname: &str) -> String {
    let mut out = format!("#[derive(Debug{})]\npub struct {}Data {{\n", if c.dataclass { ", PartialEq" } else { "" }, sname);
    for f in &c.fields {
        out += &format!("    pub {}: {},\n", ident(&f.name), rty(&f.ty, p));
    }
    out += "}\n\n";
    out += &format!("/// `@shared`: copies of a `{0}` are the same object, as in Python\n#[derive(Clone, Debug)]\npub struct {0}(pub std::rc::Rc<std::cell::RefCell<{0}Data>>);\n\n", sname);
    out += &format!("#[allow(dead_code)]\nimpl {} {{\n", sname);
    for f in &c.fields {
        let n = ident(&f.name);
        let t = rty(&f.ty, p);
        let get = if f.ty.is_copy() { format!("self.0.borrow().{}", n) } else { format!("self.0.borrow().{}.clone()", n) };
        out += &format!("    pub fn {0}(&self) -> {1} {{\n        {2}\n    }}\n", n, t, get);
        out += &format!(
            "    pub fn {0}_mut(&self) -> std::cell::RefMut<'_, {1}> {{\n        std::cell::RefMut::map(self.0.borrow_mut(), |d| &mut d.{0})\n    }}\n",
            n, t
        );
        out += &format!("    pub fn set_{0}(&self, v: {1}) {{\n        self.0.borrow_mut().{0} = v;\n    }}\n", n, t);
    }
    out += "    /// `a is b`\n    pub fn is(&self, other: &Self) -> bool {\n        std::rc::Rc::ptr_eq(&self.0, &other.0)\n    }\n}\n\n";
    if c.dataclass {
        out += &format!(
            "impl PartialEq for {0} {{\n    fn eq(&self, other: &Self) -> bool {{\n        self.is(other) || *self.0.borrow() == *other.0.borrow()\n    }}\n}}\n\n",
            sname
        );
    }
    out
}

/// A class with subclasses: an enum over the classes that are constructed.
fn gen_enum(
    p: &Program,
    o: &Own,
    x: &Exc,
    ci: ClassId,
    c: &Class,
    inst: &HashSet<ClassId>,
    fields: &(HashSet<(ClassId, String)>, HashSet<(ClassId, String)>),
) -> Res<String> {
    let vars: Vec<ClassId> = p.subtree(ci).into_iter().filter(|x| inst.contains(x)).collect();
    let all_dc = vars.iter().all(|x| p.classes[*x].dataclass);
    let mut out = format!("#[derive(Clone, Debug{})]\npub enum {} {{\n", if all_dc { ", PartialEq" } else { "" }, c.name);
    for x in &vars {
        out += &format!("    {}({}),\n", p.classes[*x].name, struct_name(p, *x));
    }
    out += "}\n\n";
    let arms = |body: &dyn Fn(&str) -> String| -> String {
        if vars.is_empty() {
            return "match *self {}".into();
        }
        let mut s = "match self {\n".to_string();
        for x in &vars {
            s += &format!("            {}::{}(x) => {},\n", c.name, p.classes[*x].name, body("x"));
        }
        s + "        }"
    };
    let mut items: Vec<String> = vec![];
    // field accessors
    for f in &c.fields {
        let key = (ci, f.name.clone());
        let n = ident(&f.name);
        if fields.0.contains(&key) {
            let (t, b) = if f.ty.is_copy() {
                (rty(&f.ty, p), format!("x.{}", n))
            } else {
                (format!("&{}", rty(&f.ty, p)), format!("&x.{}", n))
            };
            items.push(format!("    pub fn {}(&self) -> {} {{\n        {}\n    }}\n", f.name, t, arms(&|_| b.clone())));
        }
        if fields.1.contains(&key) {
            let b = format!("&mut x.{}", n);
            items.push(format!("    pub fn {}_mut(&mut self) -> &mut {} {{\n        {}\n    }}\n", f.name, rty(&f.ty, p), arms(&|_| b.clone())));
        }
    }
    // `isinstance(x, D)` for a D with subclasses: a copy as a D
    for d in narrowed_to(p) {
        if d == ci || !p.is_sub(d, ci) {
            continue;
        }
        let dv: Vec<ClassId> = p.subtree(d).into_iter().filter(|x| inst.contains(x)).collect();
        let mut m = String::new();
        for x in &dv {
            let xn = &p.classes[*x].name;
            m += &format!("            {}::{}(x) => Some({}::{}(x.clone())),\n", c.name, xn, p.classes[d].name, xn);
        }
        if dv.len() < vars.len() {
            m += "            _ => None,\n";
        }
        items.push(format!(
            "    pub fn as_{}(&self) -> Option<{}> {{\n        match self {{\n{}        }}\n    }}\n",
            snake(&p.classes[d].name), p.classes[d].name, m));
    }
    // dispatchers
    for (_, fid) in &c.dispatch {
        let mut g = FGen::new(p, o, x, *fid);
        items.push(g.func("    ")?);
    }
    // the base part of __init__, for super().__init__()
    if let Some(init) = c.init {
        if uses_super_init(p, init) {
            let mut g = FGen::new(p, o, x, init);
            g.init_tuple = true;
            items.push(g.func("    ")?);
        }
    }
    if !items.is_empty() {
        out += &format!("impl {} {{\n{}}}\n\n", c.name, items.join("\n"));
    }
    let printable = vars.iter().all(|x| {
        let k = &p.classes[*x];
        k.dataclass || k.method("__str__").is_some() || k.method("__repr__").is_some()
    });
    if printable {
        out += &format!(
            "impl pyrt::Py for {} {{\n    fn fmt_repr(&self, out: &mut String) {{\n        {}\n    }}\n    fn fmt_str(&self, out: &mut String) {{\n        {}\n    }}\n}}\n\n",
            c.name,
            arms(&|x| format!("{}.fmt_repr(out)", x)),
            arms(&|x| format!("{}.fmt_str(out)", x))
        );
    }
    // a subclass enum converts into this one
    for b in p.subtree(ci).into_iter().skip(1) {
        if p.classes[b].subs.is_empty() {
            continue;
        }
        let bv: Vec<ClassId> = p.subtree(b).into_iter().filter(|x| inst.contains(x)).collect();
        let mut m = String::new();
        for x in &bv {
            m += &format!("            {0}::{1}(x) => {2}::{1}(x),\n", p.classes[b].name, p.classes[*x].name, c.name);
        }
        let body = if bv.is_empty() { "match b {}".to_string() } else { format!("match b {{\n{}        }}", m) };
        out += &format!("impl From<{}> for {} {{\n    fn from(b: {}) -> Self {{\n        {}\n    }}\n}}\n\n", p.classes[b].name, c.name, p.classes[b].name, body);
    }
    Ok(out)
}

/// Classes with subclasses that some `isinstance` narrows to.
fn narrowed_to(p: &Program) -> Vec<ClassId> {
    let mut out = vec![];
    fn ex(p: &Program, e: &Expr, out: &mut Vec<ClassId>) {
        if let ExprKind::IsInst(_, d, Some(_)) = e.kind {
            if !p.classes[d].subs.is_empty() && !out.contains(&d) {
                out.push(d);
            }
        }
        e.children(&mut |c| ex(p, c, out));
    }
    fn st(p: &Program, s: &Stmt, out: &mut Vec<ClassId>) {
        stmt_exprs(s, &mut |e| ex(p, e, out));
        for b in s.blocks() {
            b.iter().for_each(|x| st(p, x, out));
        }
    }
    for f in &p.funcs {
        f.body.iter().for_each(|s| st(p, s, &mut out));
    }
    out
}

/// Some subclass calls `super().__init__()` reaching `init`.
fn uses_super_init(p: &Program, init: FuncId) -> bool {
    fn ex(e: &Expr, init: FuncId) -> bool {
        let mut found = matches!(e.kind, ExprKind::SuperInit(f, _) if f == init);
        e.children(&mut |c| found |= ex(c, init));
        found
    }
    p.funcs.iter().any(|f| f.is_init && f.body.iter().any(|s| {
        let mut found = false;
        stmt_exprs(s, &mut |e| found |= ex(e, init));
        found
    }))
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
    x: &'a Exc,
    /// exceptions that must be handled here: (catchers, label of the try block;
    /// None = return Err from the function)
    ctx: Vec<(HashSet<String>, Option<String>)>,
    tries: u32,
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
    /// enclosing loops: (label, label used, labeled blocks between here and the loop)
    loops: Vec<(String, bool, u32)>,
    /// strings whose characters / length a loop computed before it (`hoist_strs`)
    char_vecs: HashMap<VarId, String>,
    str_lens: HashMap<VarId, String>,
    /// `finally` blocks of the enclosing `try`s and the loop depth at each: a return,
    /// break or continue runs them first
    finals: Vec<(Vec<Stmt>, usize)>,
    loopn: u32,
    /// fields of `self` held in locals inside __init__
    init_fields: HashMap<String, String>,
    tmp: u32,
    /// statements to emit before the current one
    pre: Vec<String>,
    /// narrowed variables holding `&Box<T>`
    boxed: HashSet<VarId>,
    /// variables assigned in the region a narrowing covers
    region: HashSet<VarId>,
    renames: HashMap<VarId, String>,
    /// `self.f = f` in __init__: the parameter is the field value
    skip: HashSet<u32>,
    /// generate the base part of __init__ (`fn init(..) -> (fields)`)
    init_tuple: bool,
    /// inside `pyrt::catching(|| ..)`
    in_catching: u32,
    /// fields declared by super().__init__() that are assigned again
    init_mut: HashSet<String>,
    /// fields from super().__init__() overwritten before being read
    init_dead: HashSet<String>,
    /// variables read anywhere in the function
    used: HashSet<VarId>,
}

impl<'a> FGen<'a> {
    fn new(p: &'a Program, o: &'a Own, x: &'a Exc, fid: FuncId) -> Self {
        FGen {
            p,
            o,
            x,
            ctx: vec![(x.need[fid].clone(), None)],
            tries: 0,
            fid,
            f: &p.funcs[fid],
            kinds: HashMap::new(),
            decl: HashMap::new(),
            declared: HashSet::new(),
            muts: HashSet::new(),
            nonneg: HashSet::new(),
            labels: 0,
            loop_labels: vec![],
            finals: vec![],
            char_vecs: HashMap::new(),
            str_lens: HashMap::new(),
            loops: vec![],
            loopn: 0,
            init_fields: HashMap::new(),
            tmp: 0,
            pre: vec![],
            boxed: HashSet::new(),
            region: HashSet::new(),
            renames: HashMap::new(),
            skip: HashSet::new(),
            init_tuple: false,
            in_catching: 0,
            init_mut: HashSet::new(),
            init_dead: HashSet::new(),
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
        if let Some(n) = self.renames.get(&v) {
            return n.clone();
        }
        let var = &self.f.vars[v];
        if var.kind == VarKind::SelfParam {
            return "self".into();
        }
        if var.kind == VarKind::Bound && (var.name == "_" || !self.used.contains(&v)) {
            return "_".into();
        }
        if var.kind == VarKind::Narrow && !self.used.contains(&v) {
            return "_".into();
        }
        if matches!(var.kind, VarKind::Local | VarKind::Param) && !self.used.contains(&v) && !var.name.starts_with('_') && self.f.dispatch.is_empty() {
            return format!("_{}", ident(&var.name).trim_start_matches("r#"));
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
        } else if f.is_init && self.init_tuple {
            "init".to_string()
        } else if f.is_init {
            "new".to_string()
        } else {
            ident(&f.name)
        };
        let fields_of = |g: &Self| -> Vec<Field> { g.p.classes[f.class.unwrap()].fields.clone() };
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
            // a dispatcher lends an owned argument mutably to some implementation
            let lends = mode == Mode::Owned && f.dispatch.iter().any(|(_, x)| self.o.params[*x][i] == Mode::Mut);
            let m = if (mutable || lends) && matches!(mode, Mode::Copy | Mode::Owned) { "mut " } else { "" };
            params.push(format!("{}{}: {}", m, self.name(q.var), t));
            self.kinds.insert(q.var, k);
            self.declared.insert(q.var);
        }
        let ret = if f.is_init && self.init_tuple {
            let ts: Vec<String> = fields_of(self).iter().map(|fd| self.ty(&fd.ty)).collect();
            if ts.len() == 1 { format!(" -> ({},)", ts[0]) } else { format!(" -> ({})", ts.join(", ")) }
        } else if f.is_init {
            " -> Self".to_string()
        } else if f.ret == Ty::None {
            String::new()
        } else {
            format!(" -> {}", self.ty(&f.ret))
        };
        let ret = if self.x.fallible[self.fid] {
            PYERR.with(|c| c.set(true));
            match ret.strip_prefix(" -> ") {
                Some(t) => format!(" -> Result<{}, PyErr>", t),
                None => " -> Result<(), PyErr>".to_string(),
            }
        } else {
            ret
        };
        let ok = |v: String, g: &Self| if g.x.fallible[g.fid] { format!("Ok({})", v) } else { v };
        let vis = if f.class.is_some() { "pub " } else { "" };
        let mut out = format!("{}{}fn {}({}){} {{\n", ind, vis, name, params.join(", "), ret);
        let inner = format!("{}    ", ind);
        if is_main {
            out += &format!("{}pyrt::init();\n", inner);
        }
        if !f.dispatch.is_empty() {
            out += &self.dispatch_body(&inner);
            out += &format!("{}}}\n", ind);
            return Ok(out);
        }
        if f.is_init {
            self.init_prologue(&inner, &mut out)?;
        }
        if f.is_gen {
            // the body runs on the generator's own thread (`pyrt::Gen`)
            out += &format!("{}pyrt::Gen::new(move |y| {{\n", inner);
            let inner2 = format!("{}    ", inner);
            for s in &f.body {
                self.stmt(s, &inner2, &mut out)?;
            }
            out += &format!("{}}})\n{}}}\n", inner, ind);
            return Ok(out);
        }
        let body = &f.body;
        let n = body.len();
        for (i, s) in body.iter().enumerate() {
            // a final `return x` becomes the tail expression
            if i + 1 == n && !f.is_init {
                if let StmtKind::Return(Some(e)) = &s.kind {
                    self.pre_decls(s.id, &inner, &mut out);
                    let v = self.owned(e)?;
                    let pre = std::mem::take(&mut self.pre);
                    for p in pre {
                        out += &format!("{}{}\n", inner, p);
                    }
                    out += &format!("{}{}\n", inner, ok(v, self));
                    continue;
                }
            }
            self.stmt(s, &inner, &mut out)?;
            if matches!(s.kind, StmtKind::Return(_) | StmtKind::Raise(..)) {
                break;
            }
        }
        if f.is_init {
            let c = &self.p.classes[f.class.unwrap()];
            let fs: Vec<String> = c
                .fields
                .iter()
                .map(|fd| {
                    let local = &self.init_fields[&fd.name];
                    if boxed_field(self.p, &fd.ty) {
                        format!("{}: Box::new({})", ident(&fd.name), local)
                    } else if *local == ident(&fd.name) {
                        local.clone()
                    } else {
                        format!("{}: {}", ident(&fd.name), local)
                    }
                })
                .collect();
            if self.init_tuple {
                let ls: Vec<String> = fields_of(self).iter().map(|fd| self.init_fields[&fd.name].clone()).collect();
                let t = if ls.len() == 1 { format!("({},)", ls[0]) } else { format!("({})", ls.join(", ")) };
                out += &format!("{}{}\n", inner, ok(t, self));
            } else {
                let c = &self.p.classes[f.class.unwrap()];
                let v = if c.shared {
                    format!("Self(std::rc::Rc::new(std::cell::RefCell::new({}Data {{ {} }})))", c.name, fs.join(", "))
                } else {
                    format!("Self {{ {} }}", fs.join(", "))
                };
                out += &format!("{}{}\n", inner, ok(v, self));
            }
        } else if is_main {
            out += &format!("{}pyrt::flush();\n", inner);
        } else if matches!(f.ret, Ty::Optional(_)) && !always_exits(body) {
            out += &format!("{}{}\n", inner, ok("None".into(), self));
        } else if f.ret == Ty::None && self.x.fallible[self.fid] && !always_exits(body) {
            out += &format!("{}Ok(())\n", inner);
        } else if f.ret != Ty::None && !always_exits(body) {
            return err(f.line, format!("{}() can reach its end without returning a value", f.name))
                .map_err(|e| e.hint("add a return (Python would return None)"));
        }
        out += &format!("{}}}\n", ind);
        Ok(out)
    }

    /// `match self { C::X(x) => x.m(args), .. }`
    fn dispatch_body(&mut self, ind: &str) -> String {
        let f = self.f;
        let c = f.class.unwrap();
        let cname = &self.p.classes[c].name;
        let inst = instantiated(self.p);
        let arms: Vec<String> = f
            .dispatch
            .iter()
            .filter(|(x, _)| inst.contains(x))
            .map(|(x, imp)| {
                let args: Vec<String> = f
                    .params
                    .iter()
                    .enumerate()
                    .map(|(i, q)| {
                        let n = self.name(q.var);
                        match (self.o.params[self.fid][i], self.o.params[*imp][i]) {
                            (Mode::Owned, Mode::Ref) => format!("&{}", n),
                            (Mode::Owned, Mode::Mut) => format!("&mut {}", n),
                            _ => n,
                        }
                    })
                    .collect();
                let call = format!("x.{}({})", ident(&self.p.funcs[*imp].name), args.join(", "));
                let call = if self.x.fallible[self.fid] && !self.x.fallible[*imp] { format!("Ok({})", call) } else { call };
                format!("{}    {}::{}(x) => {},\n", ind, cname, self.p.classes[*x].name, call)
            })
            .collect();
        if arms.is_empty() {
            return format!("{}match *self {{}}\n", ind);
        }
        format!("{}match self {{\n{}{}}}\n", ind, arms.concat(), ind)
    }

    /// In `__init__`, every field is a local until `Self { .. }` is built.
    fn init_prologue(&mut self, ind: &str, out: &mut String) -> Res<()> {
        let c = &self.p.classes[self.f.class.unwrap()];
        let names: HashSet<&str> = self.f.vars.iter().map(|v| v.name.as_str()).collect();
        let sv0 = self.f.self_var.unwrap();
        let mutated0 = self.field_mutated_in_init();
        for st in &self.f.body {
            if let StmtKind::Assign(Target::Place(Expr { kind: ExprKind::Field(o, f), .. }), Expr { kind: ExprKind::Var(p), .. }) = &st.kind {
                let pv = &self.f.vars[*p];
                let assigned = assigned_vars(&self.f.body).contains(p);
                let once = self.f.body.iter().filter(|x| matches!(&x.kind, StmtKind::Assign(Target::Place(Expr { kind: ExprKind::Field(o2, f2), .. }), _) if f2 == f && matches!(o2.kind, ExprKind::Var(w) if w == sv0))).count() == 1;
                if matches!(o.kind, ExprKind::Var(w) if w == sv0) && pv.kind == VarKind::Param && pv.name == *f && !assigned && once && !mutated0.contains(f) {
                    self.skip.insert(st.id);
                    self.init_fields.insert(f.clone(), ident(f));
                    self.init_fields_declared(f);
                }
            }
        }
        for fd in &c.fields {
            if self.init_fields.contains_key(&fd.name) {
                continue;
            }
            let local = if names.contains(fd.name.as_str()) {
                format!("self_{}", fd.name)
            } else {
                ident(&fd.name)
            };
            self.init_fields.insert(fd.name.clone(), local);
        }
        // fields assigned more than once, in nested blocks or with a class default
        let mut counts: HashMap<String, (u32, bool)> = HashMap::new();
        fn walk(p: &Program, s: &[Stmt], depth: u32, sv: VarId, c: &mut HashMap<String, (u32, bool)>) {
            for st in s {
                if let StmtKind::Expr(Expr { kind: ExprKind::SuperInit(fid, _), .. }) = &st.kind {
                    for fd in &p.classes[p.funcs[*fid].class.unwrap()].fields {
                        let e = c.entry(fd.name.clone()).or_insert((0, false));
                        e.0 += 1;
                        if depth > 0 {
                            e.1 = true;
                        }
                    }
                }
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
                    walk(p, b, depth + 1, sv, c);
                }
            }
        }
        let sv = self.f.self_var.unwrap();
        walk(self.p, &self.f.body, 0, sv, &mut counts);
        let mutated = self.field_mutated_in_init();
        // fields set by a top-level super().__init__() are declared there
        let mut by_super: HashSet<String> = HashSet::new();
        for st in &self.f.body {
            if let StmtKind::Expr(Expr { kind: ExprKind::SuperInit(fid, _), .. }) = &st.kind {
                for fd in &self.p.classes[self.p.funcs[*fid].class.unwrap()].fields {
                    by_super.insert(fd.name.clone());
                }
            }
        }
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
            } else if by_super.contains(&fd.name) && !nested {
                if self.overwritten_after_super(&fd.name) {
                    // super().__init__()'s value is replaced before it is read
                    self.init_dead.insert(fd.name.clone());
                    if n > 2 || mutated.contains(&fd.name) {
                        out.push_str(&format!("{}let mut {}: {};\n", ind, local, self.ty(&fd.ty)));
                        self.init_fields_declared(&fd.name);
                    }
                } else if n > 1 || mutated.contains(&fd.name) {
                    self.init_mut.insert(fd.name.clone());
                }
            } else if n > 1 || nested || mutated.contains(&fd.name) {
                let m = if n > 1 || nested || mutated.contains(&fd.name) { "mut " } else { "" };
                out.push_str(&format!("{}let {}{}: {};\n", ind, m, local, self.ty(&fd.ty)));
                self.init_fields_declared(&fd.name);
            }
        }
        Ok(())
    }

    /// After the top-level super().__init__(), is field `f` assigned before it is read?
    fn overwritten_after_super(&self, f: &str) -> bool {
        let sv = self.f.self_var.unwrap();
        let body = &self.f.body;
        let Some(i) = body.iter().position(|s| matches!(&s.kind, StmtKind::Expr(Expr { kind: ExprKind::SuperInit(..), .. }))) else {
            return false;
        };
        fn mentions(e: &Expr, sv: VarId, f: &str) -> bool {
            let mut m = matches!(&e.kind, ExprKind::Field(o, g) if g == f && matches!(o.kind, ExprKind::Var(v) if v == sv));
            e.children(&mut |c| m |= mentions(c, sv, f));
            m
        }
        for s in &body[i + 1..] {
            if let StmtKind::Assign(Target::Place(Expr { kind: ExprKind::Field(o, g), .. }), v) = &s.kind {
                if g == f && matches!(o.kind, ExprKind::Var(w) if w == sv) {
                    return !mentions(v, sv, f);
                }
            }
            let mut m = false;
            stmt_exprs(s, &mut |e| m |= mentions(e, sv, f));
            if m || !s.blocks().is_empty() {
                return false;
            }
        }
        false
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
            // the declaring block is the block of path element `common`; when the
            // paths continue in different sub-blocks of the last shared statement
            // (if / else), declare before that statement instead
            let mut common = common;
            if common > 0 && paths.iter().any(|p| p.len() > common && p[common].0 != paths[0].get(common).map(|x| x.0).unwrap_or(usize::MAX)) {
                common -= 1;
            } else if common > 0 && paths.iter().any(|p| p.len() <= common) && paths.iter().any(|p| p.len() > common) {
                common -= 1;
            }
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
                if assign_count(blk, *v, &self.o.live_out) > 1 {
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
                let view = self.o.views[self.fid].contains(&v);
                let mview = self.o.mut_views[self.fid].contains(&v);
                let amp = if mview { "&mut " } else if view { "&" } else { "" };
                out.push_str(&format!("{}let {}{}: {}{};\n", ind, m, self.name(v), amp, self.ty(&self.f.vars[v].ty)));
                self.kinds.insert(v, if mview { Kind::Mut } else if view { Kind::Ref } else { Kind::Place });
            }
        }
    }

    // ------------------------------------------------------------ statements

    fn block(&mut self, s: &[Stmt], ind: &str, out: &mut String) -> Res<()> {
        for (i, st) in s.iter().enumerate() {
            if matches!(st.kind, StmtKind::LetSome(..)) {
                self.region = assigned_vars(&s[i + 1..]);
            }
            self.stmt(st, ind, out)?;
            // what follows a return / raise / break / continue never runs
            if matches!(st.kind, StmtKind::Return(_) | StmtKind::Raise(..) | StmtKind::Reraise(_) | StmtKind::Break | StmtKind::Continue) {
                break;
            }
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

    /// A statement; lines hoisted out of its expressions (`self.pre`) go first.
    fn stmt(&mut self, s: &Stmt, ind: &str, out: &mut String) -> Res<()> {
        let mut buf = String::new();
        let saved = std::mem::take(&mut self.pre);
        let hoisted = self.hoist_strs(s);
        let r = self.stmt_inner(s, ind, &mut buf);
        for (v, _) in &hoisted {
            self.char_vecs.remove(v);
            self.str_lens.remove(v);
        }
        for p in std::mem::replace(&mut self.pre, saved) {
            out.push_str(&format!("{}{}\n", ind, p));
        }
        for (_, line) in hoisted {
            out.push_str(&format!("{}{}\n", ind, line));
        }
        out.push_str(&buf);
        r
    }

    /// Is `v` assigned more than once in this function (parameters: at all)?
    fn assigned_in_fn(&self, v: VarId) -> bool {
        let n = assign_count(&self.f.body, v, &self.o.live_out);
        match self.f.vars[v].kind {
            VarKind::Param => n > 0,
            _ => n > 1,
        }
    }

    /// A loop that indexes (or measures) a string it does not reassign: its
    /// characters (or length) once before the loop, not at every access.
    fn hoist_strs(&mut self, s: &Stmt) -> Vec<(VarId, String)> {
        let body = match &s.kind {
            StmtKind::For(_, _, b, _) | StmtKind::While(_, b, _) => b,
            _ => return vec![],
        };
        let mut assigned = assigned_vars(body);
        if let StmtKind::For(t, _, _, _) = &s.kind {
            let mut vs = vec![];
            t.vars(&mut vs);
            assigned.extend(vs);
        }
        let (mut indexed, mut measured) = (vec![], vec![]);
        fn ex(e: &Expr, ix: &mut Vec<VarId>, ms: &mut Vec<VarId>) {
            match &e.kind {
                ExprKind::Index(b, _) | ExprKind::Slice(b, ..) if b.ty == Ty::Str => {
                    if let ExprKind::Var(v) = b.kind {
                        ix.push(v);
                    }
                }
                ExprKind::Builtin("len", a) if a[0].ty == Ty::Str => {
                    if let ExprKind::Var(v) = a[0].kind {
                        ms.push(v);
                    }
                }
                _ => {}
            }
            e.children(&mut |c| ex(c, ix, ms));
        }
        fn st(s: &Stmt, ix: &mut Vec<VarId>, ms: &mut Vec<VarId>) {
            stmt_exprs(s, &mut |e| ex(e, ix, ms));
            if let StmtKind::For(_, it, _, _) = &s.kind {
                it.exprs(&mut |e| ex(e, ix, ms));
            }
            s.blocks().iter().for_each(|b| b.iter().for_each(|x| st(x, ix, ms)));
        }
        st(s, &mut indexed, &mut measured);
        let mut out = vec![];
        let ok = |g: &Self, v: &VarId| {
            !assigned.contains(v) && !g.char_vecs.contains_key(v) && !g.str_lens.contains_key(v) && g.declared.contains(v)
        };
        indexed.sort();
        indexed.dedup();
        for v in indexed {
            if ok(self, &v) {
                let n = self.name(v);
                let cv = format!("{}_chars", n.trim_start_matches("r#"));
                out.push((v, format!("let {}: Vec<char> = {}.chars().collect();", cv, n)));
                self.char_vecs.insert(v, cv);
            }
        }
        measured.sort();
        measured.dedup();
        for v in measured {
            if ok(self, &v) {
                let n = self.name(v);
                let ln = format!("{}_len", n.trim_start_matches("r#"));
                out.push((v, format!("let {} = {}.chars().count() as i64;", ln, n)));
                self.str_lens.insert(v, ln);
            }
        }
        out
    }

    fn stmt_inner(&mut self, s: &Stmt, ind: &str, out: &mut String) -> Res<()> {
        if self.skip.contains(&s.id) {
            return Ok(());
        }
        self.pre_decls(s.id, ind, out);
        let inner = format!("{}    ", ind);
        match &s.kind {
            StmtKind::Expr(e @ Expr { kind: ExprKind::SuperInit(fid, _), .. }) => {
                let call = self.ex(e)?;
                let b = self.p.funcs[*fid].class.unwrap();
                let names: Vec<String> = self.p.classes[b].fields.iter().map(|fd| fd.name.clone()).collect();
                let all_new = names.iter().all(|n| !self.init_fields.contains_key(&format!("#declared:{}", n)));
                let locals: Vec<String> = names
                    .iter()
                    .map(|n| {
                        if self.init_dead.contains(n) {
                            return "_".to_string();
                        }
                        let l = self.init_fields[n].clone();
                        if all_new && self.init_mut.contains(n) { format!("mut {}", l) } else { l }
                    })
                    .collect();
                let pat = if locals.len() == 1 { format!("({},)", locals[0]) } else { format!("({})", locals.join(", ")) };
                if all_new {
                    out.push_str(&format!("{}let {} = {};\n", ind, pat, call.code));
                    for n in &names {
                        if !self.init_dead.contains(n) {
                            self.init_fields_declared(n);
                        }
                    }
                } else {
                    out.push_str(&format!("{}{} = {};\n", ind, pat, call.code));
                }
            }
            StmtKind::Expr(e) => {
                let v = self.ex(e)?;
                for p in std::mem::take(&mut self.pre) {
                    out.push_str(&format!("{}{}\n", ind, p));
                }
                out.push_str(&format!("{}{};\n", ind, v.code));
            }
            StmtKind::Pass => {}
            StmtKind::Break | StmtKind::Continue if !self.loops.is_empty() && self.finals.last().is_some_and(|f| f.1 >= self.loops.len()) => {
                // the finally code of the trys inside this loop, then leave
                self.run_finals(self.loops.len(), ind, out)?;
                let saved = std::mem::take(&mut self.finals);
                let r = self.stmt_inner(s, ind, out);
                self.finals = saved;
                r?;
            }
            StmtKind::Return(e) if !self.finals.is_empty() => {
                // the value first, then the finally code, then return it
                let v = match e {
                    Some(e) => Some(self.owned(e)?),
                    None => None,
                };
                for p in std::mem::take(&mut self.pre) {
                    out.push_str(&format!("{}{}\n", ind, p));
                }
                if let Some(v) = &v {
                    out.push_str(&format!("{}let ret = {};\n", ind, v));
                }
                self.run_finals(0, ind, out)?;
                let r = if v.is_some() { "ret" } else { "()" };
                if self.x.fallible[self.fid] {
                    out.push_str(&format!("{}return Ok({});\n", ind, r));
                } else if v.is_some() {
                    out.push_str(&format!("{}return ret;\n", ind));
                } else {
                    out.push_str(&format!("{}return;\n", ind));
                }
            }
            StmtKind::Break => match self.loop_labels.last() {
                Some(Some(l)) => out.push_str(&format!("{}break {};\n", ind, l)),
                _ => {
                    let l = self.loop_label();
                    out.push_str(&format!("{}break{};\n", ind, l));
                }
            },
            StmtKind::Continue => {
                let l = self.loop_label();
                out.push_str(&format!("{}continue{};\n", ind, l));
            }
            StmtKind::Return(None) if self.x.fallible[self.fid] => out.push_str(&format!("{}return Ok(());\n", ind)),
            StmtKind::Return(None) => out.push_str(&format!("{}return;\n", ind)),
            StmtKind::Return(Some(e)) => {
                let v = self.owned(e)?;
                for p in std::mem::take(&mut self.pre) {
                    out.push_str(&format!("{}{}\n", ind, p));
                }
                if self.x.fallible[self.fid] {
                    out.push_str(&format!("{}return Ok({});\n", ind, v));
                } else {
                    out.push_str(&format!("{}return {};\n", ind, v));
                }
            }
            StmtKind::Reraise(v) => {
                let n = self.name(*v);
                out.push_str(&format!("{}{};\n", ind, self.propagate(&n)));
            }
            StmtKind::Try(body, hs, orelse, fin) => self.try_stmt(body, hs, orelse, fin, ind, out)?,
            StmtKind::Assign(t, v) => self.assign(s, t, v, ind, out)?,
            StmtKind::AugAssign(t, op, v) => self.aug_assign(t, *op, v, ind, out)?,
            StmtKind::AugVar(v, op, e) => {
                let name = self.name(*v);
                let t = self.f.vars[*v].ty.clone();
                // a `nonlocal` variable is a `&mut` parameter
                let target = if self.kinds.get(v) == Some(&Kind::Mut) {
                    r(format!("*{}", name), Kind::Place, UNARY)
                } else {
                    r(name.clone(), Kind::Place, ATOM)
                };
                let line = self.aug_code(&target, &t, *op, e)?;
                out.push_str(&format!("{}{}\n", ind, line));
            }
            StmtKind::If(c, a, b) => {
                self.region = assigned_vars(a);
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
                let head = if forever {
                    "loop {\n".to_string()
                } else {
                    self.region = assigned_vars(body);
                    let cv = self.cond(c)?;
                    format!("while {} {{\n", cv)
                };
                let mut b = String::new();
                self.loop_labels.push(label.clone());
                self.loop_enter();
                self.block(body, &inner2, &mut b)?;
                let ll = self.loop_exit();
                self.loop_labels.pop();
                out.push_str(&format!("{}{}{}", ind2, ll, head));
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
                let mut b = String::new();
                self.rebind_loop_vars(t, &inner2, &mut b);
                self.loop_labels.push(label.clone());
                self.loop_enter();
                self.block(body, &inner2, &mut b)?;
                let ll = self.loop_exit();
                self.loop_labels.pop();
                out.push_str(&format!("{}{}for {} in {} {{\n", ind2, ll, pat, code));
                out.push_str(&b);
                out.push_str(&format!("{}}}\n", ind2));
                self.block(orelse, &ind2, out)?;
                if label.is_some() {
                    out.push_str(&format!("{}}}\n", ind));
                }
            }
            StmtKind::Raise(name, msg) => {
                let code = self.raise_code(name, msg.as_ref())?;
                out.push_str(&format!("{}{};\n", ind, code));
            }
            StmtKind::Assert(c, m) if self.catches(&["AssertionError"]) => {
                let cv = self.cond(c)?;
                let code = self.raise_code("AssertionError", m.as_ref())?;
                out.push_str(&format!("{}if !({}) {{\n{}    {};\n{}}}\n", ind, cv, ind, code, ind));
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
            StmtKind::LetSome(v, place, body) => {
                let l = self.let_some(*v, place)?;
                out.push_str(&format!("{}{} else {{\n", ind, l));
                self.block(body, &inner, out)?;
                out.push_str(&format!("{}}};\n", ind));
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
                        let c = self.may_fail("KeyError", "dict_pop", format!("{}, {}", self.mut_ref_of(&base), kb));
                        out.push_str(&format!("{}{};\n", ind, c));
                    }
                }
            }
        }
        Ok(())
    }

    /// ```text
    /// 'ok: {
    ///     let e = 'body: { BODY; ELSE; break 'ok; };   // an exception: break 'body e
    ///     match e { e if e.is("ValueError") => { .. } e => <pass it on> }
    /// }
    /// FINALLY
    /// ```
    /// Leaving `'ok` on success lets rustc see what the body assigned.
    fn loop_enter(&mut self) {
        self.loopn += 1;
        let l = if self.loopn == 1 { "'outer".to_string() } else { format!("'outer{}", self.loopn) };
        self.loops.push((l, false, 0));
    }

    /// `'label: ` if a break / continue needed it
    fn loop_exit(&mut self) -> String {
        let (l, used, _) = self.loops.pop().unwrap();
        if used {
            format!("{}: ", l)
        } else {
            String::new()
        }
    }

    /// ` 'label` when break / continue crosses a labeled block (a try)
    fn loop_label(&mut self) -> String {
        match self.loops.last_mut() {
            Some((l, used, n)) if *n > 0 => {
                *used = true;
                format!(" {}", l)
            }
            _ => String::new(),
        }
    }

    fn try_stmt(&mut self, body: &[Stmt], hs: &[Handler], orelse: &[Stmt], fin: &[Stmt], ind: &str, out: &mut String) -> Res<()> {
        if let Some(l) = self.loops.last_mut() {
            l.2 += 1;
        }
        let r = self.try_stmt_(body, hs, orelse, fin, ind, out);
        if let Some(l) = self.loops.last_mut() {
            l.2 -= 1;
        }
        r
    }

    fn try_stmt_(&mut self, body: &[Stmt], hs: &[Handler], orelse: &[Stmt], fin: &[Stmt], ind: &str, out: &mut String) -> Res<()> {
        let fin_on = !fin.is_empty();
        if fin_on {
            self.finals.push((fin.to_vec(), self.loops.len()));
        }
        let r = self.try_body(body, hs, orelse, fin, ind, out);
        if fin_on {
            self.finals.pop();
        }
        r
    }

    /// The finally code of the innermost `try` (outside of it).
    fn fin_block(&mut self, fin: &[Stmt], ind: &str, out: &mut String) -> Res<()> {
        let top = if fin.is_empty() { None } else { self.finals.pop() };
        let r = self.block(fin, ind, out);
        if let Some(t) = top {
            self.finals.push(t);
        }
        r
    }

    /// The `finally` code of the enclosing `try`s entered at loop depth `depth` or deeper.
    fn run_finals(&mut self, depth: usize, ind: &str, out: &mut String) -> Res<()> {
        let all = self.finals.clone();
        for (i, (fin, d)) in all.iter().enumerate().rev() {
            if *d < depth {
                break;
            }
            // the finally code itself runs outside its try
            let saved = self.finals.split_off(i);
            let r = self.block(fin, ind, out);
            self.finals.extend(saved);
            r?;
        }
        Ok(())
    }

    fn try_body(&mut self, body: &[Stmt], hs: &[Handler], orelse: &[Stmt], fin: &[Stmt], ind: &str, out: &mut String) -> Res<()> {
        let inner = format!("{}    ", ind);
        let inner2 = format!("{}    ", inner);
        self.tries += 1;
        let n = if self.tries == 1 { String::new() } else { self.tries.to_string() };
        let (lbody, lok) = (format!("'body{}", n), format!("'ok{}", n));
        self.ctx.push((exc::try_catchers(hs, fin), Some(lbody.clone())));
        let mut b = String::new();
        let r = self.block(body, &inner2, &mut b);
        self.ctx.pop();
        r?;
        if !b.contains(&format!("{} ", lbody)) && !b.contains(&format!("{})", lbody)) && !b.contains(&format!("{};", lbody)) {
            // nothing in the body can raise what is caught
            for line in b.lines() {
                out.push_str(line.strip_prefix("        ").unwrap_or(line));
                out.push('\n');
            }
            // a body that always leaves has run the finally code at each exit
            if !always_exits(body) {
                self.block(orelse, ind, out)?;
                self.fin_block(fin, ind, out)?;
            }
            return Ok(());
        }
        let has_fin = !fin.is_empty();
        let all = hs.iter().any(|h| h.kinds.iter().any(|k| k == "BaseException" || k == "Exception"));
        // an exception no handler takes runs the finally code before it leaves
        let pend = false;
        let exits = always_exits(body);
        let mut code = String::new();
        code += &format!("{}let raised = {}: {{\n{}", inner, lbody, b);
        if !exits {
            self.block(orelse, &inner2, &mut code)?;
            code += &format!("{}break {}{};\n", inner2, lok, if pend { " None" } else { "" });
        }
        code += &format!("{}}};\n", inner);
        let mut arms = String::new();
        for h in hs {
            let mut name = self.name(h.var);
            if name == "_" || name == "err" {
                name = "caught".into();
            }
            self.renames.insert(h.var, name.clone());
            self.kinds.insert(h.var, Kind::Place);
            let catch_all = h.kinds.iter().any(|k| k == "BaseException" || k == "Exception");
            let used = self.used.contains(&h.var) || body_reraises(&h.body);
            let guard = if catch_all {
                String::new()
            } else {
                format!(" if {}", h.kinds.iter().map(|k| format!("{}.is({:?})", name, k)).collect::<Vec<_>>().join(" || "))
            };
            let bind = if used || !guard.is_empty() { name.clone() } else { "_".into() };
            let mut hb = String::new();
            self.block(&h.body, &format!("{}    ", inner2), &mut hb)?;
            if pend && !always_exits(&h.body) {
                hb += &format!("{}    None\n", inner2);
            }
            arms += &format!("{}{}{} => {{\n{}{}}}\n", inner2, bind, guard, hb, inner2);
            if catch_all {
                break;
            }
        }
        if !all {
            let p = self.propagate("other");
            if has_fin {
                let mut f = String::new();
                self.fin_block(fin, &format!("{}    ", inner2), &mut f)?;
                arms += &format!("{}other => {{\n{}{}    {};\n{}}}\n", inner2, f, inner2, p, inner2);
            } else {
                arms += &format!("{}other => {},\n", inner2, p);
            }
        }
        // a single catch-all handler needs no match
        let simple = hs.len() == 1 && all && !pend;
        if simple {
            let h = &hs[0];
            let name = self.name(h.var);
            let mut hb = String::new();
            self.block(&h.body, &inner, &mut hb)?;
            code = code.replacen(&format!("{}let raised = ", inner), &format!("{}let {} = ", inner, if self.used.contains(&h.var) || body_reraises(&h.body) { name } else { "_".into() }), 1);
            code += &hb;
        } else {
            code += &format!("{}match raised {{\n{}{}}}\n", inner, arms, inner);
        }
        if exits {
            // the body never finishes normally: no 'ok block
            let c: String = code.lines().map(|l| format!("{}\n", l.strip_prefix("    ").unwrap_or(l))).collect();
            out.push_str(&c);
            if !hs.iter().all(|h| always_exits(&h.body)) {
                self.fin_block(fin, ind, out)?;
            }
            return Ok(());
        }
        if pend {
            out.push_str(&format!("{}let pending = {}: {{\n{}{}}};\n", ind, lok, code, ind));
            self.fin_block(fin, ind, out)?;
            out.push_str(&format!("{}if let Some(e) = pending {{\n{}{};\n{}}}\n", ind, inner, self.propagate("e"), ind));
        } else {
            out.push_str(&format!("{}{}: {{\n{}{}}}\n", ind, lok, code, ind));
            self.fin_block(fin, ind, out)?;
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
                    Kind::Ref | Kind::Mut if *ty == Ty::Str => format!("{}.to_string()", name),
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
                    Some(s) => matches!(s.kind, ExprKind::Int(x) if x > 0) || self.is_nonneg(s),
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
            Target::Var(w) if self.declared.contains(w)
                && self.used.contains(w)
                && !self.o.live_out.get(&s.id).map(|l| l.contains(w)).unwrap_or(true)
                && self.f.vars[*w].kind == VarKind::Local =>
            {
                // the value is never read: keep only its side effects
                if !pure(v) && !matches!(v.kind, ExprKind::Var(_) | ExprKind::Field(..) | ExprKind::Index(..)) {
                    let val = self.owned(v)?;
                    out.push_str(&format!("{}let _ = {};\n", ind, val));
                }
            }
            // a `nonlocal` variable: the caller's, through `&mut`
            Target::Var(w) if self.kinds.get(w) == Some(&Kind::Mut) && self.f.vars[*w].kind == VarKind::Param => {
                let val = self.owned(v)?;
                out.push_str(&format!("{}*{} = {};\n", ind, self.name(*w), val));
            }
            Target::Var(w) if self.o.mut_views[self.fid].contains(w) => {
                // a mutable reference into a container
                let x = self.ex_mut(v)?;
                let val = self.mut_ref_of(&x);
                let name = self.name(*w);
                if self.decl.get(w) == Some(&Decl::At(s.id)) && !self.declared.contains(w) {
                    self.declared.insert(*w);
                    self.kinds.insert(*w, Kind::Mut);
                    let m = if self.muts.contains(w) && assign_count(&self.f.body, *w, &self.o.live_out) > 1 { "mut " } else { "" };
                    out.push_str(&format!("{}let {}{} = {};\n", ind, m, name, val));
                } else {
                    out.push_str(&format!("{}{} = {};\n", ind, name, val));
                }
            }
            Target::Var(w) => {
                let view = self.o.views[self.fid].contains(w);
                let val = if view { self.borrow(v)? } else { self.owned(v)? };
                let name = self.name(*w);
                if self.decl.get(w) == Some(&Decl::At(s.id)) && !self.declared.contains(w) {
                    self.declared.insert(*w);
                    self.kinds.insert(*w, if view { Kind::Ref } else { Kind::Place });
                    let m = if self.muts.contains(w) { "mut " } else { "" };
                    // a first value that is always overwritten before it is read
                    let dead = !view
                        && self.used.contains(w)
                        && !self.o.live_out.get(&s.id).map(|l| l.contains(w)).unwrap_or(true)
                        && (pure(v) || matches!(v.kind, ExprKind::Var(_)));
                    if dead {
                        let m = if count_paths(&self.f.body, *w, &self.o.live_out).1 > 1 || self.o.mut_vars[self.fid].contains(w) { "mut " } else { "" };
                        out.push_str(&format!("{}let {}{}: {};\n", ind, m, name, self.ty(&self.f.vars[*w].ty)));
                        return Ok(());
                    }
                    let ann = if needs_annotation(v) && !view { format!(": {}", self.ty(&v.ty)) } else { String::new() };
                    let val = if ann.is_empty() { val } else { untyped_new(&val) };
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
                    let code = r(part, Kind::Val, ATOM);
                    self.store_code(pl, code, ind, out)?;
                }
                Target::Tuple(_) => self.assign_parts(s, t, &part, ind, out)?,
            }
        }
        Ok(())
    }

    /// `xs[i] = v`, `d[k] = v`, `obj.f = v`
    fn store(&mut self, pl: &Expr, v: &Expr, ind: &str, out: &mut String) -> Res<()> {
        // `d[k] = d.get(k, 0) + x`: one lookup, the key copied only when it is new
        if let (ExprKind::Index(b, k), ExprKind::Bin(op @ (BinOp::Add | BinOp::Sub | BinOp::Mul), l, x)) = (&pl.kind, &v.kind) {
            if let ExprKind::Builtin("dict.get", ga) = &l.kind {
                let plain = matches!(b.ty, Ty::Dict(_, _, DK::Plain));
                let mut u = vec![];
                uses(x, &mut u);
                let root = match &b.kind {
                    ExprKind::Var(r) => Some(*r),
                    _ => None,
                };
                if plain && v.ty.is_copy() && same_expr(&ga[0], b) && same_expr(&ga[1], k) && root.is_some_and(|r| !u.contains(&r)) {
                    let xv = self.val(x)?.code;
                    let dv = self.val(&ga[2])?.code;
                    let base = self.ex_mut(b)?;
                    let kb = self.borrow(k)?;
                    let ko = self.owned(k)?;
                    let sym = crate::front::opstr(*op);
                    out.push_str(&format!(
                        "{0}if let Some(e) = {1}.get_mut({2}) {{\n{0}    *e {3}= {4};\n{0}}} else {{\n{0}    {1}.insert({5}, {6} {3} {4});\n{0}}}\n",
                        ind, par(&base, ATOM), kb, sym, xv, ko, dv
                    ));
                    return Ok(());
                }
            }
        }
        let mut val = self.owned(v)?;
        // a value that reads the container being written is computed first
        let mut root = pl;
        while let ExprKind::Index(b, _) | ExprKind::Field(b, _) | ExprKind::TupleField(b, _) = &root.kind {
            root = b;
        }
        if let ExprKind::Var(rv) = root.kind {
            let mut u = vec![];
            uses(v, &mut u);
            if u.contains(&rv) && matches!(pl.kind, ExprKind::Index(..)) && !v.ty.is_copy() || u.contains(&rv) && matches!(&pl.kind, ExprKind::Index(b, _) if !matches!(b.kind, ExprKind::Var(_))) {
                self.tmp += 1;
                let t = format!("value{}", if self.tmp == 1 { String::new() } else { self.tmp.to_string() });
                self.pre.push(format!("let {} = {};", t, val));
                val = t;
            }
        }
        self.store_code(pl, r(val, Kind::Val, LOW), ind, out)
    }

    fn store_code(&mut self, pl: &Expr, val: R, ind: &str, out: &mut String) -> Res<()> {
        match &pl.kind {
            ExprKind::Index(b, k) => {
                let base = self.ex_mut(b)?;
                match &b.ty {
                    Ty::Dict(..) => {
                        let key = self.owned(k)?;
                        out.push_str(&format!("{}{}.insert({}, {});\n", ind, par(&base, ATOM), key, val.code));
                    }
                    _ => {
                        let i = self.index_mut_code(&base, k)?;
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
                if self.is_shared(&o.ty) {
                    let obj = self.ex(o)?;
                    out.push_str(&format!("{}{}.set_{}({});\n", ind, par(&obj, ATOM), f, val.code));
                    return Ok(());
                }
                if self.is_enum(&o.ty) {
                    let obj = self.ex_mut(o)?;
                    out.push_str(&format!("{}*{}.{}_mut() = {};\n", ind, par(&obj, ATOM), f, val.code));
                    return Ok(());
                }
                let obj = self.ex_mut(o)?;
                let v = if boxed_field(self.p, &pl.ty) { format!("Box::new({})", val.code) } else { val.code };
                out.push_str(&format!("{}{}.{} = {};\n", ind, par(&obj, ATOM), ident(f), v));
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
                r(self.init_field(o, f).unwrap(), Kind::Place, ATOM)
            }
            ExprKind::Index(b, k) if matches!(b.ty, Ty::Dict(_, _, DK::Default | DK::Counter)) => {
                let base = self.ex_mut(b)?;
                let ko = self.owned(k)?;
                r(format!("*{}.at({})", par(&base, ATOM), ko), Kind::Place, UNARY)
            }
            ExprKind::Index(b, k) if matches!(b.ty, Ty::Dict(..)) => {
                let base = self.ex_mut(b)?;
                let kb = self.borrow(k)?;
                if self.catches(&["KeyError"]) {
                    let c = self.wrap(format!("pyrt::try_key_mut({}, {})", self.mut_ref_of(&base), kb));
                    r(format!("*{}", c), Kind::Place, UNARY)
                } else {
                    r(format!("{}[{}]", par(&base, ATOM), kb), Kind::Place, ATOM)
                }
            }
            _ => {
                let x = self.ex_mut(t)?;
                if x.kind == Kind::Mut {
                    r(format!("*{}", x.code), Kind::Place, UNARY)
                } else {
                    x
                }
            }
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
                    let cur = r(tc.clone(), Kind::Place, ATOM);
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

    /// A condition (bool); narrowing tests become `let Some(x) = ..` (let chains).
    fn cond(&mut self, e: &Expr) -> Res<String> {
        match &e.kind {
            ExprKind::IsSome(x, true, Some(v)) => self.let_some(*v, x),
            ExprKind::IsInst(x, d, Some(v)) => self.let_variant(*v, x, *d),
            ExprKind::And(xs) if matches!(xs[0].kind, ExprKind::IsSome(_, true, Some(_)) | ExprKind::IsInst(_, _, Some(_))) => {
                let mut parts = vec![];
                for x in xs {
                    if let ExprKind::IsSome(p, true, Some(v)) = &x.kind {
                        parts.push(self.let_some(*v, p)?);
                    } else if let ExprKind::IsInst(p, d, Some(v)) = &x.kind {
                        parts.push(self.let_variant(*v, p, *d)?);
                    } else {
                        let c = self.val(x)?;
                        parts.push(par(&c, AND + 1));
                    }
                }
                Ok(parts.join(" && "))
            }
            _ => Ok(self.val(e)?.code),
        }
    }

    /// `let C::D(v) = &place` for `isinstance(place, D)`.
    fn let_variant(&mut self, v: VarId, place: &Expr, d: ClassId) -> Res<String> {
        let Ty::Class(c) = place.ty else { unreachable!() };
        if !instantiated(self.p).contains(&d) {
            return err(place.line, format!("`{}` is never created, so this isinstance() is always false", self.p.classes[d].name));
        }
        if !self.p.classes[d].subs.is_empty() {
            let p = self.ex(place)?;
            self.kinds.insert(v, Kind::Place);
            return Ok(format!("let Some({}) = {}.as_{}()", self.name(v), par(&p, ATOM), snake(&self.p.classes[d].name)));
        }
        let l = self.let_some(v, place)?;
        let ctor = format!("{}::{}", self.p.classes[c].name, self.p.classes[d].name);
        Ok(l.replacen("let Some(", &format!("let {}(", ctor), 1))
    }

    /// `let Some(v) = &place`, binding `v` by value, reference or mutable reference.
    fn let_some(&mut self, v: VarId, place: &Expr) -> Res<String> {
        let p = self.ex(place)?;
        let ty = self.f.vars[v].ty.clone();
        // `while cur is not None: ... cur = cur.next`: the value needs its own name
        if let ExprKind::Var(root) = place.kind {
            if self.region.contains(&root) && self.f.vars[root].name == self.f.vars[v].name {
                let taken: HashSet<&str> = self.f.vars.iter().map(|x| x.name.as_str()).collect();
                let alt = match &ty {
                    Ty::Class(c) => snake(&self.p.classes[*c].name),
                    _ => String::new(),
                };
                let n = if !alt.is_empty() && !taken.contains(alt.as_str()) { alt } else { format!("{}_", self.f.vars[v].name) };
                self.renames.insert(v, n);
            }
        }
        let name = self.name(v);
        let (src, k) = if self.o.consumed[self.fid].contains(&v) {
            // takes the value: `while let Some(mut node) = cur`
            if let Ty::Class(c) = ty {
                if recursive(self.p, c) {
                    self.boxed.insert(v);
                }
            }
            self.kinds.insert(v, Kind::Place);
            let m = if self.o.via[self.fid].contains(&v) { "mut " } else { "" };
            return Ok(format!("let Some({}{}) = {}", m, name, p.code));
        } else if ty.is_copy() {
            (self.val_r(p).code, Kind::Place)
        } else if self.o.via[self.fid].contains(&v) {
            (match p.kind {
                Kind::Mut => format!("&mut *{}", par(&p, UNARY)),
                _ => format!("&mut {}", par(&p, UNARY)),
            }, Kind::Mut)
        } else {
            (match p.kind {
                _ if p.boxed && p.kind == Kind::Place => format!("&*{}", par(&p, UNARY)),
                Kind::Ref => p.code.clone(),
                Kind::Mut => format!("&*{}", par(&p, UNARY)),
                _ => format!("&{}", par(&p, UNARY)),
            }, Kind::Ref)
        };
        self.kinds.insert(v, k);
        if let Ty::Class(c) = ty {
            if recursive(self.p, c) {
                self.boxed.insert(v);
            }
        }
        Ok(format!("let Some({}) = {}", name, src))
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
        if self.o.takes.contains(&e.id) {
            return Ok(match e.ty {
                Ty::Optional(_) => format!("{}.take()", par(&x, ATOM)),
                _ => format!("std::mem::take({})", self.mut_ref_of(&x)),
            });
        }
        let code = self.owned_r(&x, &e.ty, self.o.moves.contains(&e.id));
        if (code.ends_with(".clone()") || code.ends_with(".to_vec()") || code.ends_with(".to_string()")) && !matches!(e.kind, ExprKind::Str(_) | ExprKind::Const(_)) {
            let what = code.trim_end_matches(".clone()").trim_end_matches(".to_vec()").trim_end_matches(".to_string()").to_string();
            CLONES.with(|c| c.borrow_mut().push((self.fid, e.line, what)));
        }
        Ok(code)
    }

    fn owned_r(&self, x: &R, t: &Ty, moved: bool) -> String {
        if t.is_copy() {
            return self.val_r(x.clone()).code;
        }
        match x.kind {
            Kind::Val => x.code.clone(),
            Kind::Place if x.boxed && moved => format!("*{}", par(x, UNARY)),
            Kind::Place if x.boxed => format!("(*{}).clone()", par(x, UNARY)),
            Kind::Place if moved => x.code.clone(),
            Kind::Place => format!("{}.clone()", par(x, ATOM)),
            Kind::Ref | Kind::Mut if x.boxed => format!("(**{}).clone()", par(x, UNARY)),
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
            Ty::Class(c) if self.p.classes[*c].ext.is_some() => x.code,
            _ => format!("pyrt::s({})", self.borrow_r(&x)),
        })
    }

    fn ex(&mut self, e: &Expr) -> Res<R> {
        // an operation without a try_ variant whose exception is caught here:
        // inside `catching` everything raises plainly and is caught there
        let bad = exc::op_kinds(e).1;
        if !bad.is_empty() && self.catches(&bad) {
            self.in_catching += 1;
            let x = self.ex0(e);
            self.in_catching -= 1;
            let x = x?;
            return Ok(r(self.wrap(format!("pyrt::catching(|| {})", x.code)), Kind::Val, ATOM));
        }
        self.ex0(e)
    }

    fn ex0(&mut self, e: &Expr) -> Res<R> {
        use ExprKind as K;
        Ok(match &e.kind {
            K::Int(v) => {
                // beyond i32 an untyped literal would not compile
                let s = if i32::try_from(*v).is_err() { format!("{}_i64", v) } else { v.to_string() };
                r(s, Kind::Val, if *v < 0 { UNARY } else { ATOM })
            }
            K::Float(v) => {
                let s = float_lit(*v);
                let p = if s.starts_with('-') { UNARY } else { ATOM };
                r(s, Kind::Val, p)
            }
            K::Bool(b) => r(b.to_string(), Kind::Val, ATOM),
            K::Str(s) => r(rust_str(s), Kind::Ref, ATOM),
            K::NoneLit => r("()", Kind::Val, ATOM),
            K::Var(v) => {
                let mut x = r(self.name(*v), self.kind_of(*v), ATOM);
                x.boxed = self.boxed.contains(v);
                x
            }
            K::NoneVal => r("None", Kind::Val, ATOM),
            K::Upcast(x) => {
                let Ty::Class(c) = e.ty else { unreachable!() };
                // Upcast(Upcast(y)): straight to the outer class
                let x = match &x.kind {
                    K::Upcast(y) => y,
                    _ => x,
                };
                let v = self.owned(x)?;
                let cname = &self.p.classes[c].name;
                match x.ty {
                    Ty::Class(k) if !self.p.classes[k].subs.is_empty() => r(format!("{}::from({})", cname, v), Kind::Val, ATOM),
                    Ty::Class(k) | Ty::Obj(k) => r(format!("{}::{}({})", cname, self.p.classes[k].name, v), Kind::Val, ATOM),
                    _ => unreachable!(),
                }
            }
            K::IsInst(x, d, _) => {
                let Ty::Class(c) = x.ty else { unreachable!() };
                let inst = instantiated(self.p);
                let vs: Vec<String> = self
                    .p
                    .subtree(*d)
                    .into_iter()
                    .filter(|k| inst.contains(k))
                    .map(|k| format!("{}::{}(_)", self.p.classes[c].name, self.p.classes[k].name))
                    .collect();
                if vs.is_empty() {
                    r("false", Kind::Val, ATOM)
                } else {
                    let xv = self.ex(x)?;
                    r(format!("matches!({}, {})", xv.code, vs.join(" | ")), Kind::Val, ATOM)
                }
            }
            K::SuperInit(fid, args) => {
                let b = self.p.funcs[*fid].class.unwrap();
                let a = self.args(*fid, args)?;
                r(self.call_result(*fid, format!("{}::init({})", self.p.classes[b].name, a)), Kind::Val, ATOM)
            }
            K::SomeOf(x) if matches!(x.kind, K::Var(v) if self.boxed.contains(&v) && self.kind_of(v) == Kind::Place && self.o.moves.contains(&x.id)) => {
                r(format!("Some({})", self.name(match x.kind { K::Var(v) => v, _ => unreachable!() })), Kind::Val, ATOM)
            }
            K::SomeOf(x) => {
                let v = self.owned(x)?;
                match &x.ty {
                    Ty::Class(c) if recursive(self.p, *c) => r(format!("Some(Box::new({}))", v), Kind::Val, ATOM),
                    _ => r(format!("Some({})", v), Kind::Val, ATOM),
                }
            }
            K::IsSome(x, some, _) => {
                let v = self.ex(x)?;
                r(format!("{}.{}()", par(&v, ATOM), if *some { "is_some" } else { "is_none" }), Kind::Val, ATOM)
            }
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
            K::Call(fid, args) if self.p.funcs[*fid].ext.is_some() => r(self.ext_call(*fid, None, args)?, Kind::Val, ATOM),
            K::Method(recv, fid, args) if self.p.funcs[*fid].ext.is_some() => r(self.ext_call(*fid, Some(recv), args)?, Kind::Val, ATOM),
            K::New(c, args) if self.p.classes[*c].ext.is_some() => {
                let init = self.p.classes[*c].init.unwrap();
                r(self.ext_call(init, None, args)?, Kind::Val, ATOM)
            }
            K::Call(fid, args) => {
                let f = &self.p.funcs[*fid];
                let name = match f.class {
                    Some(c) => format!("{}::{}", self.p.classes[c].name, ident(&f.name)),
                    None if f.name == "main" => "py_main".into(),
                    None => ident(&f.name),
                };
                let a = self.args(*fid, args)?;
                r(self.call_result(*fid, format!("{}({})", name, a)), Kind::Val, ATOM)
            }
            K::Method(recv, fid, args) => {
                let rv = if self.o.self_mut[*fid] { self.ex_mut(recv)? } else { self.ex(recv)? };
                let a = self.args(*fid, args)?;
                let name = ident(&self.p.funcs[*fid].name);
                r(self.call_result(*fid, format!("{}.{}({})", par(&rv, ATOM), name, a)), Kind::Val, ATOM)
            }
            K::New(c, args) => {
                let cl = &self.p.classes[*c];
                let sname = struct_name(self.p, *c);
                if let Some(init) = cl.init {
                    let a = self.args(init, args)?;
                    r(self.call_result(init, format!("{}::new({})", sname, a)), Kind::Val, ATOM)
                } else {
                    let mut fs = vec![];
                    for (fd, a) in cl.fields.iter().zip(args.iter()) {
                        let v = self.owned(a)?;
                        let v = if boxed_field(self.p, &fd.ty) { format!("Box::new({})", v) } else { v };
                        let n = ident(&fd.name);
                        if v == n {
                            fs.push(n);
                        } else {
                            fs.push(format!("{}: {}", n, v));
                        }
                    }
                    if cl.shared {
                        r(format!("{}(std::rc::Rc::new(std::cell::RefCell::new({}Data {{ {} }})))", sname, sname, fs.join(", ")), Kind::Val, ATOM)
                    } else if fs.is_empty() {
                        r(format!("{} {{}}", sname), Kind::Val, ATOM)
                    } else {
                        r(format!("{} {{ {} }}", sname, fs.join(", ")), Kind::Val, ATOM)
                    }
                }
            }
            K::Field(o, f) => {
                if let Some(local) = self.init_field(o, f) {
                    return Ok(r(local, Kind::Place, ATOM));
                }
                let ov = self.ex(o)?;
                if self.is_enum(&o.ty) {
                    // a @shared object's getter returns a copy (no borrow outlives it)
                    let k = if e.ty.is_copy() || self.is_shared(&o.ty) { Kind::Val } else { Kind::Ref };
                    return Ok(r(format!("{}.{}()", par(&ov, ATOM), f), k, ATOM));
                }
                let mut x = r(format!("{}.{}", par(&ov, ATOM), ident(f)), Kind::Place, ATOM);
                x.boxed = boxed_field(self.p, &e.ty);
                x
            }
            K::TupleField(o, i) => {
                let ov = self.ex(o)?;
                r(format!("{}.{}", par(&ov, ATOM), i), Kind::Place, ATOM)
            }
            K::Index(b, k) if matches!(b.ty, Ty::Dict(_, _, DK::Default)) && !self.p.present.contains(&e.id) => {
                // inserts the default
                let base = self.ex_mut(b)?;
                let ko = self.owned(k)?;
                if e.ty.is_copy() {
                    // a value right away: no borrow held
                    r(format!("{}.val({})", par(&base, ATOM), ko), Kind::Val, ATOM)
                } else {
                    r(format!("{}.at({})", par(&base, ATOM), ko), Kind::Mut, ATOM)
                }
            }
            K::Index(b, k) if matches!(b.ty, Ty::Dict(_, _, DK::Counter)) => {
                let base = self.ex(b)?;
                let kb = self.borrow(k)?;
                r(format!("{}.get_count({})", par(&base, ATOM), kb), Kind::Val, ATOM)
            }
            K::Index(b, k) if matches!(&b.kind, K::Var(v) if self.char_vecs.contains_key(v)) => {
                let K::Var(v) = &b.kind else { unreachable!() };
                let cv = self.char_vecs[v].clone();
                let i = self.val(k)?;
                r(self.may_fail("IndexError", "char_at", format!("&{}, {}", cv, i.code)), Kind::Val, ATOM)
            }
            K::Slice(b, lo, hi, st) if matches!(&b.kind, K::Var(v) if self.char_vecs.contains_key(v)) => {
                let K::Var(v) = &b.kind else { unreachable!() };
                let cv = self.char_vecs[v].clone();
                let opt = |x: &Option<Box<Expr>>, g: &mut Self| -> Res<String> {
                    Ok(match x {
                        Some(e) => format!("Some({})", g.val(e)?.code),
                        None => "None".into(),
                    })
                };
                let (l, h, s) = (opt(lo, self)?, opt(hi, self)?, opt(st, self)?);
                r(format!("pyrt::chars_slice(&{}, {}, {}, {})", cv, l, h, s), Kind::Val, ATOM)
            }
            K::Index(b, k) => {
                let base = self.ex(b)?;
                match &b.ty {
                    Ty::Str => {
                        let i = self.val(k)?;
                        r(self.may_fail("IndexError", "str_at", format!("{}, {}", self.borrow_r(&base), i.code)), Kind::Val, ATOM)
                    }
                    Ty::Dict(..) if self.catches(&["KeyError"]) => {
                        let kb = self.borrow(k)?;
                        r(self.wrap(format!("pyrt::try_key({}, {})", self.borrow_r(&base), kb)), Kind::Ref, ATOM)
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
                    r(format!("Vec::<{}>::new()", self.elem_ty(&e.ty)), Kind::Val, ATOM)
                } else {
                    let mut xs = vec![];
                    for x in v {
                        xs.push(self.owned(x)?);
                    }
                    // literals only: the element type would be ambiguous for rustc
                    if v.iter().all(|x| matches!(x.kind, K::Int(_) | K::Float(_))) {
                        let suf = if v[0].ty == Ty::Int { "_i64" } else { "_f64" };
                        xs[0] = if xs[0].starts_with('-') { format!("{}{}", xs[0], suf) } else { format!("{}{}", xs[0], suf) };
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
                    r(format!("IndexSet::<{}>::new()", self.elem_ty(&e.ty)), Kind::Val, ATOM)
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
                    r(format!("IndexMap::<{}>::new()", self.elem_ty(&e.ty)), Kind::Val, ATOM)
                } else {
                    let mut xs = vec![];
                    let lit_vals = v.iter().all(|(_, x)| matches!(x.kind, K::Int(_) | K::Float(_)));
                    for (i, (k, x)) in v.iter().enumerate() {
                        let kk = self.owned(k)?;
                        let mut vv = self.owned(x)?;
                        if i == 0 && lit_vals {
                            vv += if x.ty == Ty::Int { "_i64" } else { "_f64" };
                        }
                        xs.push(format!("({}, {})", kk, vv));
                    }
                    r(format!("IndexMap::from([{}])", xs.join(", ")), Kind::Val, ATOM)
                }
            }
            K::Comp(c) => self.comp(c, &e.ty)?,
            K::FStr(parts) => self.fstring(parts)?,
            K::Builtin(name, args) => self.builtin(name, args, e)?,
            K::Lambda(ps, body) if matches!(e.ty, Ty::Func(..)) => {
                // `square` itself when the lambda only forwards to a function that
                // takes its arguments the way a Callable passes them
                if let K::Call(fid, args) = &body.kind {
                    let f = &self.p.funcs[*fid];
                    let same = args.len() == ps.len()
                        && args.iter().zip(ps).all(|(a, p)| matches!(a.kind, K::Var(v) if v == *p))
                        && f.params.iter().enumerate().all(|(i, q)| {
                            let m = self.o.params[*fid][i];
                            if f.vars[q.var].ty.is_copy() { m == Mode::Copy } else { m == Mode::Ref }
                        })
                        && !self.x.fallible[*fid]
                        && f.class.is_none()
                        && !f.cache;
                    if same {
                        let name = if f.name == "main" { "py_main".to_string() } else { ident(&f.name) };
                        return Ok(r(name, Kind::Val, ATOM));
                    }
                }
                // a closure: Copy arguments by value, others by reference (`fn_arg_ty`)
                let mut pats = vec![];
                for v in ps {
                    let t = self.f.vars[*v].ty.clone();
                    self.kinds.insert(*v, if t.is_copy() { Kind::Place } else { Kind::Ref });
                    pats.push(format!("{}: {}", self.name(*v), fn_arg_ty(&t, self.p)));
                }
                let b = self.owned(body)?;
                r(format!("|{}| {}", pats.join(", "), b), Kind::Val, LOW)
            }
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

    /// Is an exception of one of these classes handled here (caught by a try
    /// around it, or by a caller)?
    fn catches<S: AsRef<str>>(&self, kinds: &[S]) -> bool {
        if self.in_catching > 0 {
            return false;
        }
        kinds.iter().any(|k| self.ctx.iter().any(|(c, _)| exc::caught(self.p, k.as_ref(), c)))
    }

    /// A `Result` expression whose error is handled here: `x?` or `check!`.
    fn wrap(&self, code: String) -> String {
        match self.ctx.iter().rev().find_map(|c| c.1.clone()) {
            Some(l) => format!("pyrt::check!({}, {})", code, l),
            None => format!("{}?", code),
        }
    }

    /// `pyrt::try_f(args)` when `kind` is handled here, else `pyrt::f(args)`.
    fn may_fail(&self, kind: &str, f: &str, args: String) -> String {
        if self.catches(&[kind]) {
            self.wrap(format!("pyrt::try_{}({})", f, args))
        } else {
            format!("pyrt::{}({})", f, args)
        }
    }

    /// A call of a user function that returns Result.
    fn call_result(&self, g: FuncId, code: String) -> String {
        if !self.x.fallible[g] {
            return code;
        }
        let ks: Vec<String> = self.x.may[g].iter().cloned().collect();
        if self.catches(&ks) {
            self.wrap(code)
        } else {
            OR_RAISE.with(|c| c.set(true));
            format!("{}.or_raise()", code)
        }
    }

    /// Pass an exception on: to the enclosing try, the caller, or the top.
    fn propagate(&self, e: &str) -> String {
        match self.ctx.iter().rev().find_map(|c| c.1.clone()) {
            Some(l) => format!("break {} {}", l, e),
            None if self.x.fallible[self.fid] => format!("return Err({})", e),
            None => format!("pyrt::uncaught({})", e),
        }
    }

    /// `raise K(msg)`
    fn raise_code(&mut self, kind: &str, msg: Option<&Expr>) -> Res<String> {
        // an exception object with fields
        if let Some(obj @ Expr { ty: Ty::Class(_), .. }) = msg {
            let o = self.owned(obj)?;
            if self.catches(&[kind]) {
                PYERR.with(|c| c.set(true));
                EXC_USED.with(|c| c.borrow_mut().insert(kind.to_string()));
                let p = self.propagate(&format!("PyErr::with_data({}, exc.msg_.clone(), exc)", exc_const(kind)));
                return Ok(format!("{{\n    let exc = {};\n    {}\n}}", o, p));
            }
            return Ok(format!("pyrt::raise({}, &{}.msg_)", rust_str(kind), o));
        }
        let m = match msg {
            Some(m) => self.borrow_str(m)?,
            None => "\"\"".into(),
        };
        if self.catches(&[kind]) {
            PYERR.with(|c| c.set(true));
            EXC_USED.with(|c| c.borrow_mut().insert(kind.to_string()));
            return Ok(self.propagate(&format!("PyErr::new({}, {})", exc_const(kind), m)));
        }
        Ok(format!("pyrt::raise({}, {})", rust_str(kind), m))
    }

    /// `T` of list[T] / set[T], `K, V` of dict[K, V]
    fn elem_ty(&self, t: &Ty) -> String {
        match t {
            Ty::List(e) | Ty::Set(e) => self.ty(e),
            Ty::Dict(k, v, _) => format!("{}, {}", self.ty(k), self.ty(v)),
            _ => "_".into(),
        }
    }

    /// Fields through accessors (`f()` / `f_mut()`): an enum over subclasses, or a @shared object.
    fn is_enum(&self, t: &Ty) -> bool {
        matches!(t, Ty::Class(c) if !self.p.classes[*c].subs.is_empty() || self.p.classes[*c].shared)
    }

    fn is_shared(&self, t: &Ty) -> bool {
        matches!(t, Ty::Class(c) if self.p.classes[*c].shared)
    }

    /// An expression about to be modified: a field of a value that may be any
    /// subclass goes through its `_mut` accessor.
    fn ex_mut(&mut self, e: &Expr) -> Res<R> {
        match &e.kind {
            ExprKind::Field(o, f) if self.init_field(o, f).is_none() => {
                if self.is_shared(&o.ty) {
                    let ov = self.ex(o)?;
                    return Ok(r(format!("&mut *{}.{}_mut()", par(&ov, ATOM), f), Kind::Mut, UNARY));
                }
                let ov = self.ex_mut(o)?;
                if self.is_enum(&o.ty) {
                    return Ok(r(format!("{}.{}_mut()", par(&ov, ATOM), f), Kind::Mut, ATOM));
                }
                Ok(r(format!("{}.{}", par(&ov, ATOM), ident(f)), Kind::Place, ATOM))
            }
            ExprKind::TupleField(o, i) => {
                let ov = self.ex_mut(o)?;
                Ok(r(format!("{}.{}", par(&ov, ATOM), i), Kind::Place, ATOM))
            }
            ExprKind::Index(b, k) if matches!(b.ty, Ty::Dict(_, _, DK::Default | DK::Counter)) => {
                let base = self.ex_mut(b)?;
                let ko = self.owned(k)?;
                Ok(r(format!("{}.at({})", par(&base, ATOM), ko), Kind::Mut, ATOM))
            }
            ExprKind::Index(b, k) if matches!(b.ty, Ty::Dict(..)) => {
                let base = self.ex_mut(b)?;
                let kb = self.borrow(k)?;
                if self.catches(&["KeyError"]) {
                    let c = self.wrap(format!("pyrt::try_key_mut({}, {})", self.mut_ref_of(&base), kb));
                    Ok(r(c, Kind::Mut, ATOM))
                } else {
                    Ok(r(format!("{}[{}]", par(&base, ATOM), kb), Kind::Place, ATOM))
                }
            }
            ExprKind::Index(b, k) if matches!(b.ty, Ty::List(_)) => {
                let base = self.ex_mut(b)?;
                let i = self.index_mut_code(&base, k)?;
                Ok(r(format!("{}[{}]", par(&base, ATOM), i), Kind::Place, ATOM))
            }
            _ => self.ex(e),
        }
    }

    /// An index into `base` that is about to be written: computed first when it
    /// reads `base` (rustc does not allow that inside the mutable borrow).
    fn index_mut_code(&mut self, base: &R, k: &Expr) -> Res<String> {
        let i = self.index_code(base, k)?;
        if i.contains(".len()") || i.contains("pyrt::") {
            self.tmp += 1;
            let t = if self.tmp == 1 { "idx".to_string() } else { format!("idx{}", self.tmp) };
            self.pre.push(format!("let {} = {};", t, i));
            return Ok(t);
        }
        Ok(i)
    }

    /// Rust index for a list index expression.
    fn index_code(&mut self, base: &R, k: &Expr) -> Res<String> {
        if self.catches(&["IndexError"]) {
            let i = self.val(k)?;
            return Ok(self.wrap(format!("pyrt::try_idx({}, {}.len())", i.code, par(base, ATOM))));
        }
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
                // a + b (+ c ...) as one format!
                let mut parts = vec![];
                fn flat<'e>(e: &'e Expr, out: &mut Vec<&'e Expr>) {
                    match &e.kind {
                        ExprKind::Bin(BinOp::Add, x, y) if x.ty == Ty::Str => {
                            flat(x, out);
                            flat(y, out);
                        }
                        _ => out.push(e),
                    }
                }
                flat(a, &mut parts);
                flat(b, &mut parts);
                let mut fmt = String::new();
                let mut args = vec![];
                for x in parts {
                    let (f, xs) = self.fmt_piece(x)?;
                    fmt += &f;
                    args.extend(xs);
                }
                return Ok(r(format!("format!(\"{}\", {})", fmt, args.join(", ")), Kind::Val, ATOM));
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
            Div | FloorDiv | Mod if !self.nonzero_lit(b) && self.catches(&["ZeroDivisionError"]) => {
                let f = match (op, int) {
                    (Div, true) => return Ok(r(self.wrap(format!("pyrt::try_div({} as f64, {} as f64)", par(x, CAST), par(y, CAST))), Kind::Val, ATOM)),
                    (Div, false) => "div",
                    (FloorDiv, true) => "floordiv",
                    (FloorDiv, false) => "ffloordiv",
                    (_, true) => "modulo",
                    _ => "fmod",
                };
                r(self.wrap(format!("pyrt::try_{}({}, {})", f, x.code, y.code)), Kind::Val, ATOM)
            }
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
        // `lo <= f(x) <= hi`: a computed middle operand is evaluated once
        if rest.len() > 1 && rest[..rest.len() - 1].iter().any(|(_, m)| !is_simple(m)) {
            if !rest.iter().all(|(_, e)| e.ty.is_copy()) || !first.ty.is_copy() || rest.iter().any(|(op, _)| matches!(op, CmpOp::In | CmpOp::NotIn)) {
                return err(first.line, "chained comparisons of non-number values need a simple middle operand");
            }
            let mut lets = vec![];
            let mut names = vec![self.val(first)?.code];
            for (i, (_, e)) in rest.iter().enumerate() {
                let c = self.val(e)?.code;
                if i + 1 < rest.len() && !is_simple(e) {
                    self.tmp += 1;
                    let t = format!("mid{}", self.tmp);
                    lets.push(format!("let {} = {};", t, c));
                    names.push(t);
                } else {
                    names.push(c);
                }
            }
            let mut ps = vec![];
            for (i, (op, _)) in rest.iter().enumerate() {
                let sym = match op {
                    CmpOp::Eq => "==",
                    CmpOp::Ne => "!=",
                    CmpOp::Lt => "<",
                    CmpOp::Le => "<=",
                    CmpOp::Gt => ">",
                    _ => ">=",
                };
                let l = &names[i];
                let l = if l.ends_with(" as i64") || l.ends_with(" as f64") { format!("({})", l) } else { l.clone() };
                ps.push(format!("{} {} {}", l, sym, names[i + 1]));
            }
            return Ok(r(format!("{{ {} {} }}", lets.join(" "), ps.join(" && ")), Kind::Val, ATOM));
        }
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
                    Ty::List(et) | Ty::Deque(et) => {
                        if **et == Ty::Str {
                            let x = self.ex(a)?;
                            if x.kind == Kind::Place {
                                format!("{}.contains(&{})", par(&c, ATOM), x.code)
                            } else if matches!(a.kind, ExprKind::Str(_)) {
                                format!("{}.iter().any(|e| e == {})", par(&c, ATOM), x.code)
                            } else {
                                format!("{}.iter().any(|e| *e == {}[..])", par(&c, ATOM), par(&x, ATOM))
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
                    Ty::Tuple(_) => {
                        // `x in ("+", "-")`
                        let ExprKind::Tuple(items) = &b.kind else {
                            let x = self.borrow(a)?;
                            return Ok(r(format!("{}[{}].contains({})", neg, c.code.trim_start_matches('(').trim_end_matches(')'), x), Kind::Val, ATOM));
                        };
                        let lits = items.iter().all(|i| matches!(i.kind, ExprKind::Int(_) | ExprKind::Str(_) | ExprKind::Bool(_)));
                        if lits {
                            let x = self.ex(a)?;
                            let subject = if a.ty == Ty::Str { format!("&{}[..]", par(&x, ATOM)) } else { self.val_r(x).code };
                            let mut pats = vec![];
                            for i in items {
                                pats.push(self.ex(i)?.code);
                            }
                            format!("matches!({}, {})", subject, pats.join(" | "))
                        } else {
                            let x = self.borrow(a)?;
                            let mut els = vec![];
                            for i in items {
                                els.push(self.owned(i)?);
                            }
                            format!("[{}].contains({})", els.join(", "), x)
                        }
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
                    // `a as i64 < b` would parse `<` as generic arguments
                    let ends_cast = [" as i64", " as f64", " as usize", " as u32"].iter().any(|c| x.code.ends_with(c));
                    let xs = if ends_cast && sym.starts_with('<') { format!("({})", x.code) } else { par(&x, CMP + 1) };
                    return Ok(r(format!("{} {} {}", xs, sym, par(&y, CMP + 1)), Kind::Val, CMP));
                }
                let x = self.ex(a)?;
                let y = self.ex(b)?;
                if a.ty == Ty::Str {
                    // String, &String and &str all compare as str
                    let lit = |e: &Expr| matches!(e.kind, ExprKind::Str(_) | ExprKind::Const(_));
                    let view = |r: &R, e: &Expr| if lit(e) { r.code.clone() } else if r.kind == Kind::Val || r.kind == Kind::Place { format!("{}.as_str()", par(r, ATOM)) } else { format!("&{}[..]", par(r, ATOM)) };
                    let both_plain = !matches!(x.kind, Kind::Ref | Kind::Mut) && !matches!(y.kind, Kind::Ref | Kind::Mut);
                    let (xs, ys) = if both_plain || (lit(a) && !matches!(y.kind, Kind::Ref | Kind::Mut)) || (lit(b) && !matches!(x.kind, Kind::Ref | Kind::Mut)) {
                        (par(&x, CMP + 1), par(&y, CMP + 1))
                    } else {
                        (view(&x, a), view(&y, b))
                    };
                    let ord = !matches!(op, CmpOp::Eq | CmpOp::Ne);
                    let (xs, ys) = if ord && (lit(a) || lit(b)) { (view(&x, a), view(&y, b)) } else { (xs, ys) };
                    return Ok(r(format!("{} {} {}", xs, sym, ys), Kind::Val, CMP));
                }
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

    /// A call of a crate function (`recv`: the receiver of a method).
    fn ext_call(&mut self, fid: FuncId, recv: Option<&Expr>, args: &[Expr]) -> Res<String> {
        let f = &self.p.funcs[fid];
        let x = f.ext.clone().unwrap();
        let mut av = vec![];
        for (i, a) in args.iter().enumerate() {
            let q = &f.params[i];
            let s = match self.o.params[fid][i] {
                Mode::Copy => {
                    let v = self.val(a)?;
                    match &q.width {
                        Some(w) => format!("{} as {}", par(&v, CAST), w),
                        None => v.code,
                    }
                }
                Mode::Owned => self.owned(a)?,
                Mode::Ref if a.ty == Ty::Str => self.borrow_str(a)?,
                Mode::Ref => self.borrow(a)?,
                Mode::Mut => {
                    let e = self.ex_mut(a)?;
                    self.mut_ref_of(&e)
                }
            };
            av.push(s);
        }
        let rv = match recv {
            Some(r) => {
                let e = if x.self_mut { self.ex_mut(r)? } else { self.ex(r)? };
                Some(par(&e, ATOM))
            }
            None => None,
        };
        let mut code = if x.rust.contains('{') {
            let mut c = x.rust.clone();
            if let Some(r) = &rv {
                c = c.replace("{self}", r);
            }
            for (i, a) in av.iter().enumerate() {
                c = c.replace(&format!("{{{}}}", i), a);
            }
            c
        } else if let Some(r) = &rv {
            format!("{}.{}({})", r, x.rust, av.join(", "))
        } else {
            format!("{}({})", x.rust, av.join(", "))
        };
        if x.result {
            code = if self.catches(&["ValueError"]) {
                self.wrap(format!("{}.map_err(pyrt::rust_err)", code))
            } else {
                format!("pyrt::rust_ok({})", code)
            };
        }
        if let Some(w) = &x.ret_width {
            let t = if w == "f32" { "f64" } else { "i64" };
            code = format!("({}) as {}", code, t);
        }
        Ok(code)
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
                    let x = self.ex_mut(a)?;
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
                    if *conv == Some('s') {
                        let x = self.ex(e)?;
                        fmt += "{}";
                        args.push(format!("pyrt::fmt(&pyrt::str({}), {})", self.borrow_r(&x), rust_str(spec)));
                        continue;
                    }
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
            Iter::Over(x) if matches!(&x.kind, ExprKind::List(v) if v.is_empty()) => {
                let v = format!("Vec::<{}>::new()", self.elem_ty(&x.ty));
                (if for_loop { v } else { format!("{}.into_iter()", v) }, Item::Val)
            }
            // `for w in text.split()`: the pieces borrowed, no list (text is not reassigned
            // while they are in use: it is a variable the enclosing code does not assign)
            Iter::Over(Expr { kind: ExprKind::Builtin(n @ ("str.split_ws" | "str.split"), a), .. })
                if for_loop && matches!(&a[0].kind, ExprKind::Var(v) if !self.assigned_in_fn(*v))
                    && a.get(1).is_none_or(|sep| matches!(&sep.kind, ExprKind::Str(s) if !s.is_empty())) =>
            {
                let s = self.borrow_str(&a[0])?;
                let code = if *n == "str.split_ws" {
                    format!("pyrt::split_ws_iter({})", s)
                } else {
                    format!("{}.split({})", par(&r(s, Kind::Val, UNARY), ATOM), self.borrow_str(&a[1])?)
                };
                (code, Item::Ref)
            }
            Iter::Over(x) if matches!(x.ty, Ty::Gen(_)) && !matches!(x.kind, ExprKind::Call(..)) => {
                // a generator object in a variable: advance it in place
                let c = self.ex_mut(x)?;
                let c = if c.kind == Kind::Mut { c.code } else { format!("{}.by_ref()", par(&c, ATOM)) };
                (c, Item::Val)
            }
            Iter::Over(x) => {
                let c = self.ex(x)?;
                match &x.ty {
                    Ty::Str => (format!("{}.chars().map(String::from)", par(&c, ATOM)), Item::Val),
                    Ty::Dict(..) => (format!("{}.keys()", par(&c, ATOM)), Item::Ref),
                    _ if c.kind == Kind::Val => {
                        if for_loop {
                            let code = match c.code.strip_prefix("vec!") {
                                Some(arr) if matches!(x.kind, ExprKind::List(_)) => arr.to_string(),
                                _ => c.code,
                            };
                            (code, Item::Val)
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
        if c.gens.len() != 1 || narrows(c) {
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
                let mut u = vec![];
                g.conds.iter().for_each(|c| uses(c, &mut u));
                let fp = blank_unused(&fp, &g.target, &u, self);
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
        let mut u = vec![];
        uses(&c.elt, &mut u);
        if let Some(v) = &c.val {
            uses(v, &mut u);
        }
        let mut tv = vec![];
        g.target.vars(&mut tv);
        let p = if tv.iter().any(|v| u.contains(v)) { blank_unused(&p, &g.target, &u, self) } else { "_".to_string() };
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
        if c.gens.len() != 1 || narrows(c) {
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

    /// Iterator yielding owned values (references copied / cloned).
    fn iter_values(&mut self, e: &Expr) -> Res<String> {
        let (code, item) = self.iter_arg(e)?;
        Ok(match item {
            Item::Ref if e.ty.is_copy() => format!("{}.copied()", code),
            Item::Ref => format!("{}.cloned()", code),
            Item::Tup(ref is) | Item::Zip(ref is) if has_ref(&item) => {
                // (a, b) with borrowed parts: own each part
                let tys = match &e.ty {
                    Ty::Tuple(ts) => ts.clone(),
                    _ => vec![],
                };
                let zip = matches!(item, Item::Zip(_));
                let mut pat = String::new();
                let mut parts = vec![];
                for (i, it) in is.iter().enumerate() {
                    let n = format!("p{}", i);
                    let t = tys.get(i).cloned().unwrap_or(Ty::Int);
                    parts.push(match it {
                        Item::Ref | Item::Mut if t.is_copy() => format!("*{}", n),
                        Item::Ref | Item::Mut => format!("{}.clone()", n),
                        _ => n.clone(),
                    });
                    pat = if i == 0 { n } else if zip { format!("({}, {})", pat, n) } else { format!("{}, {}", pat, n) };
                }
                let pat = if zip { pat } else { format!("({})", pat) };
                format!("{}.map(|{}| ({}))", code, pat, parts.join(", "))
            }
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
            "inline" | "inline_expr" => {
                let ExprKind::Str(code) = &a[0].kind else { unreachable!() };
                let code = code.trim().trim_end_matches(';').to_string();
                if name == "inline" {
                    v(format!("{{ {} }}", code))
                } else {
                    r(code, Kind::Val, LOW)
                }
            }
            "exc.is" => {
                let x = self.ex(&a[0])?;
                let ExprKind::Str(k) = &a[1].kind else { unreachable!() };
                v(format!("{}.is({:?})", par(&x, ATOM), k))
            }
            "truth" => {
                let x = self.ex(&a[0])?;
                match &a[0].ty {
                    Ty::Int => r(format!("{} != 0", par(&self.val_r(x), CMP + 1)), Kind::Val, CMP),
                    Ty::Float => r(format!("{} != 0.0", par(&self.val_r(x), CMP + 1)), Kind::Val, CMP),
                    _ => r(format!("!{}.is_empty()", par(&x, ATOM)), Kind::Val, UNARY),
                }
            }
            "len" if matches!(&a[0].kind, ExprKind::Var(v) if self.char_vecs.contains_key(v) || self.str_lens.contains_key(v)) => {
                let ExprKind::Var(w) = &a[0].kind else { unreachable!() };
                match self.char_vecs.get(w) {
                    Some(cv) => r(format!("{}.len() as i64", cv), Kind::Val, CAST),
                    None => v(self.str_lens[w].clone()),
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
                    r(format!("pyrt::{}_by_key({}, |{}| {})", short, bare(code), p, b), kind, ATOM)
                } else {
                    r(format!("pyrt::{}({})", short, bare(code)), kind, ATOM)
                }
            }
            "sum" => {
                let code = self.iter_values(&a[0])?;
                if e.ty == Ty::Float {
                    v(format!("pyrt::sum_f64({})", bare(code)))
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
                let x = r("x", if matches!(item, Item::Ref) { Kind::Ref } else { Kind::Place }, ATOM);
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
                    v(format!("pyrt::{}({}, |{}| {})", f, bare(src), p, b))
                } else if rev {
                    v(format!("pyrt::sorted_rev({})", bare(src)))
                } else {
                    let _ = et;
                    v(format!("pyrt::sorted({})", bare(src)))
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
                Ty::Str => {
                    let s = self.borrow_str(&a[0])?;
                    v(self.may_fail("ValueError", "int_str", s))
                }
                Ty::Float => v(format!("pyrt::int_float({})", self.val(&a[0])?.code)),
                _ => {
                    let x = self.val(&a[0])?;
                    r(format!("{} as i64", par(&x, CAST)), Kind::Val, CAST)
                }
            },
            "float" => {
                let s = self.borrow_str(&a[0])?;
                v(self.may_fail("ValueError", "float_str", s))
            }
            "round" => v(format!("pyrt::round({})", self.val(&a[0])?.code)),
            "min_or" | "max_or" => {
                let code = self.iter_values(&a[0])?;
                let d = self.owned(&a[1])?;
                v(format!("pyrt::{}({}, {})", name, bare(code), d))
            }
            "bin" | "hex" | "oct" => v(format!("pyrt::{}({})", name, self.val(&a[0])?.code)),
            "int_base" => {
                let s = self.borrow_str(&a[0])?;
                let b = self.val(&a[1])?;
                v(format!("pyrt::int_str_base({}, {})", s, b.code))
            }
            "unpack" => {
                let ExprKind::Int(n) = a[1].kind else { unreachable!() };
                let x = self.owned(&a[0])?;
                v(format!("pyrt::unpack{}({})", n, x))
            }
            "round_int" => {
                let x = self.val(&a[0])?;
                let n = self.val(&a[1])?;
                v(format!("pyrt::round_int({}, {})", x.code, n.code))
            }
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
            "exc.field" => {
                let x = self.ex(&a[0])?;
                let (ExprKind::Str(c), ExprKind::Str(f)) = (&a[1].kind, &a[2].kind) else { unreachable!() };
                r(format!("{}.data::<{}>().{}", par(&x, ATOM), c, ident(f)), Kind::Place, ATOM)
            }
            "yield" => v(format!("y.yield_({})", self.owned(&a[0])?)),
            "is" => {
                let x = self.ex(&a[0])?;
                let y = self.borrow(&a[1])?;
                v(format!("{}.is({})", par(&x, ATOM), y))
            }
            "gen.next" => {
                let g = self.ex_mut(&a[0])?;
                match a.get(1) {
                    Some(d) => {
                        let dv = self.owned(d)?;
                        v(format!("{}.next().unwrap_or({})", par(&g, ATOM), dv))
                    }
                    None => v(format!("pyrt::generator::next({})", self.mut_ref_of(&g))),
                }
            }
            "call_fn" => {
                let f = self.ex(&a[0])?;
                let mut xs = vec![];
                for x in &a[1..] {
                    xs.push(if x.ty.is_copy() { self.val(x)?.code } else { self.borrow(x)? });
                }
                v(format!("{}({})", par(&f, ATOM), xs.join(", ")))
            }
            n if STD.iter().any(|m| n.starts_with(m)) => self.std_builtin(n, a, e)?,
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
                ExprKind::Builtin(n, _) if mutating_builtin(n) => *m = true,
                ExprKind::Index(b, _) if matches!(b.ty, Ty::Dict(_, _, DK::Default)) && !g.p.present.contains(&e.id) => *m = true,
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
            "sqrt" | "log" if self.catches(&["ValueError"]) => v(self.wrap(format!("pyrt::try_{}({})", n, vs[0].code))),
            _ => v(format!("pyrt::math::{}({})", n, vs.iter().map(|x| x.code.clone()).collect::<Vec<_>>().join(", "))),
        })
    }

    fn list_method(&mut self, m: &str, a: &[Expr], e: &Expr) -> Res<R> {
        let recv = if mutating_builtin(&format!("list.{}", m)) { self.ex_mut(&a[0])? } else { self.ex(&a[0])? };
        let rc = par(&recv, ATOM);
        let v = |c: String| r(c, Kind::Val, ATOM);
        let et = match &a[0].ty {
            Ty::List(t) => (**t).clone(),
            _ => unreachable!(),
        };
        // `r.log_mut().push(f(r))`: f(r) cannot read r inside the mutable borrow
        let hoist = rc.contains("_mut()") || rc.contains("try_key_mut");
        let owned_arg = |g: &mut Self, e: &Expr| -> Res<String> {
            let x = g.owned(e)?;
            if hoist && !matches!(e.kind, ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Str(_) | ExprKind::Var(_)) {
                g.tmp += 1;
                let t = format!("arg{}", g.tmp);
                g.pre.push(format!("let {} = {};", t, x));
                return Ok(t);
            }
            Ok(x)
        };
        Ok(match m {
            "append" => v(format!("{}.push({})", rc, owned_arg(self, &a[1])?)),
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
            "pop" if self.catches(&["IndexError"]) => v(self.wrap(format!("pyrt::try_pop({})", self.mut_ref_of(&recv)))),
            "pop" => v(format!("{}.pop().expect(\"IndexError: pop from empty list\")", rc)),
            "pop_at" => {
                let i = self.val(&a[1])?;
                v(self.may_fail("IndexError", "pop_at", format!("{}, {}", self.mut_ref_of(&recv), i.code)))
            }
            "remove" => {
                let x = self.owned(&a[1])?;
                v(self.may_fail("ValueError", "remove", format!("{}, &{}", self.mut_ref_of(&recv), x)))
            }
            "index" => {
                let x = self.owned(&a[1])?;
                v(self.may_fail("ValueError", "index", format!("{}, &{}", self.borrow_r(&recv), x)))
            }
            "count" => {
                let x = self.owned(&a[1])?;
                v(format!("pyrt::count({}, &{})", self.borrow_r(&recv), x))
            }
            "reverse" => v(format!("{}.reverse()", rc)),
            "clear" => v(format!("{}.clear()", rc)),
            "copy" => v(self.owned_r(&recv, &a[0].ty, false)),
            "swap" => {
                let i = self.index_mut_code(&recv, &a[1])?;
                let j = self.index_mut_code(&recv, &a[2])?;
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
        let recv = if mutating_builtin(&format!("dict.{}", m)) { self.ex_mut(&a[0])? } else { self.ex(&a[0])? };
        let rc = par(&recv, ATOM);
        let v = |c: String| r(c, Kind::Val, ATOM);
        Ok(match m {
            "get" => {
                let k = self.borrow(&a[1])?;
                let d = self.owned(&a[2])?;
                let how = if e.ty.is_copy() { "copied" } else { "cloned" };
                v(format!("{}.get({}).{}().unwrap_or({})", rc, k, how, d))
            }
            "get_opt" => {
                let k = self.borrow(&a[1])?;
                let how = if e.ty.is_copy() { "copied" } else { "cloned" };
                v(format!("{}.get({}).{}()", rc, k, how))
            }
            "pop" => {
                let k = self.borrow(&a[1])?;
                v(self.may_fail("KeyError", "dict_pop", format!("{}, {}", self.mut_ref_of(&recv), k)))
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
        let recv = if mutating_builtin(&format!("set.{}", m)) { self.ex_mut(&a[0])? } else { self.ex(&a[0])? };
        let rc = par(&recv, ATOM);
        let v = |c: String| r(c, Kind::Val, ATOM);
        Ok(match m {
            "add" => v(format!("{}.insert({})", rc, self.owned(&a[1])?)),
            "remove" => {
                let x = self.borrow(&a[1])?;
                v(self.may_fail("KeyError", "set_remove", format!("{}, {}", self.mut_ref_of(&recv), x)))
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
            "index" => {
                let x = arg(self, 1)?;
                v(self.may_fail("ValueError", "index_str", format!("{}, {}", sb, x)))
            }
            "rindex" => v(format!("pyrt::rindex_str({}, {})", sb, arg(self, 1)?)),
            "split_ws" | "splitlines" | "strip" | "lstrip" | "rstrip" | "isdigit" | "isdecimal" | "isnumeric"
            | "isalpha" | "isalnum" | "isspace" | "isupper" | "islower" | "title" | "capitalize" | "swapcase" => {
                let f = m;
                v(format!("pyrt::{}({})", f, sb))
            }
            "rsplit_all" => v(format!("pyrt::rsplit_max({}, {}, -1)", sb, arg(self, 1)?)),
            _ => {
                // helpers taking the string and the arguments: strip_chars, zfill,
                // center, replace_n, find_from, partition, split_max, ...
                let mut xs = vec![sb];
                for i in 1..a.len() {
                    xs.push(arg(self, i)?);
                }
                v(format!("pyrt::{}({})", m, xs.join(", ")))
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

/// Variables assigned (rebound) anywhere in `s`.
fn assigned_vars(s: &[Stmt]) -> HashSet<VarId> {
    let mut out = HashSet::new();
    fn go(s: &[Stmt], out: &mut HashSet<VarId>) {
        for x in s {
            match &x.kind {
                StmtKind::Assign(t, _) => {
                    let mut v = vec![];
                    t.vars(&mut v);
                    out.extend(v);
                }
                StmtKind::AugVar(v, ..) => {
                    out.insert(*v);
                }
                _ => {}
            }
            for b in x.blocks() {
                go(b, out);
            }
        }
    }
    go(s, &mut out);
    out
}

fn snake(name: &str) -> String {
    let mut o = String::new();
    for (i, c) in name.chars().enumerate() {
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

/// `Vec::<T>::new()` → `Vec::new()` where the type is written next to it.
fn untyped_new(code: &str) -> String {
    for c in ["Vec", "IndexMap", "IndexSet", "VecDeque", "pyrt::Counter"] {
        if code.starts_with(&format!("{}::<", c)) && code.ends_with(">::new()") {
            return format!("{}::new()", c);
        }
    }
    if code.starts_with("pyrt::DefaultDict::<") && code.contains(">::new(\"") {
        return format!("pyrt::DefaultDict::new({}", code.rsplit_once("::new(").unwrap().1);
    }
    code.to_string()
}

fn has_ref(i: &Item) -> bool {
    match i {
        Item::Ref | Item::Mut => true,
        Item::Tup(v) | Item::Zip(v) => v.iter().any(has_ref),
        Item::Val => false,
    }
}

/// Can this statement raise (call, raise, failing builtin)?
fn may_raise(s: &Stmt) -> bool {
    fn ex(e: &Expr) -> bool {
        let mut f = matches!(e.kind, ExprKind::Call(..) | ExprKind::Method(..) | ExprKind::New(..) | ExprKind::SuperInit(..))
            || !exc::op_kinds(e).0.is_empty();
        e.children(&mut |c| f |= ex(c));
        f
    }
    let mut f = matches!(s.kind, StmtKind::Raise(..) | StmtKind::Reraise(_) | StmtKind::Assert(..) | StmtKind::Del(_));
    stmt_exprs(s, &mut |e| f |= ex(e));
    f || s.blocks().iter().any(|b| b.iter().any(may_raise))
}

/// A closure pattern with the names it does not use replaced by `_`.
fn blank_unused(pat: &str, t: &Target, used: &[VarId], g: &FGen) -> String {
    let mut vs = vec![];
    t.vars(&mut vs);
    let mut out = pat.to_string();
    for v in vs {
        if used.contains(&v) {
            continue;
        }
        let n = g.name(v);
        if n == "_" {
            continue;
        }
        // whole-word replacement
        let mut res = String::new();
        let bytes: Vec<char> = out.chars().collect();
        let nc: Vec<char> = n.chars().collect();
        let mut i = 0;
        while i < bytes.len() {
            let is_word = |c: char| c.is_alphanumeric() || c == '_';
            if bytes[i..].starts_with(&nc[..])
                && (i == 0 || !is_word(bytes[i - 1]))
                && (i + nc.len() >= bytes.len() || !is_word(bytes[i + nc.len()]))
            {
                res.push('_');
                i += nc.len();
            } else {
                res.push(bytes[i]);
                i += 1;
            }
        }
        out = res;
    }
    out.replace("&_", "_")
}

/// `(a..b)` as a function argument needs no parentheses.
fn bare(code: String) -> String {
    if !code.starts_with('(') || !code.ends_with(')') {
        return code;
    }
    // the first parenthesis must close at the end
    let mut depth = 0;
    for (i, c) in code.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 && i + 1 < code.len() {
                    return code;
                }
            }
            _ => {}
        }
    }
    let inner = &code[1..code.len() - 1];
    if inner.contains("..") && !inner.contains("..=") || inner.contains("..=") {
        inner.to_string()
    } else {
        code
    }
}

/// A comprehension condition binds a narrowed value (needs `if let`, so a loop).
fn narrows(c: &Comp) -> bool {
    fn ex(e: &Expr) -> bool {
        let mut f = matches!(e.kind, ExprKind::IsSome(_, _, Some(_)) | ExprKind::IsInst(_, _, Some(_)));
        e.children(&mut |x| f |= ex(x));
        f
    }
    c.gens.iter().any(|g| g.conds.iter().any(ex))
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
fn assign_count(s: &[Stmt], v: VarId, live: &HashMap<u32, HashSet<VarId>>) -> u32 {
    count_paths(s, v, live).1
}

/// (assignments on paths that continue after `s`, most assignments on any path)
/// Stores whose value is never read are not emitted and do not count.
/// (assignments on paths that continue after `s`, most on any path)
fn count_paths(s: &[Stmt], v: VarId, live: &HashMap<u32, HashSet<VarId>>) -> (u32, u32) {
    let (c, p, _) = count3(s, v, live);
    (c, p)
}

/// Also: most assignments on a path that leaves by an exception (None: no such path).
fn count3(s: &[Stmt], v: VarId, live: &HashMap<u32, HashSet<VarId>>) -> (u32, u32, Option<u32>) {
    let (mut cont, mut peak, mut exc) = (0u32, 0u32, None::<u32>);
    let mx = |a: Option<u32>, b: Option<u32>| match (a, b) {
        (Some(x), Some(y)) => Some(x.max(y)),
        (x, None) | (None, x) => x,
    };
    for x in s {
        let (c, p, e) = match &x.kind {
            StmtKind::Assign(t, _) => {
                let mut d = vec![];
                t.vars(&mut d);
                let dead = matches!(t, Target::Var(_)) && !live.get(&x.id).map(|l| l.contains(&v)).unwrap_or(true);
                let n = (d.contains(&v) && !dead) as u32;
                (n, n, if may_raise(x) { Some(0) } else { None })
            }
            StmtKind::AugVar(w, ..) => {
                let n = (*w == v) as u32;
                (n, n, if may_raise(x) { Some(0) } else { None })
            }
            StmtKind::If(_, a, b) => {
                let (ca, pa, ea) = count3(a, v, live);
                let (cb, pb, eb) = count3(b, v, live);
                let ca = if always_exits(a) { 0 } else { ca };
                let cb = if always_exits(b) { 0 } else { cb };
                (ca.max(cb), pa.max(pb), mx(ea, eb))
            }
            StmtKind::While(_, a, b) | StmtKind::For(_, _, a, b) => {
                let (_, pa, ea) = count3(a, v, live);
                let n = if pa > 0 { 2 } else { 0 };
                let (cb, pb, eb) = count3(b, v, live);
                (n + cb, n + pb, mx(ea.map(|_| n), eb.map(|e| e + n)))
            }
            StmtKind::Try(b, hs, e, f) => {
                let (cb, pb, eb) = count3(b, v, live);
                let (ce, pe, ee) = count3(e, v, live);
                let (cf, pf, _) = count3(f, v, live);
                let mut cont = cb + ce;
                let mut peak = pb.max(cb + pe);
                let mut out_exc = ee.map(|x| x + cb);
                if let Some(before) = eb {
                    for h in hs {
                        let (ch, ph, eh) = count3(&h.body, v, live);
                        if !always_exits(&h.body) {
                            cont = cont.max(before + ch);
                        }
                        peak = peak.max(before + ph);
                        out_exc = mx(out_exc, eh.map(|x| x + before));
                    }
                    // not every exception is caught
                    out_exc = mx(out_exc, Some(before));
                }
                (cont + cf, (peak + pf).max(cont + cf), out_exc.map(|x| x + cf))
            }
            StmtKind::LetSome(_, _, b) => {
                let (_, pb, eb) = count3(b, v, live);
                (0, pb, eb)
            }
            StmtKind::Raise(..) | StmtKind::Reraise(_) => (0, 0, Some(0)),
            _ => (0, 0, if may_raise(x) { Some(0) } else { None }),
        };
        peak = peak.max(cont + p);
        exc = mx(exc, e.map(|e| e + cont));
        cont += c;
        if always_exits(std::slice::from_ref(x)) {
            break;
        }
    }
    (cont, peak.max(cont), exc)
}
/// Expression without variables or calls (safe to evaluate repeatedly).
fn pure(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Bool(_) | ExprKind::Str(_) | ExprKind::Const(_) | ExprKind::Var(_) | ExprKind::NoneVal => true,
        ExprKind::Bin(_, a, b) => pure(a) && pure(b),
        ExprKind::Unary(_, a) | ExprKind::ToFloat(a) => pure(a),
        ExprKind::List(v) | ExprKind::Tuple(v) => v.iter().all(pure),
        _ => false,
    }
}

fn all_literal(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Int(_) | ExprKind::Float(_) => true,
        ExprKind::Bin(_, a, b) => all_literal(a) && all_literal(b),
        ExprKind::Unary(_, a) | ExprKind::ToFloat(a) => all_literal(a),
        _ => false,
    }
}

fn is_simple(e: &Expr) -> bool {
    matches!(e.kind, ExprKind::Var(_) | ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Const(_) | ExprKind::Str(_))
}

/// `let x = ...` needs a type when inference would not find the Python one.
fn needs_annotation(v: &Expr) -> bool {
    match &v.kind {
        ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::NoneVal => true,
        ExprKind::ToFloat(x) => all_literal(x),
        ExprKind::List(x) | ExprKind::Set(x) => x.is_empty(),
        ExprKind::Dict(x) => x.is_empty(),
        ExprKind::Bin(..) | ExprKind::Unary(..) => all_literal(v),
        _ => false,
    }
}

fn body_reraises(s: &[Stmt]) -> bool {
    s.iter().any(|x| matches!(x.kind, StmtKind::Reraise(_)) || x.blocks().iter().any(|b| body_reraises(b)))
}

/// Every path through `s` ends in return / raise / an endless loop.
fn always_exits(s: &[Stmt]) -> bool {
    match s.last().map(|x| &x.kind) {
        Some(StmtKind::Return(_)) | Some(StmtKind::Raise(..)) => true,
        Some(StmtKind::If(_, a, b)) => always_exits(a) && always_exits(b),
        Some(StmtKind::While(c, body, _)) => matches!(c.kind, ExprKind::Bool(true)) && !has_break(body),
        Some(StmtKind::Reraise(_)) => true,
        Some(StmtKind::Try(b, hs, _, f)) => always_exits(f) || (always_exits(b) && hs.iter().all(|h| always_exits(&h.body))),
        _ => false,
    }
}

fn has_break(s: &[Stmt]) -> bool {
    s.iter().any(|x| match &x.kind {
        StmtKind::Break => true,
        StmtKind::If(_, a, b) => has_break(a) || has_break(b),
        StmtKind::Try(b, hs, e, f) => has_break(b) || hs.iter().any(|h| has_break(&h.body)) || has_break(e) || has_break(f),
        StmtKind::LetSome(_, _, b) => has_break(b),
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
            // numbers are right-aligned by default
            if sp.align.is_none() && sp.width.is_some() {
                s.push('>');
            }
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

/// Does `s` (or a statement inside it) call the builtin `name`?
fn stmt_has_builtin(s: &Stmt, name: &str) -> bool {
    fn ex(e: &Expr, name: &str, found: &mut bool) {
        if let ExprKind::Builtin(n, _) = &e.kind {
            if *n == name {
                *found = true;
            }
        }
        e.children(&mut |c| ex(c, name, found));
    }
    let mut found = false;
    stmt_exprs(s, &mut |e| ex(e, name, &mut found));
    found || s.blocks().iter().any(|b| b.iter().any(|x| stmt_has_builtin(x, name)))
}

/// How a `Callable` takes an argument: Copy values by value, others by reference.
fn fn_arg_ty(t: &Ty, p: &Program) -> String {
    match t {
        t if t.is_copy() => rty(t, p),
        Ty::Str => "&str".into(),
        Ty::List(e) => format!("&[{}]", rty(e, p)),
        t => format!("&{}", rty(t, p)),
    }
}

/// The same variable / field / literal (for patterns like `d[k] = d.get(k, 0) + 1`).
fn same_expr(a: &Expr, b: &Expr) -> bool {
    match (&a.kind, &b.kind) {
        (ExprKind::Var(x), ExprKind::Var(y)) => x == y,
        (ExprKind::Int(x), ExprKind::Int(y)) => x == y,
        (ExprKind::Str(x), ExprKind::Str(y)) => x == y,
        (ExprKind::Field(o, f), ExprKind::Field(p, g)) => f == g && same_expr(o, p),
        (ExprKind::TupleField(o, i), ExprKind::TupleField(p, j)) => i == j && same_expr(o, p),
        _ => false,
    }
}
