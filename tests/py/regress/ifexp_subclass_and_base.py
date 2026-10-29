# regression: BUG: a conditional expression choosing between a subclass and its base class (or two
class Order:
    def __init__(self, oid: int) -> None:
        self.oid = oid

    def total(self) -> float:
        return 10.0


class Special(Order):
    def total(self) -> float:
        return 20.0


def make(oid: int, special: bool) -> Order:
    o: Order = Special(oid) if special else Order(oid)
    return o


def main() -> None:
    print(make(1, True).total(), make(2, False).total())


main()
