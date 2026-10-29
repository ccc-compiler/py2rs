def matmul(a: list[list[int]], b: list[list[int]]) -> list[list[int]]:
    n = len(a)
    m = len(b[0])
    k = len(b)
    c = [[0] * m for _ in range(n)]
    for i in range(n):
        for j in range(m):
            s = 0
            for t in range(k):
                s += a[i][t] * b[t][j]
            c[i][j] = s
    return c


def identity(n: int) -> list[list[int]]:
    return [[1 if i == j else 0 for j in range(n)] for i in range(n)]


def matpow(a: list[list[int]], e: int) -> list[list[int]]:
    result = identity(len(a))
    base = [row.copy() for row in a]
    while e > 0:
        if e % 2 == 1:
            result = matmul(result, base)
        base = matmul(base, base)
        e //= 2
    return result


def transpose(a: list[list[int]]) -> list[list[int]]:
    return [[a[i][j] for i in range(len(a))] for j in range(len(a[0]))]


def det(a: list[list[float]]) -> float:
    n = len(a)
    m = [row.copy() for row in a]
    d = 1.0
    for col in range(n):
        piv = col
        for r in range(col + 1, n):
            if abs(m[r][col]) > abs(m[piv][col]):
                piv = r
        if abs(m[piv][col]) < 1e-12:
            return 0.0
        if piv != col:
            row = m.pop(piv)
            m.insert(col, row)
            other = m.pop(col + 1)
            m.insert(piv, other)
            d = -d
        d *= m[col][col]
        for r in range(col + 1, n):
            f = m[r][col] / m[col][col]
            for c in range(col, n):
                m[r][c] -= f * m[col][c]
    return d


def show(name: str, a: list[list[int]]) -> None:
    print(f"{name}:")
    for row in a:
        print("  [" + " ".join(f"{v:>6}" for v in row) + "]")


def main() -> None:
    a = [[1, 2, 3], [4, 5, 6]]
    b = [[7, 8], [9, 10], [11, 12]]
    show("A", a)
    show("B", b)
    show("A*B", matmul(a, b))
    show("B*A", matmul(b, a))
    show("A^T", transpose(a))
    fib = [[1, 1], [1, 0]]
    for n in [1, 5, 10, 30, 50, 63]:
        print(f"fib({n}) = {matpow(fib, n)[0][1]}")
    show("I3", identity(3))
    mats = [
        [[2.0, 0.0], [0.0, 3.0]],
        [[1.0, 2.0], [3.0, 4.0]],
        [[0.0, 1.0, 2.0], [3.0, 4.0, 5.0], [6.0, 7.0, 9.0]],
        [[1.0, 2.0, 3.0], [2.0, 4.0, 6.0], [1.0, 0.0, 1.0]],
    ]
    for mm in mats:
        print(f"det{mm} = {det(mm):.4f}")


main()
