//! Which exceptions can be caught, and so which code must return `Result`.
//!
//! An exception that no `try` catches ends the program, exactly like a Rust
//! panic with a traceback; only the paths from a `raise` (or a failing
//! builtin) to a `try` that catches it become `Result<T, PyErr>`.

use std::collections::HashSet;

use crate::diag::{err, Error, Res};
use crate::front::EXCEPTIONS;
use crate::ir::*;

#[derive(Default, Debug)]
pub struct Exc {
    /// exception classes a function may raise (uncaught inside it)
    pub may: Vec<HashSet<String>>,
    /// exception classes (and bases) that callers catch around a call to it
    pub need: Vec<HashSet<String>>,
    /// returns `Result`
    pub fallible: Vec<bool>,
}

/// Base class of an exception class.
pub fn parent(p: &Program, k: &str) -> Option<String> {
    let b = match k {
        "BaseException" => return None,
        "Exception" => "BaseException",
        "ZeroDivisionError" | "OverflowError" => "ArithmeticError",
        "IndexError" | "KeyError" => "LookupError",
        "NotImplementedError" => "RuntimeError",
        "FileNotFoundError" | "PermissionError" | "IsADirectoryError" => "OSError",
        k if EXCEPTIONS.contains(&k) => "Exception",
        k => {
            let c = p.classes.iter().find(|c| c.name == k)?;
            return match c.base {
                Some(b) => Some(p.classes[b].name.clone()),
                None => c.exc.clone(),
            };
        }
    };
    Some(b.to_string())
}

/// `k` and its base classes.
pub fn chain(p: &Program, k: &str) -> Vec<String> {
    let mut out = vec![k.to_string()];
    while let Some(b) = parent(p, out.last().unwrap()) {
        out.push(b);
    }
    out
}

/// Is an exception of class `k` caught by any of `catchers`?
pub fn caught(p: &Program, k: &str, catchers: &HashSet<String>) -> bool {
    chain(p, k).iter().any(|c| catchers.contains(c))
}

/// Exceptions an expression node itself may raise: (supported as `Result`,
/// not supported). Children are not included.
pub fn op_kinds(e: &Expr) -> (Vec<&'static str>, Vec<&'static str>) {
    use ExprKind as K;
    let nonzero = |x: &Expr| match &x.kind {
        K::Int(v) => *v != 0,
        K::Float(v) => *v != 0.0,
        K::ToFloat(i) => matches!(i.kind, K::Int(v) if v != 0),
        _ => false,
    };
    match &e.kind {
        K::Index(b, _) => match b.ty {
            Ty::Dict(_, _, DK::Plain) => (vec!["KeyError"], vec![]),
            Ty::Dict(..) => (vec![], vec![]),
            Ty::Deque(_) => (vec![], vec!["IndexError"]),
            _ => (vec!["IndexError"], vec![]),
        },
        K::Bin(BinOp::Div | BinOp::FloorDiv | BinOp::Mod, _, d) if !nonzero(d) && e.ty.is_num() => {
            (vec!["ZeroDivisionError"], vec![])
        }
        K::Builtin(n, a) => match *n {
            "int" if a[0].ty == Ty::Str => (vec!["ValueError"], vec![]),
            "int" => (vec![], vec!["ValueError", "OverflowError"]),
            "float" => (vec!["ValueError"], vec![]),
            "list.pop" | "list.pop_at" => (vec!["IndexError"], vec![]),
            "list.remove" | "list.index" | "str.index" => (vec!["ValueError"], vec![]),
            "dict.pop" | "set.remove" => (vec!["KeyError"], vec![]),
            "math.sqrt" | "math.log" => (vec!["ValueError"], vec![]),
            "math.log2" | "math.log10" | "math.asin" | "math.acos" | "math.log2args" | "math.pow" | "math.fmod"
            | "math.isqrt" | "math.factorial" | "math.comb" | "math.perm" | "chr" | "min_iter" | "max_iter"
            | "str.rindex" | "powmod" => (vec![], vec!["ValueError"]),
            // only an empty separator raises
            "str.split" if matches!(&a[1].kind, K::Str(sep) if !sep.is_empty()) => (vec![], vec![]),
            "str.split" => (vec![], vec!["ValueError"]),
            "math.exp" | "math.floor" | "math.ceil" | "math.trunc" | "round" | "math.sinh" | "math.cosh" => {
                (vec![], vec!["ValueError", "OverflowError"])
            }
            "ord" | "str.center_fill" | "str.ljust_fill" | "str.rjust_fill" => (vec![], vec!["TypeError"]),
            "unpack" | "int_base" | "str.split_max" | "str.rsplit_max" | "str.rsplit_all" | "str.partition" | "str.rpartition" => {
                (vec![], vec!["ValueError"])
            }
            "input" => (vec![], vec!["EOFError"]),
            "open" | "os.remove" => (vec![], vec!["FileNotFoundError", "PermissionError", "IsADirectoryError", "OSError"]),
            "file.read" | "file.readline" | "file.readlines" | "file.write" | "file.writelines" => (vec![], vec!["ValueError"]),
            "list.swap" | "heapq.heappop" | "heapq.heapreplace" | "deque.pop" | "deque.popleft" | "random.choice" => {
                (vec![], vec!["IndexError"])
            }
            "random.sample" | "random.randrange" | "random.randint" => (vec![], vec!["ValueError"]),
            "functools.reduce" if a.len() == 2 => (vec![], vec!["TypeError"]),
            _ => (vec![], vec![]),
        },
        _ => (vec![], vec![]),
    }
}

pub fn analyze(p: &Program) -> Res<Exc> {
    let n = p.funcs.len();
    let mut x = Exc { may: vec![HashSet::new(); n], need: vec![HashSet::new(); n], fallible: vec![false; n] };
    // may: bottom up to a fixpoint
    loop {
        let mut changed = false;
        for fid in 0..n {
            let f = &p.funcs[fid];
            let mut m = HashSet::new();
            if f.ext.as_ref().is_some_and(|e| e.result) {
                // a Rust Err becomes ValueError
                m.insert("ValueError".to_string());
            } else if f.dispatch.is_empty() {
                block_may(p, &x, &f.body, &mut m);
            } else {
                for (_, i) in &f.dispatch {
                    m.extend(x.may[*i].iter().cloned());
                }
            }
            if m != x.may[fid] {
                x.may[fid] = m;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    // need: top down to a fixpoint
    loop {
        let mut changed = false;
        for fid in 0..n {
            let f = &p.funcs[fid];
            let ctx = x.need[fid].clone();
            let mut adds: Vec<(FuncId, HashSet<String>)> = vec![];
            if f.dispatch.is_empty() {
                block_need(p, &x, &f.body, &ctx, &mut adds);
            } else {
                for (_, i) in &f.dispatch {
                    adds.push((*i, ctx.clone()));
                }
            }
            for (g, c) in adds {
                for k in c {
                    if x.need[g].insert(k) {
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    for fid in 0..n {
        // extern functions are not generated: their call sites convert the Result
        x.fallible[fid] = p.funcs[fid].ext.is_none() && x.may[fid].iter().any(|k| caught(p, k, &x.need[fid]));
        if p.funcs[fid].is_gen {
            // a generator runs on its own thread: no @shared (Rc) objects in or out
            fn shared(p: &Program, t: &Ty) -> bool {
                match t {
                    Ty::Class(c) | Ty::Obj(c) => p.classes[*c].shared,
                    Ty::List(e) | Ty::Set(e) | Ty::Optional(e) | Ty::Deque(e) | Ty::Gen(e) => shared(p, e),
                    Ty::Dict(k, v, _) => shared(p, k) || shared(p, v),
                    Ty::Tuple(ts) => ts.iter().any(|t| shared(p, t)),
                    _ => false,
                }
            }
            let f = &p.funcs[fid];
            if f.params.iter().any(|q| shared(p, &f.vars[q.var].ty)) || shared(p, &f.ret) {
                return err(f.line, "a generator cannot take or yield @shared objects (its body runs on its own thread)")
                    .map_err(|e| e.hint("return a list instead"));
            }
        }
        if x.fallible[fid] && p.funcs[fid].is_gen {
            return err(p.funcs[fid].line, format!("an exception raised inside generator `{}` would be caught by its caller", p.funcs[fid].name))
                .map_err(|e| e.hint("catch it inside the generator"));
        }
    }
    // a dispatcher and its implementations agree
    loop {
        let mut changed = false;
        for fid in 0..n {
            let f = &p.funcs[fid];
            if !f.dispatch.is_empty() {
                let any = f.dispatch.iter().any(|(_, i)| x.fallible[*i]);
                if any && !x.fallible[fid] {
                    x.fallible[fid] = true;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    for fid in 0..n {
        check_block(p, &x, &p.funcs[fid].body, &x.need[fid].clone())?;
    }
    Ok(x)
}

/// Exceptions an expression (with its children) may raise.
fn expr_may(p: &Program, x: &Exc, e: &Expr, out: &mut HashSet<String>) {
    let (a, b) = op_kinds(e);
    out.extend(a.iter().chain(b.iter()).map(|s| s.to_string()));
    match &e.kind {
        ExprKind::Call(f, _) | ExprKind::Method(_, f, _) | ExprKind::SuperInit(f, _) => out.extend(x.may[*f].iter().cloned()),
        ExprKind::New(c, _) => {
            if let Some(i) = p.classes[*c].init {
                out.extend(x.may[i].iter().cloned());
            }
        }
        _ => {}
    }
    e.children(&mut |c| expr_may(p, x, c, out));
}

fn stmt_exprs(s: &Stmt, f: &mut dyn FnMut(&Expr)) {
    match &s.kind {
        StmtKind::Expr(e) | StmtKind::Del(e) | StmtKind::Return(Some(e)) | StmtKind::Raise(_, Some(e)) => f(e),
        StmtKind::Assign(t, e) => {
            t.exprs(f);
            if let Target::Place(pl) = t {
                f(pl)
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
        StmtKind::Assert(c, m) => {
            f(c);
            if let Some(m) = m {
                f(m)
            }
        }
        _ => {}
    }
}

/// Kinds raised directly by a statement (not its expressions).
fn stmt_kinds(s: &Stmt) -> Vec<String> {
    match &s.kind {
        StmtKind::Raise(k, _) => vec![k.clone()],
        StmtKind::Assert(..) => vec!["AssertionError".into()],
        StmtKind::Del(e) => match &e.kind {
            ExprKind::Index(b, _) if matches!(b.ty, Ty::Dict(..)) => vec!["KeyError".into()],
            _ => vec!["IndexError".into()],
        },
        // `d[k] op= v` raises KeyError
        StmtKind::AugAssign(Expr { kind: ExprKind::Index(b, _), .. }, _, _) if matches!(b.ty, Ty::Dict(..)) => {
            vec!["KeyError".into()]
        }
        _ => vec![],
    }
}

fn block_may(p: &Program, x: &Exc, s: &[Stmt], out: &mut HashSet<String>) {
    for st in s {
        out.extend(stmt_kinds(st));
        stmt_exprs(st, &mut |e| expr_may(p, x, e, out));
        match &st.kind {
            StmtKind::Try(b, hs, e, f) => {
                let mut inner = HashSet::new();
                block_may(p, x, b, &mut inner);
                let catchers: HashSet<String> = hs.iter().flat_map(|h| h.kinds.iter().cloned()).collect();
                for k in inner {
                    if !caught(p, &k, &catchers) {
                        out.insert(k);
                    }
                }
                for h in hs {
                    let mut hm = HashSet::new();
                    block_may(p, x, &h.body, &mut hm);
                    // a bare `raise` re-raises what this handler catches
                    if reraises(&h.body) {
                        let mut bm = HashSet::new();
                        block_may(p, x, b, &mut bm);
                        let hc: HashSet<String> = h.kinds.iter().cloned().collect();
                        hm.extend(bm.into_iter().filter(|k| caught(p, k, &hc)));
                    }
                    out.extend(hm);
                }
                block_may(p, x, e, out);
                block_may(p, x, f, out);
            }
            _ => {
                for b in st.blocks() {
                    block_may(p, x, b, out);
                }
            }
        }
    }
}

fn reraises(s: &[Stmt]) -> bool {
    s.iter().any(|x| matches!(x.kind, StmtKind::Reraise(_)) || x.blocks().iter().any(|b| reraises(b)))
}

/// The catchers in effect inside a try body.
pub fn try_catchers(hs: &[Handler], fin: &[Stmt]) -> HashSet<String> {
    let mut c: HashSet<String> = hs.iter().flat_map(|h| h.kinds.iter().cloned()).collect();
    if !fin.is_empty() {
        c.insert("BaseException".into());
    }
    c
}

fn block_need(p: &Program, x: &Exc, s: &[Stmt], ctx: &HashSet<String>, adds: &mut Vec<(FuncId, HashSet<String>)>) {
    for st in s {
        stmt_exprs(st, &mut |e| expr_need(p, x, e, ctx, adds));
        match &st.kind {
            StmtKind::Try(b, hs, e, f) => {
                let mut inner = ctx.clone();
                inner.extend(try_catchers(hs, f));
                block_need(p, x, b, &inner, adds);
                for h in hs {
                    block_need(p, x, &h.body, ctx, adds);
                }
                block_need(p, x, e, ctx, adds);
                block_need(p, x, f, ctx, adds);
            }
            _ => {
                for b in st.blocks() {
                    block_need(p, x, b, ctx, adds);
                }
            }
        }
    }
}

fn expr_need(p: &Program, x: &Exc, e: &Expr, ctx: &HashSet<String>, adds: &mut Vec<(FuncId, HashSet<String>)>) {
    let callee = match &e.kind {
        ExprKind::Call(f, _) | ExprKind::Method(_, f, _) | ExprKind::SuperInit(f, _) => Some(*f),
        ExprKind::New(c, _) => p.classes[*c].init,
        _ => None,
    };
    if let Some(g) = callee {
        if x.may[g].iter().any(|k| caught(p, k, ctx)) {
            adds.push((g, ctx.clone()));
        }
    }
    e.children(&mut |c| expr_need(p, x, c, ctx, adds));
}

/// Reject what would be caught but cannot be expressed: an unsupported
/// failing builtin, a failing call inside a comprehension or lambda, or an
/// exception leaving a handler / else of a `try` with `finally`.
fn check_block(p: &Program, x: &Exc, s: &[Stmt], ctx: &HashSet<String>) -> Res<()> {
    for st in s {
        let mut r = Ok(());
        stmt_exprs(st, &mut |e| {
            if r.is_ok() {
                r = check_expr(p, x, e, ctx, false);
            }
        });
        r?;
        match &st.kind {
            StmtKind::Try(b, hs, e, f) => {
                let mut inner = ctx.clone();
                inner.extend(try_catchers(hs, f));
                check_block(p, x, b, &inner)?;
                for h in hs {
                    check_block(p, x, &h.body, ctx)?;
                }
                check_block(p, x, e, ctx)?;
                check_block(p, x, f, ctx)?;
                if !f.is_empty() {
                    let mut m = HashSet::new();
                    for h in hs {
                        block_may(p, x, &h.body, &mut m);
                    }
                    block_may(p, x, e, &mut m);
                    if reraises_any(hs) || m.iter().any(|k| caught(p, k, ctx)) {
                        return Err(Error::new(st.line, "an exception leaving an except / else clause of try ... finally is not supported yet"));
                    }
                }
            }
            _ => {
                for b in st.blocks() {
                    check_block(p, x, b, ctx)?;
                }
            }
        }
    }
    Ok(())
}

fn reraises_any(hs: &[Handler]) -> bool {
    hs.iter().any(|h| reraises(&h.body))
}

fn check_expr(p: &Program, x: &Exc, e: &Expr, ctx: &HashSet<String>, closure: bool) -> Res<()> {
    // the second list goes through pyrt::catching
    let (ok, bad) = op_kinds(e);
    let mut fails = ok.iter().chain(bad.iter()).any(|k| caught(p, k, ctx));
    if let ExprKind::Call(f, _) | ExprKind::Method(_, f, _) = &e.kind {
        fails |= x.fallible[*f];
    }
    if let ExprKind::New(c, _) = &e.kind {
        fails |= p.classes[*c].init.map(|i| x.fallible[i]).unwrap_or(false);
    }
    if closure && fails {
        return Err(Error::new(e.line, "an exception that a try catches cannot come from inside a comprehension, generator or lambda yet")
            .hint("write the loop out"));
    }
    let inner = closure || matches!(e.kind, ExprKind::Comp(_) | ExprKind::Lambda(..));
    let mut r = Ok(());
    e.children(&mut |c| {
        if r.is_ok() {
            r = check_expr(p, x, c, ctx, inner);
        }
    });
    r
}
