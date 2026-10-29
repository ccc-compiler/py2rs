# regression: BUG: storing a parameter into a field of a new local object is rejected, although
class Node:
    def __init__(self, v: int) -> None:
        self.v = v
        self.left: Node | None = None


def wrap(child: Node, v: int) -> Node:
    n = Node(v)
    n.left = child
    return n


def main() -> None:
    t = wrap(wrap(Node(1), 2), 3)
    print(t.v)


main()
