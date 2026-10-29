def counting_sort(xs: list[int], max_val: int) -> list[int]:
    counts: dict[int, int] = {}
    for x in xs:
        counts[x] = counts.get(x, 0) + 1
    out: list[int] = []
    for v in range(max_val + 1):
        for _ in range(counts.get(v, 0)):
            out.append(v)
    return out


def radix_sort(xs: list[int]) -> list[int]:
    a = xs.copy()
    if not a:
        return a
    biggest = max(a)
    exp = 1
    rnd = 0
    while biggest // exp > 0:
        nxt: list[int] = []
        for d in range(10):
            nxt.extend([x for x in a if (x // exp) % 10 == d])
        a = nxt
        rnd += 1
        print(f"  pass {rnd} (exp={exp}): {a}")
        exp *= 10
    return a


def bucket_histogram(xs: list[int], width: int) -> dict[int, int]:
    h: dict[int, int] = {}
    for x in xs:
        k = x // width * width
        h[k] = h.get(k, 0) + 1
    return h


def main() -> None:
    xs = [170, 45, 75, 90, 802, 24, 2, 66, 45, 1000, 7]
    print("radix:")
    r = radix_sort(xs)
    print("sorted:", r, r == sorted(xs))
    small = [3, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5, 8, 9, 7, 9]
    c = counting_sort(small, 9)
    print("counting:", c)
    h = bucket_histogram(xs, 100)
    for k in sorted(h):
        bar = "#" * h[k]
        print(f"{k:>5}-{k + 99:<5} {bar} ({h[k]})")
    print(radix_sort([]))


main()
