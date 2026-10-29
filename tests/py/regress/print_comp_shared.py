class Room:
    def __init__(self, n: int) -> None:
        self.n = n
        self.log: list[str] = []


rooms = [Room(1), Room(2)]
for r in rooms:
    r.log.append(f"checked {r.n}")
print([r.log for r in rooms])
print(len([r.log for r in rooms]), f"{[r.log for r in rooms]}")
rooms[0].log.append("again")
print([r.log for r in rooms])
