//! Typed intermediate representation: Python semantics, every name resolved,
//! every expression typed. Produced by `front`, checked by `own`, printed by `gen`.

use std::fmt;

pub type VarId = usize;
pub type FuncId = usize;
pub type ClassId = usize;

#[derive(Clone, Debug, PartialEq)]
pub enum Ty {
    Int,
    Float,
    Bool,
    Str,
    None,
    List(Box<Ty>),
    Dict(Box<Ty>, Box<Ty>),
    Set(Box<Ty>),
    Tuple(Vec<Ty>),
    Class(ClassId),
    /// `T | None`
    Optional(Box<Ty>),
    /// a caught exception (`except E as e`)
    Exc,
    /// exactly this class (`self` in a class that has subclasses); `Class(c)`
    /// is c or any subclass
    Obj(ClassId),
    /// Inference variable (only during `front`).
    Var(usize),
}

impl Ty {
    pub fn list(t: Ty) -> Ty {
        Ty::List(Box::new(t))
    }
    pub fn set(t: Ty) -> Ty {
        Ty::Set(Box::new(t))
    }
    pub fn opt(t: Ty) -> Ty {
        Ty::Optional(Box::new(t))
    }
    pub fn dict(k: Ty, v: Ty) -> Ty {
        Ty::Dict(Box::new(k), Box::new(v))
    }
    /// Rust `Copy`: numbers, bools and tuples of them.
    pub fn is_copy(&self) -> bool {
        match self {
            Ty::Int | Ty::Float | Ty::Bool | Ty::None => true,
            Ty::Tuple(ts) => ts.iter().all(|t| t.is_copy()),
            Ty::Optional(t) => t.is_copy(),
            _ => false,
        }
    }
    pub fn is_num(&self) -> bool {
        matches!(self, Ty::Int | Ty::Float)
    }
}

pub struct TyName<'a>(pub &'a Ty, pub &'a Program);

impl fmt::Display for TyName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let p = self.1;
        match self.0 {
            Ty::Int => write!(f, "int"),
            Ty::Float => write!(f, "float"),
            Ty::Bool => write!(f, "bool"),
            Ty::Str => write!(f, "str"),
            Ty::None => write!(f, "None"),
            Ty::List(t) => write!(f, "list[{}]", TyName(t, p)),
            Ty::Set(t) => write!(f, "set[{}]", TyName(t, p)),
            Ty::Dict(k, v) => write!(f, "dict[{}, {}]", TyName(k, p), TyName(v, p)),
            Ty::Tuple(ts) => {
                write!(f, "tuple[")?;
                for (i, t) in ts.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", TyName(t, p))?;
                }
                write!(f, "]")
            }
            Ty::Class(c) => write!(f, "{}", p.classes[*c].name),
            Ty::Optional(t) => write!(f, "{} | None", TyName(t, p)),
            Ty::Obj(c) => write!(f, "{}", p.classes[*c].name),
            Ty::Exc => write!(f, "Exception"),
            Ty::Var(_) => write!(f, "?"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    FloorDiv,
    Mod,
    Pow,
    Shl,
    Shr,
    BitAnd,
    BitOr,
    BitXor,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CmpOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    In,
    NotIn,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum UnOp {
    Neg,
    Pos,
    Invert,
    Not,
}

#[derive(Clone, Debug)]
pub struct Expr {
    pub id: u32,
    pub line: u32,
    pub ty: Ty,
    pub kind: ExprKind,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
    NoneLit,
    /// the value `None` of an Optional type
    NoneVal,
    /// `T` → `T | None`
    SomeOf(Box<Expr>),
    /// a subclass value where a base class is expected
    Upcast(Box<Expr>),
    /// `isinstance(x, C)`; with a variable, `x` as a `C` in the narrowed branch
    IsInst(Box<Expr>, ClassId, Option<VarId>),
    /// `super().__init__(args)` (the base's __init__)
    SuperInit(FuncId, Vec<Expr>),
    /// `x is None` (false) / `x is not None` (true); with a variable, the
    /// value is bound to it in the narrowed branch
    IsSome(Box<Expr>, bool, Option<VarId>),
    Var(VarId),
    Const(usize),
    Bin(BinOp, Box<Expr>, Box<Expr>),
    Unary(UnOp, Box<Expr>),
    /// `a < b <= c`
    Cmp(Box<Expr>, Vec<(CmpOp, Expr)>),
    And(Vec<Expr>),
    Or(Vec<Expr>),
    IfExp(Box<Expr>, Box<Expr>, Box<Expr>),
    /// int → float
    ToFloat(Box<Expr>),
    /// Call of a module-level function; arguments are positional, defaults filled in.
    Call(FuncId, Vec<Expr>),
    /// Method call `recv.m(args)` of a user class.
    Method(Box<Expr>, FuncId, Vec<Expr>),
    /// Construct a class instance (arguments in `__init__` / field order).
    New(ClassId, Vec<Expr>),
    /// Builtin function or method on a builtin type ("len", "list.append", ...);
    /// methods have the receiver as `args[0]`.
    Builtin(&'static str, Vec<Expr>),
    Field(Box<Expr>, String),
    TupleField(Box<Expr>, usize),
    Index(Box<Expr>, Box<Expr>),
    Slice(Box<Expr>, Option<Box<Expr>>, Option<Box<Expr>>, Option<Box<Expr>>),
    List(Vec<Expr>),
    Tuple(Vec<Expr>),
    Set(Vec<Expr>),
    Dict(Vec<(Expr, Expr)>),
    Comp(Box<Comp>),
    FStr(Vec<FPart>),
    Lambda(Vec<VarId>, Box<Expr>),
    /// An iterator used as a value (argument of `list`, `sum`, `sorted`, ...).
    Iter(Box<Iter>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CompKind {
    List,
    Set,
    Dict,
    Gen,
}

#[derive(Clone, Debug)]
pub struct Comp {
    pub kind: CompKind,
    pub elt: Expr,
    /// dict comprehension value
    pub val: Option<Expr>,
    pub gens: Vec<CompFor>,
}

#[derive(Clone, Debug)]
pub struct CompFor {
    pub target: Target,
    pub iter: Iter,
    pub conds: Vec<Expr>,
}

#[derive(Clone, Debug)]
pub enum FPart {
    Lit(String),
    /// value, conversion ('r' for !r), format spec
    Val(Expr, Option<char>, String),
}

#[derive(Clone, Debug)]
pub enum Iter {
    Range(Expr, Expr, Option<Expr>),
    /// list, set, tuple (homogeneous), str (characters), dict (keys)
    Over(Expr),
    Keys(Expr),
    Values(Expr),
    Items(Expr),
    Enumerate(Box<Iter>, Option<Expr>),
    Zip(Vec<Iter>),
    Reversed(Box<Iter>),
    /// a generator expression
    Gen(Expr),
}

#[derive(Clone, Debug)]
pub enum Target {
    Var(VarId),
    /// `xs[i]`, `d[k]`, `obj.f`
    Place(Expr),
    Tuple(Vec<Target>),
}

#[derive(Clone, Debug)]
pub struct Stmt {
    pub id: u32,
    pub line: u32,
    pub kind: StmtKind,
}

#[derive(Clone, Debug)]
pub enum StmtKind {
    Expr(Expr),
    Assign(Target, Expr),
    AugAssign(Expr, BinOp, Expr),
    /// `+=` on a variable of immutable type (rebinding): `x += 1`, `s += "a"`
    AugVar(VarId, BinOp, Expr),
    If(Expr, Vec<Stmt>, Vec<Stmt>),
    While(Expr, Vec<Stmt>, Vec<Stmt>),
    For(Target, Iter, Vec<Stmt>, Vec<Stmt>),
    Return(Option<Expr>),
    Break,
    Continue,
    Pass,
    Raise(String, Option<Expr>),
    Assert(Expr, Option<Expr>),
    Del(Expr),
    /// `let Some(var) = place else { body }`: the body leaves the block
    LetSome(VarId, Expr, Vec<Stmt>),
    /// try / except / else / finally
    Try(Vec<Stmt>, Vec<Handler>, Vec<Stmt>, Vec<Stmt>),
    /// bare `raise` in a handler
    Reraise(VarId),
}

#[derive(Clone, Debug)]
pub struct Handler {
    /// exception classes caught (`BaseException` for a bare `except:`)
    pub kinds: Vec<String>,
    /// the exception (named by `as`, or hidden)
    pub var: VarId,
    pub body: Vec<Stmt>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum VarKind {
    Local,
    Param,
    SelfParam,
    /// for-loop / comprehension / lambda variable
    Bound,
    /// the value of an Optional place after `is not None`
    Narrow,
}

#[derive(Clone, Debug)]
pub struct Var {
    pub name: String,
    pub ty: Ty,
    pub kind: VarKind,
}

#[derive(Clone, Debug)]
pub struct Param {
    pub var: VarId,
    pub default: Option<Expr>,
}

#[derive(Clone, Debug)]
pub struct Func {
    pub name: String,
    pub line: u32,
    pub class: Option<ClassId>,
    /// `self` variable for methods
    pub self_var: Option<VarId>,
    pub params: Vec<Param>,
    pub ret: Ty,
    pub body: Vec<Stmt>,
    pub vars: Vec<Var>,
    pub is_init: bool,
    pub is_static: bool,
    /// loop variables that reuse the name of a local: (loop var, local)
    pub shadows: Vec<(VarId, VarId)>,
    /// a method of a class with subclasses: the implementation of each class
    pub dispatch: Vec<(ClassId, FuncId)>,
}

#[derive(Clone, Debug)]
pub struct Field {
    pub name: String,
    pub ty: Ty,
    pub default: Option<Expr>,
}

#[derive(Clone, Debug)]
pub struct Class {
    pub name: String,
    pub line: u32,
    pub fields: Vec<Field>,
    pub dataclass: bool,
    pub frozen: bool,
    pub init: Option<FuncId>,
    pub methods: Vec<(String, FuncId)>,
    pub base: Option<ClassId>,
    pub subs: Vec<ClassId>,
    /// methods called on a value that may be any subclass
    pub dispatch: Vec<(String, FuncId)>,
    /// an exception class: (builtin exception it derives from)
    pub exc: Option<String>,
}

impl Class {
    pub fn method(&self, name: &str) -> Option<FuncId> {
        self.methods.iter().find(|(n, _)| n == name).map(|(_, f)| *f)
    }
    pub fn field(&self, name: &str) -> Option<&Field> {
        self.fields.iter().find(|f| f.name == name)
    }
}

#[derive(Clone, Debug)]
pub struct Const {
    pub name: String,
    pub value: Expr,
}

#[derive(Clone, Debug, Default)]
pub struct Program {
    pub funcs: Vec<Func>,
    pub classes: Vec<Class>,
    pub consts: Vec<Const>,
    /// the synthetic `main` (module-level statements)
    pub main: FuncId,
}

impl Program {
    /// `x` is `a` or a subclass of it.
    pub fn is_sub(&self, x: ClassId, a: ClassId) -> bool {
        let mut c = Some(x);
        while let Some(k) = c {
            if k == a {
                return true;
            }
            c = self.classes[k].base;
        }
        false
    }
    /// `c` and all its subclasses.
    pub fn subtree(&self, c: ClassId) -> Vec<ClassId> {
        let mut out = vec![c];
        let mut i = 0;
        while i < out.len() {
            out.extend(self.classes[out[i]].subs.iter().copied());
            i += 1;
        }
        out
    }
}

impl Expr {
    /// Visit direct sub-expressions.
    pub fn children(&self, f: &mut dyn FnMut(&Expr)) {
        use ExprKind::*;
        match &self.kind {
            Int(_) | Float(_) | Bool(_) | Str(_) | NoneLit | NoneVal | Var(_) | Const(_) => {}
            Bin(_, a, b) => {
                f(a);
                f(b)
            }
            Unary(_, a) | ToFloat(a) | Field(a, _) | TupleField(a, _) | SomeOf(a) | IsSome(a, _, _) | Upcast(a) | IsInst(a, _, _) => f(a),
            Cmp(a, rest) => {
                f(a);
                rest.iter().for_each(|(_, e)| f(e))
            }
            And(v) | Or(v) | List(v) | Tuple(v) | Set(v) | Call(_, v) | New(_, v) | Builtin(_, v) | SuperInit(_, v) => {
                v.iter().for_each(|e| f(e))
            }
            IfExp(a, b, c) => {
                f(a);
                f(b);
                f(c)
            }
            Method(r, _, v) => {
                f(r);
                v.iter().for_each(|e| f(e))
            }
            Index(a, b) => {
                f(a);
                f(b)
            }
            Slice(a, b, c, d) => {
                f(a);
                for x in [b, c, d].into_iter().flatten() {
                    f(x)
                }
            }
            Dict(v) => v.iter().for_each(|(k, x)| {
                f(k);
                f(x)
            }),
            Comp(c) => {
                for g in &c.gens {
                    g.iter.exprs(f);
                    g.target.exprs(f);
                    g.conds.iter().for_each(|e| f(e));
                }
                f(&c.elt);
                if let Some(v) = &c.val {
                    f(v)
                }
            }
            FStr(parts) => parts.iter().for_each(|p| {
                if let FPart::Val(e, _, _) = p {
                    f(e)
                }
            }),
            Lambda(_, b) => f(b),
            Iter(it) => it.exprs(f),
        }
    }
}

impl Stmt {
    /// Nested statement lists.
    pub fn blocks(&self) -> Vec<&Vec<Stmt>> {
        match &self.kind {
            StmtKind::If(_, a, b) | StmtKind::While(_, a, b) | StmtKind::For(_, _, a, b) => vec![a, b],
            StmtKind::LetSome(_, _, b) => vec![b],
            StmtKind::Try(b, hs, e, f) => {
                let mut v = vec![b];
                v.extend(hs.iter().map(|h| &h.body));
                v.push(e);
                v.push(f);
                v
            }
            _ => vec![],
        }
    }
}

impl Iter {
    pub fn exprs(&self, f: &mut dyn FnMut(&Expr)) {
        match self {
            Iter::Range(a, b, c) => {
                f(a);
                f(b);
                if let Some(c) = c {
                    f(c)
                }
            }
            Iter::Over(e) | Iter::Keys(e) | Iter::Values(e) | Iter::Items(e) | Iter::Gen(e) => f(e),
            Iter::Enumerate(i, s) => {
                i.exprs(f);
                if let Some(s) = s {
                    f(s)
                }
            }
            Iter::Zip(v) => v.iter().for_each(|i| i.exprs(f)),
            Iter::Reversed(i) => i.exprs(f),
        }
    }
}

impl Target {
    /// Sub-expressions evaluated when assigning (index / object of a place).
    pub fn exprs(&self, f: &mut dyn FnMut(&Expr)) {
        match self {
            Target::Var(_) => {}
            Target::Place(e) => e.children(f),
            Target::Tuple(v) => v.iter().for_each(|t| t.exprs(f)),
        }
    }
    pub fn vars(&self, out: &mut Vec<VarId>) {
        match self {
            Target::Var(v) => out.push(*v),
            Target::Place(_) => {}
            Target::Tuple(ts) => ts.iter().for_each(|t| t.vars(out)),
        }
    }
}
