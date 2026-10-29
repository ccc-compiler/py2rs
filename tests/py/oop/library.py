from dataclasses import dataclass


@dataclass
class Book:
    isbn: str
    title: str
    author: str
    year: int
    copies: int = 1
    on_loan: int = 0

    def available(self) -> int:
        return self.copies - self.on_loan


class Member:
    def __init__(self, name: str, limit: int) -> None:
        self.name = name
        self.limit = limit
        self.borrowed: list[str] = []
        self.history: list[str] = []

    def __repr__(self) -> str:
        return f"Member(name={self.name!r}, limit={self.limit}, borrowed={self.borrowed}, history={self.history})"


class Library:
    def __init__(self) -> None:
        self.books: dict[str, Book] = {}
        self.members: dict[str, Member] = {}
        self.day = 0
        self.waitlist: dict[str, list[str]] = {}

    def add_book(self, b: Book) -> None:
        if b.isbn in self.books:
            self.books[b.isbn].copies += b.copies
        else:
            self.books[b.isbn] = Book(b.isbn, b.title, b.author, b.year, b.copies)

    def join(self, name: str, limit: int = 3) -> None:
        self.members[name] = Member(name, limit)

    def borrow(self, name: str, isbn: str) -> str:
        if name not in self.members:
            return f"{name}: not a member"
        if isbn not in self.books:
            return f"{name}: unknown book {isbn}"
        if len(self.members[name].borrowed) >= self.members[name].limit:
            return f"{name}: limit reached"
        if self.books[isbn].available() == 0:
            if isbn not in self.waitlist:
                self.waitlist[isbn] = []
            self.waitlist[isbn].append(name)
            return f"{name}: waiting for {self.books[isbn].title}"
        self.books[isbn].on_loan += 1
        self.members[name].borrowed.append(isbn)
        self.members[name].history.append(f"day{self.day}:{isbn}")
        return f"{name}: borrowed {self.books[isbn].title}"

    def give_back(self, name: str, isbn: str) -> str:
        if isbn not in self.members[name].borrowed:
            return f"{name}: does not have {isbn}"
        self.members[name].borrowed.remove(isbn)
        self.books[isbn].on_loan -= 1
        msg = f"{name}: returned {self.books[isbn].title}"
        if isbn in self.waitlist and len(self.waitlist[isbn]) > 0:
            nxt = self.waitlist[isbn].pop(0)
            msg += " / " + self.borrow(nxt, isbn)
        return msg

    def search(self, word: str) -> list[str]:
        w = word.lower()
        return [b.title for b in self.books.values() if w in b.title.lower() or w in b.author.lower()]

    def report(self) -> None:
        for b in sorted(self.books.values(), key=lambda b: b.year):
            print(f"  {b.year} {b.title:<28}{b.author:<18}{b.available()}/{b.copies}")
        for m in self.members.values():
            print(f"  {m.name}: {m.borrowed} history={len(m.history)}")


def main() -> None:
    lib = Library()
    lib.add_book(Book("111", "Dune", "Frank Herbert", 1965))
    lib.add_book(Book("222", "Neuromancer", "William Gibson", 1984, 2))
    lib.add_book(Book("333", "The Left Hand of Darkness", "Ursula K. Le Guin", 1969))
    lib.add_book(Book("444", "Foundation", "Isaac Asimov", 1951))
    lib.add_book(Book("111", "Dune", "Frank Herbert", 1965))
    lib.join("ann")
    lib.join("ben", 1)
    lib.join("cat")
    actions = [
        ("b", "ann", "111"), ("b", "ben", "111"), ("b", "cat", "111"), ("b", "ben", "222"),
        ("b", "dan", "222"), ("b", "ann", "999"), ("b", "ann", "333"), ("r", "ann", "111"),
        ("r", "ben", "444"), ("b", "ann", "444"), ("b", "ann", "222"), ("r", "cat", "111"),
    ]
    for kind, who, isbn in actions:
        lib.day += 1
        if kind == "b":
            print(lib.borrow(who, isbn))
        else:
            print(lib.give_back(who, isbn))
    lib.report()
    print(lib.search("an"))
    print(lib.search("GIBSON"))
    print(lib.waitlist)
    print(lib.members["ann"])


main()
