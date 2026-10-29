def merge(a: list[int], b: list[int]) -> list[int]:
    out: list[int] = []
    i = 0
    j = 0
    while i < len(a) and j < len(b):
        if a[i] <= b[j]:
            out.append(a[i])
            i += 1
        else:
            out.append(b[j])
            j += 1
    out.extend(a[i:])
    out.extend(b[j:])
    return out


def merge_sort(xs: list[int]) -> list[int]:
    if len(xs) <= 1:
        return xs.copy()
    mid = len(xs) // 2
    return merge(merge_sort(xs[:mid]), merge_sort(xs[mid:]))


def top_k(scores: dict[str, int], k: int) -> list[tuple[str, int]]:
    items = [(name, s) for name, s in scores.items()]
    items.sort(key=lambda t: (-t[1], t[0]))
    return items[:k]


def main() -> None:
    shards = [
        "alice:120 bob:95 carol:130",
        "dave:88 erin:130 frank:70 grace:101",
        "heidi:99 ivan:130 judy:55",
        "bob:40 dave:20",
    ]
    totals: dict[str, int] = {}
    sorted_shards: list[list[int]] = []
    for sh in shards:
        vals: list[int] = []
        for pair in sh.split():
            name = pair.split(":")[0]
            v = int(pair.split(":")[1])
            totals[name] = totals.get(name, 0) + v
            vals.append(v)
        sorted_shards.append(merge_sort(vals))
        print(f"shard {len(sorted_shards)}: {sorted_shards[-1]}")
    merged: list[int] = []
    for s in sorted_shards:
        merged = merge(merged, s)
    print("merged:", merged)
    print("median:", merged[len(merged) // 2], "p90:", merged[int(len(merged) * 0.9)])
    for rank, (name, s) in enumerate(top_k(totals, 5), 1):
        print(f"{rank}. {name:<6} {s:>4}")
    ties: dict[int, list[str]] = {}
    for name in totals:
        ties.setdefault(totals[name], []).append(name)
    for s in sorted(ties, reverse=True):
        if len(ties[s]) > 1:
            print("tie at", s, ":", sorted(ties[s]))
    check = [5, 3, 9, 1, 5, 6, 2, 8, 7, 0, 4]
    print(merge_sort(check) == sorted(check), merge_sort(check))


main()
