import sys
from functools import cache, lru_cache
import functools

sys.setrecursionlimit(100000)

calls = 0


@cache
def fib(n: int) -> int:
    if n < 2:
        return n
    return fib(n - 1) + fib(n - 2)


@lru_cache(maxsize=None)
def paths(r: int, c: int) -> int:
    if r == 0 or c == 0:
        return 1
    return paths(r - 1, c) + paths(r, c - 1)


@functools.cache
def lcs(a: str, b: str) -> str:
    if not a or not b:
        return ""
    if a[0] == b[0]:
        return a[0] + lcs(a[1:], b[1:])
    x = lcs(a[1:], b)
    y = lcs(a, b[1:])
    return x if len(x) >= len(y) else y


@cache
def noisy(n: int) -> list[int]:
    print("computing", n)
    return [n, n * n]


def depth(n: int) -> int:
    if n == 0:
        return 0
    return 1 + depth(n - 1)


def main() -> None:
    print(fib(90))
    print(paths(16, 16))
    print(lcs("AGGTABQXYZ", "GXTXAYBQZ"))
    print(noisy(3), noisy(3), noisy(4))
    print(depth(50000))


if __name__ == "__main__":
    main()
