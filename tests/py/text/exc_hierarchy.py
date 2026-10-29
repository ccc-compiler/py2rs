class ShapeError(Exception):
    pass


class NegativeSize(ShapeError):
    pass


class TooLarge(ShapeError):
    pass


class UnknownShape(ShapeError):
    pass


def area(kind: str, a: float, b: float) -> float:
    if a < 0 or b < 0:
        raise NegativeSize(f"{kind}: negative size {a}, {b}")
    if kind == "rect":
        v = a * b
    elif kind == "tri":
        v = a * b / 2
    elif kind == "circle":
        v = 3.141592653589793 * a * a
    else:
        raise UnknownShape(f"no shape {kind!r}")
    if v > 1e6:
        raise TooLarge(f"{kind}: area {v:.3e} too large")
    return v


def main() -> None:
    jobs = [("rect", 3.0, 4.0), ("tri", 3.0, 4.0), ("circle", 2.0, 0.0), ("rect", -1.0, 2.0),
            ("hex", 1.0, 1.0), ("rect", 2000.0, 1000.0), ("circle", 1000.0, 0.0), ("tri", 0.1, 0.2)]
    total = 0.0
    counts: dict[str, int] = {}
    for kind, a, b in jobs:
        try:
            v = area(kind, a, b)
            total += v
            print(f"{kind:<7} {v:>12.4f} {v}")
        except NegativeSize as e:
            counts["negative"] = counts.get("negative", 0) + 1
            print(f"{kind:<7} size problem (negative): {e}")
        except TooLarge as e:
            counts["large"] = counts.get("large", 0) + 1
            print(f"{kind:<7} size problem (large): {e}")
        except ShapeError as e:
            counts["other"] = counts.get("other", 0) + 1
            print(f"{kind:<7} shape error: {e}")
    print(f"total {total:.6f}", counts)
    try:
        raise TooLarge("direct")
    except Exception as e:
        print("generic handler:", e)


main()
