# regression: Rust compile error: literal out of range for `i32` (literal `1000000000000`).
def scale(s: str) -> float:
    mult = {"K": 1000, "T": 1000000000000}
    return float(s[:-1]) * mult[s[-1]]


print(scale("1.5T"))
