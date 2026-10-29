# regression: BUG: isinstance narrowing is not applied in a comprehension filter (also without `and`;
class Animal:
    def __init__(self, name: str) -> None:
        self.name = name


class Bird(Animal):
    def __init__(self, name: str, can_fly: bool) -> None:
        super().__init__(name)
        self.can_fly = can_fly


def main() -> None:
    zoo: list[Animal] = [Bird("a", True), Bird("b", False), Animal("c")]
    print([a.name for a in zoo if isinstance(a, Bird) and a.can_fly])
    for a in zoo:
        if isinstance(a, Bird) and a.can_fly:
            print("flies:", a.name)


main()
