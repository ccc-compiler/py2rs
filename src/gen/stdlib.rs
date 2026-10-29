//! Standard library calls (`Builtin("module.function")` from `front::stdlib`).

use super::*;

/// `defaultdict(int)`: the factory's name, for the repr.
pub(super) fn factory(v: &Ty) -> &'static str {
    match v {
        Ty::Int => "int",
        Ty::Float => "float",
        Ty::Str => "str",
        Ty::Bool => "bool",
        Ty::List(_) => "list",
        Ty::Set(_) => "set",
        _ => "dict",
    }
}

/// A raw string literal for a pattern.
fn raw(s: &str) -> String {
    if !s.contains('"') {
        format!("r\"{}\"", s)
    } else if !s.contains("\"#") {
        format!("r#\"{}\"#", s)
    } else {
        format!("{:?}", s)
    }
}

/// The text of a literal argument.
fn lit(e: &Expr) -> &str {
    match &e.kind {
        ExprKind::Str(s) => s,
        _ => unreachable!(),
    }
}

/// `pyrt::Counter<K>` → `pyrt::Counter::<K>` (a type in expression position).
fn turbofish(t: &str) -> String {
    t.replacen('<', "::<", 1)
}

/// Builtins of the standard library that modify their first argument.
pub fn mutating(n: &str) -> bool {
    matches!(
        n,
        "heapq.heappush" | "heapq.heappop" | "heapq.heapify" | "heapq.heappushpop" | "heapq.heapreplace"
            | "bisect.insort_left" | "bisect.insort_right" | "random.shuffle" | "deque.append"
            | "deque.appendleft" | "deque.pop" | "deque.popleft" | "deque.extend" | "deque.clear"
            | "counter.update"
    ) || n.starts_with("file.")
}

/// `@functools.cache`: the function is renamed `NAME_uncached` and a wrapper
/// with a per-thread table of results takes its name.
pub(super) fn cache_wrapper(p: &Program, o: &Own, x: &Exc, fid: FuncId, code: &str) -> Res<String> {
    let f = &p.funcs[fid];
    if x.fallible[fid] {
        return err(f.line, "@cache on a function whose exceptions are caught is not supported");
    }
    let head = code.lines().find(|l| l.starts_with("fn ")).unwrap_or("");
    let name = head.trim_start_matches("fn ").split('(').next().unwrap_or("").to_string();
    let renamed = code.replacen(&format!("fn {}(", name), &format!("fn {}_uncached(", name), 1);
    let mut params = vec![];
    let mut keys = vec![];
    let mut args = vec![];
    let mut key_tys = vec![];
    for (i, q) in f.params.iter().enumerate() {
        let v = &f.vars[q.var];
        let n = ident(&v.name);
        let t = rty(&v.ty, p);
        let (pt, key) = match o.params[fid][i] {
            Mode::Copy => (t.clone(), n.clone()),
            Mode::Owned => (t.clone(), format!("{}.clone()", n)),
            Mode::Ref => match &v.ty {
                Ty::Str => ("&str".to_string(), format!("{}.to_string()", n)),
                _ => (format!("&{}", t), format!("{}.clone()", n)),
            },
            Mode::Mut => return err(f.line, "@cache on a function that modifies its arguments is not supported"),
        };
        params.push(format!("{}: {}", n, pt));
        keys.push(key);
        args.push(n);
        key_tys.push(t);
    }
    let ret = if f.ret == Ty::None { "()".to_string() } else { rty(&f.ret, p) };
    let arrow = if f.ret == Ty::None { String::new() } else { format!(" -> {}", ret) };
    let tuple = |v: &[String]| if v.len() == 1 { format!("({},)", v[0]) } else { format!("({})", v.join(", ")) };
    let (key_ty, key) = (tuple(&key_tys), tuple(&keys));
    let copy = f.ret.is_copy();
    let mut w = format!("/// `@cache`: each result is computed once\nfn {}({}){} {{\n", name, params.join(", "), arrow);
    w += &format!(
        "    thread_local! {{\n        static CACHE: std::cell::RefCell<std::collections::HashMap<{}, {}>> = Default::default();\n    }}\n",
        key_ty, ret
    );
    w += &format!("    let key = {};\n", key);
    w += &format!(
        "    if let Some(v) = CACHE.with(|t| t.borrow().get(&key){}) {{\n        return v;\n    }}\n",
        if copy { ".copied()" } else { ".cloned()" }
    );
    w += &format!("    let v = {}_uncached({});\n", name, args.join(", "));
    w += &format!("    CACHE.with(|t| t.borrow_mut().insert(key, {}));\n    v\n}}\n\n", if copy { "v" } else { "v.clone()" });
    Ok(w + &renamed)
}

impl FGen<'_> {
    /// `&[T]` of an iterable argument: the list itself, else its items collected.
    fn slice_arg(&mut self, e: &Expr) -> Res<String> {
        if let ExprKind::Iter(it) = &e.kind {
            if let Iter::Over(x) = &**it {
                if matches!(x.ty, Ty::List(_)) {
                    return self.borrow(x);
                }
            }
        }
        let code = self.iter_values(e)?;
        Ok(format!("&{}.collect::<Vec<_>>()", code))
    }

    /// `pyrt::tupN` for a list of tuples of length n.
    fn tuples(code: String, n: &Expr) -> String {
        let ExprKind::Int(n) = n.kind else { unreachable!() };
        format!("{}.into_iter().map(pyrt::tup{}).collect::<Vec<_>>()", code, n)
    }

    pub(super) fn std_builtin(&mut self, name: &str, a: &[Expr], e: &Expr) -> Res<R> {
        let v = |c: String| r(c, Kind::Val, ATOM);
        if name == "open" {
            let p = self.borrow_str(&a[0])?;
            return Ok(v(format!("pyrt::files::open({}, {:?})", p, lit(&a[1]))));
        }
        let (module, f) = name.split_once('.').unwrap();
        Ok(match (module, f) {
            ("collections", "defaultdict") => {
                let Ty::Dict(_, vt, _) = &e.ty else { unreachable!() };
                v(format!("{}::new(\"{}\")", turbofish(&self.ty(&e.ty)), factory(vt)))
            }
            ("collections", "Counter") if a.is_empty() => v(format!("{}::new()", turbofish(&self.ty(&e.ty)))),
            ("collections", "Counter") => v(format!("pyrt::Counter::of({})", self.iter_values(&a[0])?)),
            ("collections", "deque") if a.is_empty() => v(format!("{}::new()", turbofish(&self.ty(&e.ty)))),
            ("collections", "deque") => v(format!("{}.collect::<VecDeque<_>>()", self.iter_values(&a[0])?)),
            ("deque" | "counter", _) => {
                let recv = if mutating(name) { self.ex_mut(&a[0])? } else { self.ex(&a[0])? };
                let rc = par(&recv, ATOM);
                match f {
                    "append" => v(format!("{}.push_back({})", rc, self.owned(&a[1])?)),
                    "appendleft" => v(format!("{}.push_front({})", rc, self.owned(&a[1])?)),
                    "pop" => v(format!("pyrt::stdlib::pop_deque({})", self.mut_ref_of(&recv))),
                    "popleft" => v(format!("pyrt::stdlib::popleft({})", self.mut_ref_of(&recv))),
                    "extend" | "update" => v(format!("{}.{}({})", rc, f, self.iter_values(&a[1])?)),
                    "clear" => v(format!("{}.clear()", rc)),
                    "most_common" => {
                        let n = match a[1].kind {
                            ExprKind::Int(-1) => "None".to_string(),
                            _ => format!("Some({})", self.val(&a[1])?.code),
                        };
                        v(format!("{}.most_common({})", rc, n))
                    }
                    "total" | "elements" => v(format!("{}.{}()", rc, f)),
                    _ => return err(e.line, format!("internal: {}", name)),
                }
            }
            ("heapq", "heappush" | "heappop" | "heapify" | "heappushpop" | "heapreplace") => {
                let h = self.ex_mut(&a[0])?;
                let hr = self.mut_ref_of(&h);
                match a.get(1) {
                    Some(x) => {
                        let x = self.owned(x)?;
                        v(format!("pyrt::stdlib::{}({}, {})", f, hr, x))
                    }
                    None => v(format!("pyrt::stdlib::{}({})", f, hr)),
                }
            }
            ("heapq", "nlargest" | "nsmallest") => {
                let n = self.val(&a[0])?;
                v(format!("pyrt::stdlib::{}({}, {})", f, n.code, self.iter_values(&a[1])?))
            }
            ("bisect", "bisect_left" | "bisect_right") => {
                let xs = self.borrow(&a[0])?;
                let x = self.borrow(&a[1])?;
                v(format!("pyrt::stdlib::{}({}, {})", f, xs, x))
            }
            ("bisect", "insort_left" | "insort_right") => {
                let xs = self.ex_mut(&a[0])?;
                let xr = self.mut_ref_of(&xs);
                let x = self.owned(&a[1])?;
                v(format!("pyrt::stdlib::{}({}, {})", f, xr, x))
            }
            ("itertools", "permutations" | "combinations" | "product_repeat") => {
                let xs = self.slice_arg(&a[0])?;
                let n = self.val(&a[1])?;
                v(Self::tuples(format!("pyrt::stdlib::{}({}, {})", f, xs, n.code), &a[1]))
            }
            ("itertools", "product") => {
                let mut xs = vec![];
                for x in a {
                    xs.push(self.slice_arg(x)?);
                }
                v(format!("pyrt::stdlib::product{}({})", a.len(), xs.join(", ")))
            }
            ("itertools", "chain") => {
                let mut code = self.iter_values(&a[0])?;
                for x in &a[1..] {
                    code = format!("{}.chain({})", code, bare(self.iter_values(x)?));
                }
                v(format!("{}.collect::<Vec<_>>()", code))
            }
            ("itertools", "accumulate") => v(format!("pyrt::stdlib::accumulate({})", self.iter_values(&a[0])?)),
            ("itertools", "pairwise") => v(format!("pyrt::stdlib::pairwise({})", self.slice_arg(&a[0])?)),
            ("functools", "reduce") => {
                let ExprKind::Lambda(ps, body) = &a[0].kind else { unreachable!() };
                for p in ps {
                    self.kinds.insert(*p, Kind::Place);
                }
                let pat = format!("|{}, {}|", self.name(ps[0]), self.name(ps[1]));
                let it = self.iter_values(&a[1])?;
                let b = self.owned(body)?;
                match a.get(2) {
                    Some(init) => {
                        let i = self.owned(init)?;
                        v(format!("{}.fold({}, {} {})", it, i, pat, b))
                    }
                    None => v(format!(
                        "{}.reduce({} {}).unwrap_or_else(|| pyrt::raise(\"TypeError\", \"reduce() of empty iterable with no initial value\"))",
                        it, pat, b
                    )),
                }
            }
            ("random", "seed") => v(format!("pyrt::stdlib::seed({})", self.val(&a[0])?.code)),
            ("random", "random") => v("pyrt::stdlib::random()".into()),
            ("random", "randint" | "uniform" | "randrange") => {
                let mut xs = vec![];
                for x in a {
                    xs.push(self.val(x)?.code);
                }
                v(format!("pyrt::stdlib::{}({})", f, xs.join(", ")))
            }
            ("random", "choice") => v(format!("pyrt::stdlib::choice({})", self.borrow(&a[0])?)),
            ("random", "shuffle") => {
                let xs = self.ex_mut(&a[0])?;
                v(format!("pyrt::stdlib::shuffle({})", self.mut_ref_of(&xs)))
            }
            ("random", "sample") => {
                let xs = self.slice_arg(&a[0])?;
                v(format!("pyrt::stdlib::sample({}, {})", xs, self.val(&a[1])?.code))
            }
            ("re", _) => self.re_builtin(f, a, e)?,
            ("file", _) => {
                let fv = self.ex_mut(&a[0])?;
                let fc = par(&fv, ATOM);
                match f {
                    "write" => v(format!("{}.write({})", fc, self.borrow_str(&a[1])?)),
                    "writelines" => v(format!("{}.writelines({})", fc, self.borrow(&a[1])?)),
                    _ => v(format!("{}.{}()", fc, f)),
                }
            }
            ("os", "path.exists") => v(format!("pyrt::files::exists({})", self.borrow_str(&a[0])?)),
            ("os", "remove") => v(format!("pyrt::files::remove({})", self.borrow_str(&a[0])?)),
            ("os", "path.join") => {
                let mut xs = vec![];
                for x in a {
                    xs.push(self.borrow_str(x)?);
                }
                v(format!("pyrt::files::join(&[{}])", xs.join(", ")))
            }
            ("sys", "argv") => v("pyrt::stdlib::argv()".into()),
            ("sys", "exit") => v(format!("pyrt::stdlib::exit({})", self.val(&a[0])?.code)),
            ("sys", "stdin.readline") => v("pyrt::stdlib::readline()".into()),
            ("sys", "stdin.read") => v("pyrt::stdlib::read_all()".into()),
            ("sys", "stdin.readlines") => v("pyrt::stdlib::lines()".into()),
            ("sys", "stdout.write") => v(format!("print!(\"{{}}\", {})", self.borrow_str(&a[0])?)),
            ("sys", "setrecursionlimit") => {
                // the stack is set when main starts (`pyrt::init`)
                let _ = self.val(&a[0])?;
                v("()".into())
            }
            ("time", "time") => v("pyrt::stdlib::time()".into()),
            ("time", "perf_counter" | "monotonic") => v("pyrt::stdlib::perf_counter()".into()),
            ("time", "sleep") => v(format!("pyrt::stdlib::sleep({})", self.val(&a[0])?.code)),
            ("json", "dumps") => v(format!("pyrt::stdlib::dumps({})", self.borrow(&a[0])?)),
            _ => return err(e.line, format!("internal: unknown builtin {}", name)),
        })
    }

    fn re_builtin(&mut self, f: &str, a: &[Expr], e: &Expr) -> Res<R> {
        let v = |c: String| r(c, Kind::Val, ATOM);
        let n_of = |t: &Ty| match t {
            Ty::List(x) => match &**x {
                Ty::Tuple(ts) => ts.len(),
                _ => 0,
            },
            Ty::Tuple(ts) => ts.len(),
            _ => 0,
        };
        Ok(match f {
            "compile" => v(format!("pyrt::re::compile({}, {})", raw(lit(&a[0])), raw(lit(&a[1])))),
            "search" | "match" | "fullmatch" | "findall" | "finditer" | "split" | "findall_groups" => {
                let s = self.borrow_str(&a[1])?;
                let name = if f == "match" { "match_" } else { f };
                let code = format!("pyrt::re::{}({}, {})", name, raw(lit(&a[0])), s);
                if f == "findall_groups" {
                    v(format!("{}.into_iter().map(pyrt::tup{}).collect::<Vec<_>>()", code, n_of(&e.ty)))
                } else {
                    v(code)
                }
            }
            "sub" => {
                let s = self.borrow_str(&a[2])?;
                let c = self.val(&a[3])?;
                v(format!("pyrt::re::sub({}, {:?}, {}, {})", raw(lit(&a[0])), lit(&a[1]), s, c.code))
            }
            "escape" => v(format!("pyrt::re::escape({})", self.borrow_str(&a[0])?)),
            _ => {
                let recv = self.ex(&a[0])?;
                let rc = par(&recv, ATOM);
                match f {
                    "p_search" | "p_match" | "p_fullmatch" | "p_findall" | "p_finditer" | "p_split" | "p_findall_groups" => {
                        let s = self.borrow_str(&a[1])?;
                        let m = match &f[2..] {
                            "match" => "match_",
                            m => m,
                        };
                        let code = format!("{}.{}({})", rc, m, s);
                        if f == "p_findall_groups" {
                            v(format!("{}.into_iter().map(pyrt::tup{}).collect::<Vec<_>>()", code, n_of(&e.ty)))
                        } else {
                            v(code)
                        }
                    }
                    "p_sub" => {
                        let s = self.borrow_str(&a[2])?;
                        let c = self.val(&a[3])?;
                        v(format!("{}.sub({:?}, {}, {})", rc, lit(&a[1]), s, c.code))
                    }
                    "m_group" | "m_start" | "m_end" | "m_span" => {
                        let k = self.val(&a[1])?;
                        v(format!("{}.{}({})", rc, &f[2..], k.code))
                    }
                    "m_group_named" => v(format!("{}.group_named({:?})", rc, lit(&a[1]))),
                    "m_groups" => v(format!("pyrt::tup{}({}.groups())", n_of(&e.ty), rc)),
                    _ => return err(e.line, format!("internal: re.{}", f)),
                }
            }
        })
    }
}
