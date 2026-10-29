from dataclasses import dataclass


@dataclass(frozen=True)
class Pos:
    x: int
    y: int

    def step(self, d: str) -> "Pos":
        if d == "N":
            return Pos(self.x, self.y + 1)
        if d == "S":
            return Pos(self.x, self.y - 1)
        if d == "E":
            return Pos(self.x + 1, self.y)
        return Pos(self.x - 1, self.y)


class Robot:
    def __init__(self, name: str, x: int, y: int, facing: str) -> None:
        self.name = name
        self.pos = Pos(x, y)
        self.facing = facing
        self.trail: list[Pos] = [Pos(x, y)]
        self.bumps = 0

    def turn(self, c: str) -> None:
        dirs = "NESW"
        i = dirs.index(self.facing)
        if c == "R":
            self.facing = dirs[(i + 1) % 4]
        else:
            self.facing = dirs[(i + 3) % 4]

    def forward(self, arena: "Arena") -> None:
        nxt = self.pos.step(self.facing)
        if arena.blocked(nxt, self.name):
            self.bumps += 1
            return
        self.pos = nxt
        self.trail.append(nxt)

    def run(self, cmd: str, arena: "Arena") -> None:
        if cmd == "F":
            self.forward(arena)
        else:
            self.turn(cmd)

    def __str__(self) -> str:
        return f"{self.name}@({self.pos.x},{self.pos.y}){self.facing} bumps={self.bumps}"


class Arena:
    def __init__(self, w: int, h: int, walls: list[Pos]) -> None:
        self.w = w
        self.h = h
        self.walls = walls.copy()
        self.where: dict[str, Pos] = {}

    def blocked(self, p: Pos, who: str) -> bool:
        if p.x < 0 or p.y < 0 or p.x >= self.w or p.y >= self.h:
            return True
        if p in self.walls:
            return True
        for name, q in self.where.items():
            if name != who and q == p:
                return True
        return False

    def draw(self, robots: list[Robot]) -> None:
        for y in range(self.h - 1, -1, -1):
            line = ""
            for x in range(self.w):
                p = Pos(x, y)
                ch = "."
                if p in self.walls:
                    ch = "#"
                for r in robots:
                    if r.pos == p:
                        ch = r.name[0]
                line += ch
            print(line)


def main() -> None:
    arena = Arena(8, 5, [Pos(3, 1), Pos(3, 2), Pos(3, 3), Pos(6, 0)])
    robots = [Robot("Alpha", 0, 0, "N"), Robot("Beta", 7, 4, "W"), Robot("Gamma", 5, 2, "S")]
    programs = ["FFRFFFFLFRFF", "FFFFLFFRFF", "FFRRFFFLFFF"]
    for r in robots:
        arena.where[r.name] = r.pos
    arena.draw(robots)
    longest = max(len(p) for p in programs)
    for t in range(longest):
        for i, r in enumerate(robots):
            if t < len(programs[i]):
                r.run(programs[i][t], arena)
                arena.where[r.name] = r.pos
        if t % 4 == 3:
            print(f"after step {t + 1}: " + "; ".join(str(r) for r in robots))
    arena.draw(robots)
    for r in robots:
        distinct = len(set((p.x, p.y) for p in r.trail))
        print(r.name, "visited", distinct, "cells, trail length", len(r.trail))


main()
