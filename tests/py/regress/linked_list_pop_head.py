# regression: BUG: unlinking the head of a linked list is rejected; the old head becomes unreachable,
class Node:
    def __init__(self, value: int) -> None:
        self.value = value
        self.next: Node | None = None


class LL:
    def __init__(self) -> None:
        self.head: Node | None = None

    def push(self, v: int) -> None:
        n = Node(v)
        n.next = self.head
        self.head = n

    def pop_front(self) -> int | None:
        if self.head is None:
            return None
        v = self.head.value
        self.head = self.head.next
        return v


def main() -> None:
    l = LL()
    l.push(1)
    l.push(2)
    print(l.pop_front(), l.pop_front(), l.pop_front())


main()
