import math
from dataclasses import dataclass, field


@dataclass(frozen=True)
class Pos:
    x: int
    y: int


@dataclass
class Bag:
    name: str
    items: list[str] = field(default_factory=list)
    tags: dict[str, int] = field(default_factory=dict)


def classify(tok: str) -> str:
    if tok in ("+", "-"):
        return "additive"
    if tok in ("*", "/"):
        return "multiplicative"
    return "other"


def parse_pair(s: str) -> tuple[int, int]:
    a, b = s.split("/")
    return int(a), int(b)


def main() -> None:
    print([classify(t) for t in ["+", "*", "x", "-"]])
    print(parse_pair("3/4"))
    try:
        print(parse_pair("1/2/3"))
    except ValueError as e:
        print("error:", e)
    words = ["pear", "Fig", "banana", "apple"]
    print(sorted(words, key=len), sorted(words, key=str.lower), max(words, key=len))
    print(max([3, -7, 5], key=abs), min([], default=0), max([2, 9], default=-1))
    b1 = Bag("a")
    b2 = Bag("b")
    b1.items.append("x")
    print(b1, b2)
    seen: set[Pos] = set()
    for p in [Pos(1, 2), Pos(3, 4), Pos(1, 2)]:
        seen.add(p)
    print(len(seen), Pos(1, 2) in seen)
    where: dict[Pos, str] = {Pos(0, 0): "origin"}
    print(where[Pos(0, 0)])
    x = 7
    print(0 <= x * 2 <= 20, 1 < len(words) < 3)
    try:
        raise KeyError(42)
    except KeyError as e:
        print("key", e)
    print(int("ff", 16), int("0b101", 0), bin(10), hex(-255), oct(8))
    print("a,b,c,d".split(",", 2), "a b  c".split(maxsplit=1), "a/b/c".rsplit("/", 1))
    print("aaaa".replace("a", "b", 2), "hello".find("l", 3), "banana".count("a", 2))
    print("key=value=x".partition("="), "a.b.c".rpartition("."), "x.py".removesuffix(".py"))
    print("7".rjust(3, "0"), "ab".center(6, "*"), "q".ljust(3, "."), "héllo".isascii())
    print(math.radians(180), math.degrees(math.pi / 2), math.isclose(0.1 + 0.2, 0.3))


main()
