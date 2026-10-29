"""An extension module: `py2rs pyo3 tests/pyo3/fastmath.py`."""


def fib(n: int) -> int:
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a


def mean(xs: list[float]) -> float:
    if not xs:
        raise ValueError("mean of an empty list")
    return sum(xs) / len(xs)


def word_counts(text: str) -> dict[str, int]:
    counts: dict[str, int] = {}
    for w in text.split():
        counts[w] = counts.get(w, 0) + 1
    return counts


def greet(name: str, times: int = 1) -> str:
    print("greeting", name)
    return " ".join(["hello " + name] * times)


def _helper() -> int:
    return 1
