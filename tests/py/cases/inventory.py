from collections import defaultdict
from dataclasses import dataclass


@dataclass
class Item:
    name: str
    category: str
    price: float
    qty: int


def by_category(items: list[Item]) -> dict[str, list[str]]:
    groups: defaultdict[str, list[str]] = defaultdict(list)
    for it in items:
        groups[it.category].append(it.name)
    return {k: v for k, v in sorted(groups.items())}


def restock(items: list[Item], minimum: int) -> int:
    added = 0
    for it in items:
        if it.qty < minimum:
            added += minimum - it.qty
            it.qty = minimum
    return added


def report(items: list[Item]) -> None:
    for it in sorted(items, key=lambda i: (-i.price * i.qty, i.name)):
        print(f"{it.name:<8}{it.qty:>4} x {it.price:>6.2f} = {it.price * it.qty:>8.2f}")


def main() -> None:
    items = [
        Item("pen", "office", 1.5, 40),
        Item("lamp", "home", 24.0, 2),
        Item("paper", "office", 4.25, 7),
        Item("mug", "home", 6.0, 0),
    ]
    print(by_category(items))
    print("added", restock(items, 5))
    report(items)


if __name__ == "__main__":
    main()
