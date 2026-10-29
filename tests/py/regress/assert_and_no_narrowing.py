# regression: BUG: `assert a is not None and b is not None` does not narrow (two separate asserts do):
class Node:
    def __init__(self) -> None:
        self.left: Node | None = None
        self.right: Node | None = None

    def size(self) -> int:
        return 1


def main() -> None:
    n = Node()
    n.left = Node()
    n.right = Node()
    assert n.left is not None and n.right is not None
    print(n.left.size() + n.right.size())


main()
