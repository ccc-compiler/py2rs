# py2rs

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
   seen, and infers `&T` / `&mut T` / `T` for every parameter.
3. **Same output as CPython, or a clear error.** Anything that cannot be guaranteed is
   rejected at translation time with the line and a hint, never translated differently.

## Usage

```sh
cargo build --release
./target/release/py2rs emit  prog.py          # print the Rust program
./target/release/py2rs check prog.py          # types + ownership only
./target/release/py2rs build prog.py [-o DIR] # Cargo project, release build
./target/release/py2rs run   prog.py          # build (cached) and run
./target/release/py2rs test                   # differential test, see below
```

## Testing

`py2rs test [FILES|DIRS]` (default `tests/`) translates every program, builds them all
in one Cargo package and compares stdout, exit code and exception type with CPython
(`PY2RS_PYTHON`, else python3.14 / 3.13 / 3.12). Input comes from `NAME.in` if present.
A program whose first line is `# error: TEXT` must be rejected with a message containing
TEXT (`tests/err/`).

## The subset

Supported: `int` (i64, overflow is reported), `float`, `bool`, `str`, `list`, `dict`
(insertion ordered, `IndexMap`), `set`, `tuple`; functions with defaults and keyword
arguments; `@dataclass` (incl. `frozen=True`) and plain classes with `__init__`,
methods, `@staticmethod`, `__str__` / `__repr__`; if / while / for (with `else`),
comprehensions, generator expressions in `sum/min/max/any/all/sorted/list/set/join`,
`lambda` as `key=`, f-strings with format specs, `raise` (uncaught), `assert`, `del`,
`math`, most `str` / `list` / `dict` / `set` methods.

Rejected with a hint: `Any` / unannotated parameters, a variable changing type,
`eval` / `exec` / `getattr` / `globals`, `global` / `nonlocal`, mutable defaults,
`list * n` of mutable elements, observable aliasing, modifying a container while
iterating over it, `x or default` on non-bools. Not yet supported: inheritance, `try`,
`with`, `match`, `Optional`, generators (`yield`), async, imports beyond `math`,
`dataclasses`, `typing`.

Known differences: set iteration order is insertion order (CPython's is hash order);
`int` is 64-bit.

## Layout

- `src/front.rs`: ruff parser AST → typed IR (`src/ir.rs`), unification for locals
- `src/own.rs`: ownership inference, liveness, aliasing and iteration checks
- `src/gen.rs`: IR → Rust
- `pyrt/`: runtime: Python printing (float `repr`, containers), format specs,
  `//` `%` `**` semantics, `str` methods by character index, errors as CPython prints them
