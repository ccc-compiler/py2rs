class Node:
    def __init__(self, value: int) -> None:
        self.value = value
        self.next: Node | None = None


class LinkedList:
    def __init__(self) -> None:
        self.head: Node | None = None
        self.length = 0

    def push_front(self, value: int) -> None:
        node = Node(value)
        node.next = self.head
        self.head = node
        self.length += 1

    def to_list(self) -> list[int]:
        out: list[int] = []
        cur = self.head
        while cur is not None:
            out.append(cur.value)
            cur = cur.next
        return out

    def contains(self, value: int) -> bool:
        cur = self.head
        while cur is not None:
            if cur.value == value:
                return True
            cur = cur.next
        return False

    def clear(self) -> None:
        self.head = None
        self.length = 0

    def pop_front(self) -> int | None:
        if self.head is None:
            return None
        values = self.to_list()
        self.clear()
        for v in reversed(values[1:]):
            self.push_front(v)
        return values[0]

    def reverse(self) -> None:
        values = self.to_list()
        self.clear()
        for v in values:
            self.push_front(v)

    def sum(self) -> int:
        total = 0
        cur = self.head
        while cur is not None:
            total += cur.value
            cur = cur.next
        return total

    def max(self) -> int | None:
        if self.head is None:
            return None
        best = self.head.value
        cur = self.head.next
        while cur is not None:
            if cur.value > best:
                best = cur.value
            cur = cur.next
        return best

    def __str__(self) -> str:
        parts = [str(v) for v in self.to_list()]
        return " -> ".join(parts) + " -> None"


def main() -> None:
    ll = LinkedList()
    print(ll, ll.length, ll.max(), ll.pop_front())
    for v in [5, 3, 8, 1, 9, 2]:
        ll.push_front(v)
        print(f"push {v}: {ll}")
    print("length", ll.length, "sum", ll.sum(), "max", ll.max())
    print("contains 8?", ll.contains(8), "contains 7?", ll.contains(7))
    ll.reverse()
    print("reversed:", ll)
    for _ in range(3):
        v = ll.pop_front()
        print("popped", v, "now", ll)
    ll.reverse()
    print("reversed again:", ll, ll.to_list())
    while ll.length > 0:
        ll.pop_front()
    print("empty:", ll, ll.max())


main()
