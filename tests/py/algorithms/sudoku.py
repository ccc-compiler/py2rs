def parse(rows: list[str]) -> dict[int, int]:
    g: dict[int, int] = {}
    for r in range(9):
        for c in range(9):
            ch = rows[r][c]
            g[r * 9 + c] = 0 if ch == "." else int(ch)
    return g


def ok(g: dict[int, int], pos: int, d: int) -> bool:
    r = pos // 9
    c = pos % 9
    for i in range(9):
        if g[r * 9 + i] == d or g[i * 9 + c] == d:
            return False
    br = r // 3 * 3
    bc = c // 3 * 3
    for i in range(3):
        for j in range(3):
            if g[(br + i) * 9 + bc + j] == d:
                return False
    return True


def solve(g: dict[int, int], stats: dict[str, int]) -> bool:
    best = -1
    best_opts: list[int] = []
    for pos in range(81):
        if g[pos] == 0:
            opts = [d for d in range(1, 10) if ok(g, pos, d)]
            if best == -1 or len(best_opts) > len(opts):
                best = pos
                best_opts = opts
            if not opts:
                return False
    if best == -1:
        return True
    for d in best_opts:
        stats["tries"] = stats.get("tries", 0) + 1
        g[best] = d
        if solve(g, stats):
            return True
        g[best] = 0
    stats["backtracks"] = stats.get("backtracks", 0) + 1
    return False


def show(g: dict[int, int]) -> None:
    for r in range(9):
        if r % 3 == 0 and r > 0:
            print("------+-------+------")
        parts: list[str] = []
        for c in range(9):
            if c % 3 == 0 and c > 0:
                parts.append("|")
            v = g[r * 9 + c]
            parts.append(str(v) if v else ".")
        print(" ".join(parts))


def main() -> None:
    puzzle = [
        "53..7....", "6..195...", ".98....6.",
        "8...6...3", "4..8.3..1", "7...2...6",
        ".6....28.", "...419..5", "....8..79",
    ]
    g = parse(puzzle)
    show(g)
    stats: dict[str, int] = {}
    print("solved:", solve(g, stats))
    show(g)
    print("stats:", sorted(stats.items()))
    rows_ok = all(sorted(g[r * 9 + c] for c in range(9)) == list(range(1, 10)) for r in range(9))
    cols_ok = all(sorted(g[r * 9 + c] for r in range(9)) == list(range(1, 10)) for c in range(9))
    print("rows ok:", rows_ok, "cols ok:", cols_ok)
    stuck = parse(["12345678.", "........9"] + ["........."] * 7)
    print("stuck solvable:", solve(stuck, {}))


main()
