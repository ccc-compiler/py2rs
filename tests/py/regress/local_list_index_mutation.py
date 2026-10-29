# regression: BUG (Rust compile error, same cause as field_list_index_store, also for LOCAL lists):
class Player:
    def __init__(self, name: str) -> None:
        self.name = name
        self.won = 0


def main() -> None:
    cards = [1, 2, 3, 4, 5]
    j = 3
    for i in range(len(cards) - 1, 0, -1):
        j = (j * 7 + 1) % (i + 1)
        cards[i], cards[j] = cards[j], cards[i]
    print(cards)
    players = [Player("a"), Player("b")]
    best = max(range(len(players)), key=lambda i: len(players[i].name))
    players[best].won += 1
    print(players[0].won, players[1].won)


main()
