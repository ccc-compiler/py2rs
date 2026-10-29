def gcd(a: int, b: int) -> int:
    while b != 0:
        a, b = b, a % b
    return abs(a)


def gcd_rec(a: int, b: int) -> int:
    if b == 0:
        return a
    return gcd_rec(b, a % b)


def ext_gcd(a: int, b: int) -> tuple[int, int, int]:
    if b == 0:
        return a, 1, 0
    g, x, y = ext_gcd(b, a % b)
    return g, y, x - (a // b) * y


def lcm(a: int, b: int) -> int:
    return a // gcd(a, b) * b


def mod_inverse(a: int, m: int) -> int:
    g, x, _ = ext_gcd(a, m)
    if g != 1:
        return -1
    return x % m


def pow_mod(base: int, exp: int, mod: int) -> int:
    result = 1
    base %= mod
    while exp > 0:
        if exp & 1:
            result = result * base % mod
        base = base * base % mod
        exp >>= 1
    return result


def reduce_fraction(n: int, d: int) -> str:
    g = gcd(n, d)
    n //= g
    d //= g
    if d < 0:
        n = -n
        d = -d
    return f"{n}/{d}" if d != 1 else str(n)


def main() -> None:
    pairs = [(48, 18), (17, 5), (0, 9), (270, 192), (-12, 8), (1071, 462)]
    for p in pairs:
        a, b = p
        g, x, y = ext_gcd(a, b)
        print(f"gcd({a}, {b}) = {gcd(a, b)} rec={gcd_rec(a, b)} ext=({g}, {x}, {y}) check={a * x + b * y}")
    print("lcm 1..20:", end=" ")
    acc = 1
    for k in range(1, 21):
        acc = lcm(acc, k)
    print(acc)
    for a, m in [(3, 11), (10, 17), (6, 9), (7, 26)]:
        inv = mod_inverse(a, m)
        print(f"inverse of {a} mod {m}: {inv}" + (f" (check {a * inv % m})" if inv >= 0 else ""))
    for b, e, m in [(2, 10, 1000), (3, 200, 13), (7, 123456, 1000000007), (5, 0, 7)]:
        print(f"{b}^{e} mod {m} = {pow_mod(b, e, m)} (builtin {pow(b, e, m)})")
    for n, d in [(6, 8), (10, 5), (3, -9), (-14, 21), (0, 3)]:
        print(f"{n}/{d} -> {reduce_fraction(n, d)}")
    print("-7 // 2 =", -7 // 2, " -7 % 2 =", -7 % 2, " 7 % -3 =", 7 % -3)


main()
