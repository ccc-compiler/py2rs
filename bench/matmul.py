def matmul(a: list[list[float]], b: list[list[float]]) -> list[list[float]]:
    n = len(a)
    m = len(b[0])
    k = len(b)
    c = [[0.0] * m for _ in range(n)]
    for i in range(n):
        ai = a[i]
        ci = c[i]
        for p in range(k):
            aip = ai[p]
            bp = b[p]
            for j in range(m):
                ci[j] += aip * bp[j]
    return c


def main() -> None:
    n = 220
    a = [[(i * j % 7) / 7.0 for j in range(n)] for i in range(n)]
    b = [[(i + j) % 5 / 5.0 for j in range(n)] for i in range(n)]
    c = matmul(a, b)
    print(round(sum(sum(r) for r in c), 4))


if __name__ == "__main__":
    main()
