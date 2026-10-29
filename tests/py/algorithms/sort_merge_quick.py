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


def merge_sort(xs: list[int], depth: int) -> list[int]:
    if len(xs) <= 1:
        return xs.copy()
    mid = len(xs) // 2
    left = merge_sort(xs[:mid], depth + 1)
    right = merge_sort(xs[mid:], depth + 1)
    if depth < 2:
        print(f"{'  ' * depth}merge {left} + {right}")
    return merge(left, right)


def quick_sort(a: list[int], calls: list[int]) -> list[int]:
    calls[0] += 1
    if len(a) <= 1:
        return a.copy()
    pivot = a[len(a) // 2]
    less = [x for x in a if x < pivot]
    equal = [x for x in a if x == pivot]
    more = [x for x in a if x > pivot]
    return quick_sort(less, calls) + equal + quick_sort(more, calls)


def main() -> None:
    data = [38, 27, 43, 3, 9, 82, 10, 3, 55, 1, 70, 21]
    print("merge sort:")
    m = merge_sort(data, 0)
    print("result:", m)
    calls = [0]
    q = quick_sort(data, calls)
    print("quick sort:", q, "calls:", calls[0])
    print("same:", m == q)
    words = ["pear", "apple", "fig", "banana", "kiwi", "cherry"]
    print(sorted(words))
    print(sorted(words, key=lambda w: (len(w), w)))
    print(sorted(words, key=lambda w: len(w), reverse=True))
    rev = data.copy()
    rev.sort(reverse=True)
    print("desc:", rev)
    for k in range(1, 6):
        part = data[:k * 2]
        print(k, merge_sort(part, 5), sorted(part))


main()
