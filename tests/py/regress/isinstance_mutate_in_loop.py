# regression: BUG: mutating an element (not the list) after isinstance narrowing inside a for loop
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
    staff: list[E] = [H("a"), E("b")]
    for e in staff:
        if isinstance(e, H):
            e.log(1.0)
    print(len(staff))


main()
