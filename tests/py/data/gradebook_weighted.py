from dataclasses import dataclass


@dataclass
class Category:
    name: str
    weight: float
    drop_lowest: bool


@dataclass
class Record:
    student: str
    marks: dict[str, list[float]]


def category_score(marks: list[float], drop: bool) -> float:
    if not marks:
        return 0.0
    m = sorted(marks)
    if drop and len(m) > 1:
        m = m[1:]
    return sum(m) / len(m)


def final(rec: Record, cats: list[Category]) -> float:
    total = 0.0
    for c in cats:
        total += c.weight * category_score(rec.marks.get(c.name, []), c.drop_lowest)
    return total


def main() -> None:
    cats = [Category("hw", 0.3, True), Category("quiz", 0.2, True), Category("exam", 0.5, False)]
    raw = """ana hw 90 85 100 70
ana quiz 8 9 10
ana exam 88 92
bo hw 60 75 80 0
bo quiz 5 7 6
bo exam 71 64
cy hw 100 100 95 98
cy quiz 10 10 9
cy exam 97 99
di hw 50 40
di exam 55 61"""
    book: dict[str, Record] = {}
    for line in raw.split("\n"):
        p = line.split()
        if p[0] not in book:
            book[p[0]] = Record(p[0], {})
        vals = [float(x) for x in p[2:]]
        if p[1] == "quiz":
            vals = [v * 10 for v in vals]
        book[p[0]].marks[p[1]] = vals
    header = f"{'student':<8}" + "".join(f"{c.name:>8}" for c in cats) + f"{'final':>8}"
    print(header)
    print("-" * len(header))
    finals: dict[str, float] = {}
    for name in sorted(book):
        r = book[name]
        cols = "".join(f"{category_score(r.marks.get(c.name, []), c.drop_lowest):>8.1f}" for c in cats)
        finals[name] = final(r, cats)
        print(f"{name:<8}{cols}{finals[name]:>8.2f}")
    avg = sum(finals.values()) / len(finals)
    print(f"class average {avg:.2f}")
    above = [n for n in finals if finals[n] > avg]
    print("above average:", above)
    missing = [(n, c.name) for n in sorted(book) for c in cats if c.name not in book[n].marks]
    print("missing:", missing)


main()
