def pq_push(pq: list[tuple[int, int]], item: tuple[int, int]) -> None:
    pq.append(item)
    pq.sort(reverse=True)


def pq_pop(pq: list[tuple[int, int]]) -> tuple[int, int]:
    return pq.pop()


def dijkstra(graph: dict[int, list[tuple[int, int]]], src: int, n: int) -> tuple[dict[int, int], dict[int, int]]:
    inf = 1 << 60
    dist = {i: inf for i in range(n)}
    prev = {i: -1 for i in range(n)}
    dist[src] = 0
    pq: list[tuple[int, int]] = []
    pq_push(pq, (0, src))
    pops = 0
    while pq:
        d, u = pq_pop(pq)
        pops += 1
        if d > dist[u]:
            continue
        for edge in graph.get(u, []):
            v, w = edge
            nd = d + w
            if nd < dist[v]:
                dist[v] = nd
                prev[v] = u
                pq_push(pq, (nd, v))
    print(f"  ({pops} pops)")
    return dist, prev


def route(prev: dict[int, int], t: int) -> str:
    nodes: list[str] = []
    while t != -1:
        nodes.append(str(t))
        t = prev[t]
    nodes.reverse()
    return " -> ".join(nodes)


def main() -> None:
    edges = [
        (0, 1, 7), (0, 2, 9), (0, 5, 14), (1, 2, 10), (1, 3, 15),
        (2, 3, 11), (2, 5, 2), (3, 4, 6), (4, 5, 9), (6, 7, 1),
    ]
    n = 8
    graph: dict[int, list[tuple[int, int]]] = {}
    for e in edges:
        a, b, w = e
        graph.setdefault(a, []).append((b, w))
        graph.setdefault(b, []).append((a, w))
    for src in [0, 3, 6]:
        print(f"from {src}:")
        dist, prev = dijkstra(graph, src, n)
        for t in range(n):
            if dist[t] >= 1 << 60:
                print(f"  {t}: unreachable")
            else:
                print(f"  {t}: {dist[t]:>3}  {route(prev, t)}")


main()
