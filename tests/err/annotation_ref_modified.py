# error: is Ref[...] but
from rust import Ref


def grow(xs: Ref[list[int]]) -> None:
    xs.append(1)
