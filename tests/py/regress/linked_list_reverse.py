# BUG (same family as linked_list_pop_head): the textbook in-place reversal is rejected,
# although every node is owned by exactly one place at each step (take/replace in Rust):
#   error: this stores a value that is still reachable through `cur`
class Node:
    def __init__(self, value: int) -> None:
        self.value = value
        self.next: Node | None = None


def main() -> None:
    head: Node | None = None
    for v in [1, 2, 3]:
        n = Node(v)
        n.next = head
        head = n
    prev: Node | None = None
    cur = head
    head = None
    while cur is not None:
        nxt = cur.next
        cur.next = prev
        prev = cur
        cur = nxt
    head = prev
    c = head
    while c is not None:
        print(c.value)
        c = c.next


main()
