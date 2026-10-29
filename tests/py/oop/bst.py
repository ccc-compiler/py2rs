class Node:
    def __init__(self, key: int, label: str) -> None:
        self.key = key
        self.label = label
        self.left: Node | None = None
        self.right: Node | None = None

    def insert(self, key: int, label: str) -> bool:
        if key == self.key:
            self.label = label
            return False
        if key < self.key:
            if self.left is None:
                self.left = Node(key, label)
                return True
            return self.left.insert(key, label)
        if self.right is None:
            self.right = Node(key, label)
            return True
        return self.right.insert(key, label)

    def find(self, key: int) -> str | None:
        if key == self.key:
            return self.label
        if key < self.key:
            if self.left is None:
                return None
            return self.left.find(key)
        if self.right is None:
            return None
        return self.right.find(key)

    def inorder(self, out: list[str]) -> None:
        if self.left is not None:
            self.left.inorder(out)
        out.append(f"{self.key}:{self.label}")
        if self.right is not None:
            self.right.inorder(out)

    def height(self) -> int:
        lh = 0 if self.left is None else self.left.height()
        rh = 0 if self.right is None else self.right.height()
        return 1 + max(lh, rh)

    def min_key(self) -> int:
        if self.left is None:
            return self.key
        return self.left.min_key()

    def max_key(self) -> int:
        if self.right is None:
            return self.key
        return self.right.max_key()

    def count_range(self, lo: int, hi: int) -> int:
        k = self.key
        n = 1 if lo <= k <= hi else 0
        if self.left is not None and lo < self.key:
            n += self.left.count_range(lo, hi)
        if self.right is not None and hi > self.key:
            n += self.right.count_range(lo, hi)
        return n


class Tree:
    def __init__(self) -> None:
        self.root: Node | None = None
        self.size = 0

    def put(self, key: int, label: str) -> None:
        if self.root is None:
            self.root = Node(key, label)
            self.size = 1
        elif self.root.insert(key, label):
            self.size += 1

    def get(self, key: int) -> str | None:
        if self.root is None:
            return None
        return self.root.find(key)

    def items(self) -> list[str]:
        out: list[str] = []
        if self.root is not None:
            self.root.inorder(out)
        return out

    def height(self) -> int:
        if self.root is None:
            return 0
        return self.root.height()


def main() -> None:
    t = Tree()
    print("empty height", t.height(), t.get(5))
    seed = 17
    for i in range(15):
        seed = (seed * 37 + 11) % 101
        t.put(seed, f"n{i}")
    t.put(50, "fifty")
    t.put(50, "FIFTY")
    print("size", t.size, "height", t.height())
    print(t.items())
    for k in [50, 3, 67, 100]:
        print(k, "->", t.get(k))
    if t.root is not None:
        print("min", t.root.min_key(), "max", t.root.max_key())
        print("in [20, 60]:", t.root.count_range(20, 60))
    words = Tree()
    for w in "kiwi apple mango banana cherry fig".split():
        words.put(len(w) * 100 + ord(w[0]), w)
    print(words.items(), words.height())


main()
