"""py2rs ownership annotations. At run time `Ref[T]`, `Mut[T]` and `Own[T]` are
just `T`; py2rs reads them to choose `&T`, `&mut T` or `T` for a parameter."""


class _Mode:
    def __class_getitem__(cls, t):
        return t


class Ref(_Mode):
    """A parameter that is only read: `&T`."""


class Mut(_Mode):
    """A parameter that is modified in place: `&mut T`."""


class Own(_Mode):
    """A parameter the function keeps: `T` (the caller moves or copies)."""
