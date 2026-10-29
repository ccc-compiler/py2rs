class Rng:
    def __init__(self, seed: int) -> None:
        self.s = seed

    def next(self, n: int) -> int:
        self.s = (self.s * 1664525 + 1013904223) % 4294967296
        return (self.s >> 8) % n


class Creature:
    def __init__(self, name: str, x: int, energy: int) -> None:
        self.name = name
        self.x = x
        self.energy = energy
        self.alive = True
        self.age = 0

    def kind(self) -> str:
        return "creature"

    def speed(self) -> int:
        return 1

    def cost(self) -> int:
        return 1

    def tick(self, rng: Rng, width: int) -> None:
        if not self.alive:
            return
        self.age += 1
        step = rng.next(2 * self.speed() + 1) - self.speed()
        self.x = (self.x + step) % width
        self.energy -= self.cost()
        if self.energy <= 0:
            self.alive = False

    def symbol(self) -> str:
        return "?"


class Rabbit(Creature):
    def kind(self) -> str:
        return "rabbit"

    def graze(self, grass: int) -> int:
        if grass > 0 and self.alive:
            self.energy += 3
            return 1
        return 0

    def symbol(self) -> str:
        return "r"


class Fox(Creature):
    def kind(self) -> str:
        return "fox"

    def speed(self) -> int:
        return 2

    def cost(self) -> int:
        return 2

    def symbol(self) -> str:
        return "F"


def main() -> None:
    width = 20
    rng = Rng(99)
    rabbits = [Rabbit(f"r{i}", (i * 7) % width, 6) for i in range(6)]
    foxes = [Fox(f"f{i}", (i * 11 + 3) % width, 12) for i in range(2)]
    grass: dict[int, int] = {x: 2 for x in range(width)}
    for day in range(1, 13):
        for r in rabbits:
            r.tick(rng, width)
            eaten = r.graze(grass[r.x])
            grass[r.x] -= eaten
        for f in foxes:
            f.tick(rng, width)
        for f in foxes:
            for r in rabbits:
                if f.alive and r.alive and abs(f.x - r.x) <= 1:
                    r.alive = False
                    f.energy += 8
                    print(f"day {day}: {f.name} catches {r.name} at {r.x}")
        grass = {x: min(g + (1 if day % 3 == 0 else 0), 3) for x, g in grass.items()}
        occupied: dict[int, str] = {}
        for r in rabbits:
            if r.alive:
                occupied[r.x] = r.symbol()
        for f in foxes:
            if f.alive:
                occupied[f.x] = f.symbol()
        row = [occupied.get(x, ".") for x in range(width)]
        live_r = sum(1 for r in rabbits if r.alive)
        live_f = sum(1 for f in foxes if f.alive)
        print(f"day {day:2} {''.join(row)} rabbits={live_r} foxes={live_f} grass={sum(grass.values())}")
    survivors = [f"{c.name}:{c.energy}" for c in rabbits if c.alive] + [f"{c.name}:{c.energy}" for c in foxes if c.alive]
    print("survivors", survivors)
    oldest = max(rabbits, key=lambda r: (r.age if r.alive else -1, r.name))
    print("oldest rabbit", oldest.name, oldest.age, oldest.kind())


main()
