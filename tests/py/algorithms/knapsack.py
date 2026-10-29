from dataclasses import dataclass


@dataclass
class Item:
    name: str
    weight: int
    value: int


def knapsack(items: list[Item], cap: int) -> tuple[int, list[str]]:
    n = len(items)
    dp = [[0] * (cap + 1) for _ in range(n + 1)]
    for i in range(1, n + 1):
        it = items[i - 1]
        for w in range(cap + 1):
            dp[i][w] = dp[i - 1][w]
            if it.weight <= w:
                cand = dp[i - 1][w - it.weight] + it.value
                if cand > dp[i][w]:
                    dp[i][w] = cand
    chosen: list[str] = []
    w = cap
    for i in range(n, 0, -1):
        if dp[i][w] != dp[i - 1][w]:
            chosen.append(items[i - 1].name)
            w -= items[i - 1].weight
    chosen.reverse()
    return dp[n][cap], chosen


def unbounded(items: list[Item], cap: int) -> list[int]:
    best = [0] * (cap + 1)
    for w in range(1, cap + 1):
        for it in items:
            if it.weight <= w:
                best[w] = max(best[w], best[w - it.weight] + it.value)
    return best


def fractional(items: list[Item], cap: int) -> float:
    order = sorted(items, key=lambda it: it.value / it.weight, reverse=True)
    left = float(cap)
    total = 0.0
    for it in order:
        if left <= 0:
            break
        take = min(float(it.weight), left)
        total += take * it.value / it.weight
        print(f"  take {take:.1f} of {it.name} (ratio {it.value / it.weight:.3f})")
        left -= take
    return total


def main() -> None:
    items = [
        Item("map", 9, 150), Item("compass", 13, 35), Item("water", 153, 200),
        Item("sandwich", 50, 160), Item("glucose", 15, 60), Item("tin", 68, 45),
        Item("banana", 27, 60), Item("apple", 39, 40), Item("cheese", 23, 30),
    ]
    for cap in [0, 50, 100, 200, 400]:
        v, names = knapsack(items, cap)
        print(f"cap {cap:>3}: value {v:>4} items {names}")
    small = [Item("a", 3, 5), Item("b", 4, 7), Item("c", 5, 9)]
    print("unbounded:", unbounded(small, 15))
    print("fractional:")
    f = fractional(items, 100)
    print(f"  total = {f:.2f}")


main()
