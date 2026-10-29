class Matrix:
    def __init__(self, rows: list[list[float]]) -> None:
        self.n = len(rows)
        self.m = len(rows[0]) if rows else 0
        self.rows = [r.copy() for r in rows]

    @staticmethod
    def identity(n: int) -> "Matrix":
        return Matrix([[1.0 if i == j else 0.0 for j in range(n)] for i in range(n)])

    @staticmethod
    def zeros(n: int, m: int) -> "Matrix":
        return Matrix([[0.0 for _ in range(m)] for _ in range(n)])

    def get(self, i: int, j: int) -> float:
        return self.rows[i][j]

    def plus(self, o: "Matrix") -> "Matrix":
        return Matrix([[self.rows[i][j] + o.rows[i][j] for j in range(self.m)] for i in range(self.n)])

    def times(self, o: "Matrix") -> "Matrix":
        if self.m != o.n:
            raise ValueError(f"shape mismatch {self.n}x{self.m} * {o.n}x{o.m}")
        out: list[list[float]] = []
        for i in range(self.n):
            row: list[float] = []
            for j in range(o.m):
                row.append(sum(self.rows[i][k] * o.rows[k][j] for k in range(self.m)))
            out.append(row)
        return Matrix(out)

    def scale(self, k: float) -> "Matrix":
        return Matrix([[x * k for x in r] for r in self.rows])

    def transpose(self) -> "Matrix":
        return Matrix([[self.rows[i][j] for i in range(self.n)] for j in range(self.m)])

    def det(self) -> float:
        if self.n == 1:
            return self.rows[0][0]
        if self.n == 2:
            return self.rows[0][0] * self.rows[1][1] - self.rows[0][1] * self.rows[1][0]
        total = 0.0
        for c in range(self.m):
            minor = Matrix([[self.rows[r][k] for k in range(self.m) if k != c] for r in range(1, self.n)])
            sign = 1.0 if c % 2 == 0 else -1.0
            total += sign * self.rows[0][c] * minor.det()
        return total

    def power(self, e: int) -> "Matrix":
        result = Matrix.identity(self.n)
        base = Matrix([r.copy() for r in self.rows])
        while e > 0:
            if e % 2 == 1:
                result = result.times(base)
            base = base.times(base)
            e //= 2
        return result

    def __str__(self) -> str:
        lines = ["[" + " ".join(f"{x:8.2f}" for x in r) + "]" for r in self.rows]
        return "\n".join(lines)


def main() -> None:
    a = Matrix([[1.0, 2.0], [3.0, 4.0]])
    b = Matrix([[0.0, 1.0], [1.0, 0.0]])
    print(a)
    print(a.plus(b))
    print(a.times(b))
    print(a.transpose())
    print("det a", a.det())
    c = Matrix([[2.0, -1.0, 0.0], [1.0, 3.0, 2.0], [0.0, 1.0, 4.0]])
    print("det c", c.det())
    print(c.times(Matrix.identity(3)))
    print(Matrix.zeros(2, 3))
    fib = Matrix([[1.0, 1.0], [1.0, 0.0]])
    for e in [1, 5, 10, 20]:
        print(f"fib^{e}: {fib.power(e).get(0, 1):.0f}")
    try:
        a.times(c)
    except ValueError as err:
        print("error:", err)
    rot = Matrix([[0.0, -1.0], [1.0, 0.0]])
    v = Matrix([[3.0], [1.0]])
    for _ in range(4):
        v = rot.times(v)
        print(v.get(0, 0), v.get(1, 0))
    print(a.scale(0.5))


main()
