# The py2rs subset

py2rs accepts a typed subset of Python 3.12–3.14. A program it accepts behaves like
under CPython: same standard output, same exit code, same exception type on an
uncaught error. Everything else is rejected when translating, with the line and,
usually, a hint. This document lists what is accepted.

## 1. Types

| Python | Rust | notes |
|---|---|---|
| `int` | `i64` | overflow raises `OverflowError` (CPython has big ints) |
| `float` | `f64` | `repr` and format specs as CPython |
| `bool` | `bool` | |
| `str` | `String` / `&str` | indexed by character |
| `list[T]` | `Vec<T>` / `&[T]` | |
| `dict[K, V]` | `IndexMap<K, V>` | insertion ordered, like CPython |
| `set[T]` | `IndexSet<T>` | iteration order differs from CPython (see 9) |
| `tuple[A, B]` | `(A, B)` | fixed length |
| `T \| None`, `Optional[T]` | `Option<T>` | narrowed by `is None` / `is not None` / truth tests |
| `Callable[[A], R]` | `&impl Fn(A) -> R` | parameters only |
| `Iterator[T]`, `Generator[T, None, None]` | `pyrt::Gen<T>` | return type of generators |
| `defaultdict[K, V]`, `Counter[K]`, `deque[T]` | `pyrt::DefaultDict`, `pyrt::Counter`, `VecDeque` | |
| a class | a struct (an enum over the created classes when it has subclasses) | |

Keys of dicts and elements of sets are `int`, `bool`, `str`, tuples of them or frozen
dataclasses (`float` keys are rejected).

Every function parameter and return value is annotated; local variables are inferred
and keep one type. `Any` and `object` are rejected.

## 2. Statements

Assignment (also tuple unpacking, augmented, annotated), `if` / `elif` / `else`,
`while` / `for` (with `else`, `break`, `continue`), `def`, `class`, `return`, `raise`,
`try` / `except` / `else` / `finally` (also `return` / `break` / `continue` through
`finally`), `assert`, `del` (list items, dict keys), `pass`, `with open(...) as f`,
`match` (literal, capture, wildcard, `|`, guards, sequence patterns with `*rest`, class
patterns with keyword or dataclass positional arguments, `None`), `import` /
`from ... import` of the modules in section 6 and of local modules, `nonlocal` in
nested functions.

Rejected: `global`, `async` / `await`, `with` other than `open`, `yield` as an
expression (`x = yield`), mapping patterns, `del` of names.

## 3. Expressions

Arithmetic with Python's semantics (`//` and `%` floor, `**`, int / float promotion),
comparisons (also chained), `and` / `or` / `not` on bools, `x if c else y`, `in` /
`not in`, indexing and slicing (with steps and negative indices), list / set / dict
comprehensions and generator expressions, f-strings with format specs, `lambda` (as
`key=`, as a `Callable` argument, or assigned to an annotated variable), calls with
keyword arguments, defaults and `*args`.

Rejected: `x or default` / `x and y` on non-bools (Python returns an operand), `is`
except with `None` and between `@shared` objects, `:=`, `*` / `**` unpacking in calls,
`eval` / `exec` / `getattr` / `setattr` / `globals` / `type` / `id`.

## 4. Functions

Positional and keyword parameters with constant defaults, `*args: T` (a `list[T]`),
recursion, nested functions (lifted; the enclosing variables they use are passed at
each call, `nonlocal` ones by `&mut`), generators (`yield`, `yield from`, `next(g)`,
`next(g, default)`; the body runs strictly in turn with its consumer), `@functools.cache`
/ `@lru_cache(maxsize=None)`.

Generic functions are compiled once per combination of argument types (monomorphized):
type variables (`def f[T](...)` or `T = TypeVar("T")`), `Protocol` parameters, and union
parameters (`x: int | str`, where `isinstance(x, int)` is decided per instance).

Rejected: `**kwargs`, mutable default values, generator methods, `send()`, an exception
raised inside a generator that its consumer catches, nested functions that use enclosing
variables passed as values, unions outside parameters.

## 5. Classes

`@dataclass` (also `frozen=True`) and plain classes with `__init__`; methods,
`@staticmethod`, `@classmethod`, `@property` (read only), constant class attributes,
`__str__` / `__repr__`, operator methods (`__add__`, `__sub__`, `__mul__`,
`__truediv__`, `__floordiv__`, `__mod__`, `__pow__`, bitwise ones, the reflected `__r*__`
forms, `__neg__`, `__pos__`, `__abs__`, `__eq__`, `__ne__`, `__lt__`, `__le__`, `__gt__`,
`__ge__`, `__len__`, `__bool__`, `__getitem__`, `__setitem__`, `__contains__`), single
inheritance with overriding, `super()`, `isinstance`. Exception classes, also with
fields (`__init__` calling `super().__init__(message)`).

`@shared` (`from rust import shared`) makes the objects of a class `Rc<RefCell<..>>`:
names may share them as in Python (graphs, parent links), and `a is b` compares identity.

Rejected: multiple inheritance, overrides that change a method's types, property setters,
mutable class attributes, `__hash__` and other special methods, `__eq__` / `__lt__` in a
class hierarchy, `@shared` in a class hierarchy.

## 6. Standard library

`math`, `collections` (`defaultdict`, `Counter`, `deque`), `heapq`, `bisect`,
`itertools` (`permutations`, `combinations`, `product`, `chain`, `accumulate`,
`pairwise`), `functools` (`reduce`, `cache`, `lru_cache(maxsize=None)`), `random`
(CPython's Mersenne Twister: the same seed gives the same numbers), `re` (literal
patterns, checked when translating, on the `regex` crate), `sys` (`argv`, `exit`,
`stdin`, `stdout.write`, `maxsize`, `setrecursionlimit`), `time`, `json.dumps`, `os`
(`path.exists`, `path.join`, `remove`), `dataclasses`, `typing`. Text files with `open`
(`read`, `readline`, `readlines`, iteration, `write`, `writelines`).

In `re`, lookaround, backreferences, `$` without `re.MULTILINE` and function
replacements are rejected (the regex crate differs from Python there).

Rust crates: `from rust.CRATE import X` with a stub `CRATE.pyi` (see README).

## 7. Ownership

Python names share objects; Rust values have one owner. py2rs infers for each parameter
whether it is read (`&T`), modified (`&mut T`) or kept (`T`), moves a value at its last
use, borrows locals that only read other objects, and copies a value when the original
is still used. It rejects a program when the copy could be observed:

- a value stored in two places and modified through one of them later
  (`b = a; b.append(1); print(a)` is accepted as a `&mut` when possible, else rejected)
- modifying a container while a loop iterates over it
- `[[0] * m] * n` of mutable elements
- `self` stored or returned (except in `@shared` classes)

`from rust import Ref, Mut, Own` pins a parameter's mode; `py2rs explain` shows what
was inferred.

## 8. Exceptions

An exception nobody catches prints a traceback line like CPython's and exits with 1. A
function returns `Result<T, PyErr>` only when some caller catches what it may raise.
Built-in failures (index, key, division by zero, `int("x")`, overflow, ...) raise the
same exception classes as CPython.

## 9. Known differences

- `int` is 64-bit: results beyond it raise `OverflowError` instead of growing.
- `set` iteration order is insertion order; CPython's is its hash table order
  (TODO.md). Sort sets before printing them.
- Exceptions raised inside generators cannot be caught by the consumer (rejected).
