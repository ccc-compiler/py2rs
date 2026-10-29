class LRUCache:
    def __init__(self, capacity: int) -> None:
        self.capacity = capacity
        self.data: dict[str, int] = {}
        self.order: list[str] = []
        self.hits = 0
        self.misses = 0
        self.evicted: list[str] = []

    def get(self, key: str) -> int | None:
        if key not in self.data:
            self.misses += 1
            return None
        self.hits += 1
        self.order.remove(key)
        self.order.append(key)
        return self.data[key]

    def put(self, key: str, value: int) -> None:
        if key in self.data:
            self.order.remove(key)
        elif len(self.data) == self.capacity:
            old = self.order.pop(0)
            del self.data[old]
            self.evicted.append(old)
        self.data[key] = value
        self.order.append(key)

    def __str__(self) -> str:
        items = ", ".join(f"{k}={self.data[k]}" for k in self.order)
        return f"LRU[{items}]"


class SlowSource:
    def __init__(self) -> None:
        self.calls = 0

    def fetch(self, key: str) -> int:
        self.calls += 1
        return sum(ord(c) for c in key) * 7 % 1000


class CachedSource:
    def __init__(self, capacity: int) -> None:
        self.cache = LRUCache(capacity)
        self.source = SlowSource()

    def lookup(self, key: str) -> int:
        v = self.cache.get(key)
        if v is not None:
            return v
        fresh = self.source.fetch(key)
        self.cache.put(key, fresh)
        return fresh


def main() -> None:
    c = LRUCache(3)
    c.put("a", 1)
    c.put("b", 2)
    c.put("c", 3)
    print(c)
    print(c.get("a"), c.get("z"))
    c.put("d", 4)
    print(c, "evicted", c.evicted)
    c.put("c", 30)
    c.put("e", 5)
    print(c, "evicted", c.evicted)
    print("hits", c.hits, "misses", c.misses)
    cs = CachedSource(4)
    keys = "alpha beta gamma alpha delta beta epsilon alpha zeta beta gamma alpha".split()
    for k in keys:
        print(f"{k:<8}{cs.lookup(k):>5}  {cs.cache}")
    print("source calls", cs.source.calls, "of", len(keys))
    ratio = cs.cache.hits / (cs.cache.hits + cs.cache.misses)
    print(f"hit ratio {ratio:.2%}")


main()
