# regression: BUG (runtime: stack overflow, "fatal runtime error"): C.label -> super (B.label) ->
class A:
    def __init__(self, n: str) -> None:
        self.n = n

    def label(self) -> str:
        return self.n


class B(A):
    def label(self) -> str:
        return super().label() + "!"


class C(B):
    def label(self) -> str:
        return super().label() + "?"


def main() -> None:
    print(A("a").label(), B("b").label(), C("c").label())


main()
