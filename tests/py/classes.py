from dataclasses import dataclass
import math


@dataclass
class Point:
    x: float
    y: float

    def dist(self, other: "Point") -> float:
        return math.hypot(self.x - other.x, self.y - other.y)


@dataclass(frozen=True)
class Item:
    name: str
    price: float
    qty: int = 1


class Account:
    def __init__(self, owner: str, balance: int) -> None:
        self.owner = owner
        self.balance = balance
        self.history: list[int] = []

    def deposit(self, amount: int) -> None:
        if amount <= 0:
            raise ValueError("amount must be positive")
        self.balance += amount
        self.history.append(amount)

    def withdraw(self, amount: int) -> bool:
        if amount > self.balance:
            return False
        self.balance -= amount
        self.history.append(-amount)
        return True

    def __str__(self) -> str:
        return f"{self.owner}: {self.balance}"


class Stack:
    def __init__(self) -> None:
        self.items: list[int] = []

    def push(self, x: int) -> None:
        self.items.append(x)

    def pop(self) -> int:
        return self.items.pop()

    def empty(self) -> bool:
        return len(self.items) == 0


def total(items: list[Item]) -> float:
    return sum(i.price * i.qty for i in items)


def main() -> None:
    p = Point(0.0, 0.0)
    q = Point(3.0, 4.0)
    print(p, q, p.dist(q), p == q, p == Point(0.0, 0.0))
    items = [Item("pen", 1.5, 4), Item("book", 12.0)]
    items.append(Item(name="cup", price=3.25, qty=2))
    print(items[0], len(items))
    print(f"total = {total(items):.2f}")
    cheap = [i.name for i in items if i.price < 5]
    print(cheap)
    acc = Account("ann", 100)
    acc.deposit(50)
    print(acc.withdraw(500), acc.withdraw(30))
    print(acc, acc.history)
    s = Stack()
    for i in range(5):
        s.push(i * i)
    while not s.empty():
        print(s.pop(), end=" ")
    print()
    by_price = sorted(items, key=lambda i: i.price)
    print([i.name for i in by_price])
    best = max(items, key=lambda i: i.price * i.qty)
    print(best.name)


main()
