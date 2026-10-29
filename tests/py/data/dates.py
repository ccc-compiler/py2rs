

def leap(y: int) -> bool:
    return y % 4 == 0 and (y % 100 != 0 or y % 400 == 0)


def month_days(y: int, m: int) -> int:
    if m == 2 and leap(y):
        return 29
    days = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    return days[m - 1]


def parse(s: str) -> tuple[int, int, int]:
    p = s.split("-")
    return (int(p[0]), int(p[1]), int(p[2]))


def valid(s: str) -> bool:
    p = s.split("-")
    if len(p) != 3 or not all(x.isdigit() for x in p):
        return False
    y, m, d = parse(s)
    return 1 <= m <= 12 and 1 <= d <= month_days(y, m)


def ordinal(y: int, m: int, d: int) -> int:
    days = 0
    for yy in range(1970, y):
        days += 366 if leap(yy) else 365
    for mm in range(1, m):
        days += month_days(y, mm)
    return days + d - 1


def fmt(y: int, m: int, d: int) -> str:
    return f"{y:04d}-{m:02d}-{d:02d}"


def add_days(y: int, m: int, d: int, n: int) -> tuple[int, int, int]:
    for _ in range(n):
        d += 1
        if d > month_days(y, m):
            d = 1
            m += 1
            if m > 12:
                m = 1
                y += 1
    return (y, m, d)


def main() -> None:
    names = "Jan Feb Mar Apr May Jun Jul Aug Sep Oct Nov Dec".split()
    week = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
    inputs = ["2024-02-29", "2023-02-29", "1999-12-31", "2000-1-5", "abc", "2026-10-29", "2025-13-01"]
    for s in inputs:
        if not valid(s):
            print(f"{s:>12}: invalid")
            continue
        y, m, d = parse(s)
        n = ordinal(y, m, d)
        wd = week[(n + 3) % 7]
        print(f"{s:>12}: {fmt(y, m, d)} = {wd} {d} {names[m - 1]} {y}, day {n}")
    y, m, d = add_days(2024, 2, 20, 15)
    print("2024-02-20 + 15 =", fmt(y, m, d))
    a = ordinal(2000, 1, 1)
    b = ordinal(2026, 10, 29)
    print("days between:", b - a, "weeks:", (b - a) // 7, "rest:", (b - a) % 7)
    print("leap years 1990-2030:", [yy for yy in range(1990, 2031) if leap(yy)])
    print(f"{'':4}" + "".join(f"{w:>4}" for w in week))
    first = (ordinal(2026, 10, 1) + 3) % 7
    line = f"{'':4}" + "    " * first
    for day in range(1, month_days(2026, 10) + 1):
        line += f"{day:>4}"
        if (first + day) % 7 == 0:
            print(line)
            line = f"{'':4}"
    if line.strip():
        print(line)


main()
