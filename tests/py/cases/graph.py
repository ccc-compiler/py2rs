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
