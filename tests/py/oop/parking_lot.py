from dataclasses import dataclass


@dataclass(frozen=True)
class Car:
    plate: str
    size: int


@dataclass
class Spot:
    number: int
    size: int
    plate: str = ""
    since: int = 0

    def free(self) -> bool:
        return self.plate == ""


class ParkingLot:
    def __init__(self, sizes: list[int], rate: int) -> None:
        self.spots: dict[int, Spot] = {}
        for i, s in enumerate(sizes):
            self.spots[i + 1] = Spot(i + 1, s)
        self.rate = rate
        self.income = 0
        self.turned_away: list[str] = []

    def park(self, car: Car, now: int) -> int | None:
        best = -1
        for i, s in self.spots.items():
            if s.free() and s.size >= car.size:
                if best < 0 or s.size < self.spots[best].size:
                    best = i
        if best < 0:
            self.turned_away.append(car.plate)
            return None
        self.spots[best] = Spot(self.spots[best].number, self.spots[best].size, car.plate, now)
        return self.spots[best].number

    def leave(self, plate: str, now: int) -> int:
        found = -1
        for i, s in self.spots.items():
            if s.plate == plate:
                found = i
        if found < 0:
            return 0
        s = self.spots[found]
        hours = max(1, (now - s.since + 59) // 60)
        fee = hours * self.rate * s.size
        self.income += fee
        self.spots[found] = Spot(s.number, s.size)
        return fee

    def occupancy(self) -> str:
        return "".join("." if s.free() else str(s.size) for s in self.spots.values())


def main() -> None:
    lot = ParkingLot([1, 1, 2, 2, 3, 1, 2], 2)
    cars = [Car("AB-1", 1), Car("CD-2", 2), Car("EF-3", 3), Car("GH-4", 1), Car("IJ-5", 3),
            Car("KL-6", 2), Car("MN-7", 1), Car("OP-8", 2), Car("QR-9", 1)]
    t = 0
    for c in cars:
        t += 17
        n = lot.park(c, t)
        if n is None:
            print(f"t={t:4} {c.plate} size {c.size}: lot full")
        else:
            print(f"t={t:4} {c.plate} size {c.size}: spot {n}  [{lot.occupancy()}]")
    for plate in ["CD-2", "AB-1", "ZZ-0", "EF-3"]:
        t += 45
        fee = lot.leave(plate, t)
        print(f"t={t:4} {plate} leaves, pays {fee}  [{lot.occupancy()}]")
    for c in [Car("IJ-5", 3), Car("ST-1", 2)]:
        t += 5
        print(c.plate, "->", lot.park(c, t), lot.occupancy())
    print("income", lot.income)
    print("turned away", lot.turned_away)
    print([s for s in lot.spots.values() if not s.free()][:2])


main()
