class DurationError(Exception):
    pass


def parse_duration(s: str) -> int:
    total = 0
    num = ""
    units = {"d": 86400, "h": 3600, "m": 60, "s": 1}
    seen = ""
    for c in s.strip().lower():
        if c.isdigit():
            num += c
        elif c in units:
            if num == "":
                raise DurationError(f"unit {c} without a number in {s!r}")
            if c in seen:
                raise DurationError(f"unit {c} repeated in {s!r}")
            seen += c
            total += int(num) * units[c]
            num = ""
        elif c == " ":
            continue
        else:
            raise DurationError(f"unexpected {c!r} in {s!r}")
    if num != "":
        total += int(num)
    return total


def fmt_duration(secs: int) -> str:
    if secs < 0:
        return "-" + fmt_duration(-secs)
    d, rem = divmod(secs, 86400)
    h, rem = divmod(rem, 3600)
    m, s = divmod(rem, 60)
    parts: list[str] = []
    if d != 0:
        parts.append(f"{d}d")
    if h != 0:
        parts.append(f"{h}h")
    if m != 0:
        parts.append(f"{m}m")
    if s != 0 or len(parts) == 0:
        parts.append(f"{s}s")
    return " ".join(parts)


def clock(secs: int) -> str:
    h = secs // 3600
    m = secs % 3600 // 60
    s = secs % 60
    return f"{h:02d}:{m:02d}:{s:02d}"


def main() -> None:
    for s in ["1h30m", "90", "2d 3h 4m 5s", "45s", "1m1m", "h", "10x", "0", "100000", " 3H "]:
        try:
            v = parse_duration(s)
            print(f"{s!r:>14} = {v:>7} s = {fmt_duration(v):<14} {clock(v)} {v / 3600:.4f}h")
        except DurationError as e:
            print("error:", e)
    print(fmt_duration(-3725), clock(359999), fmt_duration(86400 * 400))
    total = 0
    for s in ["25m", "1h5m", "50m", "2h"]:
        total += parse_duration(s)
    print("total", fmt_duration(total), f"avg {total / 4 / 60:.1f} min")


main()
