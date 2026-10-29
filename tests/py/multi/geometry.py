"""Shapes used by app.py."""
import math
from dataclasses import dataclass

UNIT = "cm"


@dataclass
class Point:
    x: float
    y: float


def dist(a: Point, b: Point) -> float:
    return math.hypot(a.x - b.x, a.y - b.y)


def perimeter(ps: list[Point]) -> float:
    total = 0.0
    for i in range(len(ps)):
        total += dist(ps[i], ps[(i + 1) % len(ps)])
    return total


if __name__ == "__main__":
    print("geometry self-test", dist(Point(0, 0), Point(3, 4)))
