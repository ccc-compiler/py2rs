from collections import deque


def main() -> None:
    n = 200_000
    adj: list[list[int]] = [[] for _ in range(n)]
    x = 1
    for i in range(1, n):
        x = (x * 1103515245 + 12345) % 2147483648
        p = x % i
        adj[p].append(i)
        adj[i].append(p)
    dist = [-1] * n
    dist[0] = 0
    q = deque([0])
    while q:
        u = q.popleft()
        for v in adj[u]:
            if dist[v] < 0:
                dist[v] = dist[u] + 1
                q.append(v)
    print(max(dist), sum(dist))


if __name__ == "__main__":
    main()
