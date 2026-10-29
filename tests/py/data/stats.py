import math


def mean(xs: list[float]) -> float:
    return sum(xs) / len(xs)


def median(xs: list[float]) -> float:
    s = sorted(xs)
    n = len(s)
    if n % 2 == 1:
        return s[n // 2]
    return (s[n // 2 - 1] + s[n // 2]) / 2


def variance(xs: list[float]) -> float:
    m = mean(xs)
    return sum((x - m) ** 2 for x in xs) / (len(xs) - 1)


def mode(xs: list[float]) -> float:
    best = max(xs.count(x) for x in xs)
    return min(x for x in xs if xs.count(x) == best)


def percentile(xs: list[float], p: float) -> float:
    s = sorted(xs)
    k = (len(s) - 1) * p
    lo = math.floor(k)
    hi = math.ceil(k)
    if lo == hi:
        return s[lo]
    return s[lo] + (s[hi] - s[lo]) * (k - lo)


def summarize(name: str, xs: list[float]) -> None:
    sd = math.sqrt(variance(xs))
    print(f"== {name} (n={len(xs)}) ==")
    print(f"  mean   {mean(xs):10.4f}")
    print(f"  median {median(xs):10.4f}")
    print(f"  mode   {mode(xs):10.4f}")
    print(f"  stdev  {sd:10.4f}")
    print(f"  min/max {min(xs):.1f} / {max(xs):.1f}")
    for p in [0.25, 0.5, 0.75, 0.9]:
        print(f"  p{int(p * 100):02d}    {percentile(xs, p):10.4f}")
    outliers = [x for x in xs if abs(x - mean(xs)) > 2 * sd]
    print("  outliers:", outliers)


def main() -> None:
    temps = [21.5, 22.0, 19.8, 23.1, 22.0, 24.6, 18.9, 22.0, 21.1, 35.0, 20.4]
    heights = [160.0, 172.5, 181.0, 165.2, 158.4, 190.1, 175.0, 168.8]
    summarize("temps", temps)
    summarize("heights", heights)
    pairs = [(temps[i], heights[i]) for i in range(len(heights))]
    mx = mean([p[0] for p in pairs])
    my = mean([p[1] for p in pairs])
    cov = sum((a - mx) * (b - my) for a, b in pairs) / (len(pairs) - 1)
    r = cov / math.sqrt(variance([p[0] for p in pairs]) * variance([p[1] for p in pairs]))
    print(f"covariance {cov:.3f} correlation {r:+.4f}")


main()
