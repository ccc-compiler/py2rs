def bubble_sort(xs: list[int]) -> list[int]:
    a = xs.copy()
    n = len(a)
    swaps = 0
    for i in range(n):
        swapped = False
        for j in range(n - 1 - i):
            if a[j] > a[j + 1]:
                a[j], a[j + 1] = a[j + 1], a[j]
                swaps += 1
                swapped = True
        if not swapped:
            break
    print(f"  bubble swaps: {swaps}")
    return a


def insertion_sort(xs: list[int]) -> list[int]:
    a: list[int] = []
    shifts = 0
    for key in xs:
        pos = len(a)
        while pos > 0 and a[pos - 1] > key:
            pos -= 1
            shifts += 1
        a.insert(pos, key)
    print(f"  insertion shifts: {shifts}")
    return a


def selection_sort(xs: list[int]) -> list[int]:
    a = xs.copy()
    for i in range(len(a)):
        m = i
        for j in range(i + 1, len(a)):
            if a[j] < a[m]:
                m = j
        if m != i:
            v = a.pop(m)
            a.insert(i, v)
    return a


def is_sorted(xs: list[int]) -> bool:
    return all(xs[i] <= xs[i + 1] for i in range(len(xs) - 1))


def lcg(seed: int, n: int) -> list[int]:
    out: list[int] = []
    x = seed
    for _ in range(n):
        x = (x * 1103515245 + 12345) % 2147483648
        out.append(x % 100)
    return out


def main() -> None:
    cases = [[5, 2, 9, 1, 5, 6], [], [1], [3, 2, 1], lcg(7, 20), lcg(42, 15)]
    for c in cases:
        print(f"input: {c}")
        b = bubble_sort(c)
        ins = insertion_sort(c)
        s = selection_sort(c)
        print(f"  sorted: {b}")
        print(f"  agree: {b == ins == s}, ok: {is_sorted(b)}, builtin: {b == sorted(c)}")
    print("original untouched:", cases[0])


main()
