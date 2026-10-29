def add(a: str, b: str) -> str:
    i = len(a) - 1
    j = len(b) - 1
    carry = 0
    out: list[str] = []
    while i >= 0 or j >= 0 or carry > 0:
        s = carry
        if i >= 0:
            s += ord(a[i]) - 48
        if j >= 0:
            s += ord(b[j]) - 48
        out.append(chr(48 + s % 10))
        carry = s // 10
        i -= 1
        j -= 1
    return "".join(reversed(out))


def mul_small(a: str, k: int) -> str:
    if k == 0:
        return "0"
    carry = 0
    out: list[str] = []
    for c in reversed(a):
        p = int(c) * k + carry
        out.append(str(p % 10))
        carry = p // 10
    while carry:
        out.append(str(carry % 10))
        carry //= 10
    return "".join(reversed(out))


def mul(a: str, b: str) -> str:
    result = "0"
    shift = ""
    for c in reversed(b):
        result = add(result, mul_small(a, int(c)) + shift)
        shift += "0"
    r = result.lstrip("0")
    return r if r != "" else "0"


def factorial(n: int) -> str:
    r = "1"
    for k in range(2, n + 1):
        r = mul_small(r, k)
    return r


def group(s: str) -> str:
    parts: list[str] = []
    while len(s) > 3:
        parts.append(s[-3:])
        s = s[:-3]
    parts.append(s)
    return ",".join(reversed(parts))


def main() -> None:
    print(add("999", "1"), add("0", "0"), add("123456789123456789", "987654321987654321"))
    print(mul("12345", "6789"), 12345 * 6789, mul("0", "55"))
    for n in [0, 1, 5, 10, 20, 25, 30, 50]:
        f = factorial(n)
        print(f"{n:>3}! = {group(f)} ({len(f)} digits, digit sum {sum(int(c) for c in f)})")
    fib_a = "0"
    fib_b = "1"
    for i in range(150):
        fib_a, fib_b = fib_b, add(fib_a, fib_b)
    print("fib(150) =", fib_a)
    p = "1"
    for i in range(100):
        p = mul_small(p, 2)
    print("2**100 =", p, p.count("0"), p[:10], p[-5:])
    print(group(str(2 ** 62)) == f"{2 ** 62:,}")


main()
