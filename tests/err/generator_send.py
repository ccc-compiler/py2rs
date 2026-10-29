# error: send() and return values are not supported
from typing import Generator


def g() -> Generator[int, int, None]:
    yield 1
