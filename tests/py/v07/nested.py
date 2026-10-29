def count_islands(grid: list[list[str]]) -> int:
    rows = len(grid)
    cols = len(grid[0])
    seen: set[tuple[int, int]] = set()

    def dfs(r: int, c: int) -> None:
        if r < 0 or c < 0 or r >= rows or c >= cols:
            return
        if grid[r][c] != "#" or (r, c) in seen:
            return
        seen.add((r, c))
        dfs(r + 1, c)
        dfs(r - 1, c)
        dfs(r, c + 1)
        dfs(r, c - 1)

    n = 0
    for r in range(rows):
        for c in range(cols):
            if grid[r][c] == "#" and (r, c) not in seen:
                dfs(r, c)
                n += 1
    return n


def counter_demo() -> int:
    count = 0

    def bump(k: int) -> None:
        nonlocal count
        count += k

    for i in range(5):
        bump(i)
    bump(10)
    return count


def make_report(items: list[str]) -> list[str]:
    prefix = "*"
    out: list[str] = []

    def fmt(x: str) -> str:
        return f"{prefix} {x.upper()}"

    def add_all() -> None:
        for it in items:
            out.append(fmt(it))

    add_all()
    prefix = "-"
    out.append(fmt("late"))
    return out


def fib_memo(n: int) -> int:
    memo: dict[int, int] = {}

    def go(k: int) -> int:
        if k < 2:
            return k
        if k in memo:
            return memo[k]
        v = go(k - 1) + go(k - 2)
        memo[k] = v
        return v

    return go(n)


def main() -> None:
    grid = [list("##..#"), list("#...#"), list("..#.."), list("....#")]
    print(count_islands(grid))
    print(counter_demo())
    print(make_report(["a", "b"]))
    print(fib_memo(60))


if __name__ == "__main__":
    main()


def best_path(weights: list[int]) -> list[int]:
    best: list[int] = []
    total = -1

    def consider(path: list[int]) -> None:
        nonlocal best, total
        s = sum(path)
        if s > total:
            total = s
            best = path.copy()

    for i in range(len(weights)):
        consider(weights[: i + 1])
    return best


print(best_path([3, -1, 4, -10, 2]))
