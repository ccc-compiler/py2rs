//! Operator special methods: `a + b` on objects calls `__add__`, `a < b` calls
//! `__lt__`, `len(x)` calls `__len__`, ... (each becomes an ordinary method call).

use super::*;

/// Binary operators and their methods (normal, reflected).
fn names(op: BinOp) -> (&'static str, &'static str) {
    use BinOp::*;
    match op {
        Add => ("__add__", "__radd__"),
        Sub => ("__sub__", "__rsub__"),
        Mul => ("__mul__", "__rmul__"),
        Div => ("__truediv__", "__rtruediv__"),
        FloorDiv => ("__floordiv__", "__rfloordiv__"),
        Mod => ("__mod__", "__rmod__"),
        Pow => ("__pow__", "__rpow__"),
        BitAnd => ("__and__", "__rand__"),
        BitOr => ("__or__", "__ror__"),
        BitXor => ("__xor__", "__rxor__"),
        Shl => ("__lshift__", "__rlshift__"),
        Shr => ("__rshift__", "__rrshift__"),
    }
}

/// Special methods the subset supports (others are rejected).
pub const SUPPORTED: &[&str] = &[
    "__add__", "__radd__", "__sub__", "__rsub__", "__mul__", "__rmul__", "__truediv__", "__rtruediv__",
    "__floordiv__", "__rfloordiv__", "__mod__", "__rmod__", "__pow__", "__rpow__", "__and__", "__rand__",
    "__or__", "__ror__", "__xor__", "__rxor__", "__lshift__", "__rlshift__", "__rshift__", "__rrshift__",
    "__neg__", "__pos__", "__abs__", "__eq__", "__ne__", "__lt__", "__le__", "__gt__", "__ge__", "__len__",
    "__bool__", "__getitem__", "__setitem__", "__contains__",
];

/// Special methods that decide equality / order (not in class hierarchies).
pub const ORDER: &[&str] = &["__eq__", "__ne__", "__lt__", "__le__", "__gt__", "__ge__"];

impl Front {
    pub(super) fn is_obj(&self, t: &Ty) -> bool {
        matches!(self.norm(t), Ty::Class(_) | Ty::Obj(_))
    }

    /// A class attribute of `c` or a base class.
    pub(super) fn class_attr(&self, c: ClassId, name: &str) -> Option<usize> {
        let mut k = Some(c);
        while let Some(x) = k {
            if let Some(&i) = self.class_attrs.get(&(x, name.to_string())) {
                return Some(i);
            }
            k = self.p.classes[x].base;
        }
        None
    }

    /// The method `m` of an object of type `t` (through the dispatcher for a
    /// class with subclasses).
    pub(super) fn find_method(&self, t: &Ty, m: &str) -> Option<FuncId> {
        match self.norm(t) {
            Ty::Class(c) if !self.p.classes[c].subs.is_empty() => self.p.classes[c]
                .dispatch
                .iter()
                .find(|(n, _)| n == m)
                .map(|(_, f)| *f)
                .or_else(|| self.p.classes[c].method(m)),
            Ty::Class(c) | Ty::Obj(c) => self.p.classes[c].method(m),
            _ => None,
        }
    }

    /// `recv.m(args)` with arguments already lowered.
    pub(super) fn call_dunder(&mut self, recv: Expr, fid: FuncId, args: Vec<Expr>, line: u32) -> Res<Expr> {
        let params: Vec<(String, Ty)> = {
            let f = &self.p.funcs[fid];
            f.params.iter().map(|p| (f.vars[p.var].name.clone(), f.vars[p.var].ty.clone())).collect()
        };
        let name = self.p.funcs[fid].name.clone();
        if params.len() != args.len() {
            return err(line, format!("{} must take {} argument(s) besides self", name, args.len()));
        }
        let mut v = vec![];
        for (a, (n, t)) in args.into_iter().zip(params) {
            v.push(self.coerce(a, &t, &format!("argument `{}` of {}", n, name))?);
        }
        let t = self.p.funcs[fid].ret.clone();
        Ok(self.mk(line, t, ExprKind::Method(Box::new(recv), fid, v)))
    }

    /// `l op r` on objects: `l.__op__(r)` or `r.__rop__(l)`.
    pub(super) fn binary_dunder(&mut self, op: BinOp, l: Expr, r: Expr, line: u32) -> Res<Result<Expr, (Expr, Expr)>> {
        let (m, rm) = names(op);
        if let Some(f) = self.find_method(&l.ty, m) {
            return self.call_dunder(l, f, vec![r], line).map(Ok);
        }
        if let Some(f) = self.find_method(&r.ty, rm) {
            return self.call_dunder(r, f, vec![l], line).map(Ok);
        }
        Ok(Err((l, r)))
    }

    /// `a op b` for comparisons on objects (one operator).
    pub(super) fn compare_dunder(&mut self, op: CmpOp, a: Expr, b: Expr, line: u32) -> Res<Result<Expr, (Expr, Expr)>> {
        use CmpOp::*;
        let not = |s: &mut Self, e: Expr| s.mk(line, Ty::Bool, ExprKind::Unary(UnOp::Not, Box::new(e)));
        if matches!(op, In | NotIn) {
            return match self.find_method(&b.ty, "__contains__") {
                Some(f) => {
                    let e = self.call_dunder(b, f, vec![a], line)?;
                    Ok(Ok(if op == NotIn { not(self, e) } else { e }))
                }
                None => Ok(Err((a, b))),
            };
        }
        let (m, swapped) = match op {
            Eq => ("__eq__", "__eq__"),
            Ne => ("__ne__", "__ne__"),
            Lt => ("__lt__", "__gt__"),
            Le => ("__le__", "__ge__"),
            Gt => ("__gt__", "__lt__"),
            Ge => ("__ge__", "__le__"),
            In | NotIn => unreachable!(),
        };
        if let Some(f) = self.find_method(&a.ty, m) {
            return self.call_dunder(a, f, vec![b], line).map(Ok);
        }
        // `a != b` without __ne__ is `not a == b`; `a > b` is `b < a`
        if op == Ne {
            if let Some(f) = self.find_method(&a.ty, "__eq__") {
                let e = self.call_dunder(a, f, vec![b], line)?;
                return Ok(Ok(not(self, e)));
            }
        }
        if let Some(f) = self.find_method(&b.ty, swapped) {
            return self.call_dunder(b, f, vec![a], line).map(Ok);
        }
        let is_obj = |s: &Self, t: &Ty| matches!(s.norm(t), Ty::Class(_) | Ty::Obj(_));
        if !matches!(op, Eq | Ne) && (is_obj(self, &a.ty) || is_obj(self, &b.ty)) {
            let sym = match op {
                Lt => "<",
                Le => "<=",
                Gt => ">",
                _ => ">=",
            };
            return err(line, format!("`{}` is not defined for these objects", sym))
                .map_err(|e| e.hint(format!("define {}", m)));
        }
        Ok(Err((a, b)))
    }

    /// A method of the object `x` with no arguments (`__len__`, `__neg__`, ...), if defined.
    pub(super) fn unary_dunder(&mut self, x: Expr, m: &str, line: u32) -> Res<Result<Expr, Expr>> {
        match self.find_method(&x.ty, m) {
            Some(f) => self.call_dunder(x, f, vec![], line).map(Ok),
            None => Ok(Err(x)),
        }
    }

    /// The truth value of an object: `__bool__`, else `__len__() != 0`.
    pub(super) fn truth_dunder(&mut self, x: Expr, line: u32) -> Res<Result<Expr, Expr>> {
        let x = match self.unary_dunder(x, "__bool__", line)? {
            Ok(e) => return Ok(Ok(e)),
            Err(x) => x,
        };
        match self.unary_dunder(x, "__len__", line)? {
            Ok(n) => {
                let zero = self.mk(line, Ty::Int, ExprKind::Int(0));
                Ok(Ok(self.mk(line, Ty::Bool, ExprKind::Cmp(Box::new(n), vec![(CmpOp::Ne, zero)]))))
            }
            Err(x) => Ok(Err(x)),
        }
    }

    /// Checks of a special method's signature when the class is lowered.
    pub(super) fn check_dunder(&self, cid: ClassId, fid: FuncId, line: u32) -> Res<()> {
        let f = &self.p.funcs[fid];
        let name = f.name.as_str();
        let want = match name {
            "__eq__" | "__ne__" | "__lt__" | "__le__" | "__gt__" | "__ge__" | "__bool__" | "__contains__" => Some(Ty::Bool),
            "__len__" => Some(Ty::Int),
            _ => None,
        };
        if let Some(w) = want {
            if f.ret != w {
                return err(line, format!("{} must return {}", name, self.tn(&w)));
            }
        }
        let c = &self.p.classes[cid];
        if ORDER.contains(&name) && (c.base.is_some() || !c.subs.is_empty()) {
            return err(line, format!("{} in a class hierarchy is not supported", name));
        }
        Ok(())
    }
}
