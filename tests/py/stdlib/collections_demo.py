from collections import defaultdict, Counter, deque


def word_counts(text: str) -> defaultdict[str, int]:
    counts: defaultdict[str, int] = defaultdict(int)
    for w in text.split():
        counts[w] += 1
    return counts


def group_by_len(words: list[str]) -> defaultdict[int, list[str]]:
    groups = defaultdict(list)
    for w in words:
        groups[len(w)].append(w)
    return groups


def bfs(adj: dict[int, list[int]], start: int) -> list[int]:
    seen = {start}
    order: list[int] = []
    q = deque([start])
    while q:
        u = q.popleft()
        order.append(u)
        for v in adj[u]:
            if v not in seen:
                seen.add(v)
                q.append(v)
    return order


def main() -> None:
    text = "the cat and the dog and the bird"
    wc = word_counts(text)
    print(wc)
    print(wc["the"], wc["fish"], len(wc))
    for w in wc:
        print(w, wc[w])
    g = group_by_len(text.split())
    print(g)
    print(sorted(g.keys()))

    c = Counter(text.split())
    print(c)
    print(c.most_common(2))
    print(c["the"], c["zebra"], c.total())
    c["zebra"] += 2
    c.update(["cat", "cat"])
    print(c.most_common())
    letters = Counter("mississippi")
    print(letters)
    print(sorted(letters.elements()))
    empty: Counter[str] = Counter()
    print(empty, len(empty))

    adj = {0: [1, 2], 1: [3], 2: [3, 4], 3: [5], 4: [5], 5: []}
    print(bfs(adj, 0))
    d: deque[int] = deque()
    d.append(1)
    d.append(2)
    d.appendleft(0)
    print(d, len(d), d[0], d[-1], 2 in d)
    d.extend([7, 8])
    print(d.pop(), d.popleft(), d)
    total = 0
    for x in d:
        total += x
    print(total)
    try:
        e: deque[str] = deque()
        e.pop()
    except IndexError as err:
        print("error:", err)


if __name__ == "__main__":
    main()
