class Node:
    def __init__(self, key: int) -> None:
        self.key = key
        self.left: Node | None = None
        self.right: Node | None = None


class BST:
    def __init__(self) -> None:
        self.root: Node | None = None
        self.count = 0

    def insert(self, key: int) -> bool:
        if self.root is None:
            self.root = Node(key)
            self.count += 1
            return True
        added = insert_at(self.root, key)
        if added:
            self.count += 1
        return added

    def contains(self, key: int) -> bool:
        cur = self.root
        while cur is not None:
            if key == cur.key:
                return True
            cur = cur.left if key < cur.key else cur.right
        return False


def insert_at(n: Node, key: int) -> bool:
    if key == n.key:
        return False
    if key < n.key:
        if n.left is None:
            n.left = Node(key)
            return True
        return insert_at(n.left, key)
    if n.right is None:
        n.right = Node(key)
        return True
    return insert_at(n.right, key)


def inorder(n: Node | None, out: list[int]) -> None:
    if n is None:
        return
    inorder(n.left, out)
    out.append(n.key)
    inorder(n.right, out)


def height(n: Node | None) -> int:
    if n is None:
        return 0
    return 1 + max(height(n.left), height(n.right))


def show(n: Node | None, depth: int) -> None:
    if n is None:
        return
    show(n.right, depth + 1)
    print("    " * depth + str(n.key))
    show(n.left, depth + 1)


def add_levels(n: Node | None, depth: int, sums: dict[int, int]) -> None:
    if n is None:
        return
    sums[depth] = sums.get(depth, 0) + n.key
    add_levels(n.left, depth + 1, sums)
    add_levels(n.right, depth + 1, sums)


def level_sums(root: Node | None) -> list[int]:
    sums: dict[int, int] = {}
    add_levels(root, 0, sums)
    return [sums[k] for k in sorted(sums)]


def main() -> None:
    t = BST()
    for k in [50, 30, 70, 20, 40, 60, 80, 35, 45, 65, 30, 50]:
        added = t.insert(k)
        if not added:
            print(f"duplicate {k}")
    out: list[int] = []
    inorder(t.root, out)
    print("inorder:", out, "count:", t.count)
    print("height:", height(t.root))
    show(t.root, 0)
    for k in [45, 55, 80, 10]:
        print(f"contains {k}: {t.contains(k)}")
    print("level sums:", level_sums(t.root))
    skew = BST()
    for k in range(1, 8):
        skew.insert(k)
    print("skewed height:", height(skew.root), "levels:", level_sums(skew.root))


main()
