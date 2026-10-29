from dataclasses import dataclass


@dataclass
class Team:
    name: str
    played: int = 0
    won: int = 0
    drawn: int = 0
    lost: int = 0
    gf: int = 0
    ga: int = 0

    def points(self) -> int:
        return 3 * self.won + self.drawn

    def gd(self) -> int:
        return self.gf - self.ga


def record(t: Team, scored: int, conceded: int) -> None:
    t.played += 1
    t.gf += scored
    t.ga += conceded
    if scored > conceded:
        t.won += 1
    elif scored == conceded:
        t.drawn += 1
    else:
        t.lost += 1


def main() -> None:
    results = """Lions 3-1 Tigers
Bears 0-0 Wolves
Tigers 2-2 Bears
Wolves 1-4 Lions
Lions 1-1 Bears
Tigers 3-0 Wolves
Bears 2-1 Lions
Wolves 2-2 Tigers
Lions 0-2 Wolves
Bears 1-3 Tigers"""
    table: dict[str, Team] = {}
    for line in results.split("\n"):
        p = line.split()
        home = p[0]
        away = p[2]
        hs = int(p[1].split("-")[0])
        aw = int(p[1].split("-")[1])
        for name in [home, away]:
            if name not in table:
                table[name] = Team(name)
        record(table[home], hs, aw)
        record(table[away], aw, hs)
    ranked = sorted(table.values(), key=lambda t: (-t.points(), -t.gd(), -t.gf, t.name))
    print(f"{'#':>2} {'Team':<8}{'P':>3}{'W':>3}{'D':>3}{'L':>3}{'GF':>4}{'GA':>4}{'GD':>5}{'Pts':>5}")
    for i, t in enumerate(ranked, 1):
        print(f"{i:>2} {t.name:<8}{t.played:>3}{t.won:>3}{t.drawn:>3}{t.lost:>3}{t.gf:>4}{t.ga:>4}{t.gd():>+5}{t.points():>5}")
    goals = sum(t.gf for t in table.values())
    print(f"goals {goals}, per match {goals / len(results.splitlines()):.2f}")
    print("unbeaten at home? none tracked; best attack:", max(table.values(), key=lambda t: t.gf).name)
    print("leakiest defence:", max(table.values(), key=lambda t: t.ga).name)


main()
