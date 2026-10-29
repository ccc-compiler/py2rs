def mat_mul(a: list[list[int]], b: list[list[int]]) -> list[list[int]]:
    n = len(a)
    m = len(b[0])
    k = len(b)
    return [[sum(a[i][t] * b[t][j] for t in range(k)) for j in range(m)] for i in range(n)]


def transpose(a: list[list[int]]) -> list[list[int]]:
    return [[a[i][j] for i in range(len(a))] for j in range(len(a[0]))]


def identity(n: int) -> list[list[int]]:
    return [[1 if i == j else 0 for j in range(n)] for i in range(n)]


def show(name: str, a: list[list[int]]) -> None:
    w = max(len(str(x)) for row in a for x in row)
    print(f"{name} ({len(a)}x{len(a[0])}):")
    for row in a:
        print("  [" + " ".join(str(x).rjust(max(w, 3)) for x in row) + "]")


def det(a: list[list[int]]) -> int:
    n = len(a)
    if n == 1:
        return a[0][0]
    if n == 2:
        return a[0][0] * a[1][1] - a[0][1] * a[1][0]
    total = 0
    for c in range(n):
        minor = [[a[r][cc] for cc in range(n) if cc != c] for r in range(1, n)]
        sign = 1 if c % 2 == 0 else -1
        total += sign * a[0][c] * det(minor)
    return total


def power(a: list[list[int]], e: int) -> list[list[int]]:
    result = identity(len(a))
    base = [row.copy() for row in a]
    while e > 0:
        if e % 2 == 1:
            result = mat_mul(result, base)
        base = mat_mul(base, base)
        e //= 2
    return result


def main() -> None:
    a = [[1, 2, 3], [4, 5, 6]]
    b = [[7, 8], [9, 10], [11, 12]]
    show("A", a)
    show("B", b)
    show("AB", mat_mul(a, b))
    show("BA", mat_mul(b, a))
    show("A^T", transpose(a))
    m = [[2, -1, 0], [1, 3, 4], [0, 5, -2]]
    print("det =", det(m))
    print("det(I4) =", det(identity(4)))
    fib = power([[1, 1], [1, 0]], 30)
    print("fib(30) =", fib[0][1])
    show("F^10", power([[1, 1], [1, 0]], 10))
    sym = mat_mul(a, transpose(a))
    print("symmetric:", sym == transpose(sym))
    trace = sum(sym[i][i] for i in range(len(sym)))
    print("trace:", trace)
    col_sums = [sum(col) for col in transpose(b)]
    row_sums = [sum(row) for row in b]
    print("col sums", col_sums, "row sums", row_sums)


main()
