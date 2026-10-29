# A linked stack: ownership without annotations

The Python program builds a stack of nodes linked by `next`. Nothing in it says
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
  plain functions.

Source: `tests/py/cases/linked_list.py` (part of the differential test).

## Python

```python
from __future__ import annotations
from dataclasses import dataclass


@dataclass
class Node:
    value: int
    next: Node | None = None


class Stack:
    def __init__(self) -> None:
        self.head: Node | None = None
        self.size = 0

    def push(self, v: int) -> None:
        node = Node(v)
        node.next = self.head
        self.head = node
        self.size += 1

    def pop(self) -> int:
        if self.head is None:
            raise IndexError("pop from empty stack")
        node = self.head
        self.head = node.next
        self.size -= 1
        return node.value

    def total(self) -> int:
        s = 0
        cur = self.head
        while cur is not None:
            s += cur.value
            cur = cur.next
        return s


def main() -> None:
    st = Stack()
    for i in range(1, 6):
        st.push(i * i)
    print(st.total(), st.size)
    print(st.pop(), st.pop(), st.total())
    try:
        while True:
            st.pop()
    except IndexError as e:
        print("empty:", e)


if __name__ == "__main__":
    main()
```

## Rust (`py2rs emit`)

```rust
use pyrt::{println, OrRaise, PyErr};

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub value: i64,
    pub next: Option<Box<Node>>,
}

impl pyrt::Py for Node {
    fn fmt_repr(&self, out: &mut String) {
        out.push_str("Node(");
        out.push_str("value=");
        self.value.fmt_repr(out);
        out.push_str(", next=");
        self.next.fmt_repr(out);
        out.push(')');
    }
}

#[derive(Clone, Debug)]
pub struct Stack {
    pub head: Option<Box<Node>>,
    pub size: i64,
}

impl Stack {
    pub fn new() -> Self {
        let head = None;
        let size = 0;
        Self { head, size }
    }

    pub fn push(&mut self, v: i64) {
        let mut node = Node { value: v, next: None };
        node.next = self.head.take();
        self.head = Some(Box::new(node));
        self.size += 1;
    }

    pub fn pop(&mut self) -> Result<i64, PyErr> {
        let Some(head) = self.head.take() else {
            return Err(PyErr::new(pyrt::INDEX_ERROR, "pop from empty stack"));
        };
        let node = *head;
        self.head = node.next;
        self.size -= 1;
        Ok(node.value)
    }

    pub fn total(&self) -> i64 {
        let mut s: i64 = 0;
        let mut cur = &self.head;
        while let Some(node) = cur {
            s += node.value;
            cur = &node.next;
        }
        s
    }
}

fn py_main() {
    let mut st = Stack::new();
    for i in 1..6_i64 {
        st.push(i * i);
    }
    println!("{} {}", st.total(), st.size);
    let t1 = st.pop().or_raise();
    let t2 = st.pop().or_raise();
    println!("{} {} {}", t1, t2, st.total());
    let raised = 'body: {
        loop {
            pyrt::check!(st.pop(), 'body);
        }
    };
    match raised {
        e if e.is("IndexError") => {
            println!("empty: {}", pyrt::s(&e));
        }
        other => pyrt::uncaught(other),
    }
}

fn main() {
    pyrt::init();
    py_main();
    pyrt::flush();
}
```
