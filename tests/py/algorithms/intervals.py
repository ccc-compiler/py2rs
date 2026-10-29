from dataclasses import dataclass


@dataclass
class Interval:
    start: int
    end: int

    def __str__(self) -> str:
        return f"[{self.start},{self.end})"


def merge(xs: list[Interval]) -> list[Interval]:
    out: list[Interval] = []
    for iv in sorted(xs, key=lambda i: (i.start, i.end)):
        if len(out) > 0 and out[-1].end >= iv.start:
            last = out.pop()
            out.append(Interval(last.start, max(last.end, iv.end)))
        else:
            out.append(Interval(iv.start, iv.end))
    return out


def max_non_overlapping(xs: list[Interval]) -> list[Interval]:
    chosen: list[Interval] = []
    end = -(1 << 62)
    for iv in sorted(xs, key=lambda i: (i.end, i.start)):
        if iv.start >= end:
            chosen.append(iv)
            end = iv.end
    return chosen


def rooms_needed(xs: list[Interval]) -> int:
    events: list[tuple[int, int]] = []
    for iv in xs:
        events.append((iv.start, 1))
        events.append((iv.end, -1))
    events.sort()
    cur = 0
    best = 0
    for e in events:
        cur += e[1]
        best = max(best, cur)
    return best


def fmt(xs: list[Interval]) -> str:
    return " ".join(str(i) for i in xs)


def timeline(xs: list[Interval], width: int) -> None:
    for iv in xs:
        print(f"  {str(iv):>8} " + " " * iv.start + "=" * (iv.end - iv.start) + " " * (width - iv.end) + "|")


def main() -> None:
    xs = [Interval(1, 3), Interval(2, 6), Interval(8, 10), Interval(15, 18), Interval(9, 12),
          Interval(17, 20), Interval(0, 1), Interval(5, 7)]
    print("input:  ", fmt(xs))
    timeline(xs, 20)
    print("merged: ", fmt(merge(xs)))
    print("greedy: ", fmt(max_non_overlapping(xs)))
    print("rooms:  ", rooms_needed(xs))
    meetings = [Interval(9, 10), Interval(9, 12), Interval(10, 11), Interval(11, 12), Interval(9, 11)]
    print("meetings rooms:", rooms_needed(meetings))
    total = sum(i.end - i.start for i in merge(xs))
    print("covered length:", total)
    print("empty:", fmt(merge([])), rooms_needed([]))


main()
