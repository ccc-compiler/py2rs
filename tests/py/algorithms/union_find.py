class DSU:
    def __init__(self, n: int) -> None:
        self.parent: dict[int, int] = {i: i for i in range(n)}
        self.size: dict[int, int] = {i: 1 for i in range(n)}
        self.sets = n

    def find(self, x: int) -> int:
        root = x
        while self.parent[root] != root:
            root = self.parent[root]
        while self.parent[x] != root:
            nxt = self.parent[x]
            self.parent[x] = root
            x = nxt
        return root

    def union(self, a: int, b: int) -> bool:
        ra = self.find(a)
        rb = self.find(b)
        if ra == rb:
            return False
        if self.size[ra] < self.size[rb]:
            ra, rb = rb, ra
        self.parent[rb] = ra
        self.size[ra] += self.size[rb]
        self.sets -= 1
        return True


def kruskal(n: int, edges: list[tuple[int, int, int]]) -> tuple[int, list[tuple[int, int, int]]]:
    dsu = DSU(n)
    total = 0
    used: list[tuple[int, int, int]] = []
    for e in sorted(edges, key=lambda e: (e[2], e[0], e[1])):
        a, b, w = e
        if dsu.union(a, b):
            total += w
            used.append(e)
            print(f"  take {a}-{b} (w={w}), sets left {dsu.sets}")
        else:
            print(f"  skip {a}-{b} (w={w})")
    return total, used


def main() -> None:
    d = DSU(10)
    pairs = [(1, 2), (3, 4), (2, 3), (5, 6), (1, 4), (7, 7), (8, 9), (6, 9)]
    for p in pairs:
        merged = d.union(p[0], p[1])
        print(f"union{p} -> {merged}, sets={d.sets}")
    groups: dict[int, list[int]] = {}
    for i in range(10):
        groups.setdefault(d.find(i), []).append(i)
    for g in sorted(groups.values()):
        print("group", g)
    edges = [(0, 1, 4), (0, 7, 8), (1, 2, 8), (1, 7, 11), (2, 3, 7), (2, 8, 2), (2, 5, 4),
             (3, 4, 9), (3, 5, 14), (4, 5, 10), (5, 6, 2), (6, 7, 1), (6, 8, 6), (7, 8, 7)]
    total, used = kruskal(9, edges)
    print("MST weight:", total)
    print("MST edges:", used)


main()
