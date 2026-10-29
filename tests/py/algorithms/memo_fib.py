def fib_memo(n: int, memo: dict[int, int]) -> int:
    if n <= 1:
        return n
    if n in memo:
        return memo[n]
    v = fib_memo(n - 1, memo) + fib_memo(n - 2, memo)
    memo[n] = v
    return v


def fib_iter(n: int) -> int:
    a = 0
    b = 1
    for _ in range(n):
        a, b = b, a + b
    return a


def climb(n: int, steps: list[int]) -> int:
    ways = [0] * (n + 1)
    ways[0] = 1
    for i in range(1, n + 1):
        for s in steps:
            if s <= i:
                ways[i] += ways[i - s]
    return ways[n]


def tribonacci(n: int) -> list[int]:
    t = [0, 0, 1]
    while n > len(t):
        t.append(t[-1] + t[-2] + t[-3])
    return t[:n]


def collatz_len(n: int, cache: dict[int, int]) -> int:
    path: list[int] = []
    while n != 1 and n not in cache:
        path.append(n)
        n = n // 2 if n % 2 == 0 else 3 * n + 1
    base = cache.get(n, 1)
    for k in reversed(path):
        base += 1
        cache[k] = base
    return cache.get(path[0], 1) if path else base


def main() -> None:
    memo: dict[int, int] = {}
    for n in [0, 1, 2, 10, 20, 50, 90]:
        print(f"fib({n}) = {fib_memo(n, memo)} iter={fib_iter(n)}")
    print("memo size:", len(memo))
    for n in [1, 2, 3, 4, 10, 30]:
        print(f"stairs {n}: 1/2 steps {climb(n, [1, 2])}, 1/3/5 steps {climb(n, [1, 3, 5])}")
    print("tribonacci:", tribonacci(15))
    cache: dict[int, int] = {}
    best = 1
    best_len = 1
    for n in range(1, 10000):
        ln = collatz_len(n, cache)
        if ln > best_len:
            best = n
            best_len = ln
    print(f"longest collatz below 10000: {best} with {best_len} terms")
    print("27 ->", collatz_len(27, cache), "97 ->", collatz_len(97, cache))


main()
