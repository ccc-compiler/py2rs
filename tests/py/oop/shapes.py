import math


class Shape:
    def __init__(self, name: str) -> None:
        self.name = name

    def area(self) -> float:
        return 0.0

    def perimeter(self) -> float:
        return 0.0

    def describe(self) -> str:
        return f"{self.name}: area={self.area():.3f} perimeter={self.perimeter():.3f}"

    def __str__(self) -> str:
        return f"Shape({self.name})"


class Circle(Shape):
    def __init__(self, r: float) -> None:
        super().__init__("circle")
        self.r = r

    def area(self) -> float:
        return math.pi * self.r * self.r

    def perimeter(self) -> float:
        return 2.0 * math.pi * self.r

    def __str__(self) -> str:
        return f"Circle(r={self.r})"


class Rectangle(Shape):
    def __init__(self, w: float, h: float) -> None:
        super().__init__("rectangle")
        self.w = w
        self.h = h

    def area(self) -> float:
        return self.w * self.h

    def perimeter(self) -> float:
        return 2.0 * (self.w + self.h)

    def __str__(self) -> str:
        return f"Rectangle({self.w}x{self.h})"


class Square(Rectangle):
    def __init__(self, side: float) -> None:
        super().__init__(side, side)
        self.name = "square"

    def __str__(self) -> str:
        return f"Square({self.w})"


class Triangle(Shape):
    def __init__(self, a: float, b: float, c: float) -> None:
        super().__init__("triangle")
        self.a = a
        self.b = b
        self.c = c

    def perimeter(self) -> float:
        return self.a + self.b + self.c

    def area(self) -> float:
        s = self.perimeter() / 2.0
        return math.sqrt(s * (s - self.a) * (s - self.b) * (s - self.c))

    def is_right(self) -> bool:
        sides = sorted([self.a, self.b, self.c])
        return abs(sides[0] ** 2 + sides[1] ** 2 - sides[2] ** 2) < 1e-9


def largest(shapes: list[Shape]) -> Shape:
    best = shapes[0]
    for s in shapes[1:]:
        if s.area() > best.area():
            best = s
    return best


def main() -> None:
    shapes: list[Shape] = [
        Circle(1.5),
        Rectangle(2.0, 4.5),
        Square(3.0),
        Triangle(3.0, 4.0, 5.0),
        Triangle(2.0, 2.0, 3.0),
        Circle(0.5),
    ]
    for s in shapes:
        print(str(s), "->", s.describe())
    total = sum(s.area() for s in shapes)
    print(f"total area {total:.4f}")
    print("largest:", largest(shapes))
    by_area = sorted(shapes, key=lambda s: s.area())
    print([s.name for s in by_area])
    counts: dict[str, int] = {}
    for s in shapes:
        counts[s.name] = counts.get(s.name, 0) + 1
    for k, v in counts.items():
        print(f"{k:>10}: {v}")
    for s in shapes:
        if isinstance(s, Triangle):
            print("triangle", s.a, s.b, s.c, "right" if s.is_right() else "not right")
        elif isinstance(s, Square):
            print("square side", s.w)
        elif isinstance(s, Rectangle):
            print("rect", s.w, s.h)
        elif isinstance(s, Circle):
            print("circle diameter", 2 * s.r)
    print(isinstance(shapes[2], Rectangle), isinstance(shapes[0], Rectangle))


main()
