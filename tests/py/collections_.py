def group_by_length(words: list[str]) -> dict[int, list[str]]:
    groups: dict[int, list[str]] = {}
    for w in words:
        groups.setdefault(len(w), []).append(w)
    return groups


def transpose(m: list[list[int]]) -> list[list[int]]:
    return [[m[r][c] for r in range(len(m))] for c in range(len(m[0]))]


def add_one(xs: list[int]) -> None:
    for i in range(len(xs)):
        xs[i] += 1


def double_all(grid: list[list[int]]) -> None:
    for row in grid:
        for j in range(len(row)):
            row[j] *= 2


def main() -> None:
    words = ["apple", "fig", "kiwi", "plum", "banana", "pear"]
    g = group_by_length(words)
    for k in sorted(g):
        print(k, g[k])
    m = [[1, 2, 3], [4, 5, 6]]
    print(transpose(m))
    xs = [3, 1, 2]
    add_one(xs)
    print(xs)
    double_all(m)
    print(m)
    grid = [[0] * 3 for _ in range(2)]
    grid[1][2] = 7
    print(grid)
    seen: set[int] = set()
    for n in [3, 1, 3, 2, 1]:
        seen.add(n)
    print(sorted(seen), 3 in seen, 5 in seen, len(seen))
    squares = {n: n * n for n in range(5)}
    print(squares)
    evens = {n for n in range(10) if n % 2 == 0}
    print(sorted(evens))
    pairs = list(zip([1, 2, 3], ["a", "b", "c"]))
    print(pairs)
    for i, w in enumerate(words[:3], 1):
        print(i, w)
    nums = [5, 3, 8, 1]
    nums.sort()
    print(nums, nums.index(8))
    nums.sort(reverse=True)
    print(nums)
    nums.insert(0, 42)
    nums.remove(3)
    print(nums, nums.pop(), nums.pop(0), nums)
    d = {"a": 1, "b": 2}
    d["c"] = 3
    del d["a"]
    print(d, list(d.keys()), list(d.values()), "b" in d)
    print(any(x > 7 for x in [1, 9]), all(x > 0 for x in [1, -1]))
    stack = [1, 2, 3]
    while stack:
        print(stack.pop(), end=",")
    print()
    t = (1, "two", 3.0)
    a, b, c = t
    print(t, a, b, c, t[1])
    matrix = [[i * j for j in range(4)] for i in range(4)]
    print(sum(sum(row) for row in matrix))
    flat = [x for row in matrix for x in row if x % 2 == 1]
    print(flat)


main()
