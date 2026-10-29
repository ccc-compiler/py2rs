class Animal:
    def __init__(self, name: str, legs: int) -> None:
        self.name = name
        self.legs = legs
        self.energy = 10

    def sound(self) -> str:
        return "..."

    def move(self) -> str:
        return "walks"

    def act(self) -> str:
        self.energy -= 2
        return f"{self.name} {self.move()} and says {self.sound()} (energy {self.energy})"

    def eat(self, amount: int) -> None:
        self.energy += amount

    def fetch(self, times: int) -> int:
        return 0

    def __repr__(self) -> str:
        return f"Animal({self.name!r}, legs={self.legs})"


class Dog(Animal):
    def __init__(self, name: str, breed: str) -> None:
        super().__init__(name, 4)
        self.breed = breed

    def sound(self) -> str:
        return "woof"

    def fetch(self, times: int) -> int:
        done = 0
        while self.energy > 3 and done < times:
            self.energy -= 3
            done += 1
        return done


class Puppy(Dog):
    def __init__(self, name: str, breed: str) -> None:
        super().__init__(name, breed)
        self.energy = 14

    def sound(self) -> str:
        return "yip " + super().sound()

    def act(self) -> str:
        base = super().act()
        return base + " and wags its tail"


class Bird(Animal):
    def __init__(self, name: str, can_fly: bool) -> None:
        super().__init__(name, 2)
        self.can_fly = can_fly

    def sound(self) -> str:
        return "tweet"

    def move(self) -> str:
        if self.can_fly:
            return "flies"
        return "waddles"


class Fish(Animal):
    def __init__(self, name: str) -> None:
        super().__init__(name, 0)

    def move(self) -> str:
        return "swims"


def zoo_report(animals: list[Animal]) -> None:
    legs = sum(a.legs for a in animals)
    print(f"{len(animals)} animals, {legs} legs")
    flyers: list[str] = []
    for a in animals:
        if isinstance(a, Bird) and a.can_fly:
            flyers.append(a.name)
    print("flyers:", flyers)


def main() -> None:
    zoo: list[Animal] = [
        Dog("rex", "beagle"),
        Puppy("bit", "corgi"),
        Bird("tweety", True),
        Bird("pingu", False),
        Fish("nemo"),
    ]
    for round_no in range(3):
        print(f"--- round {round_no} ---")
        for a in zoo:
            print(a.act())
        zoo[round_no].eat(5)
    zoo_report(zoo)
    fetched: list[int] = []
    for a in zoo:
        fetched.append(a.fetch(5))
    for i, a in enumerate(zoo):
        if isinstance(a, Dog):
            print(f"{a.name} the {a.breed} fetched {fetched[i]} times, energy {a.energy}")
    print(zoo)
    tired = sorted(zoo, key=lambda a: a.energy)
    for a in tired:
        print(f"{a.name:<8}{a.energy:>4}")
    print(min(a.energy for a in zoo), max(a.energy for a in zoo))


main()
