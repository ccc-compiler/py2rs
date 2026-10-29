# error: cannot be used as a value
from typing import Callable


def run(f: Callable[[int], int]) -> int:
    return f(1)


def outer() -> int:
    k = 3

    def add(x: int) -> int:
        return x + k

    return run(add)


print(outer())
