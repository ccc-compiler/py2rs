from dataclasses import dataclass


@dataclass
class Answer:
    age: int
    region: str
    rating: int
    recommend: bool


def age_band(a: int) -> str:
    if a < 25:
        return "<25"
    if a < 40:
        return "25-39"
    if a < 60:
        return "40-59"
    return "60+"


def ratings(answers: list[Answer], region: str, band: str) -> list[int]:
    return [a.rating for a in answers if a.region == region and (band == "" or age_band(a.age) == band)]


def main() -> None:
    n = int(input())
    answers: list[Answer] = []
    bad = 0
    for i in range(n):
        f = input().strip().split(",")
        if len(f) != 4 or not f[0].isdigit() or f[2] not in "12345" or f[2] == "":
            bad += 1
            print(f"line {i + 1}: skipped {f}")
            continue
        answers.append(Answer(int(f[0]), f[1], int(f[2]), f[3] == "y"))
    print(f"{len(answers)} valid, {bad} skipped")
    bands = ["<25", "25-39", "40-59", "60+"]
    regions = sorted({a.region for a in answers})
    print(f"{'band':<7}" + "".join(f"{r:>8}" for r in regions) + f"{'avg':>7}")
    for b in bands:
        line = f"{b:<7}"
        rs: list[int] = []
        for r in regions:
            sel = ratings(answers, r, b)
            rs.extend(sel)
            line += f"{sum(sel) / len(sel):>8.2f}" if sel else f"{'-':>8}"
        line += f"{sum(rs) / len(rs):>7.2f}" if rs else f"{'-':>7}"
        print(line)
    dist = [sum(1 for a in answers if a.rating == k) for k in range(1, 6)]
    for k in range(5):
        print(f"{k + 1} star {'*' * dist[k]:<12} {dist[k]}")
    promoters = sum(1 for a in answers if a.recommend)
    print(f"would recommend: {promoters}/{len(answers)} = {promoters / len(answers):.1%}")
    by_region = {r: ratings(answers, r, "") for r in regions}
    best = max(regions, key=lambda r: sum(by_region[r]) / len(by_region[r]))
    print("best region:", best, by_region[best])


main()
