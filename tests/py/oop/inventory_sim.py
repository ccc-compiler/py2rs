from dataclasses import dataclass


@dataclass
class Product:
    sku: str
    name: str
    stock: int
    reorder_at: int
    reorder_qty: int
    price: float


class Event:
    def __init__(self, day: int) -> None:
        self.day = day

    def apply(self, inv: "Inventory") -> str:
        return "nothing"


class Sale(Event):
    def __init__(self, day: int, sku: str, qty: int) -> None:
        super().__init__(day)
        self.sku = sku
        self.qty = qty

    def apply(self, inv: "Inventory") -> str:
        return inv.sell(self.sku, self.qty)


class Delivery(Event):
    def __init__(self, day: int, sku: str, qty: int) -> None:
        super().__init__(day)
        self.sku = sku
        self.qty = qty

    def apply(self, inv: "Inventory") -> str:
        inv.products[self.sku].stock += self.qty
        inv.pending.remove(self.sku)
        return f"delivered {self.qty} {self.sku}"


class Audit(Event):
    def apply(self, inv: "Inventory") -> str:
        value = sum(p.stock * p.price for p in inv.products.values())
        return f"audit: stock value {value:.2f}"


class Inventory:
    def __init__(self) -> None:
        self.products: dict[str, Product] = {}
        self.pending: list[str] = []
        self.revenue = 0.0
        self.lost = 0

    def add(self, sku: str, name: str, stock: int, at: int, qty: int, price: float) -> None:
        self.products[sku] = Product(sku, name, stock, at, qty, price)

    def sell(self, sku: str, qty: int) -> str:
        sold = min(qty, self.products[sku].stock)
        self.products[sku].stock -= sold
        self.lost += qty - sold
        self.revenue += sold * self.products[sku].price
        return f"sold {sold}/{qty} {self.products[sku].name}"

    def reorders(self, day: int) -> list[Event]:
        out: list[Event] = []
        for p in self.products.values():
            if p.stock <= p.reorder_at and p.sku not in self.pending:
                out.append(Delivery(day + 3, p.sku, p.reorder_qty))
        for e in out:
            if isinstance(e, Delivery):
                self.pending.append(e.sku)
        return out


def main() -> None:
    inv = Inventory()
    inv.add("A1", "widget", 20, 5, 15, 2.5)
    inv.add("B2", "gadget", 8, 3, 10, 12.0)
    inv.add("C3", "doohickey", 2, 1, 4, 40.0)
    events: list[Event] = []
    for d in range(1, 15):
        events.append(Sale(d, "A1", (d * 7) % 6 + 1))
        if d % 2 == 0:
            events.append(Sale(d, "B2", d % 4 + 1))
        if d % 5 == 0:
            events.append(Sale(d, "C3", 2))
        if d % 7 == 0:
            events.append(Audit(d))
    day = 1
    while day <= 16:
        todays = [e for e in events if e.day == day]
        for e in todays:
            print(f"day {day:2}: {e.apply(inv)}")
        new = inv.reorders(day)
        for e in new:
            if isinstance(e, Delivery):
                print(f"day {day:2}: reorder {e.qty} {e.sku} for day {e.day}")
            events.append(e)
        day += 1
    for p in inv.products.values():
        print(p)
    print(f"revenue {inv.revenue:.2f}, lost sales {inv.lost}, pending {inv.pending}")


main()
