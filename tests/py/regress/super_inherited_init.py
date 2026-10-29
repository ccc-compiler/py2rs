# regression: BUG: Puppy inherits Dog.__init__; inside it super() should mean Animal, but the
class Animal:
    def __init__(self, name: str, legs: int) -> None:
        self.name = name
        self.legs = legs


class Dog(Animal):
    def __init__(self, name: str, breed: str) -> None:
        super().__init__(name, 4)
        self.breed = breed


class Puppy(Dog):
    def sound(self) -> str:
        return "yip"


def main() -> None:
    p = Puppy("bit", "corgi")
    print(p.name, p.legs, p.breed, p.sound())


main()
