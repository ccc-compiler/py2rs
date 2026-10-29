from dataclasses import dataclass


@dataclass(frozen=True)
class Poly:
    coeffs: list[int]

    @staticmethod
    def make(cs: list[int]) -> "Poly":
        out = cs.copy()
        while len(out) > 1 and out[-1] == 0:
            out.pop()
        return Poly(out)

    def degree(self) -> int:
        return len(self.coeffs) - 1

    def at(self, x: int) -> int:
        acc = 0
        for c in reversed(self.coeffs):
            acc = acc * x + c
        return acc

    def plus(self, o: "Poly") -> "Poly":
        n = max(len(self.coeffs), len(o.coeffs))
        cs = [0] * n
        for i, c in enumerate(self.coeffs):
            cs[i] += c
        for i, c in enumerate(o.coeffs):
            cs[i] += c
        return Poly.make(cs)

    def times(self, o: "Poly") -> "Poly":
        cs = [0] * (len(self.coeffs) + len(o.coeffs) - 1)
        for i, a in enumerate(self.coeffs):
            for j, b in enumerate(o.coeffs):
                cs[i + j] += a * b
        return Poly.make(cs)

    def derivative(self) -> "Poly":
        if len(self.coeffs) == 1:
            return Poly([0])
        return Poly.make([i * c for i, c in enumerate(self.coeffs)][1:])

    def __str__(self) -> str:
        terms: list[str] = []
        for i in range(len(self.coeffs) - 1, -1, -1):
            c = self.coeffs[i]
            if c == 0:
                continue
            sign = "-" if c < 0 else "+"
            a = abs(c)
            if i == 0:
                body = str(a)
            elif i == 1:
                body = "x" if a == 1 else f"{a}x"
            else:
                body = f"x^{i}" if a == 1 else f"{a}x^{i}"
            terms.append(f"{sign} {body}")
        if not terms:
            return "0"
        s = " ".join(terms)
        if s.startswith("+ "):
            return s[2:]
        return "-" + s[2:]


def roots_in(p: Poly, lo: int, hi: int) -> list[int]:
    return [x for x in range(lo, hi + 1) if p.at(x) == 0]


def main() -> None:
    p = Poly.make([-6, 11, -6, 1])
    q = Poly.make([1, 1])
    print("p =", p, "degree", p.degree())
    print("q =", q)
    print("p + q =", p.plus(q))
    print("p * q =", p.times(q))
    print("p' =", p.derivative(), " p'' =", p.derivative().derivative())
    print([p.at(x) for x in range(-1, 5)])
    print("roots of p:", roots_in(p, -10, 10))
    r = Poly.make([1])
    for _ in range(5):
        r = r.times(q)
        print(f"(x+1)^{r.degree()} = {r}")
    print(Poly.make([0, 0, 0]), Poly.make([3, 0, 0]), Poly.make([0, -1]))
    print(p == Poly.make([-6, 11, -6, 1, 0]), p == q)
    z = p.plus(Poly.make([6, -11, 6, -1]))
    print("p - p =", z, z.degree())


main()
