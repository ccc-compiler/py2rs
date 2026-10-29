READINGS = """2026-07-01 23.4 18.2
2026-07-02 25.1 17.9
2026-07-03 28.7 19.5
2026-07-04 31.2 21.0
2026-07-05 33.0 22.4
2026-07-06 29.8 20.1
2026-07-07 26.4 18.8
2026-07-08 24.0 16.5
2026-07-09 27.3 17.7
2026-07-10 30.9 20.6
2026-07-11 34.5 23.8
2026-07-12 35.1 24.2
2026-07-13 32.2 22.0
2026-07-14 28.0 19.4"""


def moving_average(xs: list[float], k: int) -> list[float]:
    out: list[float] = []
    for i in range(len(xs) - k + 1):
        out.append(sum(xs[i:i + k]) / k)
    return out


def longest_streak(xs: list[float], threshold: float) -> tuple[int, int]:
    best = 0
    best_start = -1
    cur = 0
    for i, x in enumerate(xs):
        if x >= threshold:
            cur += 1
            if cur > best:
                best = cur
                best_start = i - cur + 1
        else:
            cur = 0
    return (best_start, best)


def main() -> None:
    days: list[str] = []
    highs: list[float] = []
    lows: list[float] = []
    for line in READINGS.splitlines():
        d, h, l = line.split()[0], line.split()[1], line.split()[2]
        days.append(d[5:])
        highs.append(float(h))
        lows.append(float(l))
    print(f"{'day':<6}{'high':>6}{'low':>6}{'range':>7}  chart")
    lo_all = min(lows)
    for i in range(len(days)):
        pad = int(lows[i] - lo_all)
        width = int(highs[i] - lows[i])
        print(f"{days[i]:<6}{highs[i]:>6.1f}{lows[i]:>6.1f}{highs[i] - lows[i]:>7.1f}  {' ' * pad}{'=' * width}")
    ma = moving_average(highs, 3)
    print("3-day MA of highs:", [round(x, 2) for x in ma])
    start, n = longest_streak(highs, 30.0)
    print(f"heat wave: {n} days from {days[start]}")
    diffs = [round(highs[i + 1] - highs[i], 1) for i in range(len(highs) - 1)]
    print("daily change:", diffs)
    up = sum(1 for d in diffs if d > 0)
    print(f"warmer days {up}, cooler days {len(diffs) - up}")
    weeks = [highs[i:i + 7] for i in range(0, len(highs), 7)]
    for w, vals in enumerate(weeks, 1):
        print(f"week {w}: mean {sum(vals) / len(vals):.2f} max {max(vals)} min {min(vals)}")
    print(f"overall mean high {sum(highs) / len(highs):.3f}, low {sum(lows) / len(lows):.3f}")


main()
