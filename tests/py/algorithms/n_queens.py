def solve(n: int, row: int, cols: list[int], used_c: set[int], d1: set[int], d2: set[int], out: list[list[int]]) -> None:
    if row == n:
        out.append(cols.copy())
        return
    for c in range(n):
        if c in used_c or row - c in d1 or row + c in d2:
            continue
        cols.append(c)
        used_c.add(c)
        d1.add(row - c)
        d2.add(row + c)
        solve(n, row + 1, cols, used_c, d1, d2, out)
        cols.pop()
        used_c.remove(c)
        d1.remove(row - c)
        d2.remove(row + c)


def render(cols: list[int]) -> list[str]:
    n = len(cols)
    return ["".join("Q" if cols[r] == c else "." for c in range(n)) for r in range(n)]


def count_bits(n: int, row: int, cols: int, d1: int, d2: int) -> int:
    if row == n:
        return 1
    total = 0
    full = (1 << n) - 1
    avail = full & ~(cols | d1 | d2)
    while avail:
        bit = avail & -avail
        avail -= bit
        total += count_bits(n, row + 1, cols | bit, ((d1 | bit) << 1) & full, (d2 | bit) >> 1)
    return total


def main() -> None:
    for n in range(1, 9):
        out: list[list[int]] = []
        solve(n, 0, [], set(), set(), set(), out)
        print(f"n={n}: {len(out)} solutions, bitmask count {count_bits(n, 0, 0, 0, 0)}")
    out6: list[list[int]] = []
    solve(6, 0, [], set(), set(), set(), out6)
    for sol in out6:
        print(sol)
        for line in render(sol):
            print("   ", line)
    out8: list[list[int]] = []
    solve(8, 0, [], set(), set(), set(), out8)
    print("first 8-queens:", out8[0], "last:", out8[-1])


main()
