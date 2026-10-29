def luhn_ok(num: str) -> bool:
    total = 0
    for i, ch in enumerate(reversed(num)):
        d = int(ch)
        if i % 2 == 1:
            d *= 2
            if d > 9:
                d -= 9
        total += d
    return total % 10 == 0


def isbn10_ok(s: str) -> bool:
    s = s.replace("-", "")
    if len(s) != 10:
        return False
    total = 0
    for i in range(10):
        ch = s[i]
        if ch == "X" and i == 9:
            v = 10
        elif ch.isdigit():
            v = int(ch)
        else:
            return False
        total += (10 - i) * v
    return total % 11 == 0


def adler32(data: str) -> int:
    a = 1
    b = 0
    for ch in data:
        a = (a + ord(ch)) % 65521
        b = (b + a) % 65521
    return (b << 16) | a


def main() -> None:
    n = int(input())
    kinds: dict[str, list[bool]] = {}
    for _ in range(n):
        line = input().strip()
        kind = line.split(":")[0]
        value = line.split(":")[1].strip()
        if kind == "card":
            ok = value.isdigit() and luhn_ok(value)
        elif kind == "isbn":
            ok = isbn10_ok(value)
        else:
            ok = False
        kinds.setdefault(kind, []).append(ok)
        masked = value if kind != "card" else "*" * (len(value) - 4) + value[-4:]
        print(f"{kind:<5} {masked:>20}  {'valid' if ok else 'INVALID'}")
    for k in sorted(kinds):
        v = kinds[k]
        print(f"{k}: {sum(1 for x in v if x)}/{len(v)} valid")
    for text in ["Wikipedia", "data processing", ""]:
        h = adler32(text)
        print(f"adler32({text!r}) = {h} = 0x{h:08x}")


main()
