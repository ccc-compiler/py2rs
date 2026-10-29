from dataclasses import dataclass


class OrderError(Exception):
    pass


@dataclass(frozen=True)
class Line:
    sku: str
    qty: int
    unit: float


class Order:
    def __init__(self, oid: int, customer: str) -> None:
        self.oid = oid
        self.customer = customer
        self.lines: list[Line] = []
        self.status = "new"
        self.history: list[str] = ["new"]

    def add(self, sku: str, qty: int, unit: float) -> None:
        if self.status != "new":
            raise OrderError(f"order {self.oid} is {self.status}, cannot add")
        if qty <= 0:
            raise OrderError(f"order {self.oid}: bad quantity {qty}")
        self.lines.append(Line(sku, qty, unit))

    def total(self) -> float:
        return round(sum(l.qty * l.unit for l in self.lines), 2)

    def move(self, to: str) -> None:
        allowed = {"new": ["paid", "cancelled"], "paid": ["shipped", "refunded"], "shipped": ["delivered"]}
        if to not in allowed.get(self.status, []):
            raise OrderError(f"order {self.oid}: cannot go from {self.status} to {to}")
        self.status = to
        self.history.append(to)


class PriorityOrder(Order):
    def total(self) -> float:
        return round(super().total() + 9.99, 2)

    def move(self, to: str) -> None:
        super().move(to)
        if to == "paid":
            self.history.append("expedite")


class Shop:
    def __init__(self) -> None:
        self.orders: dict[int, Order] = {}
        self.stock = {"apple": 10, "pear": 4, "fig": 0}
        self.next_id = 1

    def open(self, customer: str, priority: bool = False) -> int:
        oid = self.next_id
        self.next_id += 1
        if priority:
            self.orders[oid] = PriorityOrder(oid, customer)
        else:
            self.orders[oid] = Order(oid, customer)
        return oid

    def pay(self, oid: int) -> None:
        for l in self.orders[oid].lines:
            if self.stock.get(l.sku, 0) < l.qty:
                raise OrderError(f"order {oid}: not enough {l.sku}")
        for l in self.orders[oid].lines:
            self.stock[l.sku] -= l.qty
        self.orders[oid].move("paid")


def attempt(shop: Shop, what: str, oid: int) -> None:
    try:
        if what == "pay":
            shop.pay(oid)
        else:
            shop.orders[oid].move(what)
        print(f"  {what:<9} #{oid}: ok -> {shop.orders[oid].status}")
    except OrderError as err:
        print(f"  {what:<9} #{oid}: FAILED {err}")


def main() -> None:
    shop = Shop()
    a = shop.open("ann")
    b = shop.open("bob", True)
    c = shop.open("cy")
    shop.orders[a].add("apple", 3, 0.5)
    shop.orders[a].add("pear", 2, 1.25)
    shop.orders[b].add("apple", 6, 0.5)
    shop.orders[c].add("pear", 3, 1.25)
    shop.orders[c].add("fig", 1, 3.0)
    try:
        shop.orders[a].add("fig", 0, 3.0)
    except OrderError as err:
        print("error:", err)
    for oid, what in [(a, "pay"), (b, "pay"), (c, "pay"), (a, "shipped"), (c, "cancelled"), (b, "delivered"),
                      (b, "shipped"), (b, "delivered"), (a, "refunded"), (c, "paid")]:
        attempt(shop, what, oid)
    try:
        shop.orders[b].add("pear", 1, 1.25)
    except OrderError as err:
        print("error:", err)
    for o in shop.orders.values():
        kind = "priority" if isinstance(o, PriorityOrder) else "standard"
        print(f"#{o.oid} {o.customer:<4}{kind:<9}{o.total():>7.2f} {o.status:<10} {' > '.join(o.history)}")
    print("stock", shop.stock)


main()
