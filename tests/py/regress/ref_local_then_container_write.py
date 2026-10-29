# regression: BUG (Rust compile error): a local read from a dict of frozen dataclasses becomes a
from dataclasses import dataclass


@dataclass(frozen=True)
class Task:
    priority: int
    name: str


class Slots:
    def __init__(self) -> None:
        self.slots: dict[int, Task] = {0: Task(1, "a"), 1: Task(2, "b")}

    def swap(self, i: int, j: int) -> None:
        t = self.slots[i]
        self.slots[i] = self.slots[j]
        self.slots[j] = t


def main() -> None:
    s = Slots()
    s.swap(0, 1)
    print(s.slots)


main()
