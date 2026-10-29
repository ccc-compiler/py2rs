from dataclasses import dataclass


@dataclass
class Employee:
    eid: int
    name: str
    dept: str
    rate: float
    hours: list[float]

    def regular(self) -> float:
        return sum(min(h, 8.0) for h in self.hours)

    def overtime(self) -> float:
        return sum(max(h - 8.0, 0.0) for h in self.hours)

    def gross(self) -> float:
        return self.regular() * self.rate + self.overtime() * self.rate * 1.5


def tax(gross: float) -> float:
    brackets = [(500.0, 0.10), (1000.0, 0.20), (1.0e12, 0.30)]
    owed = 0.0
    prev = 0.0
    for limit, rate in brackets:
        if gross > prev:
            owed += (min(gross, limit) - prev) * rate
        prev = limit
    return owed


def main() -> None:
    n = int(input())
    staff: list[Employee] = []
    for i in range(n):
        f = input().split(";")
        hours = [float(h) for h in f[3].split()]
        staff.append(Employee(100 + i, f[0], f[1], float(f[2]), hours))
    print(f"{'ID':>4} {'Name':<10} {'Dept':<6} {'Reg':>6} {'OT':>5} {'Gross':>9} {'Tax':>8} {'Net':>9}")
    totals: dict[str, float] = {}
    for e in staff:
        g = e.gross()
        t = tax(g)
        totals[e.dept] = totals.get(e.dept, 0.0) + g
        print(f"{e.eid:>4} {e.name:<10} {e.dept:<6} {e.regular():>6.1f} {e.overtime():>5.1f} {g:>9.2f} {t:>8.2f} {g - t:>9.2f}")
    print()
    for d in sorted(totals, key=lambda k: -totals[k]):
        amt = totals[d]
        share = amt / sum(totals.values()) * 100
        print(f"{d:<6} {amt:>10,.2f} {share:5.1f}% {'=' * int(share / 2)}")
    hardest = max(staff, key=lambda e: sum(e.hours))
    print("most hours:", hardest.name, sum(hardest.hours))
    print("no overtime:", sorted(e.name for e in staff if e.overtime() == 0))


main()
