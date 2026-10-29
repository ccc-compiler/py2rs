# py2rs

English · [繁體中文](README.tw.md) · [简体中文](README.cn.md)

A translator from a typed subset of Python to readable Rust.

```python
@dataclass
class Item:
    name: str
    price: float

def total(items: list[Item], tax: float = 0.05) -> float:
    return sum(i.price for i in items) * (1 + tax)
```

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub name: String,
    pub price: f64,
}

fn total(items: &[Item], tax: f64) -> f64 {
    pyrt::sum_f64(items.iter().map(|i| i.price)) * (1.0 + tax)
}
```

## Promises

1. **Every value has a static type.** Functions are annotated; locals are inferred.
2. **No observable sharing.** Python shares mutable objects between names; Rust copies,
   moves or borrows them. py2rs accepts a program only when the difference cannot be
   seen, and infers `&T` / `&mut T` / `T` for every parameter. (`@shared` classes keep
   Python's sharing, as `Rc<RefCell<..>>`.)
3. **Same output as CPython, or a clear error.** Anything that cannot be guaranteed is
   rejected at translation time with the line and a hint, never translated differently.
4. **Rust crates can be used directly** (`from rust.regex import Regex`).

## Install and use

```sh
cargo install py2rs            # or: cargo build --release in a checkout
py2rs emit  prog.py            # print the Rust program
py2rs check prog.py            # types + ownership only
py2rs explain prog.py          # parameter modes, references, copies, Result functions
py2rs build prog.py [-o DIR]   # Cargo project, release build
py2rs run   prog.py            # build (cached) and run
py2rs pyo3  mod.py [-o DIR]    # a Python extension module (PyO3): DIR/mod.so
py2rs test                     # differential test against CPython (a checkout)
py2rs fuzz 300                 # random programs against CPython
```

A program may import its own modules (`import geometry`, `from geometry import Point`
with `geometry.py` next to it).

## What is supported

The full definition is [SPEC.md](SPEC.md). In short: `int` (64-bit, overflow is an
error), `float`, `bool`, `str`, `list`, `dict`, `set`, `tuple`, `Optional`; functions
with defaults, keywords and `*args`; dataclasses and classes with methods, properties,
class methods, operator methods and single inheritance; exceptions (also with fields)
and `try` / `finally`; comprehensions, f-strings, `match`, `with open`, generators,
nested functions with `nonlocal`, lambdas and `Callable`; generic functions, `Protocol`
and union parameters (compiled per argument type); `math`, `collections`, `heapq`,
`bisect`, `itertools`, `functools`, `random` (same numbers as CPython for the same
seed), `re`, `sys`, `time`, `json.dumps`, `os.path`.

How some constructs come out (more in [docs/cases](docs/cases/README.md)):

| Python | Rust |
|---|---|
| `x: T \| None`, `if x is not None:` | `Option<T>`, `if let Some(x) = &x` |
| `node.next = self.head; self.head = node` | `node.next = self.head.take();` |
| a local that only reads (`cur = cur.next`) | a reference: `let mut cur = &self.head;` |
| `item = self.items[k]; item.qty -= 1` | `let item = &mut self.items[k];` |
| a class with subclasses | an `enum` over the created classes, methods dispatch with `match` |
| a `raise` that a caller catches | `Result<T, PyErr>` on that path only, `?` |
| a nested function using `seen` and `nonlocal count` | `fn dfs(.., seen: &mut HashSet<..>, count: &mut i64)`, the variables passed at each call |
| `def first[T](xs: list[T]) -> T` | `first_int(xs: &[i64])`, `first_str(xs: &[String])`, one per argument type |
| `def gen(n) -> Iterator[int]: ... yield i` | `pyrt::Gen<i64>`: the body runs strictly in turn with its consumer |
| `@shared class Node` | `struct Node(Rc<RefCell<NodeData>>)` with getters and setters |

Known differences from CPython: `int` is 64-bit (results beyond it raise
`OverflowError`); set iteration order is insertion order ([TODO.md](TODO.md)).

## Speed

`python3 bench/run.py`, CPython 3.14 against the translated program (Apple silicon):

| program | CPython | Rust | |
|---|---:|---:|---:|
| nbody (float objects) | 1.21 s | 0.012 s | 101x |
| matmul (lists of floats) | 0.47 s | 0.013 s | 38x |
| fib (recursion) | 0.26 s | 0.013 s | 20x |
| sieve | 0.48 s | 0.027 s | 18x |
| graph (BFS with deque) | 0.14 s | 0.017 s | 8x |
| objects (dataclasses, sort, dict) | 0.59 s | 0.10 s | 6x |
| printing | 0.21 s | 0.056 s | 4x |
| strings, wordfreq | | | 2x |

## Ownership annotations

Parameter modes are inferred. To pin one, annotate it (the module `python/rust` makes the
annotations work under CPython too; put that directory on `PYTHONPATH`):

```python
from rust import Ref, Mut, Own

def fill(xs: Mut[list[int]], n: int) -> None: ...   # &mut Vec<i64>
def keep(xs: Own[list[int]]) -> None: ...           # Vec<i64>
```

An annotation the code contradicts (modifying a `Ref[...]`) is an error.

## Rust crates

`from rust.CRATE import Name` uses a Rust crate. Its declarations come from a stub,
`CRATE.pyi`, looked up in `./stubs`, `PY2RS_STUBS` and the stubs shipped with py2rs
(`stubs/regex.pyi`). The crate becomes a Cargo dependency of the generated project.

```python
from rust.regex import Regex

date = Regex.new(r"(\d{4})-(\d{2})-(\d{2})")      # Result → ValueError when invalid
print(date.replace_all("on 2024-05-17", "$3/$2/$1"))
```

A stub is a Python file of typed declarations; each maps to a Rust path or an expression
template (`@rust("{self}.find({0}).map(|m| m.as_str().to_string())")`). `py2rs stub
CRATE[@VERSION]` writes one from the crate's rustdoc JSON. `from rust import inline` is
the escape hatch: `inline("xs.sort_unstable();")`, `inline[int]("n.count_ones() as i64")`.

## Python extension modules

`py2rs pyo3 mod.py` builds `mod.so`, which Python imports like the original module. Its
module-level functions are exported (int, float, bool, str, list, tuple, dict and
`None` arguments and results); an exception they raise is raised in Python with the same
class; `print` writes to Python's `sys.stdout`. `tests/pyo3/run.sh` checks that importing
the module prints what importing the Python file prints.

## Testing

`py2rs test [FILES|DIRS]` (default `tests/`) translates every program, builds them all
in one Cargo package and compares stdout, exit code and exception type with CPython
(`PY2RS_PYTHON`, else python3.14 / 3.13 / 3.12). Input comes from `NAME.in` if present.
A program whose first line is `# error: TEXT` must be rejected with a message containing
TEXT (`tests/err/`). The corpus has about 240 programs and 45 rejections; `py2rs fuzz`
adds random programs.

## Layout

- `src/front.rs` (+ `src/front/`): ruff parser AST → typed IR (`src/ir.rs`); the
  standard library, `re`, operator methods, nested functions, `match`, generics
- `src/own.rs`: ownership inference, liveness, aliasing and iteration checks, views
- `src/exc.rs`: which exceptions can be caught where, so which functions return `Result`
- `src/gen.rs` (+ `src/gen/`): IR → Rust
- `src/modules.rs`: programs of several files; `src/pyo3.rs`: extension modules
- `pyrt/`: runtime: Python printing, format specs, `//` `%` `**` semantics, `str`
  methods by character, exceptions, the standard library, generators, buffered output
