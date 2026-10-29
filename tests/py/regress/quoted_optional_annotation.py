# regression: BUG: a quoted forward reference to an Optional is not parsed ("Node" alone works):
class Node:
    def __init__(self, v: int, nxt: "Node | None" = None) -> None:
        self.v = v
        self.nxt = nxt


def main() -> None:
    n = Node(2, Node(1))
    print(n.v, n.nxt is None)


main()
