from typing import Callable


def apply_twice(f: Callable[[int], int], x: int) -> int:
    return f(f(x))


def compose(xs: list[int], f: Callable[[int], int], g: Callable[[int], int]) -> list[int]:
    return [g(f(x)) for x in xs]


def count_if(words: list[str], pred: Callable[[str], bool]) -> int:
    n = 0
    for w in words:
        if pred(w):
            n += 1
    return n


def square(n: int) -> int:
    return n * n


def is_long(w: str) -> bool:
    return len(w) > 3


def total(xs: list[float], weight: Callable[[float], float]) -> float:
    return sum(weight(x) for x in xs)


def forward(f: Callable[[int], int], x: int) -> int:
    return apply_twice(f, x) + 1


def main() -> None:
    print(apply_twice(square, 3))
    k = 10
    print(apply_twice(lambda x: x + k, 1))
    print(compose([1, 2, 3], square, lambda v: v - 1))
    words = ["tree", "a", "house", "sky"]
    print(count_if(words, is_long), count_if(words, lambda w: w.startswith("s")))
    print(total([1.0, 2.5], lambda x: x * 2.0))
    inc: Callable[[int], int] = lambda x: x + 1
    print(inc(41), apply_twice(inc, 0), forward(inc, 5))


if __name__ == "__main__":
    main()
