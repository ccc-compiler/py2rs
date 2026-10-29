def edit_distance(a: str, b: str) -> int:
    n = len(a)
    m = len(b)
    prev = list(range(m + 1))
    for i in range(1, n + 1):
        cur = [i] + [0] * m
        for j in range(1, m + 1):
            cost = 0 if a[i - 1] == b[j - 1] else 1
            cur[j] = min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + cost)
        prev = cur
    return prev[m]


def edit_ops(a: str, b: str) -> list[str]:
    n = len(a)
    m = len(b)
    dp = [[0] * (m + 1) for _ in range(n + 1)]
    for i in range(n + 1):
        dp[i][0] = i
    for j in range(m + 1):
        dp[0][j] = j
    for i in range(1, n + 1):
        for j in range(1, m + 1):
            if a[i - 1] == b[j - 1]:
                dp[i][j] = dp[i - 1][j - 1]
            else:
                dp[i][j] = 1 + min(dp[i - 1][j], dp[i][j - 1], dp[i - 1][j - 1])
    ops: list[str] = []
    i = n
    j = m
    while i > 0 or j > 0:
        if i > 0 and j > 0 and a[i - 1] == b[j - 1]:
            ops.append(f"keep {a[i - 1]}")
            i -= 1
            j -= 1
        elif i > 0 and j > 0 and dp[i][j] == dp[i - 1][j - 1] + 1:
            ops.append(f"sub {a[i - 1]}->{b[j - 1]}")
            i -= 1
            j -= 1
        elif i > 0 and dp[i][j] == dp[i - 1][j] + 1:
            ops.append(f"del {a[i - 1]}")
            i -= 1
        else:
            ops.append(f"ins {b[j - 1]}")
            j -= 1
    ops.reverse()
    return ops


def closest(word: str, vocab: list[str]) -> str:
    return min(vocab, key=lambda v: (edit_distance(word, v), v))


def main() -> None:
    pairs = [("kitten", "sitting"), ("flaw", "lawn"), ("intention", "execution"), ("", "abc"), ("same", "same")]
    for a, b in pairs:
        d = edit_distance(a, b)
        print(f"{a!r} -> {b!r}: {d}")
        print("   ", ", ".join(edit_ops(a, b)))
    vocab = ["apple", "apply", "ample", "maple", "angle", "ankle"]
    for w in ["appel", "mapel", "angel", "xyz"]:
        print(f"closest to {w}: {closest(w, vocab)}")


main()
