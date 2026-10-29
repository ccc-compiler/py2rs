def bucketize(values: list[int], lo: int, width: int, n: int) -> list[int]:
    slots: list[int] = []
    for v in values:
        idx = (v - lo) // width
        if idx < 0:
            idx = 0
        if idx >= n:
            idx = n - 1
        slots.append(idx)
    return [slots.count(i) for i in range(n)]


def horizontal(counts: list[int], lo: int, width: int) -> None:
    peak = max(counts)
    for i, c in enumerate(counts):
        a = lo + i * width
        b = a + width - 1
        bar = "*" * (c * 40 // peak) if peak > 0 else ""
        print(f"{a:3d}-{b:<3d} |{bar:<40}| {c}")


def vertical(counts: list[int]) -> None:
    peak = max(counts)
    for level in range(peak, 0, -1):
        row = ""
        for c in counts:
            row += " # " if c >= level else "   "
        print(f"{level:2d} |{row}")
    print("   +" + "---" * len(counts))
    print("    " + "".join(f"{i:^3}" for i in range(len(counts))))


def lcg(seed: int, n: int) -> list[int]:
    out: list[int] = []
    x = seed
    for _ in range(n):
        x = (x * 1103515245 + 12345) % 2147483648
        out.append(x)
    return out


def main() -> None:
    raw = lcg(42, 200)
    values = [(r % 50 + (r // 7) % 50) for r in raw]
    print("n =", len(values), "min =", min(values), "max =", max(values))
    counts = bucketize(values, 0, 10, 10)
    print(counts, sum(counts))
    horizontal(counts, 0, 10)
    print()
    small = bucketize([v // 10 for v in values], 0, 1, 10)
    vertical([c // 4 for c in small])
    rolls = [r % 6 + 1 + (r // 6) % 6 + 1 for r in raw]
    dice = [rolls.count(t) for t in range(13)]
    for total in range(2, 13):
        print(f"{total:2}: {'o' * dice[total]}")


main()
