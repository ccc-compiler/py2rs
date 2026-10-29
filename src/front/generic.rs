//! Generic functions are monomorphized: type variables (`def f[T](..)`,
//! `T = TypeVar("T")`), `Protocol` parameters and `X | Y` parameters. Each call
//! binds them from the argument types and calls the instance for those types
//! (`first_i64`, `first_string`), lowered from the same AST. Inside an instance
//! `isinstance(x, int)` is known, and the branch that cannot run is not lowered.

use super::*;

/// A function to instantiate: its AST and the type environment.
pub(super) type Pending = (FuncId, &'static ast::StmtFunctionDef, HashMap<String, Ty>);

/// Is `e` a union other than `X | None`?
fn is_union(e: &AExpr) -> bool {
    match e {
        AExpr::BinOp(b) if b.op == ast::Operator::BitOr => {
            !matches!(*b.left, AExpr::NoneLiteral(_)) && !matches!(*b.right, AExpr::NoneLiteral(_))
        }
        AExpr::Subscript(s) => matches!(&*s.value, AExpr::Name(n) if n.id.as_str() == "Union")
            && !matches!(&*s.slice, AExpr::Tuple(t) if t.elts.iter().any(|x| matches!(x, AExpr::NoneLiteral(_)))),
        _ => false,
    }
}

pub(super) fn is_union_ann(e: &AExpr) -> bool {
    is_union(e)
}

/// Members of a union annotation.
fn members(e: &AExpr) -> Vec<&AExpr> {
    match e {
        AExpr::BinOp(b) if b.op == ast::Operator::BitOr => {
            let mut v = members(&b.left);
            v.extend(members(&b.right));
            v
        }
        AExpr::Subscript(s) => match &*s.slice {
            AExpr::Tuple(t) => t.elts.iter().collect(),
            x => vec![x],
        },
        x => vec![x],
    }
}

/// A short name of a type for instance names.
fn suffix(t: &Ty, p: &Program) -> String {
    match t {
        Ty::Int => "int".into(),
        Ty::Float => "float".into(),
        Ty::Str => "str".into(),
        Ty::Bool => "bool".into(),
        Ty::List(e) => format!("list_{}", suffix(e, p)),
        Ty::Set(e) => format!("set_{}", suffix(e, p)),
        Ty::Dict(k, v, _) => format!("dict_{}_{}", suffix(k, p), suffix(v, p)),
        Ty::Tuple(ts) => format!("tuple_{}", ts.iter().map(|t| suffix(t, p)).collect::<Vec<_>>().join("_")),
        Ty::Optional(e) => format!("opt_{}", suffix(e, p)),
        Ty::Class(c) | Ty::Obj(c) => snake_name(&p.classes[*c].name),
        _ => "t".into(),
    }
}

impl Front {
    /// Does this function need instances (type variables, Protocol or union parameters)?
    pub(super) fn is_template(&self, d: &ast::StmtFunctionDef) -> bool {
        if d.type_params.is_some() {
            return true;
        }
        let ps = &d.parameters;
        ps.posonlyargs.iter().chain(ps.args.iter()).chain(ps.kwonlyargs.iter()).any(|p| {
            p.parameter.annotation.as_deref().is_some_and(|a| self.mentions_generic(a))
        })
    }

    fn mentions_generic(&self, a: &AExpr) -> bool {
        if is_union(a) {
            return true;
        }
        use ruff_python_ast::visitor::{walk_expr, Visitor};
        struct V<'s>(&'s Front, bool);
        impl<'a> Visitor<'a> for V<'_> {
            fn visit_expr(&mut self, e: &'a AExpr) {
                if let AExpr::Name(n) = e {
                    if self.0.typevars.contains(n.id.as_str()) || self.0.protocols.contains_key(n.id.as_str()) {
                        self.1 = true;
                    }
                }
                walk_expr(self, e);
            }
        }
        let mut v = V(self, false);
        v.visit_expr(a);
        v.1
    }

    /// Binds the type variables of annotation `a` to the parts of `t`.
    fn bind(&mut self, a: &AExpr, t: &Ty, tps: &HashSet<String>, env: &mut HashMap<String, Ty>, line: u32) -> Res<()> {
        let t = self.norm(t);
        if is_union(a) {
            // the argument's type must be one of the members
            for m in members(a) {
                let mt = self.ann(m)?;
                if self.norm(&mt) == t {
                    env.insert(format!("U:{}", self.src[a.range()].trim()), t);
                    return Ok(());
                }
            }
            return err(line, format!("{} is not one of `{}`", self.tn(&t), &self.src[a.range()]));
        }
        match a {
            AExpr::Name(n) if tps.contains(n.id.as_str()) || self.protocols.contains_key(n.id.as_str()) => {
                let k = n.id.to_string();
                if let Some(p) = self.protocols.get(&k).cloned() {
                    let Ty::Class(c) = t else {
                        return err(line, format!("{} does not implement `{}`", self.tn(&t), k));
                    };
                    for m in &p {
                        if self.find_method(&t, m).is_none() {
                            return err(line, format!("`{}` has no method `{}` (protocol `{}`)", self.p.classes[c].name, m, k));
                        }
                    }
                }
                match env.get(&k) {
                    Some(old) if *old != t => err(line, format!("`{}` is {} and {} here", k, self.tn(old), self.tn(&t))),
                    _ => {
                        if matches!(t, Ty::Var(_)) {
                            return err(line, format!("the type of `{}` is not known here; add an annotation", k));
                        }
                        env.insert(k, t);
                        Ok(())
                    }
                }
            }
            AExpr::Subscript(s) => {
                let args: Vec<&AExpr> = match &*s.slice {
                    AExpr::Tuple(x) => x.elts.iter().collect(),
                    x => vec![x],
                };
                let name = match &*s.value {
                    AExpr::Name(n) => n.id.as_str(),
                    _ => return Ok(()),
                };
                match (name, &t) {
                    ("list" | "List", Ty::List(e)) | ("set" | "Set", Ty::Set(e)) | ("Optional", Ty::Optional(e)) => {
                        self.bind(args[0], e, tps, env, line)
                    }
                    ("dict" | "Dict", Ty::Dict(k, v, _)) if args.len() == 2 => {
                        self.bind(args[0], k, tps, env, line)?;
                        self.bind(args[1], v, tps, env, line)
                    }
                    ("tuple" | "Tuple", Ty::Tuple(ts)) if ts.len() == args.len() => {
                        for (x, y) in args.iter().zip(ts) {
                            self.bind(x, y, tps, env, line)?;
                        }
                        Ok(())
                    }
                    ("Callable", Ty::Func(ps, r)) if args.len() == 2 => {
                        if let AExpr::List(l) = args[0] {
                            for (x, y) in l.elts.iter().zip(ps) {
                                self.bind(x, y, tps, env, line)?;
                            }
                        }
                        self.bind(args[1], r, tps, env, line)
                    }
                    _ => Ok(()),
                }
            }
            AExpr::BinOp(b) if b.op == ast::Operator::BitOr => match &t {
                // `T | None`
                Ty::Optional(e) => {
                    let inner = if matches!(*b.left, AExpr::NoneLiteral(_)) { &b.right } else { &b.left };
                    self.bind(inner, e, tps, env, line)
                }
                _ => Ok(()),
            },
            _ => Ok(()),
        }
    }

    /// A call of a generic function: the instance for the argument types.
    pub(super) fn instantiate(&mut self, name: &str, args: &ast::Arguments, line: u32) -> Res<Expr> {
        let d: &'static ast::StmtFunctionDef = self.templates[name];
        let mut tps: HashSet<String> = self.typevars.clone();
        if let Some(tp) = &d.type_params {
            for t in &tp.type_params {
                tps.insert(t.name().id.to_string());
            }
        }
        let ps = &d.parameters;
        if ps.vararg.is_some() || ps.kwarg.is_some() {
            return err(line, "generic functions with *args are not supported");
        }
        let all: Vec<&ast::ParameterWithDefault> = ps.posonlyargs.iter().chain(ps.args.iter()).chain(ps.kwonlyargs.iter()).collect();
        // the arguments' types bind the type variables
        let mut env = HashMap::new();
        for (i, a) in args.args.iter().enumerate() {
            let Some(p) = all.get(i) else { break };
            let x = self.expr(a, None)?;
            self.settle(&x.ty);
            if let Some(ann) = &p.parameter.annotation {
                let t = x.ty.clone();
                self.bind(ann, &t, &tps, &mut env, line)?;
            }
        }
        for k in &args.keywords {
            let n = k.arg.as_ref().map(|a| a.id.as_str()).unwrap_or("");
            let Some(p) = all.iter().find(|p| p.parameter.name.id.as_str() == n) else { continue };
            let x = self.expr(&k.value, None)?;
            self.settle(&x.ty);
            if let Some(ann) = &p.parameter.annotation {
                let t = x.ty.clone();
                self.bind(ann, &t, &tps, &mut env, line)?;
            }
        }
        let mut keys: Vec<(&String, &Ty)> = env.iter().collect();
        keys.sort_by(|a, b| a.0.cmp(b.0));
        let key = format!("{}|{}", name, keys.iter().map(|(k, t)| format!("{}={}", k, self.tn(t))).collect::<Vec<_>>().join(","));
        let fid = match self.instances.get(&key) {
            Some(&f) => f,
            None => {
                let sfx: Vec<String> = keys.iter().map(|(_, t)| suffix(t, &self.p)).collect();
                let saved = self.save();
                let old_env = std::mem::replace(&mut self.type_env, env.clone());
                let r = self.signature(d, None);
                self.type_env = old_env;
                self.restore(saved);
                let fid = r?;
                self.p.funcs[fid].name = format!("{}_{}", name, sfx.join("_"));
                self.instances.insert(key, fid);
                self.pending.push((fid, d, env));
                fid
            }
        };
        let a = self.call_args(fid, args, false, line)?;
        let t = self.p.funcs[fid].ret.clone();
        Ok(self.mk(line, t, ExprKind::Call(fid, a)))
    }

    /// Lowers the bodies of the instances created so far (and those they create).
    pub(super) fn lower_instances(&mut self) -> Res<()> {
        while let Some((fid, d, env)) = self.pending.pop() {
            let old = std::mem::replace(&mut self.type_env, env);
            let r = self.body(fid, &d.body);
            self.type_env = old;
            r?;
        }
        Ok(())
    }

    /// `isinstance(x, T)` decided by x's static type (x not an object of a class
    /// hierarchy): Some(result).
    pub(super) fn static_isinstance(&mut self, test: &AExpr) -> Res<Option<bool>> {
        match test {
            AExpr::UnaryOp(u) if u.op == ast::UnaryOp::Not => Ok(self.static_isinstance(&u.operand)?.map(|b| !b)),
            AExpr::Call(c) if matches!(&*c.func, AExpr::Name(n) if n.id.as_str() == "isinstance" && self.lookup("isinstance").is_none()) => {
                let [AExpr::Name(x), cls] = &c.arguments.args[..] else { return Ok(None) };
                let Some(v) = self.lookup(x.id.as_str()) else { return Ok(None) };
                let t = self.norm(&self.var(v).ty);
                if matches!(t, Ty::Class(_) | Ty::Obj(_) | Ty::Exc | Ty::Optional(_) | Ty::Var(_)) {
                    return Ok(None);
                }
                let names: Vec<&AExpr> = match cls {
                    AExpr::Tuple(tu) => tu.elts.iter().collect(),
                    x => vec![x],
                };
                let mut any = false;
                for n in names {
                    let AExpr::Name(n) = n else { return Ok(None) };
                    any |= match (n.id.as_str(), &t) {
                        ("int", Ty::Int | Ty::Bool) | ("float", Ty::Float) | ("str", Ty::Str) | ("bool", Ty::Bool) => true,
                        ("list", Ty::List(_)) | ("dict", Ty::Dict(..)) | ("set", Ty::Set(_)) | ("tuple", Ty::Tuple(_)) => true,
                        _ => false,
                    };
                }
                Ok(Some(any))
            }
            _ => Ok(None),
        }
    }

    /// `class P(Protocol)`: the names of its methods.
    pub(super) fn protocol(&mut self, d: &ast::StmtClassDef) -> Res<bool> {
        let is = d.arguments.as_ref().is_some_and(|a| a.args.iter().any(|b| matches!(b, AExpr::Name(n) if n.id.as_str() == "Protocol")));
        if !is {
            return Ok(false);
        }
        let mut ms = vec![];
        for s in &d.body {
            match s {
                AStmt::FunctionDef(f) => ms.push(f.name.id.to_string()),
                AStmt::Pass(_) => {}
                _ if is_docstring(s) => {}
                _ => return err(self.line(s), "a Protocol may only declare methods"),
            }
        }
        self.protocols.insert(d.name.id.to_string(), ms);
        Ok(true)
    }
}
