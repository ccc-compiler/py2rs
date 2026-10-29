def dfs(adj: dict[str, list[str]], node: str, seen: set[str], order: list[str], depth: int) -> None:
    seen.add(node)
    order.append(node)
    print(f"{'  ' * depth}enter {node}")
    for nb in adj[node]:
        if nb not in seen:
            dfs(adj, nb, seen, order, depth + 1)
    print(f"{'  ' * depth}leave {node}")


def iterative_dfs(adj: dict[str, list[str]], start: str) -> list[str]:
    stack = [start]
    seen: set[str] = set()
    order: list[str] = []
    while stack:
        node = stack.pop()
        if node in seen:
            continue
        seen.add(node)
        order.append(node)
        for nb in reversed(adj[node]):
            if nb not in seen:
                stack.append(nb)
    return order


def components(adj: dict[str, list[str]]) -> list[list[str]]:
    seen: set[str] = set()
    out: list[list[str]] = []
    for node in adj:
        if node not in seen:
            order: list[str] = []
            dfs(adj, node, seen, order, 0)
            out.append(sorted(order))
    return out


def has_cycle(adj: dict[str, list[str]]) -> bool:
    color: dict[str, int] = {}
    for n in adj:
        color[n] = 0
    stack: list[tuple[str, int]] = []
    for root in adj:
        if color[root] != 0:
            continue
        stack.append((root, 0))
        color[root] = 1
        while stack:
            node, i = stack.pop()
            if i < len(adj[node]):
                stack.append((node, i + 1))
                nb = adj[node][i]
                if color[nb] == 1:
                    return True
                if color[nb] == 0:
                    color[nb] = 1
                    stack.append((nb, 0))
            else:
                color[node] = 2
    return False


def main() -> None:
    adj: dict[str, list[str]] = {
        "a": ["b", "c"], "b": ["a", "d"], "c": ["a", "d", "e"],
        "d": ["b", "c"], "e": ["c"], "x": ["y"], "y": ["x"], "z": [],
    }
    print("components:", components(adj))
    print("iterative from a:", iterative_dfs(adj, "a"))
    dag: dict[str, list[str]] = {"a": ["b", "c"], "b": ["d"], "c": ["d"], "d": []}
    cyc: dict[str, list[str]] = {"a": ["b"], "b": ["c"], "c": ["a"]}
    print("dag has cycle:", has_cycle(dag))
    print("cyc has cycle:", has_cycle(cyc))


main()
