# regression: BUG / severe limitation: a plain class __init__ cannot store a mutable parameter (list,
class W:
    def __init__(self, xs: list[int]) -> None:
        self.xs = xs


def main() -> None:
    w = W([1, 2])
    ys = [3]
    v = W(ys)
    print(w.xs, v.xs)


main()
