def column(rows: list[list[float]], c: int) -> list[float]:
    return [r[c] for r in rows]


def normalize(xs: list[float]) -> list[float]:
    lo = min(xs)
    hi = max(xs)
    if hi == lo:
        return [0.0 for _ in xs]
    return [(x - lo) / (hi - lo) for x in xs]


def zscores(xs: list[float]) -> list[float]:
    m = sum(xs) / len(xs)
    sd = (sum((x - m) ** 2 for x in xs) / len(xs)) ** 0.5
    return [(x - m) / sd for x in xs]


def main() -> None:
    header = ["city", "pop_k", "area", "rain_mm", "sun_h"]
    data = """Taipei,2602,271.8,2405,1405
Kaohsiung,2733,2951.9,1885,2212
Taichung,2850,2214.9,1700,2040
Tainan,1858,2191.7,1690,2195
Hsinchu,451,104.1,1720,1840
Keelung,362,132.8,3700,1060"""
    names: list[str] = []
    rows: list[list[float]] = []
    for line in data.split("\n"):
        f = line.split(",")
        names.append(f[0])
        rows.append([float(x) for x in f[1:]])
    density = [r[0] * 1000 / r[1] for r in rows]
    print(f"{'':<10}" + "".join(f"{h:>10}" for h in header[1:]) + f"{'density':>10}")
    for i in range(len(rows)):
        print(f"{names[i]:<10}" + "".join(f"{v:>10.1f}" for v in rows[i]) + f"{density[i]:>10.0f}")
    cols = [column(rows, c) for c in range(len(header) - 1)]
    print("min".ljust(10) + "".join(f"{min(c):>10.1f}" for c in cols))
    print("max".ljust(10) + "".join(f"{max(c):>10.1f}" for c in cols))
    print("mean".ljust(10) + "".join(f"{sum(c) / len(c):>10.1f}" for c in cols))
    print()
    norm = [normalize(c) for c in cols]
    for i in range(len(rows)):
        cells = " ".join(f"{norm[c][i]:.2f}" for c in range(len(norm)))
        print(f"{names[i]:<10} {cells}")
    z = zscores(cols[2])
    for i in sorted(range(len(names)), key=lambda i: z[i]):
        print(f"rain z {names[i]:<10} {z[i]:+.3f}")
    wet_sunny = [names[i] for i in range(len(names)) if norm[2][i] < 0.3 and norm[3][i] > 0.7]
    print("dry and sunny:", wet_sunny)


main()
