from dataclasses import dataclass

DATA = """region,product,units,price
north,widget,12,2.50
south,gadget,5,10.00
north,gadget,7,10.00
east,widget,20,2.50
south,widget,3,2.50
east,gizmo,4,7.25
north,gizmo,9,7.25
west,widget,15,2.50
west,gadget,2,10.00
east,gadget,6,10.00"""


@dataclass
class Sale:
    region: str
    product: str
    units: int
    price: float

    def total(self) -> float:
        return self.units * self.price


def parse(text: str) -> list[Sale]:
    lines = text.strip().split("\n")
    out: list[Sale] = []
    for line in lines[1:]:
        f = line.split(",")
        out.append(Sale(f[0], f[1], int(f[2]), float(f[3])))
    return out


def by_key(sales: list[Sale], region: bool) -> dict[str, float]:
    res: dict[str, float] = {}
    for s in sales:
        k = s.region if region else s.product
        res[k] = res.get(k, 0.0) + s.total()
    return res


def main() -> None:
    sales = parse(DATA)
    print(f"{len(sales)} records")
    print(f"{'Region':<8}{'Product':<8}{'Units':>6}{'Total':>10}")
    print("-" * 32)
    for s in sales:
        print(f"{s.region:<8}{s.product:<8}{s.units:>6}{s.total():>10.2f}")
    print("-" * 32)
    regions = by_key(sales, True)
    for name in sorted(regions):
        print(f"region {name:>6}: {regions[name]:8.2f}")
    products = by_key(sales, False)
    ranked = sorted(products.items(), key=lambda kv: kv[1], reverse=True)
    for i, (name, amount) in enumerate(ranked, 1):
        print(f"#{i} {name:<7} {amount:>8.2f}")
    grand = sum(s.total() for s in sales)
    print(f"grand total {grand:.2f}")
    best = max(sales, key=lambda s: s.total())
    print("best sale:", best.region, best.product, f"{best.total():.2f}")
    units = sum(s.units for s in sales)
    print(f"average price per unit: {grand / units:.3f}")


main()
