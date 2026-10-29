# regression: BUG (Rust compile error): a subclass holding a field of its base class type (an
class Expr:
    def eval(self) -> int:
        return 0


class Num(Expr):
    def __init__(self, v: int) -> None:
        self.v = v

    def eval(self) -> int:
        return self.v


class Neg(Expr):
    def __init__(self, inner: Expr) -> None:
        self.inner = inner

    def eval(self) -> int:
        return -self.inner.eval()


def main() -> None:
    e = Neg(Neg(Num(5)))
    print(e.eval())


main()
