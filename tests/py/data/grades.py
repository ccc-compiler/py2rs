from dataclasses import dataclass


@dataclass
class Student:
    name: str
    scores: list[int]

    def average(self) -> float:
        return sum(self.scores) / len(self.scores)


def letter(avg: float) -> str:
    if avg >= 90:
        return "A"
    elif avg >= 80:
        return "B"
    elif avg >= 70:
        return "C"
    elif avg >= 60:
        return "D"
    return "F"


def load() -> list[Student]:
    raw = [
        "Alice:93,88,97,91",
        "Bob:72,65,80,70",
        "Chen:55,61,58,49",
        "Dana:85,90,78,88",
        "Eve:100,98,99,97",
        "Farid:68,74,71,77",
        "Gita:81,79,85,90",
    ]
    out: list[Student] = []
    for r in raw:
        parts = r.split(":")
        out.append(Student(parts[0], [int(x) for x in parts[1].split(",")]))
    return out


def main() -> None:
    students = load()
    print(f"{'Name':<8}|{'T1':>4}{'T2':>4}{'T3':>4}{'T4':>4}|{'Avg':>7}| G")
    print("=" * 36)
    dist: dict[str, int] = {}
    for s in students:
        cols = "".join(f"{x:>4}" for x in s.scores)
        avg = s.average()
        g = letter(avg)
        dist[g] = dist.get(g, 0) + 1
        print(f"{s.name:<8}|{cols}|{avg:>7.2f}| {g}")
    print("=" * 36)
    n_tests = len(students[0].scores)
    for t in range(n_tests):
        col = [s.scores[t] for s in students]
        print(f"test {t + 1}: min {min(col):3} max {max(col):3} mean {sum(col) / len(col):6.2f}")
    for g in "ABCDF":
        print(g, "#" * dist.get(g, 0))
    ranking = sorted(students, key=lambda s: -s.average())
    print("top 3:", [s.name for s in ranking[:3]])
    failing = [s.name for s in students if letter(s.average()) == "F"]
    print("failing:", failing if failing else ["none"])


main()
