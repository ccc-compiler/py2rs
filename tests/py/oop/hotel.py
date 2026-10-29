from dataclasses import dataclass


@dataclass(frozen=True)
class Booking:
    guest: str
    room: int
    start: int
    nights: int

    def end(self) -> int:
        return self.start + self.nights


class Room:
    def __init__(self, number: int, rate: int) -> None:
        self.number = number
        self.rate = rate
        self.bookings: list[Booking] = []

    def kind(self) -> str:
        return "room"

    def capacity(self) -> int:
        return 1

    def free(self, start: int, nights: int) -> bool:
        for b in self.bookings:
            if start < b.end() and b.start < start + nights:
                return False
        return True

    def price(self, nights: int) -> int:
        return self.rate * nights


class Single(Room):
    def kind(self) -> str:
        return "single"


class Double(Room):
    def kind(self) -> str:
        return "double"

    def capacity(self) -> int:
        return 2


class Suite(Double):
    def __init__(self, number: int, rate: int, butler: bool) -> None:
        super().__init__(number, rate)
        self.butler = butler

    def kind(self) -> str:
        return "suite+butler" if self.butler else "suite"

    def capacity(self) -> int:
        return 4

    def price(self, nights: int) -> int:
        base = super().price(nights)
        if nights >= 3:
            base = base * 9 // 10
        return base + (50 * nights if self.butler else 0)


class Hotel:
    def __init__(self) -> None:
        self.rooms: list[Room] = []
        self.revenue = 0

    def book(self, guest: str, people: int, start: int, nights: int) -> str:
        options = [r.number for r in self.rooms if r.capacity() >= people and r.free(start, nights)]
        if not options:
            return f"{guest}: no room for {people} on day {start}"
        prices = {r.number: r.price(nights) for r in self.rooms}
        best = min(options, key=lambda n: (prices[n], n))
        kind = ""
        for r in self.rooms:
            if r.number == best:
                r.bookings.append(Booking(guest, best, start, nights))
                kind = r.kind()
        cost = prices[best]
        self.revenue += cost
        return f"{guest}: {kind} {best}, days {start}-{start + nights}, {cost}"

    def occupancy(self, day: int) -> str:
        marks = []
        for r in self.rooms:
            marks.append("#" if not r.free(day, 1) else ".")
        return "".join(marks)


def main() -> None:
    h = Hotel()
    h.rooms.append(Single(101, 80))
    h.rooms.append(Single(102, 85))
    h.rooms.append(Double(201, 120))
    h.rooms.append(Double(202, 110))
    h.rooms.append(Suite(301, 300, False))
    h.rooms.append(Suite(302, 280, True))
    requests = [
        ("ann", 1, 1, 3), ("bob", 2, 2, 2), ("cid", 1, 2, 1), ("dee", 1, 2, 4), ("eve", 3, 1, 5),
        ("fay", 2, 3, 1), ("gil", 4, 2, 2), ("hal", 2, 2, 2), ("ivy", 1, 3, 2), ("jon", 5, 1, 1),
    ]
    for g, p, s, n in requests:
        print(h.book(g, p, s, n))
    for day in range(1, 8):
        print(f"day {day}: {h.occupancy(day)}")
    print("revenue", h.revenue)
    for r in h.rooms:
        guests = [b.guest for b in r.bookings]
        print(r.number, r.kind(), guests)


main()
