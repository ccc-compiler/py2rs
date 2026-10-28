from typing import Optional


class Node:
    def __init__(self, val: int) -> None:
        self.val = val
        self.next: Node | None = None


class LinkedList:
    def __init__(self) -> None:
        self.head: Optional[Node] = None
        self.size = 0

    def push(self, val: int) -> None:
        node = Node(val)
        node.next = self.head
        self.head = node
        self.size += 1

    def total(self) -> int:
        s = 0
        cur = self.head
        while cur is not None:
            s += cur.val
            cur = cur.next
        return s

    def first(self) -> int | None:
        if self.head is None:
            return None
        return self.head.val


class Tree:
    def __init__(self, key: int) -> None:
        self.key = key
        self.left: Tree | None = None
        self.right: Tree | None = None

    def insert(self, key: int) -> None:
        if key < self.key:
            if self.left is None:
                self.left = Tree(key)
            else:
                self.left.insert(key)
        else:
            if self.right is None:
                self.right = Tree(key)
            else:
                self.right.insert(key)

    def inorder(self, out: list[int]) -> None:
        if self.left is not None:
            self.left.inorder(out)
        out.append(self.key)
        if self.right:
            self.right.inorder(out)

    def height(self) -> int:
        lh = self.left.height() if self.left is not None else 0
        rh = 0
        if self.right is not None:
            rh = self.right.height()
        return 1 + max(lh, rh)


def find(xs: list[str], target: str) -> int | None:
    for i, x in enumerate(xs):
        if x == target:
            return i
    return None


def describe(n: int | None) -> str:
    if n is None:
        return "nothing"
    return f"number {n + 1}"


def main() -> None:
    ll = LinkedList()
    print(ll.first(), ll.total())
    for v in [3, 1, 4, 1, 5]:
        ll.push(v)
    print(ll.first(), ll.total(), ll.size)
    t = Tree(50)
    for k in [30, 70, 20, 40, 60, 80, 35]:
        t.insert(k)
    keys: list[int] = []
    t.inorder(keys)
    print(keys)
    words = ["a", "b", "c"]
    print(find(words, "b"), find(words, "z"))
    print(describe(None), describe(41))
    ages = {"ann": 31}
    a = ages.get("ann")
    b = ages.get("bob")
    print(a, b, a is None, b is None)
    if a is not None and a > 30:
        print("ann is over 30:", a)
    x: int | None = None
    print(x)
    x = 5
    assert x is not None
    print(x * 2)


main()
