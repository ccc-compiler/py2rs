from dataclasses import dataclass


@dataclass(frozen=True)
class Card:
    rank: int
    suit: str

    def __str__(self) -> str:
        names = {11: "J", 12: "Q", 13: "K", 14: "A"}
        r = names.get(self.rank, str(self.rank))
        return r + self.suit


class Rng:
    def __init__(self, seed: int) -> None:
        self.state = seed

    def next(self) -> int:
        self.state = (self.state * 1103515245 + 12345) % 2147483648
        return self.state

    def below(self, n: int) -> int:
        return self.next() % n


class Deck:
    def __init__(self) -> None:
        self.cards: list[Card] = []
        for s in "SHDC":
            for r in range(2, 15):
                self.cards.append(Card(r, s))

    def shuffle(self, rng: Rng) -> None:
        rest = self.cards.copy()
        out: list[Card] = []
        while rest:
            j = rng.below(len(rest))
            out.append(rest.pop(j))
        self.cards = out

    def deal(self) -> Card:
        return self.cards.pop()


class Player:
    def __init__(self, name: str) -> None:
        self.name = name
        self.pile: list[Card] = []
        self.won = 0

    def play(self) -> Card:
        return self.pile.pop(0)

    def has_cards(self) -> bool:
        return len(self.pile) > 0


def main() -> None:
    rng = Rng(2024)
    deck = Deck()
    deck.shuffle(rng)
    print("top five:", " ".join(str(c) for c in deck.cards[-5:]))
    players = [Player("north"), Player("east"), Player("south")]
    while len(deck.cards) >= len(players):
        for p in players:
            p.pile.append(deck.deal())
    print("left in deck:", [str(c) for c in deck.cards])
    for p in players:
        print(p.name, len(p.pile), " ".join(str(c) for c in p.pile[:6]))
    rnd = 0
    while all(p.has_cards() for p in players) and rnd < 30:
        rnd += 1
        table: list[Card] = []
        for p in players:
            table.append(p.play())
        best = max(range(len(players)), key=lambda i: (table[i].rank, table[i].suit))
        for i, p in enumerate(players):
            if i == best:
                p.won += 1
                for c in table:
                    p.pile.append(c)
        shown = " ".join(f"{str(c):>3}" for c in table)
        print(f"round {rnd:2}: {shown}  -> {players[best].name}")
    for p in sorted(players, key=lambda p: -len(p.pile)):
        print(f"{p.name:<6} cards={len(p.pile):2} rounds won={p.won}")


main()
