from dataclasses import dataclass


@dataclass
class Vec2:
    x: float
    y: float

    def __add__(self, other: "Vec2") -> "Vec2":
        return Vec2(self.x + other.x, self.y + other.y)

    def __sub__(self, other: "Vec2") -> "Vec2":
        return Vec2(self.x - other.x, self.y - other.y)

    def __mul__(self, k: float) -> "Vec2":
        return Vec2(self.x * k, self.y * k)

    def __rmul__(self, k: float) -> "Vec2":
        return Vec2(self.x * k, self.y * k)

    def __neg__(self) -> "Vec2":
        return Vec2(-self.x, -self.y)

    def __abs__(self) -> float:
        return (self.x * self.x + self.y * self.y) ** 0.5


class Money:
    def __init__(self, cents: int) -> None:
        self.cents = cents

    def __eq__(self, other: "Money") -> bool:
        return self.cents == other.cents

    def __lt__(self, other: "Money") -> bool:
        return self.cents < other.cents

    def __str__(self) -> str:
        return f"${self.cents // 100}.{self.cents % 100:02d}"


class Bag:
    def __init__(self) -> None:
        self.items: list[str] = []

    def add(self, x: str) -> None:
        self.items.append(x)

    def __len__(self) -> int:
        return len(self.items)

    def __contains__(self, x: str) -> bool:
        return x in self.items

    def __getitem__(self, i: int) -> str:
        return self.items[i]

    def __setitem__(self, i: int, v: str) -> None:
        self.items[i] = v


def main() -> None:
    a = Vec2(1.0, 2.0)
    b = Vec2(3.0, -1.0)
    print(a + b, a - b, a * 2.0, 3.0 * b, -a, abs(Vec2(3.0, 4.0)))
    c = a
    c += b
    print(c, a)
    m1 = Money(250)
    m2 = Money(199)
    print(m1 == Money(250), m1 != m2, m1 < m2, m1 > m2)
    prices = [Money(500), m1, m2, Money(1)]
    print([str(p) for p in sorted(prices)])
    print(str(min(prices)), str(max(prices)))
    print(Money(199) in prices, prices.index(Money(500)))
    bag = Bag()
    print(len(bag), bool(bag))
    if not bag:
        print("empty")
    bag.add("apple")
    bag.add("pear")
    bag[1] = "plum"
    print(len(bag), "plum" in bag, "pear" not in bag, bag[0], bag[-1])
    if bag:
        print("not empty")


if __name__ == "__main__":
    main()
