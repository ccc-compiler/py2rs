DIGITS = "0123456789abcdefghijklmnopqrstuvwxyz"


def to_base(n: int, b: int) -> str:
    if b < 2 or b > 36:
        raise ValueError(f"base {b} out of range")
    if n == 0:
        return "0"
    neg = n < 0
    if neg:
        n = -n
    out: list[str] = []
    while n > 0:
        out.append(DIGITS[n % b])
        n //= b
    s = "".join(reversed(out))
    return "-" + s if neg else s


def from_base(s: str, b: int) -> int:
    s = s.strip().lower()
    neg = s.startswith("-")
    if neg:
        s = s[1:]
    if s == "":
        raise ValueError("empty")
    v = 0
    for c in s:
        d = DIGITS.find(c)
        if d < 0 or d >= b:
            raise ValueError(f"digit {c!r} not valid in base {b}")
        v = v * b + d
    return -v if neg else v


def main() -> None:
    for n in [0, 1, 255, -255, 1000, 65535, 2 ** 32, 123456789, -1, 9223372036854775807]:
        print(n, to_base(n, 2)[-20:], to_base(n, 8), to_base(n, 16), to_base(n, 36))
        assert from_base(to_base(n, 7), 7) == n
        print(f"  {n:x} {n:o} {n:b}"[:60])
    for s, b in [("ff", 16), ("777", 8), ("1010", 2), ("zz", 36), ("12", 2), ("", 10), ("-7f", 16)]:
        try:
            v = from_base(s, b)
            print(f"{s!r} base {b} = {v}")
        except ValueError as e:
            print("error:", e)
    try:
        to_base(5, 40)
    except ValueError as e:
        print("error:", e)
    print(f"{10:#b}", f"{64:#o}", f"{-255:#x}", int("0012"), int("-0"))


main()
