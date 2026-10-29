def minutes(hhmm: str) -> int:
    h = int(hhmm.split(":")[0])
    m = int(hhmm.split(":")[1])
    return h * 60 + m


def hhmm(total: int) -> str:
    return f"{total // 60}:{total % 60:02d}"


def main() -> None:
    raw = """mon ana 08:55 12:02 12:45 17:30
mon ben 09:10 13:00 13:30 18:15
tue ana 09:00 12:00 13:00 16:00
tue ben 07:45 11:45 12:15 19:05
wed ana 08:30 12:30 13:15 18:45
wed ben 10:00 14:00
thu ana 09:05 11:55 12:40 17:20
thu ben 09:00 12:00 12:30 17:00
fri ana 08:00 12:00
fri ben 09:30 12:30 13:00 16:45"""
    week: dict[str, dict[str, int]] = {}
    days: list[str] = []
    for line in raw.splitlines():
        p = line.split()
        day = p[0]
        who = p[1]
        if day not in days:
            days.append(day)
        stamps = [minutes(x) for x in p[2:]]
        worked = 0
        for k in range(0, len(stamps), 2):
            worked += stamps[k + 1] - stamps[k]
        if who not in week:
            week[who] = {}
        week[who][day] = worked
    print(f"{'who':<5}" + "".join(f"{d:>7}" for d in days) + f"{'total':>8}{'ot':>7}")
    for who in sorted(week):
        total = sum(week[who].values())
        ot = sum(max(0, m - 480) for m in week[who].values())
        cells = "".join(f"{hhmm(week[who].get(d, 0)):>7}" for d in days)
        print(f"{who:<5}{cells}{hhmm(total):>8}{hhmm(ot):>7}")
    for d in days:
        team = sum(week[w].get(d, 0) for w in week)
        print(f"{d}: {hhmm(team):>6} {'#' * (team // 60)}")
    longest = max(((w, d) for w in week for d in week[w]), key=lambda wd: week[wd[0]][wd[1]])
    print("longest day:", longest[0], longest[1], hhmm(week[longest[0]][longest[1]]))
    short = sorted((d, w) for w in week for d in week[w] if week[w][d] < 6 * 60)
    print("short days:", short)


main()
