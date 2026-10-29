# regression: BUG (same root as super_inherited_init): a method using super() that a grandchild
class A:
    def __init__(self, n: str) -> None:
        self.n = n

    def label(self) -> str:
        return self.n


class B(A):
    def label(self) -> str:
        return super().label() + "!"


class C(B):
    def other(self) -> int:
        return 1


def main() -> None:
    print(B("b").label(), C("c").label())


main()
