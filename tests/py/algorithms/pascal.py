def pascal(n: int) -> list[list[int]]:
    rows: list[list[int]] = [[1]]
    for i in range(1, n):
        prev = rows[-1]
        row = [1]
        for j in range(1, i):
            row.append(prev[j - 1] + prev[j])
        row.append(1)
        rows.append(row)
    return rows


def binom(n: int, k: int) -> int:
    if k < 0 or k > n:
        return 0
    k = min(k, n - k)
    r = 1
    for i in range(1, k + 1):
        r = r * (n - k + i) // i
    return r


def catalan(n: int) -> int:
    return binom(2 * n, n) // (n + 1)


def stirling2(n: int, k: int) -> int:
    t = [[0] * (k + 1) for _ in range(n + 1)]
    t[0][0] = 1
    for i in range(1, n + 1):
        for j in range(1, k + 1):
            t[i][j] = j * t[i - 1][j] + t[i - 1][j - 1]
    return t[n][k]


def main() -> None:
    rows = pascal(12)
    width = len(" ".join(str(x) for x in rows[-1]))
    for row in rows:
        print(" ".join(str(x) for x in row).center(width))
    print("row sums:", [sum(r) for r in rows])
    for n, k in [(5, 2), (10, 3), (20, 10), (52, 5), (60, 30), (7, 9)]:
        print(f"C({n},{k}) = {binom(n, k)}")
    print("catalan:", [catalan(i) for i in range(15)])
    print("stirling S(n,k) for n=6:", [stirling2(6, k) for k in range(7)])
    odd = ["".join("*" if x % 2 else " " for x in row) for row in pascal(16)]
    for line in odd:
        print("|" + line.rstrip())


main()
