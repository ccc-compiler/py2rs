from dataclasses import dataclass

ORG = """1,Grace,CEO,0,250000
2,Alan,CTO,1,190000
3,Ada,CFO,1,185000
4,Linus,Eng Manager,2,150000
5,Ken,Engineer,4,120000
6,Dennis,Engineer,4,125000
7,Barbara,Engineer,4,118000
8,Margaret,Accountant,3,90000
9,Edsger,Researcher,2,140000
10,Donald,Intern,9,40000"""


@dataclass
class Emp:
    eid: int
    name: str
    title: str
    boss: int
    salary: int


def load(text: str) -> dict[int, Emp]:
    out: dict[int, Emp] = {}
    for line in text.splitlines():
        f = line.split(",")
        out[int(f[0])] = Emp(int(f[0]), f[1], f[2], int(f[3]), int(f[4]))
    return out


def reports(staff: dict[int, Emp], boss: int) -> list[int]:
    return sorted(e.eid for e in staff.values() if e.boss == boss)


def show(staff: dict[int, Emp], eid: int, depth: int) -> None:
    e = staff[eid]
    print(f"{'    ' * depth}- {e.name} ({e.title}) ${e.salary:,}")
    for r in reports(staff, eid):
        show(staff, r, depth + 1)


def team_cost(staff: dict[int, Emp], eid: int) -> int:
    return staff[eid].salary + sum(team_cost(staff, r) for r in reports(staff, eid))


def chain(staff: dict[int, Emp], eid: int) -> list[str]:
    names: list[str] = []
    cur = eid
    while cur != 0:
        names.append(staff[cur].name)
        cur = staff[cur].boss
    return names


def main() -> None:
    staff = load(ORG)
    show(staff, 1, 0)
    print()
    for eid in [1, 2, 3, 4, 9]:
        print(f"{staff[eid].name:<8} team cost {team_cost(staff, eid):>10,}")
    print(" -> ".join(chain(staff, 10)))
    depth = {eid: len(chain(staff, eid)) for eid in staff}
    deepest = max(depth.values())
    print("levels:", deepest, [staff[e].name for e in depth if depth[e] == deepest])
    by_title: dict[str, list[int]] = {}
    for e in staff.values():
        by_title.setdefault(e.title, []).append(e.salary)
    for t in sorted(by_title):
        s = by_title[t]
        print(f"{t:<12} n={len(s)} avg={sum(s) // len(s):>7}")
    managers = [e.name for e in staff.values() if reports(staff, e.eid)]
    print("managers:", managers)


main()
