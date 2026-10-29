def factor(unit: str) -> tuple[str, float]:
    table = {
        "mm": ("m", 0.001), "cm": ("m", 0.01), "m": ("m", 1.0), "km": ("m", 1000.0),
        "in": ("m", 0.0254), "ft": ("m", 0.3048), "mi": ("m", 1609.344),
        "g": ("kg", 0.001), "kg": ("kg", 1.0), "lb": ("kg", 0.45359237), "oz": ("kg", 0.028349523125),
        "ml": ("l", 0.001), "l": ("l", 1.0), "gal": ("l", 3.785411784),
    }
    if unit not in table:
        raise KeyError(unit)
    return table[unit]


def convert(value: float, src: str, dst: str) -> float:
    a = factor(src)
    b = factor(dst)
    if a[0] != b[0]:
        raise ValueError(f"cannot convert {a[0]} to {b[0]}")
    return value * a[1] / b[1]


def main() -> None:
    queries = ["5 km mi", "12 in cm", "3.5 lb kg", "1 gal ml", "100 g oz",
               "6 ft m", "2 kg l", "7 parsec m", "1 mi ft", "250 ml gal"]
    ok = 0
    for q in queries:
        p = q.split()
        v = float(p[0])
        try:
            r = convert(v, p[1], p[2])
            ok += 1
            print(f"{v:>8.2f} {p[1]:<4}= {r:>12.4f} {p[2]}")
        except ValueError as e:
            print(f"{q:<14} error: {e}")
        except KeyError as e:
            print(f"{q:<14} unknown unit {e}")
    print(f"{ok}/{len(queries)} converted")
    print(f"{'ft':>6}{'m':>9}")
    for ft in range(0, 31, 5):
        print(f"{ft:>6}{convert(float(ft), 'ft', 'm'):>9.3f}")
    total_m = 0.0
    for x in ["1 km", "300 m", "2 mi", "500 ft"]:
        total_m += convert(float(x.split()[0]), x.split()[1], "m")
    print(f"route length {total_m:.1f} m = {total_m / 1000:.3f} km")


main()
