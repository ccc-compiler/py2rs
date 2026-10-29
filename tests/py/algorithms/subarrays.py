def kadane(xs: list[int]) -> tuple[int, int, int]:
    best = xs[0]
    best_lo = 0
    best_hi = 0
    cur = 0
    lo = 0
    for i in range(len(xs)):
        if cur <= 0:
            cur = xs[i]
            lo = i
        else:
            cur += xs[i]
        if cur > best:
            best = cur
            best_lo = lo
            best_hi = i
    return best, best_lo, best_hi


def two_sum_sorted(xs: list[int], target: int) -> tuple[int, int]:
    i = 0
    j = len(xs) - 1
    while i < j:
        s = xs[i] + xs[j]
        if s == target:
            return i, j
        if s < target:
            i += 1
        else:
            j -= 1
    return -1, -1


def longest_unique(s: str) -> str:
    last: dict[str, int] = {}
    start = 0
    best = ""
    for i in range(len(s)):
        c = s[i]
        if c in last and last[c] >= start:
            start = last[c] + 1
        last[c] = i
        if i - start + 1 > len(best):
            best = s[start:i + 1]
    return best


def prefix_sums(xs: list[int]) -> list[int]:
    out = [0]
    for x in xs:
        out.append(out[-1] + x)
    return out


def count_subarrays_with_sum(xs: list[int], k: int) -> int:
    seen: dict[int, int] = {0: 1}
    total = 0
    count = 0
    for x in xs:
        total += x
        count += seen.get(total - k, 0)
        seen[total] = seen.get(total, 0) + 1
    return count


def max_window(xs: list[int], k: int) -> list[int]:
    out: list[int] = []
    for i in range(len(xs) - k + 1):
        out.append(max(xs[i:i + k]))
    return out


def main() -> None:
    for xs in [[-2, 1, -3, 4, -1, 2, 1, -5, 4], [1, 2, 3], [-3, -1, -2], [5, -9, 6, -2, 3]]:
        b, lo, hi = kadane(xs)
        print(f"{xs}: best {b} = sum{xs[lo:hi + 1]}")
    nums = [1, 2, 4, 7, 11, 15, 20]
    for t in [15, 9, 35, 100]:
        print(f"two-sum {t}: {two_sum_sorted(nums, t)}")
    for s in ["abcabcbb", "bbbbb", "pwwkew", "", "dvdf"]:
        print(f"{s!r}: {longest_unique(s)!r}")
    ps = prefix_sums(nums)
    print("prefix:", ps)
    for lo, hi in [(0, 3), (2, 5), (1, 7)]:
        print(f"  sum[{lo}:{hi}] = {ps[hi] - ps[lo]} check {sum(nums[lo:hi])}")
    print("subarrays summing to 7:", count_subarrays_with_sum([3, 4, 7, 2, -3, 1, 4, 2], 7))
    print("window max:", max_window([1, 3, -1, -3, 5, 3, 6, 7], 3))


main()
