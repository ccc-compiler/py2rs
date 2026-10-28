import math
from dataclasses import dataclass


class Shape:
    def __init__(self, name: str) -> None:
        self.name = name

    def area(self) -> float:
        return 0.0

    def describe(self) -> str:
        return f"{self.name} with area {self.area():.2f}"


class Circle(Shape):
    def __init__(self, r: float) -> None:
        super().__init__("circle")
        self.r = r

    def area(self) -> float:
        return math.pi * self.r ** 2


class Rect(Shape):
    def __init__(self, w: float, h: float) -> None:
        super().__init__("rect")
        self.w = w
        self.h = h

    def area(self) -> float:
        return self.w * self.h


class Square(Rect):
    def __init__(self, side: float) -> None:
        super().__init__(side, side)
        self.name = "square"

    def describe(self) -> str:
        return "a square: " + super().describe()


class Animal:
    def __init__(self, name: str) -> None:
        self.name = name
        self.tricks: list[str] = []

    def sound(self) -> str:
        return "..."

    def speak(self) -> str:
        return f"{self.name} says {self.sound()}"

    def learn(self, trick: str) -> None:
        self.tricks.append(trick)


class Dog(Animal):
    def sound(self) -> str:
        return "woof"


class Cat(Animal):
    def sound(self) -> str:
        return "meow"


@dataclass
class Base:
    x: int


@dataclass
class Derived(Base):
    y: int = 0


def total_area(shapes: list[Shape]) -> float:
    return sum(s.area() for s in shapes)


def main() -> None:
    shapes: list[Shape] = [Circle(1.0), Rect(2.0, 3.0), Square(2.0), Shape("blob")]
    for s in shapes:
        print(s.describe())
    print(f"total {total_area(shapes):.3f}")
    big = [s.name for s in shapes if s.area() > 3.5]
    print(big)
    for s in shapes:
        if isinstance(s, Circle):
            print("radius", s.r)
        elif isinstance(s, Rect):
            print("rect", s.w, s.h, s.name)
    pets: list[Animal] = [Dog("rex"), Cat("tom"), Animal("generic")]
    pets[0].learn("sit")
    pets[0].learn("roll")
    for p in pets:
        print(p.speak(), len(p.tricks))
    print(pets[0].tricks, pets[0].name)
    pets[1].name = "felix"
    print(pets[1].speak())
    d = Dog("solo")
    print(d.speak(), isinstance(d, Animal))
    print(Derived(1, 2), Derived(5), Derived(1, 2) == Derived(1, 2))


main()
