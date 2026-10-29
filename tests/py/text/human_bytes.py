def human(n: int, binary: bool = False) -> str:
    base = 1024 if binary else 1000
    units = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"] if binary else ["B", "kB", "MB", "GB", "TB", "PB"]
    if abs(n) < base:
        return f"{n} B"
    v = float(n)
    i = 0
    while abs(v) >= base and i < len(units) - 1:
        v /= base
        i += 1
    return f"{v:.1f} {units[i]}"


def parse_size(s: str) -> float:
    s = s.strip().upper().replace(" ", "")
    mult = {"K": 1e3, "M": 1e6, "G": 1e9, "T": 1e12}
    if s.endswith("B"):
        s = s[:-1]
    if s == "":
        raise ValueError("empty size")
    last = s[-1]
    if last in mult:
        m = mult[last]
        return float(s[:-1]) * m
    return float(s)


def main() -> None:
    for n in [0, 1, 999, 1000, 1023, 1024, 1536, 10 ** 6, 123456789, 2 ** 30, 2 ** 40 + 2 ** 39,
              10 ** 15, 10 ** 18, -2048, 9223372036854775807]:
        print(f"{n:>22,} {human(n):>10} {human(n, True):>10}")
    for s in ["10", "1.5K", "2 MB", "0.25G", "3t", "kb", "", "7.5", "1e3K", "12Q"]:
        try:
            v = parse_size(s)
            print(f"{s!r:>8} -> {v:,.1f} {v}")
        except ValueError as e:
            print(f"{s!r:>8} -> ValueError: {e}")
    total = 0
    for x in ["1K", "2K", "500"]:
        total += int(x.replace("K", "000"))
    print(total, human(total), f"{total / 1024:.3f}", f"{total:e}", f"{total:_d}")


main()
