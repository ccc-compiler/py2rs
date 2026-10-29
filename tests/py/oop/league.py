from dataclasses import dataclass


@dataclass
class Team:
    name: str
    strength: int
    played: int = 0
    won: int = 0
    drawn: int = 0
    lost: int = 0
    goals_for: int = 0
    goals_against: int = 0

    def points(self) -> int:
        return 3 * self.won + self.drawn

    def diff(self) -> int:
        return self.goals_for - self.goals_against

    def record(self, gf: int, ga: int) -> None:
        self.played += 1
        self.goals_for += gf
        self.goals_against += ga
        if gf > ga:
            self.won += 1
        elif gf == ga:
            self.drawn += 1
        else:
            self.lost += 1


class Dice:
    def __init__(self, seed: int) -> None:
        self.x = seed

    def roll(self, n: int) -> int:
        self.x = (self.x * 48271) % 2147483647
        return self.x % n


class League:
    def __init__(self, names: list[str], strengths: list[int]) -> None:
        self.teams: dict[str, Team] = {}
        for n, s in zip(names, strengths):
            self.teams[n] = Team(n, s)
        self.results: list[str] = []

    def play(self, home: str, away: str, dice: Dice) -> None:
        hs = self.teams[home].strength + 1
        aw = self.teams[away].strength
        gh = dice.roll(hs) // 2
        ga = dice.roll(aw) // 2
        self.teams[home].record(gh, ga)
        self.teams[away].record(ga, gh)
        self.results.append(f"{home} {gh}-{ga} {away}")

    def schedule(self) -> list[tuple[str, str]]:
        names = list(self.teams.keys())
        games: list[tuple[str, str]] = []
        for i in range(len(names)):
            for j in range(len(names)):
                if i != j:
                    games.append((names[i], names[j]))
        return games

    def table(self) -> list[Team]:
        return sorted(self.teams.values(), key=lambda t: (-t.points(), -t.diff(), -t.goals_for, t.name))


def main() -> None:
    league = League(["Lions", "Tigers", "Bears", "Wolves", "Eagles"], [9, 7, 6, 6, 4])
    dice = Dice(7)
    games = league.schedule()
    print(len(games), "games")
    for h, a in games:
        league.play(h, a, dice)
    for r in league.results[:8]:
        print(r)
    print("...")
    print(f"{'#':>2} {'team':<8}{'P':>3}{'W':>3}{'D':>3}{'L':>3}{'GF':>4}{'GA':>4}{'GD':>4}{'Pts':>5}")
    for pos, t in enumerate(league.table(), 1):
        print(f"{pos:>2} {t.name:<8}{t.played:>3}{t.won:>3}{t.drawn:>3}{t.lost:>3}"
              f"{t.goals_for:>4}{t.goals_against:>4}{t.diff():>+4}{t.points():>5}")
    champion = league.table()[0]
    print("champion:", champion.name, champion)
    total_goals = sum(t.goals_for for t in league.teams.values())
    print("goals per game", round(total_goals / len(games), 3))


main()
