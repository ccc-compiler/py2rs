# py2rs roadmap

Every version: the differential test passes, generated Rust has no warnings, and every
rejection has a test in `tests/err/`.

| version | theme | status |
|---|---|---|
| M1 (v0.1) | typed subset, ownership inference, aliasing check, diff test | done |
| M2 (v0.2) | Optional, inheritance, exceptions | done |
| v0.3 | reliability: ~150-program corpus, definite assignment, random program fuzzer, CI | done |
| v0.4 | mutable reference locals, `from rust import Ref, Mut, Own`, `py2rs explain`, `@shared` (`Rc<RefCell>`) | |
| v0.5 | `from rust.CRATE import X`: Cargo dependencies, rustdoc-JSON stubs, borrow / Result / Option mapping, `rust_inline` | |
| v0.6 | stdlib: collections, itertools, functools, heapq, bisect, random (MT19937), re, json, sys, time, os.path, files | |
| v0.7 | `with`, `match`, closures / nested functions / `Callable`, `@property`, `@classmethod`, class attributes, operator dunders | |
| v0.8 | generators, exception fields, return in try/finally, `Union`, `Protocol`, generics, `*args`, multi-file projects | |
| v0.9 | benchmarks vs CPython, ASCII strings, fewer clones, buffered print, optional bigint / fixed-width ints, CPython set order | |
| v1.0 | crates.io packages (publish needs the owner's go-ahead), docs in three languages, subset spec, case studies, PyO3 module output | |

## v0.3 result

- corpus: 201 programs (`tests/py/{algorithms,data,oop,text,regress}` + top level), 24 rejections
- `py2rs fuzz N SEED`: thousands of random programs, 0 differences
- the corpus agents found ~50 translator bugs; every reproducer is in `tests/py/regress/`
- deferred to v0.4: moving out of narrowed values (in-place linked-list reversal),
  `item = d[k]; item.qty -= 1` (mutable reference locals)

## v0.3 details

- corpus: `tests/py/<area>/*.py` (algorithms, data, oop, text, errors, optional); stdin from `NAME.in`
- `py2rs test` runs programs in parallel; `PY2RS_TEST_DIR` selects the build directory
- definite assignment: our error instead of rustc E0381
- `py2rs fuzz N`: random well-typed programs, CPython vs Rust
- `.github/workflows/ci.yml`: build, clippy-free, `py2rs test`
