def to_roman(n: int) -> str:
    vals = [1000, 900, 500, 400, 100, 90, 50, 40, 10, 9, 5, 4, 1]
    syms = ["M", "CM", "D", "CD", "C", "XC", "L", "XL", "X", "IX", "V", "IV", "I"]
    out = ""
    for v, s in zip(vals, syms):
        while n >= v:
            out += s
            n -= v
    return out


def from_roman(s: str) -> int:
    val = {"I": 1, "V": 5, "X": 10, "L": 50, "C": 100, "D": 500, "M": 1000}
    total = 0
    for i in range(len(s)):
        v = val[s[i]]
        if i + 1 < len(s) and val[s[i + 1]] > v:
            total -= v
        else:
            total += v
    return total


def main() -> None:
    years = [1066, 1492, 1776, 1999, 2026, 3888, 4, 49, 944]
    print(f"{'arabic':>6} | {'roman':<16} | back")
    for y in years:
        r = to_roman(y)
        print(f"{y:>6} | {r:<16} | {from_roman(r)}")
    lengths: dict[int, int] = {}
    longest = 0
    for n in range(1, 400):
        k = len(to_roman(n))
        lengths[k] = lengths.get(k, 0) + 1
        if k > len(to_roman(longest)) or longest == 0:
            longest = n
    for k in sorted(lengths):
        print(f"len {k:2}: {lengths[k]:3} {'#' * (lengths[k] // 4)}")
    print("first longest below 400:", longest, to_roman(longest))
    total = sum(from_roman(r) for r in ["XII", "XLII", "MCMXC", "CDXLIV"])
    print("sum:", total, "=", to_roman(total))


main()
