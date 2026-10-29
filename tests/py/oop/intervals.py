from dataclasses import dataclass


@dataclass(frozen=True)
class Interval:
    start: int
    end: int

    def length(self) -> int:
        return self.end - self.start

    def overlaps(self, o: "Interval") -> bool:
        return self.start < o.end and o.start < self.end

    def merge(self, o: "Interval") -> "Interval":
        return Interval(min(self.start, o.start), max(self.end, o.end))

    def __str__(self) -> str:
        return f"[{self.start},{self.end})"


class Calendar:
    def __init__(self, owner: str) -> None:
        self.owner = owner
        self.busy: list[Interval] = []
        self.titles: dict[str, Interval] = {}

    def book(self, title: str, start: int, end: int) -> bool:
        iv = Interval(start, end)
        for b in self.busy:
            if b.overlaps(iv):
                return False
        self.busy.append(iv)
        self.busy.sort(key=lambda i: i.start)
        self.titles[title] = Interval(start, end)
        return True

    def cancel(self, title: str) -> bool:
        if title not in self.titles:
            return False
        iv = self.titles.pop(title)
        self.busy.remove(iv)
        return True

    def free(self, day_start: int, day_end: int) -> list[Interval]:
        out: list[Interval] = []
        t = day_start
        for b in self.busy:
            if b.start > t:
                out.append(Interval(t, b.start))
            t = max(t, b.end)
        if t < day_end:
            out.append(Interval(t, day_end))
        return out


def merge_all(ivs: list[Interval]) -> list[Interval]:
    out: list[Interval] = []
    for iv in sorted(ivs, key=lambda i: (i.start, i.end)):
        if len(out) > 0 and (out[-1].overlaps(iv) or out[-1].end == iv.start):
            last = out.pop()
            out.append(last.merge(iv))
        else:
            out.append(Interval(iv.start, iv.end))
    return out


def common_free(a: Calendar, b: Calendar, lo: int, hi: int) -> list[Interval]:
    busy = merge_all(a.busy + b.busy)
    out: list[Interval] = []
    t = lo
    for iv in busy:
        if iv.start > t:
            out.append(Interval(t, iv.start))
        t = max(t, iv.end)
    if t < hi:
        out.append(Interval(t, hi))
    return out


def fmt(ivs: list[Interval]) -> str:
    return " ".join(str(i) for i in ivs)


def main() -> None:
    ann = Calendar("ann")
    bob = Calendar("bob")
    for title, s, e in [("standup", 9, 10), ("review", 11, 13), ("lunch", 12, 13), ("gym", 17, 18), ("1:1", 14, 15)]:
        print("ann", title, ann.book(title, s, e))
    for title, s, e in [("standup", 9, 10), ("call", 10, 11), ("design", 13, 16)]:
        print("bob", title, bob.book(title, s, e))
    print("ann busy", fmt(ann.busy))
    print("ann free", fmt(ann.free(8, 19)))
    print("bob free", fmt(bob.free(8, 19)))
    print("both free", fmt(common_free(ann, bob, 8, 19)))
    print(ann.cancel("review"), ann.cancel("nothing"))
    print("ann free", fmt(ann.free(8, 19)))
    print("both free", fmt(common_free(ann, bob, 8, 19)))
    raw = [Interval(1, 3), Interval(2, 6), Interval(8, 10), Interval(15, 18), Interval(10, 11), Interval(17, 20)]
    m = merge_all(raw)
    print("merged", fmt(m), "total", sum(i.length() for i in m))
    print(Interval(1, 2) == Interval(1, 2), Interval(1, 2))


main()
