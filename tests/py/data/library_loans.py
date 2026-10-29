from dataclasses import dataclass


@dataclass
class Book:
    isbn: str
    title: str
    author: str
    copies: int


@dataclass
class Loan:
    isbn: str
    member: str
    day_out: int
    day_back: int


def late_fee(days: int) -> int:
    if days <= 14:
        return 0
    late = days - 14
    return min(late * 25, 1000)


def main() -> None:
    catalog: dict[str, Book] = {}
    n = int(input())
    for _ in range(n):
        p = input().split("/")
        catalog[p[0]] = Book(p[0], p[1], p[2], int(p[3]))
    loans: list[Loan] = []
    m = int(input())
    for _ in range(m):
        p = input().split()
        loans.append(Loan(p[0], p[1], int(p[2]), int(p[3]) if len(p) > 3 else -1))
    out_now: dict[str, int] = {}
    for l in loans:
        if l.day_back < 0:
            out_now[l.isbn] = out_now.get(l.isbn, 0) + 1
    print("Availability")
    for isbn in sorted(catalog):
        b = catalog[isbn]
        avail = b.copies - out_now.get(isbn, 0)
        flag = " (all out)" if avail <= 0 else ""
        print(f"  {b.title[:22]:<22} {b.author:<12} {avail}/{b.copies}{flag}")
    fees: dict[str, int] = {}
    for l in loans:
        if l.day_back >= 0:
            fee = late_fee(l.day_back - l.day_out)
            if fee > 0:
                fees[l.member] = fees.get(l.member, 0) + fee
    print("Fees")
    for mem in sorted(fees, key=lambda k: (-fees[k], k)):
        print(f"  {mem:<8} ${fees[mem] / 100:6.2f}")
    popular: dict[str, int] = {}
    for l in loans:
        popular[l.isbn] = popular.get(l.isbn, 0) + 1
    top = max(popular, key=lambda k: popular[k])
    print("most borrowed:", catalog[top].title, popular[top])
    by_member: dict[str, list[str]] = {}
    for l in loans:
        by_member.setdefault(l.member, []).append(catalog[l.isbn].title.split()[0])
    for mem in sorted(by_member):
        print(f"  {mem}: {', '.join(by_member[mem])}")


main()
