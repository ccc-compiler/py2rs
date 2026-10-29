from dataclasses import dataclass
from typing import Protocol, TypeVar, Union

T = TypeVar("T")


def first(xs: list[T]) -> T:
    return xs[0]


def last[U](xs: list[U]) -> U:
    return xs[-1]


def swap[A, B](p: tuple[A, B]) -> tuple[B, A]:
    return (p[1], p[0])


def count_in[K](d: dict[K, int], keys: list[K]) -> int:
    n = 0
    for k in keys:
        if k in d:
            n += d[k]
    return n


def describe(x: int | str) -> str:
    if isinstance(x, int):
        return f"number {x * 2}"
    else:
        return f"text {x.upper()}"


def size(x: Union[list[int], str]) -> int:
    if isinstance(x, str):
        return len(x) * 10
    return len(x)


class HasArea(Protocol):
    def area(self) -> float: ...


@dataclass
class Square:
    side: float

    def area(self) -> float:
        return self.side * self.side


@dataclass
class Circle:
    r: float

    def area(self) -> float:
        return 3.14 * self.r * self.r


def report(shape: HasArea) -> str:
    return f"area {shape.area():.2f}"


def biggest(a: T, b: T) -> T:
    return a if a > b else b


def main() -> None:
    print(first([3, 4]), first(["a", "b"]), last([1.5, 2.5]))
    print(swap((1, "x")), swap(("y", 2.0)))
    print(count_in({"a": 2, "b": 3}, ["a", "c", "b"]), count_in({1: 5}, [1, 1]))
    print(describe(21), describe("hi"))
    print(size([1, 2, 3]), size("ab"))
    print(report(Square(2.0)), report(Circle(1.0)))
    print(biggest(3, 7), biggest("pear", "apple"))


if __name__ == "__main__":
    main()
