from typing import Iterator, Generator


def count_up(n: int) -> Iterator[int]:
    i = 0
    while i < n:
        print("  producing", i)
        yield i
        i += 1
    print("  done")


def fib() -> Iterator[int]:
    a, b = 0, 1
    while True:
        yield a
        a, b = b, a + b


def words(text: str) -> Generator[str, None, None]:
    for w in text.split():
        if w.startswith("#"):
            return
        yield w.upper()


def chained(xs: list[int], ys: list[int]) -> Iterator[int]:
    yield from xs
    yield -1
    yield from ys


def evens(src: Iterator[int]) -> Iterator[int]:
    for x in src:
        if x % 2 == 0:
            yield x


def main() -> None:
    for x in count_up(3):
        print("got", x)
    f = fib()
    print([next(f) for _ in range(10)])
    print(next(f), next(f))
    print(list(words("hello big world # ignored")))
    print(sum(count_up(4)))
    print(list(chained([1, 2], [3])))
    print(list(evens(count_up(5))))
    g = count_up(2)
    print(next(g))
    for x in g:
        print("rest", x)
    print(next(g, -99))
    big = fib()
    for v in big:
        if v > 1000:
            print("first above 1000:", v)
            break


if __name__ == "__main__":
    main()
