class Room:
    def __init__(self, name: str, desc: str) -> None:
        self.name = name
        self.desc = desc
        self.exits: dict[str, str] = {}
        self.items: list[str] = []
        self.locked: dict[str, str] = {}


class Player:
    def __init__(self, start: str) -> None:
        self.where = start
        self.bag: list[str] = []
        self.moves = 0
        self.score = 0


class World:
    def __init__(self) -> None:
        self.rooms: dict[str, Room] = {}

    def add(self, name: str, desc: str, items: list[str]) -> None:
        r = Room(name, desc)
        for it in items:
            r.items.append(it)
        self.rooms[name] = r

    def link(self, a: str, d: str, b: str, back: str, key: str = "") -> None:
        self.rooms[a].exits[d] = b
        self.rooms[b].exits[back] = a
        if key:
            self.rooms[a].locked[d] = key

    def look(self, p: Player) -> str:
        r = self.rooms[p.where]
        things = ", ".join(r.items) if r.items else "nothing"
        ways = " ".join(sorted(r.exits.keys()))
        return f"{r.name}: {r.desc} You see {things}. Exits: {ways}"

    def do(self, p: Player, cmd: str) -> str:
        words = cmd.split()
        verb = words[0]
        p.moves += 1
        if verb == "go":
            d = words[1]
            if d not in self.rooms[p.where].exits:
                return "You can't go that way."
            if d in self.rooms[p.where].locked:
                key = self.rooms[p.where].locked[d]
                if key not in p.bag:
                    return f"The way {d} is locked."
                del self.rooms[p.where].locked[d]
                p.score += 10
            p.where = self.rooms[p.where].exits[d]
            return self.look(p)
        if verb == "take":
            it = words[1]
            if it not in self.rooms[p.where].items:
                return f"No {it} here."
            self.rooms[p.where].items.remove(it)
            p.bag.append(it)
            p.score += 5
            return f"Taken: {it}."
        if verb == "drop":
            it = words[1]
            if it not in p.bag:
                return f"You have no {it}."
            p.bag.remove(it)
            self.rooms[p.where].items.append(it)
            return f"Dropped {it}."
        if verb == "inv":
            return "You carry: " + (", ".join(p.bag) if p.bag else "nothing")
        return f"I don't know how to {verb}."


def main() -> None:
    w = World()
    w.add("hall", "A dusty hall.", ["lamp"])
    w.add("kitchen", "Smells of old soup.", ["knife", "key"])
    w.add("cellar", "Dark and damp.", ["wine"])
    w.add("vault", "Gold glitters everywhere!", ["gold", "crown"])
    w.link("hall", "east", "kitchen", "west")
    w.link("kitchen", "down", "cellar", "up")
    w.link("cellar", "north", "vault", "south", "key")
    p = Player("hall")
    print(w.look(p))
    script = ["take lamp", "go north", "go east", "take key", "take spoon", "go down", "go north",
              "take gold", "drop lamp", "inv", "dance", "go south", "go up", "go west", "inv"]
    for cmd in script:
        print(f"> {cmd}")
        print(w.do(p, cmd))
    print(f"moves {p.moves}, score {p.score}, bag {p.bag}")
    for name, r in w.rooms.items():
        print(name, r.items, r.locked)


main()
