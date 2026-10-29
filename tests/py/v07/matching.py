from dataclasses import dataclass


@dataclass
class Shape:
    name: str


@dataclass
class Circle(Shape):
    r: float


@dataclass
class Rect(Shape):
    w: float
    h: float


def area(s: Shape) -> float:
    match s:
        case Circle(r=r):
            return 3.0 * r * r
        case Rect(w=w, h=h) if w == h:
            return w * w
        case Rect(w=w, h=h):
            return w * h
        case _:
            return 0.0


def http(code: int) -> str:
    match code:
        case 200 | 201:
            return "ok"
        case 404:
            return "not found"
        case 500:
            return "server error"
        case other:
            return f"code {other}"


def command(words: list[str]) -> str:
    match words:
        case []:
            return "nothing"
        case ["go", direction]:
            return f"going {direction}"
        case ["take", item, *rest]:
            return f"take {item} and {len(rest)} more"
        case [single]:
            return f"just {single}"
        case [first, *_, last]:
            return f"{first}..{last}"
        case _:
            return "?"


def point(p: tuple[int, int]) -> str:
    match p:
        case (0, 0):
            return "origin"
        case (0, y):
            return f"y={y}"
        case (x, 0):
            return f"x={x}"
        case (x, y) if x == y:
            return "diagonal"
        case _:
            return "elsewhere"


def maybe(x: int | None) -> str:
    match x:
        case None:
            return "none"
        case n if n > 0:
            return "positive"
        case _:
            return "other"


def main() -> None:
    shapes: list[Shape] = [Circle("c", 1.0), Rect("sq", 2.0, 2.0), Rect("r", 2.0, 3.0), Shape("s")]
    print([area(s) for s in shapes])
    print([http(c) for c in [200, 201, 404, 500, 302]])
    for ws in [[], ["go", "north"], ["take", "lamp", "key", "map"], ["look"], ["a", "b", "c"]]:
        print(command(ws))
    print([point(p) for p in [(0, 0), (0, 5), (3, 0), (4, 4), (1, 2)]])
    print(maybe(None), maybe(3), maybe(-1))
    match len(shapes) * 2:
        case 8:
            print("eight")
        case _:
            print("not eight")


if __name__ == "__main__":
    main()
