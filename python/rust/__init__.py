"""py2rs annotations. At run time `Ref[T]`, `Mut[T]`, `Own[T]` and `Result[T]` are
just `T`, and `usize`, `u32`, ... are `int`; py2rs reads them to choose Rust types.
Subpackages (`rust.regex`, ...) are CPython stand-ins for crates."""


class _Mode:
    def __class_getitem__(cls, t):
        return t


class Ref(_Mode):
    """A parameter that is only read: `&T`."""


class Mut(_Mode):
    """A parameter that is modified in place: `&mut T`."""


class Own(_Mode):
    """A parameter the function keeps: `T` (the caller moves or copies)."""


class Result(_Mode):
    """In stubs: the crate returns Result; Err raises ValueError."""


usize = isize = u64 = u32 = u16 = u8 = i32 = i16 = i8 = int
f32 = float


def rust(_template: str):
    """In stubs: the Rust path or expression of a declaration."""
    def deco(f):
        return f
    return deco


class inline:
    """Raw Rust: `inline("...")` runs only in the translation (a no-op here);
    `inline[T]("...")` is a Rust expression of type T (None here)."""

    def __class_getitem__(cls, t):
        return cls

    def __new__(cls, code: str):
        return None
