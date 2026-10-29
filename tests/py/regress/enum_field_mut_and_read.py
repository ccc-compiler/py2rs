# regression: BUG (Rust compile error): on a class with subclasses, appending to one field while the
class Room:
    def __init__(self, number: int) -> None:
        self.number = number
        self.log: list[str] = []


class Suite(Room):
    pass


def main() -> None:
    rooms: list[Room] = [Room(1), Suite(2)]
    for r in rooms:
        r.log.append(f"room {r.number}")
    for r in rooms:
        print(r.log)


main()
