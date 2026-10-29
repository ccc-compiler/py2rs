def kahn(deps: dict[str, list[str]]) -> list[str]:
    indeg: dict[str, int] = {}
    users: dict[str, list[str]] = {}
    for node in deps:
        indeg.setdefault(node, 0)
        for d in deps[node]:
            indeg.setdefault(d, 0)
            indeg[node] += 1
            users.setdefault(d, []).append(node)
    ready = sorted([n for n in indeg if indeg[n] == 0])
    order: list[str] = []
    while ready:
        n = ready.pop(0)
        order.append(n)
        for u in sorted(users.get(n, [])):
            indeg[u] -= 1
            if indeg[u] == 0:
                ready.append(u)
                ready.sort()
    if len(order) != len(indeg):
        print("  cycle among:", sorted([n for n in indeg if indeg[n] > 0]))
    return order


def dfs_order(deps: dict[str, list[str]], node: str, state: dict[str, int], out: list[str]) -> bool:
    st = state.get(node, 0)
    if st == 1:
        return False
    if st == 2:
        return True
    state[node] = 1
    for d in deps.get(node, []):
        if not dfs_order(deps, d, state, out):
            return False
    state[node] = 2
    out.append(node)
    return True


def levels(deps: dict[str, list[str]], order: list[str]) -> dict[str, int]:
    lv: dict[str, int] = {}
    for n in order:
        below = [lv[d] for d in deps.get(n, [])]
        lv[n] = 1 + max(below) if below else 0
    return lv


def main() -> None:
    build: dict[str, list[str]] = {
        "app": ["lib", "utils", "config"],
        "lib": ["utils", "core"],
        "utils": ["core"],
        "tests": ["app", "mock"],
        "mock": ["core"],
        "core": [],
        "config": [],
    }
    order = kahn(build)
    print("kahn order:", order)
    out: list[str] = []
    state: dict[str, int] = {}
    ok = True
    for n in sorted(build):
        ok = ok and dfs_order(build, n, state, out)
    print("dfs order:", out, ok)
    lv = levels(build, order)
    for n in order:
        print(f"  {'.' * lv[n]}{n} (level {lv[n]})")
    bad: dict[str, list[str]] = {"a": ["b"], "b": ["c"], "c": ["a"], "d": []}
    print("cyclic:", kahn(bad))
    st2: dict[str, int] = {}
    out2: list[str] = []
    print("dfs on cyclic:", dfs_order(bad, "a", st2, out2), out2)


main()
