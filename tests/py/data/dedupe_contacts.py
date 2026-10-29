from dataclasses import dataclass


@dataclass
class Contact:
    name: str
    email: str
    phone: str
    city: str


def norm_phone(p: str) -> str:
    digits = "".join(ch for ch in p if ch.isdigit())
    if len(digits) == 11 and digits.startswith("1"):
        digits = digits[1:]
    return digits


def norm_name(n: str) -> str:
    parts = n.replace(",", " ").split()
    if "," in n and len(parts) == 2:
        parts = [parts[1], parts[0]]
    return " ".join(p.capitalize() for p in parts)


def fmt_phone(d: str) -> str:
    if len(d) != 10:
        return d if d else "-"
    return f"({d[:3]}) {d[3:6]}-{d[6:]}"


def main() -> None:
    n = int(input())
    raw: list[Contact] = []
    for _ in range(n):
        f = input().split(";")
        raw.append(Contact(norm_name(f[0]), f[1].strip().lower(), norm_phone(f[2]), f[3].strip().title()))
    merged: dict[str, Contact] = {}
    order: list[str] = []
    dupes = 0
    for c in raw:
        key = c.email if c.email else c.phone
        if key in merged:
            dupes += 1
            old = merged[key]
            merged[key] = Contact(old.name if len(old.name) >= len(c.name) else c.name,
                                  old.email if old.email else c.email,
                                  old.phone if old.phone else c.phone,
                                  old.city if old.city else c.city)
        else:
            merged[key] = Contact(c.name, c.email, c.phone, c.city)
            order.append(key)
    print(f"{n} rows, {dupes} duplicates, {len(merged)} contacts")
    print(f"{'Name':<18}{'Email':<24}{'Phone':<16}City")
    for k in order:
        c = merged[k]
        print(f"{c.name:<18}{c.email if c.email else '-':<24}{fmt_phone(c.phone):<16}{c.city if c.city else '-'}")
    cities: dict[str, int] = {}
    for c in merged.values():
        if c.city:
            cities[c.city] = cities.get(c.city, 0) + 1
    print(", ".join(f"{city}={cities[city]}" for city in sorted(cities)))
    domains = sorted({c.email.split("@")[1] for c in merged.values() if "@" in c.email})
    print("domains:", domains)


main()
