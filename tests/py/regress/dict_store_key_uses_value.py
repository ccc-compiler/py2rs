# regression: Wrongly rejected: `it` is dead after the store; hoisting the key (k = it.sku; stock[k] = it) is accepted:
from dataclasses import dataclass


@dataclass
class Item:
    sku: str
    qty: int


def main() -> None:
    stock: dict[str, Item] = {}
    for s in ["a", "b"]:
        it = Item(s, 1)
        stock[it.sku] = it
    stock["a"].qty += 1
    print(stock)


main()
