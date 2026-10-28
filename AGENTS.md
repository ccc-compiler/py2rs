# AGENTS.md

py2rs translates a typed subset of Python into readable Rust. Workspace: the `py2rs`
crate (translator + CLI) and `pyrt/` (runtime used by generated code).

## Commands

```sh
cargo build --release
./target/release/py2rs test          # all of tests/: diff vs CPython + expected rejections
./target/release/py2rs emit F.py     # inspect generated Rust
```

`py2rs test` builds every generated program as a bin of one package under
`target/py2rs-cache/difftest`; to check generated code for rustc warnings,
`touch src/bin/*.rs && cargo build --release` there. Keep generated code warning-free.

## Pipeline

`front::lower` (ruff_python_parser 0.0.20 → `ir::Program`, types via unification,
`finish` resolves type variables) → `own::analyze` → `exc::analyze` → `gen::generate`.
Generated projects use edition 2024 (let chains).

- Rule 3 of the design: never emit code that may behave differently from CPython;
  reject with `diag::Error` (line + hint) instead. Error tests in `tests/err/` start
  with `# error: TEXT`.
- `own`: parameter modes (`Ref`/`Mut`/`Owned`) by fixpoint over all functions;
  `Mutation { via, root, path, deep, pos }` (loop variables resolve to what they borrow
  via `origin`); `pending` aliases are checked against later mutations (`after`, loops
  count as "after"); moves only when a variable occurs once in the statement and is dead
  (`live_out`). `shadows`: a loop variable reusing a local's name is fine unless the local
  is live after the loop.
- `gen`: expression results carry a `Kind` (`Val`, `Place`, `Ref`, `Mut`); use
  `owned` / `borrow` / `val` / `mut_ref_of` to convert. Locals are declared at the
  deepest block containing all uses (`plan_decls`), hoisted out of loops when live at
  the loop head; `mut` from `assign_count` (if-branches exclusive) and `own.mut_vars`.
  Int literals that rustc could type as i32 get `_i64` (`recv`, ranges, `needs_annotation`).
- `pyrt` matches CPython ≥ 3.12 (e.g. `sum` of floats is Neumaier-compensated).
- `__init__` becomes `fn new(..) -> Self`; fields are locals until `Self { .. }`.
- Optional (M2): `Ty::Optional`, `ExprKind::{NoneVal, SomeOf, IsSome(place, some, var)}`,
  `StmtKind::LetSome`. Narrowing in `front` (`narrowing` / `narrow_var` / `narrow` stack keyed
  by (root var, fields), ended by `unnarrow` on assignment): `if` / `elif` (`lower_if`),
  `while`, conditional expressions, `assert`, let-else for a diverging `if x is None:`. A
  narrowed value is a `VarKind::Narrow` variable whose `origin` (`own`) is the place +
  `Step::Some`; `gen::let_some` binds it by value / `&` / `&mut` (`own.via`). Recursive
  classes are `Option<Box<T>>` (`gen::recursive`, `R.boxed`). `own.takes`: a place
  overwritten by the next statements is moved with `.take()` / `std::mem::take`.
  `own.views`: locals only assigned from places of params / self / themselves and never
  modified are `&T`.
- Inheritance (M2): `Class.base / subs`; a class with subclasses is an enum over the
  constructed classes (`gen::instantiated`), its own objects are `{C}Obj` (`Ty::Obj` is the
  type of `self` there). Inherited methods are lowered again for each subclass (from the
  base's AST), `super().m` as a `super_m` method; `Class.dispatch` holds dispatcher
  `Func`s (`Func.dispatch`) whose ownership modes are the strongest of the
  implementations. `ExprKind::Upcast` (coercion to a base), `IsInst` (narrowing to a
  class without subclasses borrows the variant; with subclasses copies via `as_d()`),
  `SuperInit` (calls the base's `init` returning its fields as a tuple). Fields of an enum
  value go through generated accessors `f()` / `f_mut()` (`enum_fields`, `ex_mut`).
- Exceptions (M2): `exc.rs` computes `may` (bottom up) and `need` (catchers of callers,
  top down); a function returns `Result<T, PyErr>` only if it may raise something a
  caller catches. Failing builtins have `pyrt::try_*` variants used only where caught
  (`FGen::catches`, `may_fail`, `wrap`); `op_kinds` lists what each node may raise, and
  catching an unsupported one is an error. `FGen.ctx` stacks (catchers, try label).
  `try` → `'ok: { let e = 'body: { ...; break 'ok; }; match e {..} }` so rustc's
  definite-assignment sees the success path; loops crossed by break / continue get labels
  (`loop_label`). Exception classes are `pyrt::*` / generated `const X: &[&str]` chains.

