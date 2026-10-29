def with_commas(n: int) -> str:
    s = str(abs(n))
    groups: list[str] = []
    while len(s) > 3:
        groups.append(s[-3:])
        s = s[:-3]
    groups.append(s)
    groups.reverse()
    return ("-" if n < 0 else "") + ",".join(groups)


def to_base(n: int, base: int) -> str:
    digits = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ"
    if n == 0:
        return "0"
    neg = n < 0
    n = abs(n)
    out: list[str] = []
    while n > 0:
        out.append(digits[n % base])
        n //= base
    if neg:
        out.append("-")
    return "".join(reversed(out))


def from_base(s: str, base: int) -> int:
    value = 0
    for ch in s.upper():
        d = ord(ch) - ord("0") if ch.isdigit() else ord(ch) - ord("A") + 10
        value = value * base + d
    return value


def to_roman(n: int) -> str:
    table = [(1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"),
             (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")]
    out = ""
    for v, sym in table:
        while n >= v:
            out += sym
            n -= v
    return out


def from_roman(s: str) -> int:
    vals = {"I": 1, "V": 5, "X": 10, "L": 50, "C": 100, "D": 500, "M": 1000}
    total = 0
    for i in range(len(s)):
        v = vals[s[i]]
        if i + 1 < len(s) and vals[s[i + 1]] > v:
            total -= v
        else:
            total += v
    return total


def human_bytes(n: int) -> str:
    units = ["B", "KB", "MB", "GB", "TB"]
    x = float(n)
    i = 0
    while x >= 1024 and i < len(units) - 1:
        x /= 1024
        i += 1
    return f"{x:.1f} {units[i]}" if i > 0 else f"{n} B"


def main() -> None:
    for n in [0, 7, 1234, -98765, 1000000, 9223372036854775807]:
        print(f"{n:>22} -> {with_commas(n):>27}  builtin {n:,}")
    for n in [0, 5, 255, 1000, -42, 65535]:
        b2 = to_base(n, 2)
        b16 = to_base(n, 16)
        b36 = to_base(n, 36)
        print(f"{n}: bin={b2} hex={b16} b36={b36} ({n:b} {n:x} {n:o})")
        if n >= 0:
            print(f"   roundtrip: {from_base(b2, 2)} {from_base(b16, 16)} {from_base(b36, 36)}")
    for n in [1, 4, 9, 14, 40, 90, 400, 1994, 2024, 3999]:
        r = to_roman(n)
        print(f"{n:>4} = {r:<12} back {from_roman(r)}")
    for n in [500, 2048, 1536000, 5 * 1024 ** 3, 3 * 1024 ** 4 + 7]:
        print(f"{n} bytes = {human_bytes(n)}")
    print(f"{3.14159:8.3f}|{-2.5:+.2f}|{42:05d}|{0.000123:.2e}|{1234567.891:,.2f}|{0.25:.1%}")
    print(f"{'left':<8}|{'right':>8}|{'mid':^8}|{7:*^9}")


main()
