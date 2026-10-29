def step(grid: list[list[int]]) -> list[list[int]]:
    rows = len(grid)
    cols = len(grid[0])
    new = [[0] * cols for _ in range(rows)]
    for r in range(rows):
        for c in range(cols):
            n = 0
            for dr in [-1, 0, 1]:
                for dc in [-1, 0, 1]:
                    if dr == 0 and dc == 0:
                        continue
                    rr = (r + dr) % rows
                    cc = (c + dc) % cols
                    n += grid[rr][cc]
            if grid[r][c] == 1 and (n == 2 or n == 3):
                new[r][c] = 1
            elif grid[r][c] == 0 and n == 3:
                new[r][c] = 1
    return new


def show(grid: list[list[int]], gen: int) -> None:
    alive = sum(sum(row) for row in grid)
    print(f"generation {gen} ({alive} alive)")
    for row in grid:
        print("  " + "".join("#" if v else "." for v in row))


def from_cells(rows: int, cols: int, cells: list[tuple[int, int]]) -> list[list[int]]:
    g = [[0] * cols for _ in range(rows)]
    for r in range(rows):
        for c in range(cols):
            if (r, c) in cells:
                g[r][c] = 1
    return g


def key(grid: list[list[int]]) -> str:
    return "".join("".join(str(v) for v in row) for row in grid)


def main() -> None:
    glider = from_cells(8, 8, [(0, 1), (1, 2), (2, 0), (2, 1), (2, 2)])
    g = glider
    for gen in range(5):
        show(g, gen)
        g = step(g)
    blinker = from_cells(5, 5, [(2, 1), (2, 2), (2, 3)])
    seen: dict[str, int] = {}
    b = blinker
    for gen in range(10):
        k = key(b)
        if k in seen:
            print(f"blinker repeats at gen {gen} (first seen {seen[k]}), period {gen - seen[k]}")
            break
        seen[k] = gen
        b = step(b)
    block = from_cells(4, 4, [(1, 1), (1, 2), (2, 1), (2, 2)])
    print("block is still life:", step(block) == block)
    g = glider
    for _ in range(32):
        g = step(g)
    print("glider back after 32 gens on 8x8 torus:", g == glider)


main()
