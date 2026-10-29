def spiral_order(m: list[list[int]]) -> list[int]:
    out: list[int] = []
    if not m:
        return out
    top = 0
    bottom = len(m) - 1
    left = 0
    right = len(m[0]) - 1
    while top <= bottom and left <= right:
        for c in range(left, right + 1):
            out.append(m[top][c])
        top += 1
        for r in range(top, bottom + 1):
            out.append(m[r][right])
        right -= 1
        if top <= bottom:
            for c in range(right, left - 1, -1):
                out.append(m[bottom][c])
            bottom -= 1
        if left <= right:
            for r in range(bottom, top - 1, -1):
                out.append(m[r][left])
            left += 1
    return out


def spiral_fill(n: int) -> dict[tuple[int, int], int]:
    cells: dict[tuple[int, int], int] = {}
    r = 0
    c = 0
    dr = 0
    dc = 1
    for k in range(1, n * n + 1):
        cells[(r, c)] = k
        nr = r + dr
        nc = c + dc
        if not (0 <= nr < n and 0 <= nc < n) or (nr, nc) in cells:
            dr, dc = dc, -dr
            nr = r + dr
            nc = c + dc
        r = nr
        c = nc
    return cells


def rotate(m: list[list[int]]) -> list[list[int]]:
    n = len(m)
    return [[m[n - 1 - c][r] for c in range(n)] for r in range(len(m[0]))]


def show(m: list[list[int]]) -> None:
    for row in m:
        print("  " + " ".join(f"{v:>3}" for v in row))


def main() -> None:
    m = [[r * 4 + c + 1 for c in range(4)] for r in range(3)]
    show(m)
    print("spiral:", spiral_order(m))
    print("single row:", spiral_order([[1, 2, 3]]), "single col:", spiral_order([[1], [2], [3]]))
    for n in [1, 4, 5]:
        cells = spiral_fill(n)
        grid = [[cells[(r, c)] for c in range(n)] for r in range(n)]
        print(f"spiral fill {n}:")
        show(grid)
        print("  back:", spiral_order(grid) == list(range(1, n * n + 1)))
    sq = [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
    r1 = rotate(sq)
    print("rotated:")
    show(r1)
    r4 = rotate(rotate(rotate(r1)))
    print("four rotations identity:", r4 == sq)


main()
