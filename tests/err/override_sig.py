# error: must take and return the same types
class A:
    def f(self, x: int) -> int:
        return x


class B(A):
    def f(self, x: str) -> int:
        return len(x)


items: list[A] = [A(), B()]
print(items[0].f(1))
