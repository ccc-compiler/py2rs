# A road graph: shared objects with @shared

Cities point at each other, so the same object is reachable from several places:
plain Rust ownership cannot express it. `@shared` (from `rust`) makes a class's objects
`Rc<RefCell<..>>`:

- copying a `City` copies the handle (`Rc::clone`), so `connect` puts the same objects
  into both road lists, as Python does;
- fields are read through getters that return copies (`c.visited()`), changed through
  setters (`c.set_visited(true)`) or `roads_mut()`; no borrow outlives an expression, so
  a `RefCell` borrow can never fail at run time;
- methods take `&self` even when they change the object.

Without `@shared` py2rs rejects the program: `connect` stores `self` in another
object's list (`self cannot be stored or returned`; the hint suggests `@shared`).

Source: `tests/py/cases/graph.py` (part of the differential test).

## Python

```python
from __future__ import annotations
from rust import shared


@shared
class City:
    def __init__(self, name: str) -> None:
        self.name = name
        self.roads: list[City] = []
        self.visited = False

    def connect(self, other: City) -> None:
        self.roads.append(other)
        other.roads.append(self)


def reachable(start: City) -> list[str]:
    order: list[str] = []
    stack = [start]
    while stack:
        c = stack.pop()
        if c.visited:
            continue
        c.visited = True
        order.append(c.name)
        for n in c.roads:
            if not n.visited:
                stack.append(n)
    return order


def main() -> None:
    cities = {n: City(n) for n in ["Taipei", "Hsinchu", "Taichung", "Tainan", "Hualien"]}
    cities["Taipei"].connect(cities["Hsinchu"])
    cities["Hsinchu"].connect(cities["Taichung"])
    cities["Taichung"].connect(cities["Tainan"])
    for name in reachable(cities["Taipei"]):
        print("visit", name)
    print([n for n, c in cities.items() if not c.visited])


if __name__ == "__main__":
    main()
```

## Rust (`py2rs emit`)

```rust
use pyrt::{IndexMap, println};

#[derive(Debug)]
pub struct CityData {
    pub name: String,
    pub roads: Vec<City>,
    pub visited: bool,
}

/// `@shared`: copies of a `City` are the same object, as in Python
#[derive(Clone, Debug)]
pub struct City(pub std::rc::Rc<std::cell::RefCell<CityData>>);

#[allow(dead_code)]
impl City {
    pub fn name(&self) -> String {
        self.0.borrow().name.clone()
    }
    pub fn name_mut(&self) -> std::cell::RefMut<'_, String> {
        std::cell::RefMut::map(self.0.borrow_mut(), |d| &mut d.name)
    }
    pub fn set_name(&self, v: String) {
        self.0.borrow_mut().name = v;
    }
    pub fn roads(&self) -> Vec<City> {
        self.0.borrow().roads.clone()
    }
    pub fn roads_mut(&self) -> std::cell::RefMut<'_, Vec<City>> {
        std::cell::RefMut::map(self.0.borrow_mut(), |d| &mut d.roads)
    }
    pub fn set_roads(&self, v: Vec<City>) {
        self.0.borrow_mut().roads = v;
    }
    pub fn visited(&self) -> bool {
        self.0.borrow().visited
    }
    pub fn visited_mut(&self) -> std::cell::RefMut<'_, bool> {
        std::cell::RefMut::map(self.0.borrow_mut(), |d| &mut d.visited)
    }
    pub fn set_visited(&self, v: bool) {
        self.0.borrow_mut().visited = v;
    }
    /// `a is b`
    pub fn is(&self, other: &Self) -> bool {
        std::rc::Rc::ptr_eq(&self.0, &other.0)
    }
}

impl City {
    pub fn new(name: String) -> Self {
        let roads = Vec::<City>::new();
        let visited = false;
        Self(std::rc::Rc::new(std::cell::RefCell::new(CityData { name, roads, visited })))
    }

    pub fn connect(&self, other: City) {
        self.roads_mut().push(other.clone());
        other.roads_mut().push(self.clone());
    }
}

fn reachable(start: City) -> Vec<String> {
    let mut order: Vec<String> = Vec::new();
    let mut stack = vec![start];
    while !stack.is_empty() {
        let c = stack.pop().expect("IndexError: pop from empty list");
        if c.visited() {
            continue;
        }
        c.set_visited(true);
        order.push(c.name());
        for n in c.roads() {
            if !n.visited() {
                stack.push(n.clone());
            }
        }
    }
    order
}

fn py_main() {
    let cities = vec!["Taipei".to_string(), "Hsinchu".to_string(), "Taichung".to_string(), "Tainan".to_string(), "Hualien".to_string()].into_iter().map(|n| (n.clone(), City::new(n.clone()))).collect::<IndexMap<String, City>>();
    cities["Taipei"].connect(cities["Hsinchu"].clone());
    cities["Hsinchu"].connect(cities["Taichung"].clone());
    cities["Taichung"].connect(cities["Tainan"].clone());
    for name in reachable(cities["Taipei"].clone()) {
        println!("visit {}", name);
    }
    println!("{}", pyrt::s(&cities.iter().filter(|&(_, c)| !c.visited()).map(|(n, _)| n.to_string()).collect::<Vec<String>>()));
}

fn main() {
    pyrt::init();
    py_main();
    pyrt::flush();
}
```
