from dataclasses import dataclass


@dataclass
class Row:
    name: str
    qty: int
    price: float
    rate: float


def render(rows: list[Row]) -> list[str]:
    w = max(len(r.name) for r in rows)
    w = max(w, 4)
    head = f"{'Name':<8}"
    lines: list[str] = []
    lines.append("Name".ljust(w) + " | " + f"{'Qty':>5} | {'Price':>10} | {'Total':>12} | {'Rate':>7}")
    lines.append("-" * w + "-+-" + "-" * 5 + "-+-" + "-" * 10 + "-+-" + "-" * 12 + "-+-" + "-" * 7)
    for r in rows:
        total = r.qty * r.price
        lines.append(r.name.ljust(w) + " | " + f"{r.qty:>5,} | {r.price:>10.2f} | {total:>12,.2f} | {r.rate:>7.2%}")
    grand = sum(r.qty * r.price for r in rows)
    lines.append(head.strip() + "s " + str(len(rows)))
    lines.append("TOTAL".ljust(w) + " | " + f"{sum(r.qty for r in rows):>5,} | {'':>10} | {grand:>12,.2f} |")
    return lines


def main() -> None:
    rows = [Row("apple", 120, 0.5, 0.05), Row("banana split", 3, 7.25, 0.125),
            Row("café au lait", 1500, 3.1, 0.0), Row("x", 1, 1234567.891, 1.0),
            Row("negative", -4, 2.675, -0.0333)]
    for line in render(rows):
        print(line)
    print(rows[0])
    print(Row("q", 0, 0.0, 0.0) == Row("q", 0, 0.0, 0.0))


main()
