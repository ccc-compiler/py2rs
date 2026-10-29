class Request:
    def __init__(self, time: int, origin: int, dest: int, who: str) -> None:
        self.time = time
        self.origin = origin
        self.dest = dest
        self.who = who

    def __repr__(self) -> str:
        return f"{self.who}({self.origin}->{self.dest})"


class Elevator:
    def __init__(self, name: str, floor: int, capacity: int) -> None:
        self.name = name
        self.floor = floor
        self.capacity = capacity
        self.riders: list[Request] = []
        self.direction = 0
        self.moved = 0

    def targets(self, waiting: list[Request]) -> list[int]:
        t = [r.dest for r in self.riders]
        if self.capacity > len(self.riders):
            t.extend(r.origin for r in waiting)
        return t

    def step(self, waiting: list[Request], now: int, log: list[str]) -> list[Request]:
        staying: list[Request] = []
        for r in self.riders:
            if r.dest == self.floor:
                log.append(f"t={now} {self.name}: {r.who} exits at {self.floor}")
            else:
                staying.append(r)
        self.riders = staying
        left: list[Request] = []
        for r in waiting:
            if r.origin == self.floor and self.capacity > len(self.riders):
                log.append(f"t={now} {self.name}: {r.who} enters at {self.floor}")
                self.riders.append(r)
            else:
                left.append(r)
        t = self.targets(left)
        if not t:
            self.direction = 0
        else:
            up = [x for x in t if x > self.floor]
            down = [x for x in t if x < self.floor]
            if self.direction >= 0 and len(up) > 0:
                self.direction = 1
            elif len(down) > 0:
                self.direction = -1
            elif len(up) > 0:
                self.direction = 1
            else:
                self.direction = 0
        self.floor += self.direction
        if self.direction != 0:
            self.moved += 1
        return left

    def __str__(self) -> str:
        arrow = {1: "^", -1: "v", 0: "-"}[self.direction]
        return f"{self.name}@{self.floor}{arrow}{self.riders}"


def main() -> None:
    reqs = [
        Request(0, 0, 5, "amy"), Request(1, 3, 0, "bo"), Request(2, 7, 1, "cy"),
        Request(2, 0, 2, "di"), Request(4, 4, 8, "ed"), Request(6, 8, 0, "flo"),
        Request(7, 2, 6, "gus"), Request(9, 5, 3, "hal"),
    ]
    a = Elevator("A", 0, 2)
    b = Elevator("B", 8, 3)
    waiting_a: list[Request] = []
    waiting_b: list[Request] = []
    log: list[str] = []
    for now in range(30):
        for r in reqs:
            if r.time == now:
                if abs(r.origin - a.floor) <= abs(r.origin - b.floor):
                    waiting_a.append(r)
                else:
                    waiting_b.append(r)
        waiting_a = a.step(waiting_a, now, log)
        waiting_b = b.step(waiting_b, now, log)
        if now % 3 == 0:
            print(f"t={now:2} {a}  {b}  wait={len(waiting_a) + len(waiting_b)}")
        if now > 10 and not waiting_a and not waiting_b and not a.riders and not b.riders:
            print("all served at", now)
            break
    for line in log:
        print(line)
    print("moves:", a.moved, b.moved)


main()
