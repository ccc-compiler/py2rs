from dataclasses import dataclass


@dataclass(frozen=True)
class Task:
    priority: int
    name: str


class MinHeap:
    def __init__(self) -> None:
        self.slots: dict[int, Task] = {}
        self.size = 0

    def less(self, i: int, j: int) -> bool:
        a = self.slots[i]
        b = self.slots[j]
        return (a.priority, a.name) < (b.priority, b.name)

    def swap(self, i: int, j: int) -> None:
        self.slots[i], self.slots[j] = self.slots[j], self.slots[i]

    def push(self, t: Task) -> None:
        self.slots[self.size] = t
        i = self.size
        self.size += 1
        while i > 0:
            parent = (i - 1) // 2
            if self.less(i, parent):
                self.swap(i, parent)
                i = parent
            else:
                break

    def pop(self) -> Task:
        if self.size == 0:
            raise IndexError("pop from empty heap")
        top = Task(self.slots[0].priority, self.slots[0].name)
        self.size -= 1
        last = self.slots.pop(self.size)
        if self.size > 0:
            self.slots[0] = last
            i = 0
            while True:
                left = 2 * i + 1
                right = left + 1
                best = i
                if left < self.size and self.less(left, best):
                    best = left
                if right < self.size and self.less(right, best):
                    best = right
                if best == i:
                    break
                self.swap(i, best)
                i = best
        return top

    def peek(self) -> Task | None:
        if self.size == 0:
            return None
        return self.slots[0]


class Scheduler:
    def __init__(self) -> None:
        self.heap = MinHeap()
        self.clock = 0
        self.done: list[str] = []

    def submit(self, name: str, priority: int) -> None:
        self.heap.push(Task(priority, name))

    def run(self, steps: int) -> None:
        for _ in range(steps):
            if self.heap.size == 0:
                print(f"  t={self.clock}: idle")
                break
            t = self.heap.pop()
            self.clock += 1
            self.done.append(t.name)
            print(f"  t={self.clock}: run {t.name} (p{t.priority})")
            if t.name.startswith("repeat") and t.priority < 9:
                self.submit(t.name, t.priority + 3)


def main() -> None:
    h = MinHeap()
    vals = [(5, "e"), (3, "c"), (8, "h"), (1, "a"), (9, "i"), (2, "b"), (3, "bb"), (7, "g")]
    for p, n in vals:
        h.push(Task(p, n))
        top = h.peek()
        if top is not None:
            print(f"push {n}: top {top.name}, size {h.size}")
    out: list[str] = []
    while h.size > 0:
        out.append(h.pop().name)
    print(out)
    try:
        h.pop()
    except IndexError as err:
        print("error:", err)
    s = Scheduler()
    for i, name in enumerate(["build", "repeat-ping", "test", "deploy", "lint"]):
        s.submit(name, (i * 7) % 5)
    s.run(6)
    s.submit("hotfix", 0)
    s.run(10)
    print(s.done)


main()
