from dataclasses import dataclass


@dataclass
class Customer:
    cid: int
    name: str
    tier: str


@dataclass
class Order:
    oid: int
    cid: int
    sku: str
    qty: int


def main() -> None:
    customers = [Customer(1, "Acme", "gold"), Customer(2, "Globex", "silver"),
                 Customer(3, "Initech", "bronze"), Customer(4, "Umbrella", "gold")]
    prices = {"bolt": 0.10, "nut": 0.05, "gear": 4.50, "belt": 12.00, "chain": 25.00}
    order_text = """1001 1 bolt 500
1002 2 gear 10
1003 1 belt 2
1004 3 chain 1
1005 2 nut 1000
1006 5 gear 3
1007 1 gear 4
1008 4 chain 6
1009 3 sprocket 2"""
    orders: list[Order] = []
    for line in order_text.splitlines():
        p = [int(x) if x.isdigit() else 0 for x in line.split()]
        orders.append(Order(p[0], p[1], line.split()[2], p[3]))
    by_id = {c.cid: c for c in customers}
    discount = {"gold": 0.15, "silver": 0.05, "bronze": 0.0}
    spend: dict[int, float] = {}
    print(f"{'order':<6}{'customer':<10}{'item':<7}{'qty':>5}{'amount':>10}")
    for o in orders:
        if o.cid not in by_id:
            print(f"{o.oid:<6}{'???':<10}{o.sku:<7}{o.qty:>5}{'orphan':>10}")
            continue
        if o.sku not in prices:
            print(f"{o.oid:<6}{by_id[o.cid].name:<10}{o.sku:<7}{o.qty:>5}{'no price':>10}")
            continue
        c = by_id[o.cid]
        amount = prices[o.sku] * o.qty * (1 - discount[c.tier])
        spend[c.cid] = spend.get(c.cid, 0.0) + amount
        print(f"{o.oid:<6}{c.name:<10}{o.sku:<7}{o.qty:>5}{amount:>10.2f}")
    print()
    for c in sorted(customers, key=lambda c: -spend.get(c.cid, 0.0)):
        n = len([o for o in orders if o.cid == c.cid])
        print(f"{c.name:<10}{c.tier:<8}{n:>3} orders {spend.get(c.cid, 0.0):>10.2f}")
    no_orders = [c.name for c in customers if c.cid not in spend]
    print("customers without valid orders:", no_orders)
    sku_qty: dict[str, int] = {}
    for o in orders:
        sku_qty[o.sku] = sku_qty.get(o.sku, 0) + o.qty
    print(sorted(sku_qty.items()))
    unsold = sorted(set(prices) - set(sku_qty))
    print("never ordered:", unsold)


main()
