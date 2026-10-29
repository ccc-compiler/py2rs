class TrieNode:
    def __init__(self) -> None:
        self.children: dict[str, TrieNode] = {}
        self.terminal = False
        self.count = 0

    def add(self, word: str, i: int) -> bool:
        if i == len(word):
            fresh = not self.terminal
            self.terminal = True
            return fresh
        ch = word[i]
        if ch not in self.children:
            self.children[ch] = TrieNode()
        self.children[ch].count += 1
        return self.children[ch].add(word, i + 1)


class Trie:
    def __init__(self) -> None:
        self.root = TrieNode()
        self.words = 0

    def insert(self, word: str) -> None:
        if self.root.add(word, 0):
            self.words += 1

    def contains(self, word: str) -> bool:
        node = self.root
        for ch in word:
            if ch not in node.children:
                return False
            node = node.children[ch]
        return node.terminal

    def prefix_count(self, prefix: str) -> int:
        node = self.root
        for ch in prefix:
            if ch not in node.children:
                return 0
            node = node.children[ch]
        return node.count

    def collect(self, node: TrieNode, prefix: str, out: list[str]) -> None:
        if node.terminal:
            out.append(prefix)
        for ch in sorted(node.children.keys()):
            self.collect(node.children[ch], prefix + ch, out)

    def complete(self, prefix: str) -> list[str]:
        node = self.root
        for ch in prefix:
            if ch not in node.children:
                return []
            node = node.children[ch]
        out: list[str] = []
        self.collect(node, prefix, out)
        return out

    def depth(self, node: TrieNode) -> int:
        best = 0
        for child in node.children.values():
            best = max(best, 1 + self.depth(child))
        return best


def main() -> None:
    t = Trie()
    text = "the then there these theme tea team ten tent to toe top tops tap the ten"
    for w in text.split():
        t.insert(w)
    print("distinct words:", t.words)
    for w in ["the", "th", "tent", "tents", "top", "x"]:
        print(f"{w!r:8} in trie: {t.contains(w)}")
    for p in ["t", "th", "the", "te", "to", "z"]:
        print(f"prefix {p!r}: {t.prefix_count(p)} -> {t.complete(p)}")
    print("depth", t.depth(t.root))
    print("first letters", sorted(t.root.children.keys()))


main()
