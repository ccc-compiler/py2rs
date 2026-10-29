def lcs_table(a: str, b: str) -> list[list[int]]:
    n = len(a)
    m = len(b)
    dp = [[0] * (m + 1) for _ in range(n + 1)]
    for i in range(1, n + 1):
        for j in range(1, m + 1):
            if a[i - 1] == b[j - 1]:
                dp[i][j] = dp[i - 1][j - 1] + 1
            else:
                dp[i][j] = max(dp[i - 1][j], dp[i][j - 1])
    return dp


def lcs_string(a: str, b: str, dp: list[list[int]]) -> str:
    i = len(a)
    j = len(b)
    out: list[str] = []
    while i > 0 and j > 0:
        if a[i - 1] == b[j - 1]:
            out.append(a[i - 1])
            i -= 1
            j -= 1
        elif dp[i - 1][j] >= dp[i][j - 1]:
            i -= 1
        else:
            j -= 1
    out.reverse()
    return "".join(out)


def print_table(a: str, b: str, dp: list[list[int]]) -> None:
    print("    " + " ".join(f"{c:>2}" for c in " " + b))
    for i in range(len(dp)):
        label = a[i - 1] if i > 0 else " "
        print(f"{label:>2}  " + " ".join(f"{v:>2}" for v in dp[i]))


def lis(xs: list[int]) -> list[int]:
    n = len(xs)
    length = [1] * n
    prev = [-1] * n
    for i in range(n):
        for j in range(i):
            if xs[j] < xs[i] and length[j] + 1 > length[i]:
                length[i] = length[j] + 1
                prev[i] = j
    if n == 0:
        return []
    best = 0
    for i in range(n):
        if length[i] > length[best]:
            best = i
    seq: list[int] = []
    k = best
    while k != -1:
        seq.append(xs[k])
        k = prev[k]
    return seq[::-1]


def main() -> None:
    pairs = [("ABCBDAB", "BDCABA"), ("AGGTAB", "GXTXAYB"), ("kitten", "sitting"), ("", "abc")]
    for a, b in pairs:
        dp = lcs_table(a, b)
        s = lcs_string(a, b, dp)
        print(f"LCS({a!r}, {b!r}) = {dp[len(a)][len(b)]} {s!r}")
    a, b = pairs[0]
    print_table(a, b, lcs_table(a, b))
    for xs in [[10, 9, 2, 5, 3, 7, 101, 18], [0, 8, 4, 12, 2, 10, 6, 14, 1, 9], [5, 4, 3]]:
        s2 = lis(xs)
        print(f"LIS of {xs}: {s2} (len {len(s2)})")


main()
