//! Standard library modules: collections, heapq, bisect, itertools, functools,
//! random, sys, time, json (and `from math import ...`). Every function becomes
//! a `Builtin` named `module.function`, printed by `gen::stdlib`.

use super::re::re_flag;
use super::*;

/// Modules that `import M` / `from M import x` accept.
pub const MODULES: &[&str] =
    &["math", "sys", "collections", "heapq", "bisect", "itertools", "functools", "random", "time", "json", "re", "os"];

/// The value type a `defaultdict` factory produces.
fn factory_ty(name: &str) -> Option<Ty> {
    Some(match name {
        "int" => Ty::Int,
        "float" => Ty::Float,
        "str" => Ty::Str,
        "bool" => Ty::Bool,
        _ => return None,
    })
}

impl Front {
    /// `x` imported by `from M import x`, or `M.x` after `import M`: the
    /// qualified name `M.x` (`sys.stdin.readline` for nested attributes).
    pub(super) fn std_name(&self, e: &AExpr) -> Option<String> {
        match e {
            AExpr::Name(n) => {
                let n = n.id.as_str();
                if self.lookup(n).is_some() || self.funcs.contains_key(n) || self.classes.contains_key(n) {
                    return None;
                }
                self.from_names.get(n).cloned()
            }
            AExpr::Attribute(a) => match &*a.value {
                AExpr::Name(m)
                    if self.lookup(m.id.as_str()).is_none()
                        && MODULES.contains(&m.id.as_str())
                        && self.imports.contains(m.id.as_str()) =>
                {
                    Some(format!("{}.{}", m.id, a.attr.id))
                }
                AExpr::Attribute(_) => self.std_name(&a.value).map(|q| format!("{}.{}", q, a.attr.id)),
                _ => None,
            },
            _ => None,
        }
    }

    /// A container type of `collections`: `defaultdict[K, V]`, `Counter[K]`, `deque[T]`.
    pub(super) fn std_ann(&mut self, q: &str, args: &[&AExpr], line: u32) -> Res<Ty> {
        match (q, args.len()) {
            ("collections.defaultdict", 2) => {
                let k = self.ann(args[0])?;
                let v = self.ann(args[1])?;
                if !default_ok(&v) {
                    return err(line, "defaultdict values must be int, float, str, bool, list, set or dict");
                }
                Ok(Ty::Dict(Box::new(k), Box::new(v), DK::Default))
            }
            ("collections.Counter", 1) => Ok(Ty::Dict(Box::new(self.ann(args[0])?), Box::new(Ty::Int), DK::Counter)),
            ("collections.deque", 1) => Ok(Ty::Deque(Box::new(self.ann(args[0])?))),
            _ => err(line, format!("unsupported type annotation `{}`", q)),
        }
    }

    /// `sys.argv`, `math.pi`, `sys.maxsize`, ...
    pub(super) fn std_value(&mut self, q: &str, line: u32) -> Res<Expr> {
        if let Some(c) = q.strip_prefix("math.") {
            return self.math_const(c, line);
        }
        match q {
            "sys.argv" => Ok(self.mk(line, Ty::list(Ty::Str), ExprKind::Builtin("sys.argv", vec![]))),
            q if q.starts_with("re.") => match re_flag(&q[3..]) {
                Some(v) => Ok(self.mk(line, Ty::Int, ExprKind::Int(v as i64))),
                None => err(line, format!("`{}` is not supported", q)),
            },
            "sys.maxsize" => Ok(self.mk(line, Ty::Int, ExprKind::Int(i64::MAX))),
            "sys.stdin" => err(line, "sys.stdin can be iterated, or read with readline() / read() / readlines()"),
            _ => err(line, format!("`{}` is not supported", q)),
        }
    }

    /// A literal `r` of permutations / combinations / product(repeat=).
    fn tuple_len(&mut self, e: &AExpr, line: u32) -> Res<usize> {
        let x = self.expr(e, Some(&Ty::Int))?;
        match x.kind {
            ExprKind::Int(r) if (1..=6).contains(&r) => Ok(r as usize),
            ExprKind::Int(_) => err(line, "the tuple length must be 1 to 6"),
            _ => err(line, "the tuple length must be a literal (each tuple gets a static type)"),
        }
    }

    pub(super) fn std_call(&mut self, q: &str, args: &ast::Arguments, want: Option<&Ty>, line: u32) -> Res<Expr> {
        use ExprKind as K;
        if let Some(m) = q.strip_prefix("math.") {
            return self.math(m, args, line);
        }
        if let Some(f) = q.strip_prefix("re.") {
            return self.re_call(f, args, line);
        }
        let short = q.rsplit('.').next().unwrap_or(q).to_string();
        let name: &'static str = leak(q.to_string());
        let n_kw = |s: &Self, k: &[&str]| s.kw(args, k, line);
        let (t, n, v): (Ty, &'static str, Vec<Expr>) = match q {
            // ------------------------------------------------ collections
            "collections.defaultdict" => {
                n_kw(self, &[])?;
                let [AExpr::Name(f)] = &args.args[..] else {
                    return err(line, "defaultdict() needs a factory: int, float, str, bool, list, set or dict");
                };
                let v = match f.id.as_str() {
                    "list" => Ty::list(self.fresh()),
                    "set" => Ty::set(self.fresh()),
                    "dict" => {
                        let (a, b) = (self.fresh(), self.fresh());
                        Ty::dict(a, b)
                    }
                    x => match factory_ty(x) {
                        Some(t) => t,
                        None => return err(line, "defaultdict() needs a factory: int, float, str, bool, list, set or dict")
                            .map_err(|e| e.hint("lambdas and classes as factories are not supported")),
                    },
                };
                let k = match want.map(|w| self.norm(w)) {
                    Some(Ty::Dict(k, wv, DK::Default)) => {
                        if !self.unify(&wv, &v) {
                            return err(line, "the factory does not match the annotated value type");
                        }
                        *k
                    }
                    _ => self.fresh(),
                };
                (Ty::Dict(Box::new(k), Box::new(v), DK::Default), name, vec![])
            }
            "collections.Counter" => {
                n_kw(self, &[])?;
                self.nargs(args, 0, 1, "Counter", line)?;
                if args.args.is_empty() {
                    let k = match want.map(|w| self.norm(w)) {
                        Some(Ty::Dict(k, _, DK::Counter)) => *k,
                        _ => self.fresh(),
                    };
                    (Ty::Dict(Box::new(k), Box::new(Ty::Int), DK::Counter), name, vec![])
                } else {
                    let (it, t) = self.iter_arg(&args.args[0])?;
                    if let ExprKind::Iter(i) = &it.kind {
                        if let Iter::Over(x) = &**i {
                            if matches!(self.norm(&x.ty), Ty::Dict(..)) {
                                return err(line, "Counter(dict) is not supported; count an iterable");
                            }
                        }
                    }
                    self.hashable(&t, line)?;
                    (Ty::Dict(Box::new(t), Box::new(Ty::Int), DK::Counter), name, vec![it])
                }
            }
            "collections.deque" => {
                if !args.keywords.is_empty() {
                    return err(line, "deque(maxlen=) is not supported");
                }
                self.nargs(args, 0, 1, "deque", line)?;
                if args.args.is_empty() {
                    let t = match want.map(|w| self.norm(w)) {
                        Some(Ty::Deque(t)) => *t,
                        _ => self.fresh(),
                    };
                    (Ty::Deque(Box::new(t)), name, vec![])
                } else {
                    let (it, t) = self.iter_arg(&args.args[0])?;
                    (Ty::Deque(Box::new(t)), name, vec![it])
                }
            }
            // ----------------------------------------------------- heapq
            "heapq.heappush" | "heapq.heappushpop" | "heapq.heapreplace" | "heapq.heappop" | "heapq.heapify" => {
                n_kw(self, &[])?;
                let two = !matches!(q, "heapq.heappop" | "heapq.heapify");
                self.nargs(args, 1 + two as usize, 1 + two as usize, q, line)?;
                let h = self.expr(&args.args[0], None)?;
                let Ty::List(et) = self.norm(&h.ty) else {
                    return err(line, format!("{}() needs a list", q));
                };
                let mut v = vec![h];
                if two {
                    v.push(self.arg(args, 1, &et, "heap item")?);
                }
                self.orderable(&et, line)?;
                let t = match q {
                    "heapq.heappush" | "heapq.heapify" => Ty::None,
                    _ => self.norm(&et),
                };
                (t, name, v)
            }
            "heapq.nlargest" | "heapq.nsmallest" => {
                n_kw(self, &[])?;
                self.nargs(args, 2, 2, q, line)?;
                let k = self.arg(args, 0, &Ty::Int, "count")?;
                let (it, t) = self.iter_arg(&args.args[1])?;
                self.orderable(&t, line)?;
                (Ty::list(t), name, vec![k, it])
            }
            // ---------------------------------------------------- bisect
            "bisect.bisect_left" | "bisect.bisect_right" | "bisect.bisect" | "bisect.insort_left" | "bisect.insort_right"
            | "bisect.insort" => {
                n_kw(self, &[])?;
                self.nargs(args, 2, 2, q, line)?;
                let a = self.expr(&args.args[0], None)?;
                let Ty::List(et) = self.norm(&a.ty) else {
                    return err(line, format!("{}() needs a list", q));
                };
                let x = self.arg(args, 1, &et, "item")?;
                self.orderable(&et, line)?;
                let (t, n) = match q {
                    "bisect.bisect_left" => (Ty::Int, "bisect.bisect_left"),
                    "bisect.bisect_right" | "bisect.bisect" => (Ty::Int, "bisect.bisect_right"),
                    "bisect.insort_left" => (Ty::None, "bisect.insort_left"),
                    _ => (Ty::None, "bisect.insort_right"),
                };
                (t, n, vec![a, x])
            }
            // ------------------------------------------------- itertools
            "itertools.permutations" | "itertools.combinations" => {
                n_kw(self, &[])?;
                self.nargs(args, 1, 2, q, line)?;
                let r = match (args.args.get(1), &args.args[0]) {
                    (Some(r), _) => self.tuple_len(r, line)?,
                    (None, AExpr::List(l)) if q == "itertools.permutations" && (1..=6).contains(&l.elts.len()) => l.elts.len(),
                    (None, AExpr::StringLiteral(s)) if q == "itertools.permutations" && (1..=6).contains(&s.value.to_str().chars().count()) => {
                        s.value.to_str().chars().count()
                    }
                    _ => return err(line, format!("{}() needs the tuple length r as a literal", short)),
                };
                let (it, t) = self.iter_arg(&args.args[0])?;
                let rr = self.mk(line, Ty::Int, K::Int(r as i64));
                (Ty::list(Ty::Tuple(vec![t; r])), name, vec![it, rr])
            }
            "itertools.product" => {
                let kws = n_kw(self, &["repeat"])?;
                if let Some(rep) = kws.get("repeat") {
                    self.nargs(args, 1, 1, "product", line)?;
                    let r = self.tuple_len(rep, line)?;
                    let (it, t) = self.iter_arg(&args.args[0])?;
                    let rr = self.mk(line, Ty::Int, K::Int(r as i64));
                    (Ty::list(Ty::Tuple(vec![t; r])), "itertools.product_repeat", vec![it, rr])
                } else {
                    self.nargs(args, 2, 3, "product", line)?;
                    let mut v = vec![];
                    let mut ts = vec![];
                    for a in args.args.iter() {
                        let (it, t) = self.iter_arg(a)?;
                        v.push(it);
                        ts.push(t);
                    }
                    (Ty::list(Ty::Tuple(ts)), name, v)
                }
            }
            "itertools.chain" => {
                n_kw(self, &[])?;
                if args.args.is_empty() {
                    return err(line, "chain() needs at least one iterable");
                }
                let mut v = vec![];
                let t = self.fresh();
                for a in args.args.iter() {
                    let (it, et) = self.iter_arg(a)?;
                    if !self.unify(&t, &et) {
                        return err(line, "chain() arguments must have the same element type");
                    }
                    v.push(it);
                }
                (Ty::list(self.norm(&t)), name, v)
            }
            "itertools.accumulate" => {
                n_kw(self, &[])?;
                self.nargs(args, 1, 1, "accumulate", line)?;
                let (it, t) = self.iter_arg(&args.args[0])?;
                if !self.norm(&t).is_num() {
                    return err(line, "accumulate() needs numbers");
                }
                (Ty::list(t), name, vec![it])
            }
            "itertools.pairwise" => {
                n_kw(self, &[])?;
                self.nargs(args, 1, 1, "pairwise", line)?;
                let (it, t) = self.iter_arg(&args.args[0])?;
                (Ty::list(Ty::Tuple(vec![t.clone(), t])), name, vec![it])
            }
            // ------------------------------------------------- functools
            "functools.reduce" => {
                n_kw(self, &[])?;
                self.nargs(args, 2, 3, "reduce", line)?;
                let (it, t) = self.iter_arg(&args.args[1])?;
                let init = match args.args.get(2) {
                    Some(a) => Some(self.expr(a, None)?),
                    None => None,
                };
                let acc = init.as_ref().map(|i| i.ty.clone()).unwrap_or_else(|| t.clone());
                let f = self.lambda(&args.args[0], &[acc.clone(), t])?;
                if !self.unify(&f.ty, &acc) {
                    return err(line, "the reduce() function must return the accumulator's type");
                }
                let mut v = vec![f, it];
                v.extend(init);
                (self.norm(&acc), name, v)
            }
            "functools.cache" | "functools.lru_cache" => return err(line, format!("{} is only supported as a decorator", short)),
            // ---------------------------------------------------- random
            "random.seed" => {
                n_kw(self, &[])?;
                self.nargs(args, 1, 1, "seed", line)?;
                let x = self.arg(args, 0, &Ty::Int, "seed")?;
                (Ty::None, name, vec![x])
            }
            "random.random" => {
                self.nargs(args, 0, 0, "random", line)?;
                (Ty::Float, name, vec![])
            }
            "random.randint" | "random.uniform" => {
                n_kw(self, &[])?;
                self.nargs(args, 2, 2, q, line)?;
                let t = if q == "random.randint" { Ty::Int } else { Ty::Float };
                let a = self.arg(args, 0, &t, "bound")?;
                let b = self.arg(args, 1, &t, "bound")?;
                (t, name, vec![a, b])
            }
            "random.randrange" => {
                n_kw(self, &[])?;
                self.nargs(args, 1, 3, "randrange", line)?;
                let mut v = vec![];
                for i in 0..args.args.len() {
                    v.push(self.arg(args, i, &Ty::Int, "randrange() argument")?);
                }
                if v.len() == 1 {
                    v.insert(0, self.mk(line, Ty::Int, K::Int(0)));
                }
                if v.len() == 2 {
                    v.push(self.mk(line, Ty::Int, K::Int(1)));
                }
                (Ty::Int, name, v)
            }
            "random.choice" | "random.shuffle" => {
                n_kw(self, &[])?;
                self.nargs(args, 1, 1, q, line)?;
                let xs = self.expr(&args.args[0], None)?;
                let Ty::List(et) = self.norm(&xs.ty) else {
                    return err(line, format!("{}() needs a list", short));
                };
                let t = if q == "random.choice" { *et } else { Ty::None };
                (t, name, vec![xs])
            }
            "random.sample" => {
                n_kw(self, &[])?;
                self.nargs(args, 2, 2, "sample", line)?;
                let (it, t) = self.iter_arg(&args.args[0])?;
                let k = self.arg(args, 1, &Ty::Int, "sample size")?;
                (Ty::list(t), name, vec![it, k])
            }
            // ------------------------------------------------------- sys
            "sys.exit" => {
                n_kw(self, &[])?;
                self.nargs(args, 0, 1, "exit", line)?;
                let x = match args.args.first() {
                    Some(_) => self.arg(args, 0, &Ty::Int, "exit code")?,
                    None => self.mk(line, Ty::Int, K::Int(0)),
                };
                (Ty::None, name, vec![x])
            }
            "sys.stdin.readline" | "sys.stdin.read" | "sys.stdin.readlines" => {
                self.nargs(args, 0, 0, q, line)?;
                let t = if q == "sys.stdin.readlines" { Ty::list(Ty::Str) } else { Ty::Str };
                (t, name, vec![])
            }
            "sys.stdout.write" => {
                self.nargs(args, 1, 1, "write", line)?;
                let s = self.arg(args, 0, &Ty::Str, "write() argument")?;
                (Ty::None, name, vec![s])
            }
            "sys.setrecursionlimit" => {
                self.nargs(args, 1, 1, "setrecursionlimit", line)?;
                let x = self.arg(args, 0, &Ty::Int, "limit")?;
                (Ty::None, name, vec![x])
            }
            // ------------------------------------------------------ time
            "time.time" | "time.perf_counter" | "time.monotonic" => {
                self.nargs(args, 0, 0, q, line)?;
                (Ty::Float, name, vec![])
            }
            "time.sleep" => {
                self.nargs(args, 1, 1, "sleep", line)?;
                let x = self.arg(args, 0, &Ty::Float, "seconds")?;
                (Ty::None, name, vec![x])
            }
            // ------------------------------------------------------ json
            "json.dumps" => {
                n_kw(self, &[])?;
                self.nargs(args, 1, 1, "dumps", line)?;
                let x = self.expr(&args.args[0], None)?;
                self.settle(&x.ty);
                self.jsonable(&x.ty, line)?;
                (Ty::Str, name, vec![x])
            }
            "os.path.exists" | "os.remove" => {
                n_kw(self, &[])?;
                self.nargs(args, 1, 1, q, line)?;
                let p = self.arg(args, 0, &Ty::Str, "path")?;
                (if q == "os.remove" { Ty::None } else { Ty::Bool }, name, vec![p])
            }
            "os.path.join" => {
                n_kw(self, &[])?;
                if args.args.is_empty() {
                    return err(line, "os.path.join() needs a path");
                }
                let mut v = vec![];
                for i in 0..args.args.len() {
                    v.push(self.arg(args, i, &Ty::Str, "path")?);
                }
                (Ty::Str, name, v)
            }
            "json.loads" => return err(line, "json.loads() is outside the typed subset (its result has no static type)"),
            _ => return err(line, format!("`{}` is not supported", q)),
        };
        Ok(self.mk(line, t, K::Builtin(n, v)))
    }

    /// `@cache`, `@lru_cache(maxsize=None)`: Some(true); not a cache decorator: None.
    pub(super) fn cache_decorator(&self, e: &AExpr, method: bool, line: u32) -> Res<Option<bool>> {
        let (q, args) = match e {
            AExpr::Call(c) => (self.std_name(&c.func), Some(&c.arguments)),
            _ => (self.std_name(e), None),
        };
        match q.as_deref() {
            Some("functools.cache" | "functools.lru_cache") => {}
            _ => return Ok(None),
        }
        if let Some(a) = args {
            let none = |x: &AExpr| matches!(x, AExpr::NoneLiteral(_));
            let ok = match (&a.args[..], &a.keywords[..]) {
                ([], []) => false,
                ([x], []) => none(x),
                ([], [k]) => k.arg.as_ref().map(|n| n.id.as_str()) == Some("maxsize") && none(&k.value),
                _ => false,
            };
            if !ok {
                return err(line, "only an unbounded cache is supported: @cache or @lru_cache(maxsize=None)")
                    .map_err(|e| e.hint("a bounded cache would run evicted calls (and their side effects) again"));
            }
        } else if q.as_deref() == Some("functools.lru_cache") {
            return err(line, "write @lru_cache(maxsize=None) or @cache");
        }
        if method {
            return err(line, "@cache on methods is not supported");
        }
        Ok(Some(true))
    }

    /// `open(path, mode="r", encoding="utf-8")`
    pub(super) fn open_call(&mut self, args: &ast::Arguments, line: u32) -> Res<Expr> {
        let kws = self.kw(args, &["mode", "encoding"], line)?;
        self.nargs(args, 1, 2, "open", line)?;
        let p = self.arg(args, 0, &Ty::Str, "path")?;
        let mode = match args.args.get(1).or_else(|| kws.get("mode").copied()) {
            Some(AExpr::StringLiteral(s)) => s.value.to_str().to_string(),
            Some(_) => return err(line, "the mode must be a string literal"),
            None => "r".into(),
        };
        if !["r", "w", "a", "rt", "wt", "at"].contains(&mode.as_str()) {
            return err(line, format!("mode `{}` is not supported (text files: r, w, a)", mode));
        }
        match kws.get("encoding") {
            Some(AExpr::StringLiteral(s)) if ["utf-8", "utf8", "UTF-8"].contains(&s.value.to_str()) => {}
            Some(_) => return err(line, "only encoding=\"utf-8\" is supported"),
            None => {}
        }
        let m = self.mk(line, Ty::Str, ExprKind::Str(mode));
        Ok(self.mk(line, Ty::File, ExprKind::Builtin("open", vec![p, m])))
    }

    fn jsonable(&self, t: &Ty, line: u32) -> Res<()> {
        match self.norm(t) {
            Ty::Int | Ty::Float | Ty::Bool | Ty::Str => Ok(()),
            Ty::Optional(t) | Ty::List(t) => self.jsonable(&t, line),
            Ty::Tuple(ts) => ts.iter().try_for_each(|t| self.jsonable(t, line)),
            Ty::Dict(k, v, DK::Plain) => {
                if !matches!(*k, Ty::Str | Ty::Int | Ty::Bool) {
                    return err(line, "json.dumps() needs str, int or bool dict keys");
                }
                self.jsonable(&v, line)
            }
            t => err(line, format!("json.dumps() of {} is not supported", self.tn(&t))),
        }
    }

    /// Methods of `deque`, `Counter`, `re.Pattern` and `re.Match` (other Counter /
    /// defaultdict methods are the dict's).
    /// `Err(recv)` hands the receiver back when `m` is not one of them.
    pub(super) fn std_method(&mut self, recv: Expr, m: &str, args: &ast::Arguments, line: u32) -> Res<Result<Expr, Expr>> {
        use ExprKind as K;
        let rt = self.norm(&recv.ty);
        let b = |s: &mut Self, n: &'static str, t: Ty, mut v: Vec<Expr>, recv: Expr| {
            v.insert(0, recv);
            Ok(Ok(s.mk(line, t, K::Builtin(n, v))))
        };
        match &rt {
            Ty::Pattern(_) | Ty::Match(_) => self.re_method(recv, m, args, line).map(Ok),
            Ty::File => {
                self.kw(args, &[], line)?;
                let (n, t, v): (&'static str, Ty, Vec<Expr>) = match m {
                    "read" | "readline" | "readlines" | "close" => {
                        self.nargs(args, 0, 0, m, line)?;
                        match m {
                            "read" => ("file.read", Ty::Str, vec![]),
                            "readline" => ("file.readline", Ty::Str, vec![]),
                            "readlines" => ("file.readlines", Ty::list(Ty::Str), vec![]),
                            _ => ("file.close", Ty::None, vec![]),
                        }
                    }
                    "write" => {
                        self.nargs(args, 1, 1, m, line)?;
                        let s = self.arg(args, 0, &Ty::Str, "write() argument")?;
                        ("file.write", Ty::Int, vec![s])
                    }
                    "writelines" => {
                        self.nargs(args, 1, 1, m, line)?;
                        let s = self.arg(args, 0, &Ty::list(Ty::Str), "writelines() argument")?;
                        ("file.writelines", Ty::None, vec![s])
                    }
                    _ => return err(line, format!("files have no method `{}` in the subset", m)),
                };
                b(self, n, t, v, recv)
            }
            Ty::Deque(et) => {
                let et = (**et).clone();
                self.kw(args, &[], line)?;
                match m {
                    "append" | "appendleft" => {
                        self.nargs(args, 1, 1, m, line)?;
                        let x = self.arg(args, 0, &et, "deque item")?;
                        b(self, if m == "append" { "deque.append" } else { "deque.appendleft" }, Ty::None, vec![x], recv)
                    }
                    "pop" | "popleft" => {
                        self.nargs(args, 0, 0, m, line)?;
                        b(self, if m == "pop" { "deque.pop" } else { "deque.popleft" }, et, vec![], recv)
                    }
                    "extend" => {
                        self.nargs(args, 1, 1, m, line)?;
                        let (it, t) = self.iter_arg(&args.args[0])?;
                        if !self.unify(&t, &et) {
                            return err(line, "extend() items have the wrong type");
                        }
                        b(self, "deque.extend", Ty::None, vec![it], recv)
                    }
                    "clear" => {
                        self.nargs(args, 0, 0, m, line)?;
                        b(self, "deque.clear", Ty::None, vec![], recv)
                    }
                    _ => err(line, format!("deque has no method `{}` in the subset", m)),
                }
            }
            Ty::Dict(k, _, DK::Counter) => {
                let k = (**k).clone();
                match m {
                    "most_common" => {
                        self.kw(args, &[], line)?;
                        self.nargs(args, 0, 1, m, line)?;
                        let n = match args.args.first() {
                            Some(_) => self.arg(args, 0, &Ty::Int, "count")?,
                            None => self.mk(line, Ty::Int, K::Int(-1)),
                        };
                        b(self, "counter.most_common", Ty::list(Ty::Tuple(vec![k, Ty::Int])), vec![n], recv)
                    }
                    "total" => {
                        self.nargs(args, 0, 0, m, line)?;
                        b(self, "counter.total", Ty::Int, vec![], recv)
                    }
                    "elements" => {
                        self.nargs(args, 0, 0, m, line)?;
                        b(self, "counter.elements", Ty::list(k), vec![], recv)
                    }
                    "update" => {
                        self.kw(args, &[], line)?;
                        self.nargs(args, 1, 1, m, line)?;
                        let (it, t) = self.iter_arg(&args.args[0])?;
                        if !self.unify(&t, &k) {
                            return err(line, "update() items have the wrong type");
                        }
                        b(self, "counter.update", Ty::None, vec![it], recv)
                    }
                    "subtract" => err(line, "Counter.subtract() is not supported"),
                    _ => Ok(Err(recv)),
                }
            }
            _ => Ok(Err(recv)),
        }
    }
}

/// Value types that a `defaultdict` can create.
fn default_ok(v: &Ty) -> bool {
    matches!(v, Ty::Int | Ty::Float | Ty::Str | Ty::Bool | Ty::List(_) | Ty::Set(_) | Ty::Dict(_, _, DK::Plain))
}
