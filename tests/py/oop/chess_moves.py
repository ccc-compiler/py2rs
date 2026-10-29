class Piece:
    def __init__(self, color: str, col: int, row: int) -> None:
        self.color = color
        self.col = col
        self.row = row

    def letter(self) -> str:
        return "?"

    def symbol(self) -> str:
        l = self.letter()
        return l.upper() if self.color == "white" else l.lower()

    def directions(self) -> list[tuple[int, int]]:
        return []

    def slides(self) -> bool:
        return True

    def moves(self, board: "Board") -> list[str]:
        out: list[str] = []
        for dc, dr in self.directions():
            c = self.col + dc
            r = self.row + dr
            while 0 <= c < 8 and 0 <= r < 8:
                other = board.color_at(c, r)
                if other == self.color:
                    break
                out.append(square(c, r) + ("x" if other != "" else ""))
                if other != "" or not self.slides():
                    break
                c += dc
                r += dr
        return out

    def value(self) -> int:
        return 0


class Rook(Piece):
    def letter(self) -> str:
        return "r"

    def directions(self) -> list[tuple[int, int]]:
        return [(1, 0), (-1, 0), (0, 1), (0, -1)]

    def value(self) -> int:
        return 5


class Bishop(Piece):
    def letter(self) -> str:
        return "b"

    def directions(self) -> list[tuple[int, int]]:
        return [(1, 1), (1, -1), (-1, 1), (-1, -1)]

    def value(self) -> int:
        return 3


class Queen(Piece):
    def letter(self) -> str:
        return "q"

    def directions(self) -> list[tuple[int, int]]:
        return [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)]

    def value(self) -> int:
        return 9


class Knight(Piece):
    def letter(self) -> str:
        return "n"

    def slides(self) -> bool:
        return False

    def directions(self) -> list[tuple[int, int]]:
        return [(1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1), (-2, 1), (-1, 2)]

    def value(self) -> int:
        return 3


class King(Knight):
    def letter(self) -> str:
        return "k"

    def directions(self) -> list[tuple[int, int]]:
        return [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)]

    def value(self) -> int:
        return 100


def square(c: int, r: int) -> str:
    return "abcdefgh"[c] + str(r + 1)


class Board:
    def __init__(self) -> None:
        self.pieces: list[Piece] = []

    def color_at(self, c: int, r: int) -> str:
        for p in self.pieces:
            if p.col == c and p.row == r:
                return p.color
        return ""

    def show(self) -> None:
        for r in range(7, -1, -1):
            line = str(r + 1) + " "
            for c in range(8):
                ch = "."
                for p in self.pieces:
                    if p.col == c and p.row == r:
                        ch = p.symbol()
                line += ch
            print(line)
        print("  abcdefgh")


def main() -> None:
    b = Board()
    b.pieces.append(Rook("white", 0, 0))
    b.pieces.append(Knight("white", 1, 0))
    b.pieces.append(Bishop("white", 2, 3))
    b.pieces.append(Queen("white", 3, 3))
    b.pieces.append(King("white", 4, 0))
    b.pieces.append(Rook("black", 3, 6))
    b.pieces.append(Knight("black", 5, 5))
    b.pieces.append(Queen("black", 0, 7))
    b.pieces.append(King("black", 4, 7))
    b.show()
    for p in b.pieces:
        ms = p.moves(b)
        print(f"{p.symbol()} {square(p.col, p.row)}: {len(ms):2} {' '.join(ms)}")
    for white in [True, False]:
        color = "white" if white else "black"
        material = sum(p.value() for p in b.pieces if (p.color == "white") == white and not isinstance(p, King))
        mobility = sum(len(p.moves(b)) for p in b.pieces if (p.color == "white") == white)
        caps = [m for p in b.pieces if (p.color == "white") == white for m in p.moves(b) if m.endswith("x")]
        print(f"{color}: material {material}, mobility {mobility}, captures {caps}")


main()
