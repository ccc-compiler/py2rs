# regression: BUG: storing a parameter into a container is accepted (`self.books[b.isbn] = b` alone is
class Book:
    def __init__(self, isbn: str, copies: int) -> None:
        self.isbn = isbn
        self.copies = copies


class Lib:
    def __init__(self) -> None:
        self.books: dict[str, Book] = {}

    def add(self, b: Book) -> None:
        if b.isbn in self.books:
            self.books[b.isbn].copies += b.copies
        else:
            self.books[b.isbn] = b


def main() -> None:
    lib = Lib()
    lib.add(Book("1", 1))
    lib.add(Book("1", 2))
    print(lib.books["1"].copies)


main()
