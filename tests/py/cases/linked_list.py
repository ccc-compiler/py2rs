from __future__ import annotations
from dataclasses import dataclass


@dataclass
class Node:
    value: int
    next: Node | None = None


class Stack:
    def __init__(self) -> None:
        self.head: Node | None = None
        self.size = 0

    def push(self, v: int) -> None:
        node = Node(v)
        node.next = self.head
        self.head = node
        self.size += 1

    def pop(self) -> int:
        if self.head is None:
            raise IndexError("pop from empty stack")
        node = self.head
        self.head = node.next
        self.size -= 1
        return node.value

    def total(self) -> int:
        s = 0
        cur = self.head
        while cur is not None:
            s += cur.value
            cur = cur.next
        return s


def main() -> None:
    st = Stack()
    for i in range(1, 6):
        st.push(i * i)
    print(st.total(), st.size)
    print(st.pop(), st.pop(), st.total())
    try:
        while True:
            st.pop()
    except IndexError as e:
        print("empty:", e)


if __name__ == "__main__":
    main()
