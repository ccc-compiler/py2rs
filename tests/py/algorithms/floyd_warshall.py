def floyd(n: int, edges: list[tuple[int, int, int]]) -> tuple[list[list[int]], list[list[int]]]:
    inf = 1 << 40
    d = [[0 if i == j else inf for j in range(n)] for i in range(n)]
    nxt = [[-1] * n for _ in range(n)]
    for i in range(n):
        nxt[i][i] = i
    for a in range(n):
        for b in range(n):
            for e in edges:
                if e[0] == a and e[1] == b and e[2] < d[a][b]:
                    d[a][b] = e[2]
                    nxt[a][b] = b
    for k in range(n):
        for i in range(n):
            for j in range(n):
                if d[i][k] + d[k][j] < d[i][j]:
                    d[i][j] = d[i][k] + d[k][j]
                    nxt[i][j] = nxt[i][k]
    return d, nxt


def path(nxt: list[list[int]], a: int, b: int) -> list[int]:
    if nxt[a][b] == -1:
        return []
    p = [a]
    while a != b:
        a = nxt[a][b]
        p.append(a)
    return p


def bellman_ford(n: int, edges: list[tuple[int, int, int]], src: int) -> dict[int, int] | None:
    inf = 1 << 40
    dist = {v: inf for v in range(n)}
    dist[src] = 0
    for rnd in range(n - 1):
        changed = False
        for e in edges:
            a, b, w = e
            if dist[a] != inf and dist[a] + w < dist[b]:
                dist[b] = dist[a] + w
                changed = True
        if not changed:
            print(f"  converged after {rnd + 1} rounds")
            break
    for e in edges:
        a, b, w = e
        if dist[a] != inf and dist[a] + w < dist[b]:
            return None
    return dist


def main() -> None:
    edges = [(0, 1, 3), (0, 2, 8), (0, 4, -4), (1, 3, 1), (1, 4, 7), (2, 1, 4), (3, 0, 2), (3, 2, -5), (4, 3, 6)]
    n = 5
    d, nxt = floyd(n, edges)
    print("     " + "".join(f"{j:>5}" for j in range(n)))
    for i in range(n):
        print(f"{i:>3}: " + "".join(f"{v:>5}" for v in d[i]))
    for a, b in [(0, 2), (2, 4), (4, 1), (3, 3)]:
        print(f"path {a}->{b}: {path(nxt, a, b)} cost {d[a][b]}")
    res = bellman_ford(n, edges, 0)
    if res is not None:
        print("bellman-ford from 0:", list(res.values()))
    neg = edges + [(2, 0, -10)]
    res2 = bellman_ford(n, neg, 0)
    print("with negative cycle:", "detected" if res2 is None else str(res2))


main()
