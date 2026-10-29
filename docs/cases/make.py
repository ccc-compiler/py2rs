"""Writes docs/cases/*.md: commentary + the Python program + what py2rs makes of it.

    python3 docs/cases/make.py     (after cargo build --release)
"""
import os
import subprocess

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
CASES = {
    "linked_list": (
        "A linked stack: ownership without annotations",
        """The Python program builds a stack of nodes linked by `next`. Nothing in it says
who owns what; py2rs works it out:

- `Node | None` is `Option<Box<Node>>` (a recursive class is boxed).
- `node.next = self.head; self.head = node` moves the old head into the new node:
  `self.head.take()` (the field is assigned again right after, so nobody sees `None`).
- `pop` narrows `self.head` with `if ... is None: raise` (a let-else), then takes the
  node out of the field and moves its `next` back in: no copy of the list.
- `total` only reads: `cur` becomes a reference that walks the list
  (`let mut cur = &self.head; while let Some(node) = cur`).
- `pop` raises `IndexError` and `main` catches it, so `pop` returns `Result` and the
  loop in the `try` uses `check!` to leave it; `push` and `total` cannot fail and stay
  plain functions.""",
    ),
    "inventory": (
        "An inventory: dataclasses, the standard library, borrowing",
        """Typical data-processing code: a dataclass, a `defaultdict`, sorting with a key and
formatted output.

- `items` is read by `by_category` and `report` (`&[Item]`) and modified by `restock`
  (`&mut Vec<Item>`): the modes are inferred from the bodies.
- `groups[it.category].append(...)` on a `defaultdict(list)` inserts the default:
  `groups.at(key).push(..)`.
- `sorted(..., key=lambda i: (-i.price * i.qty, i.name))` sorts by a tuple key, computed
  once per item like CPython does.
- The f-string format specs (`:<8`, `:>6.2f`) print exactly as in Python.
- `print("added", restock(items, 5))`: Python evaluates the arguments first, then
  prints; the generated code keeps that order with temporaries.""",
    ),
    "graph": (
        "A road graph: shared objects with @shared",
        """Cities point at each other, so the same object is reachable from several places:
plain Rust ownership cannot express it. `@shared` (from `rust`) makes a class's objects
`Rc<RefCell<..>>`:

- copying a `City` copies the handle (`Rc::clone`), so `connect` puts the same objects
  into both road lists, as Python does;
- fields are read through getters that return copies (`c.visited()`), changed through
  setters (`c.set_visited(true)`) or `roads_mut()`; no borrow outlives an expression, so
  a `RefCell` borrow can never fail at run time;
- methods take `&self` even when they change the object.

Without `@shared` py2rs rejects the program: `connect` stores `self` in another
object's list (`self cannot be stored or returned`; the hint suggests `@shared`).""",
    ),
}


def main() -> None:
    py2rs = os.path.join(ROOT, "target", "release", "py2rs")
    for name, (title, text) in CASES.items():
        src = os.path.join(ROOT, "tests", "py", "cases", name + ".py")
        rust = subprocess.run([py2rs, "emit", src], check=True, capture_output=True, text=True).stdout
        py = open(src).read()
        out = f"# {title}\n\n{text}\n\nSource: `tests/py/cases/{name}.py` (part of the differential test).\n\n"
        out += f"## Python\n\n```python\n{py}```\n\n## Rust (`py2rs emit`)\n\n```rust\n{rust}```\n"
        with open(os.path.join(ROOT, "docs", "cases", name + ".md"), "w") as f:
            f.write(out)
        print("wrote", name)


if __name__ == "__main__":
    main()
