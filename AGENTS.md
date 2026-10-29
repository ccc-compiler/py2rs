# AGENTS.md

py2rs translates a typed subset of Python into readable Rust. Workspace: the `py2rs`
crate (translator + CLI) and `pyrt/` (runtime used by generated code).

## Commands

```sh
cargo build --release
./target/release/py2rs test          # all of tests/: diff vs CPython + expected rejections
./target/release/py2rs emit F.py     # inspect generated Rust
./target/release/py2rs fuzz 300 SEED # random programs vs CPython (failures in target/py2rs-cache/fuzz-fail/)
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
- v0.3: `own::definite` (definite assignment, our error instead of rustc E0381); `fuzz.rs`
  (random programs); `PY2RS_TEST_DIR` for parallel test runs; dead stores are dropped and
  never-read names get `_` (generated code stays warning-free); `count3` counts assignments
  per path (incl. exception paths) for `mut`; writes with a computed list index hoist the
  index (`index_mut_code`, `FGen.pre` lines are flushed before each statement);
  `pyrt::catching(|| op)` turns `raise` / panics into `Err` for failing builtins without a
  `try_` variant (`exc::op_kinds` second list; `FGen.in_catching`).
- v0.4: `own.mut_views` (`mut_view_candidates` → fixpoint → `accepted_mut_views`: modified,
  never an owned value, `conflicts` = no other use of the borrowed part while live; their
  `origin` points at the container so mutations count for it), `own.consumed` (narrowed
  value bound by value when the Optional local is dead at the region start: `consumable`,
  `bound_in` makes narrowing conditions define their variable for liveness), views also
  from locals (`assigned_while_live`), `Param.force` (`Ref`/`Mut`/`Own` from
  `from rust import`, module `python/rust` for CPython; the test runner sets PYTHONPATH),
  `py2rs explain` (`lib::explain`, copies recorded in `gen::clones`).
- v0.5 crates: `front::load_stub` (`from rust.X import` → `X.pyi` in `stubs/`, `PY2RS_STUBS`,
  repo `stubs/`; `# crate:` line → `Program.crates` → `// py2rs-crate:` header →
  `project::crates_of` → Cargo.toml), extern classes (`Class.ext`) and functions
  (`Func.ext`: path / method / template with `{self}` `{0}`, `result`, `self_mut`,
  `ret_width`; `Param.width`), `gen::ext_call`, `pyrt::rust_ok` / `rust_err`;
  `stubgen.rs` + `py2rs stub` (rustdoc JSON via `cargo doc` with RUSTDOCFLAGS
  `--output-format json`); CPython stand-ins in `python/rust/` (`regex.py`); `inline`.

- v0.6 stdlib: `front/stdlib.rs` (`MODULES`, `from_names` for `from M import x`,
  `std_name` resolves `M.x` / `x` to a qualified name, `std_call` / `std_value` /
  `std_ann` / `std_method`) → `Builtin("module.function")` → `gen/stdlib.rs`
  (`std_builtin`, `mutating`, `cache_wrapper`) → `pyrt/src/stdlib.rs`. `re`:
  `front/re.rs` converts and checks patterns with the regex crate (py2rs depends on
  it), `Ty::Pattern(n)` / `Ty::Match(n)` carry the group count, `pyrt/src/re.rs`
  behind feature `re` (`Program.crates` entry `pyrt/re` becomes a pyrt feature in
  Cargo.toml). `own.stores`: a copy stored in a container is checked against later
  modifications of the source, or of anything reached through a path / deeply while the
  source is live.
- v0.7: `front/dunder.rs` (`find_method`, `call_dunder`, `binary_dunder`,
  `compare_dunder`, `unary_dunder`, `truth_dunder`; gen names them `op_*` in `ident`),
  `Func.property` / `Func.cls` (classmethods: `cls` is put into `classes` while the body is
  lowered), class attributes → `p.consts` via `class_attrs`. `front/nested.rs`:
  `nested_def` saves the enclosing lowering state, computes captures from names
  (`enclosing` stack of variable names, plus captures of nested functions it calls),
  appends them as parameters (`Func.captures`; `nonlocal` ones `Force::Mut`, mode Mut in
  `own::nonlocal`, assigned through `*x`), `capture_args` at calls; `func_value` /
  `call_value` / `Ty::Func` (`Builtin("call_fn")`, gen `fn_arg_ty`). `front/matching.rs`
  rewrites `match` into Python `if` source (`parse_at` relocates parsed snippets to the
  case's range; guards get bound names substituted). Files: `Ty::File`,
  `pyrt/src/files.rs`, `with` lowers to assignment + body + `file.close`.
- v0.8: `pyrt/src/generator.rs` (`Gen::new(move |y| ..)`, `Yield::yield_`, rendezvous
  channels; `gen` is a keyword in edition 2024), `Func.is_gen` (params Owned in own,
  fallible generators rejected in exc); exception classes with `__init__`
  (`front/dunder.rs::exc_class` rewrites `super().__init__(m)` to `self.msg_ = str(m)`,
  `exc_vars`, `Builtin("exc.field")`, `PyErr.data`); `FGen.finals` + `run_finals` /
  `fin_block` for exits through finally; `Func.varargs`; `front/generic.rs` (templates,
  `instantiate`, `bind`, `type_env` with `U:<source>` keys for unions, `pending` bodies,
  `static_isinstance` in `lower_if`, `protocols`); `src/modules.rs` (`combine`,
  `translate_file`, `local_name` for `m.x`); `Class.shared` (own: immutable / no `&mut`
  / self may escape; gen: `shared_struct`, accessors through `is_enum`).
- v0.9: `bench/run.py` (CPython vs `py2rs build`); `FGen::hoist_strs` / `char_vecs` /
  `str_lens`; `pyrt::out` (buffered `print!` / `println!` macros imported by generated
  code, `pyrt::flush()` at the end of main); key-once `sort_by_key` (`permute`);
  `d[k] = d.get(k, 0) + x` peephole in `store`; borrowed `split` in for loops. Known
  differences still to fix: TODO.md.
- v1.0: `src/pyo3.rs` (`py2rs pyo3`; `wrappers` + `crate_files`, `project::cargo_toml_lib`,
  macOS needs `-undefined dynamic_lookup` in the generated `.cargo/config.toml`,
  `PYO3_PYTHON`); `own::field_consumable` (let-else narrowing of a field, taken with
  `.take()` in `gen::let_some`), `own::assigns_owned` / `field_dead_after` (partial moves);
  the generator/@shared check is in `exc::analyze`. Docs: `SPEC.md`, `docs/cases/make.py`,
  `README.tw.md` (edit) → `README.cn.md` (OpenCC `tw2sp` + mainland terms). Publishing:
  `pyrt` first, then `py2rs`, only with the owner's go-ahead.
