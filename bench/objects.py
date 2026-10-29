from dataclasses import dataclass


@dataclass
class Item:
    name: str
    price: int
    qty: int


def main() -> None:
    items: list[Item] = []
    x = 7
    for i in range(300_000):
        x = (x * 48271) % 2147483647
        items.append(Item(f"item{x % 1000}", x % 997, x % 13))
    items.sort(key=lambda it: (it.price, it.name))
    total = 0
    for it in items:
        total += it.price * it.qty
    by_name: dict[str, int] = {}
    for it in items:
        by_name[it.name] = by_name.get(it.name, 0) + it.qty
    print(items[0], items[-1], total, len(by_name))


if __name__ == "__main__":
    main()
