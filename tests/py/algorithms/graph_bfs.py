def build_graph(n: int, edges: list[tuple[int, int]]) -> dict[int, list[int]]:
    adj: dict[int, list[int]] = {i: [] for i in range(n)}
    for e in edges:
        a, b = e
        adj[a].append(b)
        adj[b].append(a)
    for i in range(n):
        adj[i].sort()
    return adj


def bfs(adj: dict[int, list[int]], start: int) -> tuple[dict[int, int], dict[int, int]]:
    dist = {k: -1 for k in adj}
    parent = {k: -1 for k in adj}
    dist[start] = 0
    queue = [start]
    head = 0
    order: list[int] = []
    while head < len(queue):
        u = queue[head]
        head += 1
        order.append(u)
        for v in adj[u]:
            if dist[v] == -1:
                queue.append(v)
                dist[v] = dist[u] + 1
                parent[v] = u
    print(f"visit order from {start}: {order}")
    return dist, parent


def path_to(parent: dict[int, int], target: int) -> list[int]:
    path: list[int] = []
    cur = target
    while cur != -1:
        path.append(cur)
        cur = parent[cur]
    return path[::-1]


def grid_shortest(grid: list[str]) -> int:
    rows = len(grid)
    cols = len(grid[0])
    seen: set[tuple[int, int]] = set()
    frontier = [(0, 0)]
    seen.add((0, 0))
    steps = 0
    while frontier:
        nxt: list[tuple[int, int]] = []
        for cell in frontier:
            r, c = cell
            if r == rows - 1 and c == cols - 1:
                return steps
            for d in [(0, 1), (1, 0), (0, -1), (-1, 0)]:
                nr = r + d[0]
                nc = c + d[1]
                if 0 <= nr < rows and 0 <= nc < cols and grid[nr][nc] == "." and (nr, nc) not in seen:
                    seen.add((nr, nc))
                    nxt.append((nr, nc))
        frontier = nxt
        steps += 1
    return -1


def main() -> None:
    edges = [(0, 1), (0, 2), (1, 3), (2, 3), (3, 4), (4, 5), (5, 6), (2, 6), (7, 8)]
    adj = build_graph(9, edges)
    for i in range(len(adj)):
        print(f"{i}: {adj[i]}")
    dist, parent = bfs(adj, 0)
    print("dist:", list(dist.values()))
    for t in [4, 6, 8]:
        if dist[t] >= 0:
            print(f"path 0->{t}: {path_to(parent, t)}")
        else:
            print(f"{t} unreachable")
    grid = [
        "..#.....",
        ".##.###.",
        "....#...",
        "#.#...#.",
        "..#.#...",
    ]
    for line in grid:
        print(line)
    print("grid shortest:", grid_shortest(grid))
    blocked = [".#", "#."]
    print("blocked:", grid_shortest(blocked))


main()
