from dataclasses import dataclass


@dataclass
class Item:
    name: str
    qty: int


class Store:
    def __init__(self) -> None:
        self.items: dict[str, Item] = {}
        self.log: list[str] = []

    def add(self, name: str, qty: int) -> None:
        self.items[name] = Item(name, qty)

    def sell(self, name: str, n: int) -> bool:
        item = self.items[name]
        if item.qty < n:
            return False
        item.qty -= n
        self.log.append(f"sold {n} {name}")
        return True


class Trie:
    def __init__(self) -> None:
        self.children: dict[str, Trie] = {}
        self.count = 0
        self.end = False


class Words:
    def __init__(self) -> None:
        self.root = Trie()

    def insert(self, word: str) -> None:
        node = self.root
        for ch in word:
            if ch not in node.children:
                node.children[ch] = Trie()
            node = node.children[ch]
            node.count += 1
        node.end = True

    def prefix_count(self, p: str) -> int:
        node = self.root
        for ch in p:
            if ch not in node.children:
                return 0
            node = node.children[ch]
        return node.count


def bump(grid: list[list[int]], r: int) -> None:
    row = grid[r]
    for i in range(len(row)):
        row[i] += 1


def main() -> None:
    s = Store()
    s.add("pen", 5)
    print(s.sell("pen", 2), s.sell("pen", 9), s.items["pen"].qty, s.log)
    w = Words()
    for word in ["car", "cart", "care", "dog"]:
        w.insert(word)
    print(w.prefix_count("car"), w.prefix_count("ca"), w.prefix_count("d"), w.prefix_count("x"))
    g = [[0, 0], [5, 5]]
    bump(g, 1)
    bump(g, 1)
    print(g)


main()
