class FieldError(Exception):
    pass


def field(rec: dict[str, str], key: str) -> str:
    try:
        return rec[key]
    except KeyError:
        raise FieldError(f"no field {key}")


def int_field(rec: dict[str, str], key: str, lo: int, hi: int) -> int:
    s = field(rec, key)
    try:
        v = int(s)
    except ValueError:
        raise FieldError(f"{key}={s!r} is not an integer")
    if v < lo or v > hi:
        raise FieldError(f"{key}={v} not in [{lo}, {hi}]")
    return v


def parse_record(line: str) -> dict[str, str]:
    rec: dict[str, str] = {}
    for part in line.split(";"):
        part = part.strip()
        if part == "":
            continue
        k = part.find("=")
        if k <= 0:
            raise FieldError(f"malformed part {part!r}")
        rec[part[:k].strip()] = part[k + 1:].strip()
    return rec


def date_str(line: str) -> str:
    rec = parse_record(line)
    y = int_field(rec, "year", 1, 9999)
    m = int_field(rec, "month", 1, 12)
    days = [31, 29 if (y % 4 == 0 and y % 100 != 0) or y % 400 == 0 else 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    d = int_field(rec, "day", 1, days[m - 1])
    return f"{y:04d}-{m:02d}-{d:02d}"


def main() -> None:
    lines = ["year=2024; month=2; day=29", "year=2023; month=2; day=29", "year=1900;month=2;day=29",
             "year=2000; month=2; day=29", "year=99; month=12; day=31", "month=1; day=1",
             "year=2024; month=13; day=1", "year=2024; month=x; day=1", "garbage", "year=2024;;month=4;day=30;",
             "=5", "year = 2025 ; month = 07 ; day = 04 "]
    good = 0
    for line in lines:
        try:
            print(f"{line!r:<34} -> {date_str(line)}")
            good += 1
        except FieldError as e:
            print(f"{line!r:<34} -> FieldError: {e}")
    print(f"{good}/{len(lines)} valid ({good / len(lines):.1%})")


main()
