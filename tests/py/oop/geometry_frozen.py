import math
from dataclasses import dataclass


@dataclass(frozen=True)
class Vec2:
    x: float
    y: float

    def plus(self, o: "Vec2") -> "Vec2":
        return Vec2(self.x + o.x, self.y + o.y)

    def minus(self, o: "Vec2") -> "Vec2":
        return Vec2(self.x - o.x, self.y - o.y)

    def scale(self, k: float) -> "Vec2":
        return Vec2(self.x * k, self.y * k)

    def dot(self, o: "Vec2") -> float:
        return self.x * o.x + self.y * o.y

    def cross(self, o: "Vec2") -> float:
        return self.x * o.y - self.y * o.x

    def length(self) -> float:
        return math.sqrt(self.dot(self))

    def rotate(self, deg: float) -> "Vec2":
        r = deg * math.pi / 180.0
        c = math.cos(r)
        s = math.sin(r)
        return Vec2(round(self.x * c - self.y * s, 9), round(self.x * s + self.y * c, 9))


@dataclass(frozen=True)
class Segment:
    a: Vec2
    b: Vec2

    def length(self) -> float:
        return self.b.minus(self.a).length()

    def midpoint(self) -> Vec2:
        return self.a.plus(self.b).scale(0.5)

    def intersects(self, o: "Segment") -> bool:
        d1 = self.b.minus(self.a).cross(o.a.minus(self.a))
        d2 = self.b.minus(self.a).cross(o.b.minus(self.a))
        d3 = o.b.minus(o.a).cross(self.a.minus(o.a))
        d4 = o.b.minus(o.a).cross(self.b.minus(o.a))
        return d1 * d2 < 0 and d3 * d4 < 0


@dataclass(frozen=True)
class Polygon:
    points: list[Vec2]

    def area(self) -> float:
        n = len(self.points)
        s = 0.0
        for i in range(n):
            s += self.points[i].cross(self.points[(i + 1) % n])
        return abs(s) / 2.0

    def perimeter(self) -> float:
        n = len(self.points)
        return sum(Segment(self.points[i], self.points[(i + 1) % n]).length() for i in range(n))

    def centroid(self) -> Vec2:
        sx = sum(p.x for p in self.points)
        sy = sum(p.y for p in self.points)
        return Vec2(sx / len(self.points), sy / len(self.points))


def report(name: str, p: Polygon) -> None:
    print(f"{name}: area={p.area():.2f} perimeter={p.perimeter():.2f} centroid={p.centroid()}")


def main() -> None:
    a = Vec2(3.0, 4.0)
    b = Vec2(-1.0, 2.0)
    print(a, b, a.plus(b), a.minus(b), a.scale(2.0))
    print(a.dot(b), a.cross(b), a.length(), a == Vec2(3.0, 4.0))
    print(a.rotate(90.0), a.rotate(180.0), Vec2(1.0, 0.0).rotate(45.0))
    s1 = Segment(Vec2(0.0, 0.0), Vec2(4.0, 4.0))
    s2 = Segment(Vec2(0.0, 4.0), Vec2(4.0, 0.0))
    s3 = Segment(Vec2(5.0, 5.0), Vec2(6.0, 9.0))
    print(s1.length(), s1.midpoint())
    print(s1.intersects(s2), s1.intersects(s3), s2.intersects(s3))
    square = Polygon([Vec2(0.0, 0.0), Vec2(2.0, 0.0), Vec2(2.0, 2.0), Vec2(0.0, 2.0)])
    tri = Polygon([Vec2(0.0, 0.0), Vec2(4.0, 0.0), Vec2(0.0, 3.0)])
    report("square", square)
    report("triangle", tri)
    pts = {(1, 1), (1, 1), (2, 1)}
    print(len(pts))
    path = [Vec2(0.0, 0.0)]
    step = Vec2(1.0, 0.0)
    for i in range(8):
        path.append(path[-1].plus(step))
        step = step.rotate(45.0).scale(1.1)
    print([f"({p.x:.2f},{p.y:.2f})" for p in path])
    print(f"walked {sum(Segment(path[i], path[i + 1]).length() for i in range(len(path) - 1)):.3f}")


main()
