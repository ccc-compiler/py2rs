import math


class Fraction:
    def __init__(self, num: int, den: int) -> None:
        if den == 0:
            raise ZeroDivisionError("fraction with zero denominator")
        if den < 0:
            num = -num
            den = -den
        g = math.gcd(num, den)
        self.num = num // g
        self.den = den // g

    def add(self, o: "Fraction") -> "Fraction":
        return Fraction(self.num * o.den + o.num * self.den, self.den * o.den)

    def sub(self, o: "Fraction") -> "Fraction":
        return Fraction(self.num * o.den - o.num * self.den, self.den * o.den)

    def mul(self, o: "Fraction") -> "Fraction":
        return Fraction(self.num * o.num, self.den * o.den)

    def div(self, o: "Fraction") -> "Fraction":
        return Fraction(self.num * o.den, self.den * o.num)

    def to_float(self) -> float:
        return self.num / self.den

    def __str__(self) -> str:
        if self.den == 1:
            return str(self.num)
        return f"{self.num}/{self.den}"


def parse_fraction(s: str) -> Fraction:
    s = s.strip()
    if "/" in s:
        i = s.find("/")
        return Fraction(int(s[:i]), int(s[i + 1:]))
    return Fraction(int(s), 1)


def mixed(f: Fraction) -> str:
    whole = abs(f.num) // f.den
    rest = abs(f.num) % f.den
    sign = "-" if f.num < 0 else ""
    if whole == 0:
        return str(f)
    if rest == 0:
        return f"{sign}{whole}"
    return f"{sign}{whole} {rest}/{f.den}"


def main() -> None:
    a = Fraction(1, 3)
    b = Fraction(-3, 4)
    print(a, b, a.add(b), a.sub(b), a.mul(b), a.div(b), b.to_float())
    h = Fraction(0, 1)
    for k in range(1, 11):
        h = h.add(Fraction(1, k))
        print(f"H({k:2}) = {str(h):>12} = {h.to_float():.6f} {mixed(h)}")
    for s in ["6/8", "-10/4", "7", " 3 / 9", "1/0", "x/2", "22/-7"]:
        try:
            f = parse_fraction(s)
            print(f"{s!r:>8} -> {f} ({mixed(f)}) {f.to_float()}")
        except ZeroDivisionError as e:
            print(f"{s!r:>8} -> zero: {e}")
        except ValueError as e:
            print(f"{s!r:>8} -> value: {e}")


main()
