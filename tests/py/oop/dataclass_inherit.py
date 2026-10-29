from dataclasses import dataclass


@dataclass
class Item:
    name: str
    price: float

    def total(self) -> float:
        return self.price

    def label(self) -> str:
        return f"{self.name} @ {self.price:.2f}"


@dataclass
class Bulk(Item):
    qty: int = 1
    discount: float = 0.0

    def total(self) -> float:
        gross = self.price * self.qty
        return gross * (1.0 - self.discount)

    def label(self) -> str:
        return f"{self.name} @ {self.price:.2f} x{self.qty}"


@dataclass
class Taxed(Bulk):
    tax: float = 0.2

    def total(self) -> float:
        return round(super().total() * (1.0 + self.tax), 2)

    def label(self) -> str:
        return super().label() + f" +{self.tax:.0%} tax"


@dataclass(frozen=True)
class Coupon:
    code: str
    off: float
    min_total: float = 0.0


class Cart:
    def __init__(self) -> None:
        self.items: list[Item] = []
        self.coupons: list[Coupon] = []

    def add(self, item: Item) -> None:
        self.items.append(item)

    def subtotal(self) -> float:
        return sum(i.total() for i in self.items)

    def apply(self, c: Coupon) -> bool:
        if self.subtotal() >= c.min_total and c not in self.coupons:
            self.coupons.append(c)
            return True
        return False

    def total(self) -> float:
        t = self.subtotal()
        for c in self.coupons:
            t -= c.off
        return max(t, 0.0)


def main() -> None:
    cart = Cart()
    cart.add(Item("pen", 1.25))
    cart.add(Bulk("paper", 0.05, 500, 0.1))
    cart.add(Taxed("lamp", 20.0, 2, 0.0, 0.25))
    cart.add(Taxed("desk", 150.0))
    cart.add(Bulk("clip", 0.02, qty=100))
    for it in cart.items:
        print(f"{it.label():<30}{it.total():>10.2f}")
    print(cart.items[1])
    print(cart.items[2])
    print(f"subtotal {cart.subtotal():.2f}")
    c1 = Coupon("SAVE10", 10.0, 100.0)
    c2 = Coupon("BIG", 50.0, 1000.0)
    print(cart.apply(c1), cart.apply(c2), cart.apply(Coupon("SAVE10", 10.0, 100.0)))
    print(f"total {cart.total():.2f}")
    print(c1 == Coupon("SAVE10", 10.0, 100.0), c1 == c2)
    print(Bulk("a", 1.0) == Bulk("a", 1.0, 1, 0.0), Taxed("a", 1.0).tax)
    kinds = [type_name(i) for i in cart.items]
    print(kinds)


def type_name(i: Item) -> str:
    if isinstance(i, Taxed):
        return "taxed"
    if isinstance(i, Bulk):
        return "bulk"
    return "plain"


main()
