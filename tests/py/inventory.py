from dataclasses import dataclass


class OutOfStock(Exception):
    pass


@dataclass
class Item:
    name: str
    price: float
    qty: int


class Store:
    def __init__(self) -> None:
        self.items: dict[str, Item] = {}
        self.log: list[str] = []

    def add(self, name: str, price: float, qty: int) -> None:
        if price < 0:
            raise ValueError(f"negative price for {name}")
        self.items[name] = Item(name, price, qty)

    def find(self, name: str) -> Item | None:
        return self.items.get(name)

    def buy(self, name: str, n: int) -> float:
        item = self.items[name]
        if item.qty < n:
            raise OutOfStock(f"only {item.qty} {name} left")
        cost = item.price * n
        self.items[name].qty -= n
        self.log.append(f"{n} x {name}")
        return cost


class Discount:
    def apply(self, total: float) -> float:
        return total

    def label(self) -> str:
        return "none"


class Percent(Discount):
    def __init__(self, pct: int) -> None:
        if pct > 100:
            raise ValueError("percent over 100")
        self.pct = pct

    def apply(self, total: float) -> float:
        return total * (100 - self.pct) / 100

    def label(self) -> str:
        return f"{self.pct}% off"


class Fixed(Discount):
    def __init__(self, amount: float) -> None:
        self.amount = amount

    def apply(self, total: float) -> float:
        if self.amount > total:
            raise ValueError("discount larger than total")
        return total - self.amount

    def label(self) -> str:
        return f"{self.amount} off"


def checkout(store: Store, cart: list[tuple[str, int]], d: Discount) -> float:
    total = 0.0
    for name, n in cart:
        try:
            total += store.buy(name, n)
        except OutOfStock as e:
            print("skip:", e)
        except KeyError as e:
            print("unknown item", e)
    try:
        return d.apply(total)
    except ValueError as e:
        print("discount ignored:", e)
        return total


def main() -> None:
    s = Store()
    s.add("apple", 0.5, 10)
    s.add("pear", 0.75, 2)
    try:
        s.add("bad", -1.0, 1)
    except ValueError as e:
        print("error:", e)
    found = s.find("apple")
    if found is not None:
        print("found", found.name, found.qty)
    print(s.find("kiwi"))
    discounts: list[Discount] = [Discount(), Percent(10), Fixed(100.0)]
    for d in discounts:
        cart = [("apple", 3), ("pear", 1), ("kiwi", 1), ("pear", 5)]
        print(d.label(), f"{checkout(s, cart, d):.2f}")
    print(s.log)
    try:
        p = Percent(150)
        print(p.label())
    except ValueError as e:
        print("bad discount:", e)
    left = s.find("apple")
    print(left.qty if left is not None else -1)


main()
