def min_coins(coins: list[int], amount: int) -> tuple[int, list[int]]:
    inf = amount + 1
    best = [inf] * (amount + 1)
    last = [-1] * (amount + 1)
    best[0] = 0
    for a in range(1, amount + 1):
        for c in coins:
            if c <= a and best[a - c] + 1 < best[a]:
                best[a] = best[a - c] + 1
                last[a] = c
    if best[amount] == inf:
        return -1, []
    used: list[int] = []
    a = amount
    while a > 0:
        used.append(last[a])
        a -= last[a]
    return best[amount], sorted(used, reverse=True)


def count_ways(coins: list[int], amount: int) -> int:
    ways = [0] * (amount + 1)
    ways[0] = 1
    for c in coins:
        for a in range(amount + 1):
            if a >= c:
                ways[a] += ways[a - c]
    return ways[amount]


def greedy(coins: list[int], amount: int) -> list[int]:
    out: list[int] = []
    for c in sorted(coins, reverse=True):
        while amount >= c:
            amount -= c
            out.append(c)
    if amount != 0:
        return []
    return out


def main() -> None:
    systems = [[1, 5, 10, 25], [1, 3, 4], [2, 5], [7]]
    for coins in systems:
        print(f"coins {coins}")
        for amt in [0, 6, 11, 30, 63]:
            n, used = min_coins(coins, amt)
            g = greedy(coins, amt)
            flag = "" if len(g) == n or n < 0 else "  <- greedy is worse"
            print(f"  {amt:>3}: min={n:>2} {used} greedy={g}{flag}")
        print(f"  ways to make 100: {count_ways(coins, 100)}")


main()
