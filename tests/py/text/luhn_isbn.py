class ChecksumError(ValueError):
    pass


def digits_only(s: str) -> str:
    out = ""
    for c in s:
        if c.isdigit():
            out += c
        elif c not in " -":
            raise ChecksumError(f"invalid character {c!r}")
    return out


def luhn_ok(number: str) -> bool:
    ds = digits_only(number)
    if 2 > len(ds):
        raise ChecksumError("too short")
    total = 0
    for i, c in enumerate(reversed(ds)):
        d = int(c)
        if i % 2 == 1:
            d *= 2
            if d > 9:
                d -= 9
        total += d
    return total % 10 == 0


def isbn10_ok(s: str) -> bool:
    s = s.replace("-", "").upper()
    if len(s) != 10:
        raise ChecksumError(f"ISBN-10 needs 10 characters, got {len(s)}")
    total = 0
    for i in range(10):
        c = s[i]
        if c == "X" and i == 9:
            v = 10
        elif c.isdigit():
            v = int(c)
        else:
            raise ChecksumError(f"bad ISBN character {c!r}")
        total += (10 - i) * v
    return total % 11 == 0


def isbn13_check_digit(s12: str) -> int:
    total = 0
    for i in range(12):
        total += int(s12[i]) * (1 if i % 2 == 0 else 3)
    return (10 - total % 10) % 10


def main() -> None:
    for n in ["4539 1488 0343 6467", "8273 1232 7352 0569", "0", "79927398713", "1234-5678-9012-3456", "12a4"]:
        try:
            print(f"{n:<22} luhn={luhn_ok(n)}")
        except ChecksumError as e:
            print(f"{n:<22} error: {e}")
    for s in ["0-306-40615-2", "0-306-40615-3", "0-8044-2957-X", "123", "12345678Y0"]:
        try:
            print(f"{s:<16} isbn10={isbn10_ok(s)}")
        except ValueError as e:
            print(f"{s:<16} error: {e}")
    for s in ["978030640615", "978316148410", "000000000000"]:
        print(s, isbn13_check_digit(s), s + str(isbn13_check_digit(s)))


main()
