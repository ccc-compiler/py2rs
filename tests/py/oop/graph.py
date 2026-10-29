class Graph:
    def __init__(self, directed: bool) -> None:
        self.directed = directed
        self.adj: dict[str, list[str]] = {}
        self.weights: dict[str, int] = {}

    def add_node(self, n: str) -> None:
        if n not in self.adj:
            self.adj[n] = []

    def add_edge(self, a: str, b: str, w: int = 1) -> None:
        self.add_node(a)
        self.add_node(b)
        self.adj[a].append(b)
        self.weights[a + ">" + b] = w
        if not self.directed:
            self.adj[b].append(a)
            self.weights[b + ">" + a] = w

    def bfs(self, start: str) -> list[str]:
        seen = {start}
        order = [start]
        queue = [start]
        while queue:
            n = queue.pop(0)
            for m in sorted(self.adj[n]):
                if m not in seen:
                    seen.add(m)
                    order.append(m)
                    queue.append(m)
        return order

    def dfs(self, n: str, seen: set[str], order: list[str]) -> None:
        seen.add(n)
        order.append(n)
        for m in sorted(self.adj[n]):
            if m not in seen:
                self.dfs(m, seen, order)

    def shortest(self, src: str) -> dict[str, int]:
        dist: dict[str, int] = {src: 0}
        done: set[str] = set()
        while True:
            best = ""
            for n, d in dist.items():
                if n not in done and (best == "" or d < dist[best]):
                    best = n
            if best == "":
                break
            done.add(best)
            for m in self.adj[best]:
                nd = dist[best] + self.weights[best + ">" + m]
                if m not in dist or nd < dist[m]:
                    dist[m] = nd
        return dist

    def topo(self) -> list[str]:
        indeg = {n: 0 for n in self.adj}
        for n in self.adj:
            for m in self.adj[n]:
                indeg[m] += 1
        ready = sorted(n for n in indeg if indeg[n] == 0)
        out: list[str] = []
        while ready:
            n = ready.pop(0)
            out.append(n)
            for m in self.adj[n]:
                indeg[m] -= 1
                if indeg[m] == 0:
                    ready.append(m)
            ready.sort()
        return out

    def components(self) -> list[list[str]]:
        seen: set[str] = set()
        comps: list[list[str]] = []
        for n in sorted(self.adj):
            if n not in seen:
                order: list[str] = []
                self.dfs(n, seen, order)
                comps.append(sorted(order))
        return comps


def main() -> None:
    g = Graph(False)
    for a, b, w in [("a", "b", 4), ("a", "c", 1), ("c", "b", 2), ("b", "d", 5), ("c", "d", 8), ("d", "e", 3), ("x", "y", 1)]:
        g.add_edge(a, b, w)
    print("bfs", g.bfs("a"))
    seen: set[str] = set()
    order: list[str] = []
    g.dfs("a", seen, order)
    print("dfs", order)
    print("dist", g.shortest("a"))
    print("components", g.components())
    d = Graph(True)
    for a, b in [("shop", "cook"), ("cook", "eat"), ("wake", "shop"), ("wake", "shower"), ("shower", "dress"), ("dress", "shop"), ("eat", "nap")]:
        d.add_edge(a, b)
    print("topo", d.topo())
    print("adjacency", d.adj)


main()
