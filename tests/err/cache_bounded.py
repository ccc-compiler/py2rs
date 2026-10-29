# error: only an unbounded cache
from functools import lru_cache


@lru_cache(maxsize=2)
def f(n: int) -> int:
    print("computing", n)
    return n * 2


print(f(1), f(2), f(3), f(1))
