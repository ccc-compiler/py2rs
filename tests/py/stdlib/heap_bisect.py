import heapq
import bisect


def dijkstra(adj: dict[str, list[tuple[str, int]]], src: str) -> dict[str, int]:
    dist: dict[str, int] = {src: 0}
    pq = [(0, src)]
    while pq:
        d, u = heapq.heappop(pq)
        if d > dist.get(u, 10**9):
            continue
        for v, w in adj[u]:
            nd = d + w
            if nd < dist.get(v, 10**9):
                dist[v] = nd
                heapq.heappush(pq, (nd, v))
    return dist


def heapsort(xs: list[int]) -> list[int]:
    h = list(xs)
    heapq.heapify(h)
    out = []
    while h:
        out.append(heapq.heappop(h))
    return out


def main() -> None:
    adj = {"a": [("b", 4), ("c", 1)], "b": [("d", 1)], "c": [("b", 2), ("d", 5)], "d": []}
    print(dijkstra(adj, "a"))
    xs = [5, 3, 9, 1, 7, 3, 8]
    print(heapsort(xs))
    h: list[int] = []
    for x in xs:
        heapq.heappush(h, x)
    print(h)
    print(heapq.heappushpop(h, 0), heapq.heapreplace(h, 10), h)
    print(heapq.nlargest(3, xs), heapq.nsmallest(2, xs))
    words = ["pear", "apple", "fig"]
    heapq.heapify(words)
    print(words)

    a = [1, 3, 3, 3, 7, 9]
    print(bisect.bisect_left(a, 3), bisect.bisect_right(a, 3), bisect.bisect(a, 4))
    bisect.insort(a, 5)
    bisect.insort_left(a, 3)
    print(a)
    grades = [60, 70, 80, 90]
    for score in [55, 60, 75, 99]:
        print(score, "FDCBA"[bisect.bisect(grades, score)])
    try:
        heapq.heappop([])
    except IndexError as e:
        print("error:", e)


if __name__ == "__main__":
    main()
