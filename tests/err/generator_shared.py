# error: cannot take or yield @shared objects
from typing import Iterator
from rust import shared


@shared
class Node:
    def __init__(self, v: int) -> None:
        self.v = v


def values(ns: list[Node]) -> Iterator[int]:
    for n in ns:
        yield n.v
