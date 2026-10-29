# An inventory: dataclasses, the standard library, borrowing

Typical data-processing code: a dataclass, a `defaultdict`, sorting with a key and
formatted output.

- `items` is read by `by_category` and `report` (`&[Item]`) and modified by `restock`
  (`&mut Vec<Item>`): the modes are inferred from the bodies.
- `groups[it.category].append(...)` on a `defaultdict(list)` inserts the default:
  `groups.at(key).push(..)`.
- `sorted(..., key=lambda i: (-i.price * i.qty, i.name))` sorts by a tuple key, computed
  once per item like CPython does.
- The f-string format specs (`:<8`, `:>6.2f`) print exactly as in Python.
- `print("added", restock(items, 5))`: Python evaluates the arguments first, then
  prints; the generated code keeps that order with temporaries.

Source: `tests/py/cases/inventory.py` (part of the differential test).

## Python

```python
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
```

## Rust (`py2rs emit`)

```rust
use pyrt::{IndexMap, println};

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub name: String,
    pub category: String,
    pub price: f64,
    pub qty: i64,
}

impl pyrt::Py for Item {
    fn fmt_repr(&self, out: &mut String) {
        out.push_str("Item(");
        out.push_str("name=");
        self.name.fmt_repr(out);
        out.push_str(", category=");
        self.category.fmt_repr(out);
        out.push_str(", price=");
        self.price.fmt_repr(out);
        out.push_str(", qty=");
        self.qty.fmt_repr(out);
        out.push(')');
    }
}

fn by_category(items: &[Item]) -> IndexMap<String, Vec<String>> {
    let mut groups = pyrt::DefaultDict::<String, Vec<String>>::new("list");
    for it in items.iter() {
        groups.at(it.category.clone()).push(it.name.clone());
    }
    pyrt::sorted(groups.iter().map(|(p0, p1)| (p0.clone(), p1.clone()))).into_iter().map(|(k, v)| (k.clone(), v.clone())).collect::<IndexMap<String, Vec<String>>>()
}

fn restock(items: &mut Vec<Item>, minimum: i64) -> i64 {
    let mut added: i64 = 0;
    for it in items.iter_mut() {
        if it.qty < minimum {
            added += minimum - it.qty;
            it.qty = minimum;
        }
    }
    added
}

fn report(items: &[Item]) {
    for it in pyrt::sorted_by_key(items.iter().cloned(), |i| (-i.price * i.qty as f64, i.name.clone())) {
        println!("{:<8}{:>4} x {:>6.2} = {:>8.2}", it.name, it.qty, it.price, it.price * it.qty as f64);
    }
}

fn py_main() {
    let mut items = vec![Item { name: "pen".to_string(), category: "office".to_string(), price: 1.5, qty: 40 }, Item { name: "lamp".to_string(), category: "home".to_string(), price: 24.0, qty: 2 }, Item { name: "paper".to_string(), category: "office".to_string(), price: 4.25, qty: 7 }, Item { name: "mug".to_string(), category: "home".to_string(), price: 6.0, qty: 0 }];
    println!("{}", pyrt::s(&by_category(&items)));
    let t1 = "added".to_string();
    let t2 = restock(&mut items, 5);
    println!("{} {}", t1, t2);
    report(&items);
}

fn main() {
    pyrt::init();
    py_main();
    pyrt::flush();
}
```
