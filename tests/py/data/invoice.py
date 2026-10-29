from dataclasses import dataclass


@dataclass(frozen=True)
class Line:
    desc: str
    qty: int
    unit_cents: int
    taxable: bool


def cents(c: int) -> str:
    return f"{c // 100:,}.{c % 100:02d}"


def parse(text: str) -> list[Line]:
    out: list[Line] = []
    for raw in text.strip().splitlines():
        if raw.startswith("#") or not raw.strip():
            continue
        f = [p.strip() for p in raw.split(",")]
        price = f[2].split(".")
        out.append(Line(f[0], int(f[1]), int(price[0]) * 100 + int(price[1]), f[3] == "T"))
    return out


def main() -> None:
    text = """
# description, qty, unit price, taxable
Consulting hours, 12, 150.00, F
USB cable, 3, 7.99, T
Monitor 27in, 2, 289.50, T

Shipping, 1, 25.00, F
Laptop stand, 1, 51.25, T
"""
    lines = parse(text)
    rate_bp = 825
    w = max(len(l.desc) for l in lines)
    print('Item'.ljust(w) + f"  {'Qty':>3}  {'Unit':>9}  {'Amount':>10}")
    subtotal = 0
    taxable = 0
    for l in lines:
        amount = l.qty * l.unit_cents
        subtotal += amount
        if l.taxable:
            taxable += amount
        mark = "*" if l.taxable else " "
        print(f"{l.desc.ljust(w)}  {l.qty:>3}  {cents(l.unit_cents):>9}  {cents(amount):>10}{mark}")
    tax = (taxable * rate_bp + 5000) // 10000
    print("=" * (w + 30))
    print(f"{'Subtotal'.rjust(w + 17)}  {cents(subtotal):>10}")
    print(f"{'Tax 8.25% on'.rjust(w + 6)} {cents(taxable):>10}  {cents(tax):>10}")
    print(f"{'TOTAL'.rjust(w + 17)}  {cents(subtotal + tax):>10}")
    biggest = max(lines, key=lambda l: l.qty * l.unit_cents)
    print("largest line:", biggest.desc)
    print(sorted(l.desc for l in lines if not l.taxable))


main()
