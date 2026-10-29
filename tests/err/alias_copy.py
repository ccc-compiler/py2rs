# error: share one object
class E:
    def __init__(self, name: str) -> None:
        self.name = name


class H(E):
    def __init__(self, name: str) -> None:
        super().__init__(name)
        self.hours: list[float] = []

    def log(self, h: float) -> None:
        self.hours.append(h)


def main() -> None:
    staff: list[E] = [H("a"), H("b")]
    for i in range(len(staff)):
        e = staff[i]
        if isinstance(e, H) and i == 0:
            e.log(1.0)
    s = staff[0]
    if isinstance(s, H):
        print(len(s.hours))


main()
