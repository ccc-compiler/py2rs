def pivot(rows: list[tuple[str, str, int]]) -> tuple[list[str], list[str], dict[str, dict[str, int]]]:
    row_keys: list[str] = []
    col_keys: list[str] = []
    cells: dict[str, dict[str, int]] = {}
    for r, c, v in rows:
        if r not in row_keys:
            row_keys.append(r)
        if c not in col_keys:
            col_keys.append(c)
        if r not in cells:
            cells[r] = {}
        old = cells[r].get(c, 0)
        cells[r][c] = old + v
    return (sorted(row_keys), sorted(col_keys), cells)


def main() -> None:
    raw = """Q1 apples 120
Q1 pears 45
Q2 apples 130
Q2 plums 20
Q3 pears 60
Q3 apples 90
Q4 plums 75
Q4 apples 150
Q1 apples 30
Q2 pears 50"""
    rows: list[tuple[str, str, int]] = []
    for line in raw.split("\n"):
        p = line.split()
        rows.append((p[0], p[1], int(p[2])))
    rk, ck, cells = pivot(rows)
    print(f"{'':<4}" + "".join(f"{c:>8}" for c in ck) + f"{'total':>8}")
    col_tot = {c: 0 for c in ck}
    for r in rk:
        line = f"{r:<4}"
        tot = 0
        for c in ck:
            v = cells[r].get(c, 0)
            col_tot[c] += v
            tot += v
            line += f"{v:>8}" if v else f"{'.':>8}"
        print(line + f"{tot:>8}")
    grand = sum(col_tot.values())
    print(f"{'all':<4}" + "".join(f"{col_tot[c]:>8}" for c in ck) + f"{grand:>8}")
    print(f"{'%':<4}" + "".join(f"{col_tot[c] * 100 / grand:>7.1f}%" for c in ck))
    best_q = max(rk, key=lambda r: sum(cells[r].values()))
    print("best quarter:", best_q)
    growth = [sum(cells[rk[i + 1]].values()) - sum(cells[rk[i]].values()) for i in range(len(rk) - 1)]
    print("quarter-on-quarter:", growth)
    flipped: dict[str, dict[str, int]] = {}
    for r in rk:
        for c, v in cells[r].items():
            flipped.setdefault(c, {})[r] = v
    for c in ck:
        print(c, flipped[c])


main()
