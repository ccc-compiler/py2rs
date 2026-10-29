from dataclasses import dataclass


@dataclass
class Item:
    sku: str
    name: str
    qty: int
    cost: float
    reorder: int


def parse_item(line: str) -> Item:
    f = [x.strip() for x in line.split("|")]
    return Item(f[0], f[1], int(f[2]), float(f[3]), int(f[4]))


def main() -> None:
    n = int(input())
    stock: dict[str, Item] = {}
    for _ in range(n):
        it = parse_item(input())
        key = it.sku
        stock[key] = it
    m = int(input())
    for _ in range(m):
        parts = input().split()
        kind = parts[0]
        sku = parts[1]
        amount = int(parts[2])
        if sku not in stock:
            print(f"?? unknown sku {sku}")
            continue
        if kind == "in":
            stock[sku].qty += amount
            print(f"+{amount:<4} {stock[sku].name}")
        elif kind == "out":
            if stock[sku].qty < amount:
                print(f"!! short {stock[sku].name}: want {amount}, have {stock[sku].qty}")
                stock[sku].qty = 0
            else:
                stock[sku].qty -= amount
                print(f"-{amount:<4} {stock[sku].name}")
    print()
    print(f"{'SKU':<6} {'Name':<14} {'Qty':>5} {'Value':>9}  Status")
    total = 0.0
    for it in sorted(stock.values(), key=lambda i: i.name):
        value = it.qty * it.cost
        total += value
        status = "REORDER" if it.qty <= it.reorder else "ok"
        print(f"{it.sku:<6} {it.name:<14} {it.qty:>5} {value:>9.2f}  {status}")
    print(f"{'':<27}{total:>9.2f}")
    low = [it.sku for it in stock.values() if it.qty <= it.reorder]
    print("to reorder:", ", ".join(sorted(low)) if low else "nothing")


main()
