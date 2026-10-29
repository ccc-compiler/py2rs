def parse_age(s: str) -> int:
    n = int(s)
    if n < 0 or n > 150:
        raise ValueError(f"age out of range: {n}")
    return n


def parse_price(s: str) -> float:
    s = s.strip()
    if s.startswith("$"):
        s = s[1:]
    v = float(s.replace(",", ""))
    if v < 0:
        raise ValueError("negative price")
    return v


def try_int(s: str) -> str:
    try:
        return f"int ok: {int(s)}"
    except ValueError as e:
        return f"error: {e}"


def try_float(s: str) -> str:
    try:
        return f"float ok: {float(s)}"
    except ValueError as e:
        return f"error: {e}"


def main() -> None:
    for s in ["42", " 17 ", "-3", "+8", "3.5", "", "abc", "1_000", "0x1f", "007", "1e3", "  -0  "]:
        print(repr(s), "->", try_int(s))
    for s in ["3.25", "-0.5", "1e-3", "inf", "-inf", "1.5e300", ".5", "5.", "nope", "1,5", " 2.0 ", "1_0.5"]:
        print(repr(s), "->", try_float(s))
    for s in ["30", "-1", "200", "x"]:
        try:
            print("age", parse_age(s))
        except ValueError as e:
            print("bad age:", e)
    total = 0.0
    bad = 0
    for s in ["$1,200.50", " 3.99", "$-2", "free", "0.01"]:
        try:
            p = parse_price(s)
            total += p
            print(f"price {p:>10.2f}")
        except ValueError as e:
            bad += 1
            print("bad price:", e)
    print(f"total={total:.2f} bad={bad}")
    print(round(total, 1), round(total), round(-2.5), round(0.125, 2))


main()
