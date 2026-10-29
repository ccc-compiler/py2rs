//! Nested functions: lifted to module level. The enclosing variables a nested
//! function uses become extra parameters, passed at every call (Python reads them
//! when the function runs, so passing the current values is the same); `nonlocal`
//! ones are `&mut`.

use ruff_python_ast::visitor::{walk_expr, Visitor};

use super::*;

/// Names read or written anywhere in `body` (nested functions included).
fn names_in(body: &[AStmt]) -> HashSet<String> {
    struct V(HashSet<String>);
    impl<'a> Visitor<'a> for V {
        fn visit_expr(&mut self, e: &'a AExpr) {
            if let AExpr::Name(n) = e {
                self.0.insert(n.id.to_string());
            }
            walk_expr(self, e);
        }
    }
    let mut v = V(HashSet::new());
    for s in body {
        v.visit_stmt(s);
    }
    v.0
}

/// `nonlocal a, b` statements directly in `body` (not in nested functions).
fn nonlocals(body: &[AStmt]) -> HashSet<String> {
    let mut out = HashSet::new();
    fn go(body: &[AStmt], out: &mut HashSet<String>) {
        for s in body {
            match s {
                AStmt::Nonlocal(n) => n.names.iter().for_each(|x| {
                    out.insert(x.id.to_string());
                }),
                AStmt::If(i) => {
                    go(&i.body, out);
                    i.elif_else_clauses.iter().for_each(|c| go(&c.body, out));
                }
                AStmt::For(f) => go(&f.body, out),
                AStmt::While(w) => go(&w.body, out),
                AStmt::Try(t) => go(&t.body, out),
                _ => {}
            }
        }
    }
    go(body, &mut out);
    out
}

/// What the lowering of the enclosing function keeps while a nested one is lowered.
pub(super) struct Saved {
    cur: FuncId,
    scopes: Vec<HashMap<String, VarId>>,
    active: HashSet<VarId>,
    loops: u32,
    narrow: Vec<((VarId, Vec<String>), VarId)>,
    handlers: Vec<VarId>,
    loop_keys: HashMap<VarId, VarId>,
}

impl Front {
    pub(super) fn save(&mut self) -> Saved {
        Saved {
            cur: self.cur,
            scopes: std::mem::take(&mut self.scopes),
            active: std::mem::take(&mut self.active),
            loops: self.loops,
            narrow: std::mem::take(&mut self.narrow),
            handlers: std::mem::take(&mut self.handlers),
            loop_keys: std::mem::take(&mut self.loop_keys),
        }
    }

    pub(super) fn restore(&mut self, s: Saved) {
        self.cur = s.cur;
        self.scopes = s.scopes;
        self.active = s.active;
        self.loops = s.loops;
        self.narrow = s.narrow;
        self.handlers = s.handlers;
        self.loop_keys = s.loop_keys;
    }

    /// `def f(...)` inside a function.
    pub(super) fn nested_def(&mut self, d: &ast::StmtFunctionDef, line: u32) -> Res<()> {
        let name = d.name.id.to_string();
        if !d.decorator_list.is_empty() {
            return err(line, "decorators on nested functions are not supported");
        }
        // the enclosing functions' variables it uses (and those of the nested
        // functions it calls)
        let params: HashSet<String> = d
            .parameters
            .posonlyargs
            .iter()
            .chain(d.parameters.args.iter())
            .chain(d.parameters.kwonlyargs.iter())
            .map(|p| p.parameter.name.id.to_string())
            .collect();
        let nl = nonlocals(&d.body);
        let mut assigned = HashMap::new();
        assigned_names(&d.body, &mut assigned);
        let used = names_in(&d.body);
        let enclosing: HashSet<&String> = self.enclosing.iter().flatten().collect();
        let mut caps: Vec<String> = vec![];
        for n in &used {
            let local = params.contains(n) || (assigned.contains_key(n) && !nl.contains(n));
            if !local && enclosing.contains(n) {
                caps.push(n.clone());
            }
            // a nested function called from here: its captures too
            if let Some(&g) = self.funcs.get(n) {
                for c in self.p.funcs[g].captures.clone() {
                    if !params.contains(&c) && !caps.contains(&c) {
                        caps.push(c);
                    }
                }
            }
        }
        caps.sort();
        caps.dedup();
        for n in &nl {
            if !caps.contains(n) {
                return err(line, format!("no binding for nonlocal `{}`", n));
            }
        }
        let cap_tys: Vec<Ty> = caps
            .iter()
            .map(|n| match self.lookup(n) {
                Some(v) => self.var(v).ty.clone(),
                None => self.fresh(),
            })
            .collect();
        let saved = self.save();
        let res = (|| -> Res<FuncId> {
            let fid = self.signature(d, None)?;
            // a unique Rust name
            if self.p.funcs.iter().enumerate().any(|(i, f)| i != fid && f.class.is_none() && f.name == name) {
                let outer = self.p.funcs[saved.cur].name.clone();
                self.p.funcs[fid].name = format!("{}_{}", outer, name);
            }
            for (n, t) in caps.iter().zip(cap_tys) {
                let v = self.new_var(n, t, VarKind::Param);
                let force = if nl.contains(n) { Some(Force::Mut) } else { None };
                self.p.funcs[fid].params.push(Param { var: v, default: None, force, width: None });
            }
            self.p.funcs[fid].captures = caps.clone();
            // visible inside its own body (recursion)
            let old = self.funcs.insert(name.clone(), fid);
            if let Some(frame) = self.local_funcs.last_mut() {
                frame.push((name.clone(), old));
            }
            Ok(fid)
        })();
        let fid = match res {
            Ok(f) => f,
            Err(e) => {
                self.restore(saved);
                return Err(e);
            }
        };
        let r = self.body(fid, &d.body);
        self.restore(saved);
        r
    }

    /// The arguments for a nested function's captured variables at a call.
    pub(super) fn capture_args(&mut self, fid: FuncId, line: u32) -> Res<Vec<Expr>> {
        let caps = self.p.funcs[fid].captures.clone();
        let n = self.p.funcs[fid].params.len() - caps.len();
        let mut out = vec![];
        for (i, c) in caps.iter().enumerate() {
            let e = self.name(c, line).map_err(|e| {
                Error::new(line, format!("{} (used by `{}`)", e.msg, self.p.funcs[fid].name))
            })?;
            let p = &self.p.funcs[fid].params[n + i];
            let t = self.p.funcs[fid].vars[p.var].ty.clone();
            out.push(self.coerce(e, &t, &format!("captured `{}`", c))?);
        }
        Ok(out)
    }

    /// A module-level function used as a value: a lambda calling it.
    pub(super) fn func_value(&mut self, fid: FuncId, line: u32) -> Res<Expr> {
        let f = &self.p.funcs[fid];
        if f.class.is_some() || f.ext.is_some() {
            return err(line, format!("`{}` cannot be used as a value", f.name));
        }
        if !f.captures.is_empty() {
            return err(line, format!("`{}` uses variables of the enclosing function and cannot be used as a value", f.name))
                .map_err(|e| e.hint("pass those variables as parameters"));
        }
        let ps: Vec<(String, Ty)> = f.params.iter().map(|p| (f.vars[p.var].name.clone(), f.vars[p.var].ty.clone())).collect();
        let ret = f.ret.clone();
        self.scopes.push(HashMap::new());
        let mut vs = vec![];
        let mut args = vec![];
        for (n, t) in &ps {
            let vars = &mut self.p.funcs[self.cur].vars;
            vars.push(Var { name: n.clone(), ty: t.clone(), kind: VarKind::Bound });
            let v = vars.len() - 1;
            self.scopes.last_mut().unwrap().insert(format!("\u{0}{}", n), v);
            self.active.insert(v);
            vs.push(v);
            args.push(self.mk(line, t.clone(), ExprKind::Var(v)));
        }
        let body = self.mk(line, ret.clone(), ExprKind::Call(fid, args));
        self.scopes.pop();
        for v in &vs {
            self.active.remove(v);
        }
        let tys = ps.into_iter().map(|(_, t)| t).collect();
        Ok(self.mk(line, Ty::Func(tys, Box::new(ret)), ExprKind::Lambda(vs, Box::new(body))))
    }

    /// `f(args)` where f is a `Callable` variable.
    pub(super) fn call_value(&mut self, v: VarId, ps: &[Ty], r: &Ty, args: &ast::Arguments, line: u32) -> Res<Expr> {
        if !args.keywords.is_empty() {
            return err(line, "a Callable takes positional arguments only");
        }
        if args.args.len() != ps.len() {
            return err(line, format!("this Callable takes {} argument(s)", ps.len()));
        }
        let t = self.var(v).ty.clone();
        let mut out = vec![self.mk(line, t, ExprKind::Var(v))];
        for (i, p) in ps.iter().enumerate() {
            out.push(self.arg(args, i, p, "argument")?);
        }
        Ok(self.mk(line, r.clone(), ExprKind::Builtin("call_fn", out)))
    }

    /// `m.x` of a local module m: the name `x`.
    pub(super) fn local_name(&self, e: &AExpr) -> Option<AExpr> {
        let AExpr::Attribute(a) = e else { return None };
        let AExpr::Name(m) = &*a.value else { return None };
        if !self.local_modules.contains(m.id.as_str()) || self.lookup(m.id.as_str()).is_some() {
            return None;
        }
        Some(AExpr::Name(ast::ExprName {
            node_index: Default::default(),
            range: a.range,
            id: a.attr.id.clone(),
            ctx: ast::ExprContext::Load,
        }))
    }
}
