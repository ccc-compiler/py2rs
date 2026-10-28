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
`finish` resolves type variables) → `own::analyze` → `gen::generate`.

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
