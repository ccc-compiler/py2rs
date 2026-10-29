class Fraction:
    def __init__(self, num: int, den: int) -> None:
        if den == 0:
            raise ZeroDivisionError("zero denominator")
        g = Fraction.gcd(abs(num), abs(den))
        if den < 0:
            num = -num
            den = -den
        self.num = num // g
        self.den = den // g

    @staticmethod
    def gcd(a: int, b: int) -> int:
        while b:
            a, b = b, a % b
        return a if a > 0 else 1

    @staticmethod
    def parse(s: str) -> "Fraction":
        if "/" in s:
            parts = s.split("/")
            return Fraction(int(parts[0]), int(parts[1]))
        return Fraction(int(s), 1)

    def plus(self, o: "Fraction") -> "Fraction":
        return Fraction(self.num * o.den + o.num * self.den, self.den * o.den)

    def minus(self, o: "Fraction") -> "Fraction":
        return Fraction(self.num * o.den - o.num * self.den, self.den * o.den)

    def times(self, o: "Fraction") -> "Fraction":
        return Fraction(self.num * o.num, self.den * o.den)

    def divided(self, o: "Fraction") -> "Fraction":
        return Fraction(self.num * o.den, self.den * o.num)

    def same(self, o: "Fraction") -> bool:
        return self.num == o.num and self.den == o.den

    def smaller(self, o: "Fraction") -> bool:
        return self.num * o.den < o.num * self.den

    def value(self) -> float:
        return self.num / self.den

    def __str__(self) -> str:
        if self.den == 1:
            return str(self.num)
        return f"{self.num}/{self.den}"

    def __repr__(self) -> str:
        return f"Fraction({self.num}, {self.den})"


def harmonic(n: int) -> Fraction:
    total = Fraction(0, 1)
    for k in range(1, n + 1):
        total = total.plus(Fraction(1, k))
    return total


def egyptian(f: Fraction) -> list[Fraction]:
    out: list[Fraction] = []
    cur = Fraction(f.num, f.den)
    while cur.num != 0:
        d = (cur.den + cur.num - 1) // cur.num
        unit = Fraction(1, d)
        out.append(unit)
        cur = cur.minus(unit)
    return out


def main() -> None:
    a = Fraction(6, -8)
    b = Fraction.parse("5/12")
    print(a, b, repr(a))
    print(a.plus(b), a.minus(b), a.times(b), a.divided(b))
    print(a.same(Fraction(-3, 4)), a.smaller(b), b.smaller(a))
    for n in [1, 2, 5, 10]:
        h = harmonic(n)
        print(f"H({n}) = {h} ~ {h.value():.6f}")
    for s in ["4/13", "5/6", "7/15", "3"]:
        parts = egyptian(Fraction.parse(s))
        print(s, "=", " + ".join(str(p) for p in parts))
    xs = [Fraction.parse(s) for s in ["1/3", "-2/5", "7/8", "1/2", "0"]]
    xs.sort(key=lambda f: f.value())
    print(xs)
    try:
        Fraction(1, 0)
    except ZeroDivisionError as err:
        print("error:", err)
    try:
        a.divided(Fraction(0, 5))
    except ZeroDivisionError as err:
        print("error:", err)


main()
