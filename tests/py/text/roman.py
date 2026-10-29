class RomanError(ValueError):
    pass


def values() -> list[tuple[int, str]]:
    return [(1000, "M"), (900, "CM"), (500, "D"), (400, "CD"), (100, "C"), (90, "XC"),
          (50, "L"), (40, "XL"), (10, "X"), (9, "IX"), (5, "V"), (4, "IV"), (1, "I")]


def to_roman(n: int) -> str:
    if n <= 0 or n >= 4000:
        raise RomanError(f"cannot write {n} in roman numerals")
    out = ""
    for v, s in values():
        while n >= v:
            out += s
            n -= v
    return out


def from_roman(s: str) -> int:
    single = {"I": 1, "V": 5, "X": 10, "L": 50, "C": 100, "D": 500, "M": 1000}
    total = 0
    prev = 0
    for c in reversed(s.upper()):
        if c not in single:
            raise RomanError(f"bad roman digit {c!r}")
        v = single[c]
        if v < prev:
            total -= v
        else:
            total += v
            prev = v
    if to_roman(total) != s.upper():
        raise RomanError(f"non-canonical numeral {s}")
    return total


def main() -> None:
    for n in [1, 4, 9, 14, 40, 90, 400, 1994, 2024, 3999, 0, 4000, -5]:
        try:
            r = to_roman(n)
            print(f"{n:5d} -> {r:<15} -> {from_roman(r)}")
        except RomanError as e:
            print("error:", e)
    for s in ["XLII", "mcmxcix", "IIII", "IC", "ABC", "MMMCMXCIX", "VX"]:
        try:
            print(s, "=", from_roman(s))
        except RomanError as e:
            print("RomanError", e)
        except ValueError as e:
            print("ValueError", e)
    total = 0
    for i in range(1, 500):
        total += from_roman(to_roman(i))
    print("roundtrip sum", total)
    longest = 1
    for i in range(1, 4000):
        if len(to_roman(i)) > len(to_roman(longest)):
            longest = i
    print("longest", longest, to_roman(longest), len(to_roman(longest)))


main()
