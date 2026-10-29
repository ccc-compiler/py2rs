# regression: BUG (Rust compile error): comparing a str field of a class with subclasses (read through
class Piece:
    def __init__(self, color: str) -> None:
        self.color = color

    def value(self) -> int:
        return 1


class Rook(Piece):
    def value(self) -> int:
        return 5


def main() -> None:
    pieces: list[Piece] = [Rook("white"), Piece("black"), Piece("white")]
    for color in ["white", "black"]:
        print(color, sum(p.value() for p in pieces if p.color == color))


main()
