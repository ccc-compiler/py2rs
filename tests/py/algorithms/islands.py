def parse(rows: list[str]) -> list[list[int]]:
    return [[1 if ch == "#" else 0 for ch in row] for row in rows]


def flood(grid: list[list[int]], r: int, c: int, label: int, labels: dict[tuple[int, int], int]) -> int:
    stack = [(r, c)]
    size = 0
    while stack:
        cell = stack.pop()
        if cell in labels:
            continue
        cr, cc = cell
        if cr < 0 or cc < 0 or cr >= len(grid) or cc >= len(grid[0]) or grid[cr][cc] == 0:
            continue
        labels[cell] = label
        size += 1
        stack.append((cr + 1, cc))
        stack.append((cr - 1, cc))
        stack.append((cr, cc + 1))
        stack.append((cr, cc - 1))
    return size


def perimeter(grid: list[list[int]]) -> int:
    p = 0
    rows = len(grid)
    cols = len(grid[0])
    for r in range(rows):
        for c in range(cols):
            if grid[r][c] == 1:
                p += 4
                if r > 0 and grid[r - 1][c] == 1:
                    p -= 2
                if c > 0 and grid[r][c - 1] == 1:
                    p -= 2
    return p


def main() -> None:
    rows = [
        "##...#....",
        "##..###...",
        "......#..#",
        "..#.......",
        ".###...###",
        "..#....#.#",
        "#......###",
    ]
    grid = parse(rows)
    labels: dict[tuple[int, int], int] = {}
    sizes: list[int] = []
    for r in range(len(grid)):
        for c in range(len(grid[0])):
            if grid[r][c] == 1 and (r, c) not in labels:
                sizes.append(flood(grid, r, c, len(sizes) + 1, labels))
    print(f"{len(sizes)} islands, sizes {sizes}, largest {max(sizes)}")
    letters = "ABCDEFGHIJ"
    for r in range(len(grid)):
        line = ""
        for c in range(len(grid[0])):
            line += letters[labels[(r, c)] - 1] if (r, c) in labels else "."
        print("  " + line)
    print("perimeter:", perimeter(grid))
    for k in sorted(set(labels.values())):
        cells = sorted(cell for cell in labels if labels[cell] == k)
        print(f"  island {letters[k - 1]}: first {cells[0]} last {cells[-1]}")


main()
