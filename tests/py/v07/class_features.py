from dataclasses import dataclass
from typing import ClassVar
import math


class Circle:
    PI_APPROX = 3.14
    unit: ClassVar[str] = "cm"

    def __init__(self, r: float) -> None:
        self.r = r

    @property
    def area(self) -> float:
        return math.pi * self.r * self.r

    @property
    def label(self) -> str:
        return f"circle r={self.r}{self.unit}"

    @classmethod
    def unit_circle(cls) -> "Circle":
        return cls(1.0)

    @classmethod
    def from_diameter(cls, d: float) -> "Circle":
        return cls(d / 2)

    @staticmethod
    def describe() -> str:
        return "round " + Circle.unit


@dataclass
class Temperature:
    celsius: float
    FREEZING: ClassVar[float] = 0.0

    @property
    def fahrenheit(self) -> float:
        return self.celsius * 9 / 5 + 32

    @property
    def frozen(self) -> bool:
        return self.celsius <= Temperature.FREEZING

    @classmethod
    def from_f(cls, f: float) -> "Temperature":
        return cls((f - 32) * 5 / 9)


class Animal:
    sound = "..."
    legs = 4

    def __init__(self, name: str) -> None:
        self.name = name

    def speak(self) -> str:
        return f"{self.name} says {self.sound} on {self.legs} legs"


class Bird(Animal):
    sound = "tweet"
    legs = 2


def main() -> None:
    c = Circle.from_diameter(4.0)
    print(round(c.area, 3), c.label, Circle.unit_circle().r, Circle.describe())
    print(Circle.PI_APPROX, c.PI_APPROX, Circle.unit)
    t = Temperature.from_f(212.0)
    print(t, t.fahrenheit, t.frozen, Temperature(-3.0).frozen)
    print(Animal("cat").speak(), Bird("robin").speak())


if __name__ == "__main__":
    main()
