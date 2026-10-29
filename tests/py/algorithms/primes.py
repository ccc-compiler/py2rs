import math


def sieve(n: int) -> list[int]:
    is_prime = [True] * (n + 1)
    is_prime[0] = False
    is_prime[1] = False
    for p in range(2, math.isqrt(n) + 1):
        if is_prime[p]:
            for k in range(p, n // p + 1):
                is_prime[p * k] = False
    return [i for i in range(n + 1) if is_prime[i]]


def is_prime_trial(n: int) -> bool:
    if n < 2:
        return False
    if n % 2 == 0:
        return n == 2
    d = 3
    while d * d <= n:
        if n % d == 0:
            return False
        d += 2
    return True


def factorize(n: int) -> list[tuple[int, int]]:
    out: list[tuple[int, int]] = []
    d = 2
    while d * d <= n:
        e = 0
        while n % d == 0:
            n //= d
            e += 1
        if e > 0:
            out.append((d, e))
        d += 1
    if n > 1:
        out.append((n, 1))
    return out


def fmt_factors(fs: list[tuple[int, int]]) -> str:
    parts: list[str] = []
    for f in fs:
        p, e = f
        parts.append(str(p) if e == 1 else f"{p}^{e}")
    return " * ".join(parts)


def goldbach(n: int, primes: set[int]) -> tuple[int, int]:
    for p in sorted(primes):
        if p > n // 2:
            break
        if n - p in primes:
            return p, n - p
    return 0, 0


def main() -> None:
    ps = sieve(100)
    print(f"{len(ps)} primes below 100:")
    for i in range(0, len(ps), 10):
        print("  " + " ".join(f"{p:>3}" for p in ps[i:i + 10]))
    big = sieve(10000)
    print("count below 10000:", len(big), "last:", big[-1])
    print("agree with trial division:", all(is_prime_trial(k) == (k in set(big)) for k in range(2000)))
    for n in [1, 2, 360, 97, 1001, 65536, 999999, 600851475143]:
        print(f"{n} = {fmt_factors(factorize(n))}")
    pset = set(big)
    for n in range(4, 41, 6):
        a, b = goldbach(n, pset)
        print(f"{n} = {a} + {b}")
    twins = [(p, p + 2) for p in ps if p + 2 in set(ps)]
    print("twin primes:", twins)


main()
