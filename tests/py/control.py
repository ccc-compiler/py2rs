def find(xs: list[int], target: int) -> int:
    for i, x in enumerate(xs):
        if x == target:
            break
    else:
        return -1
    return 1


def search(xs: list[int], target: int) -> str:
    lo, hi = 0, len(xs) - 1
    while lo <= hi:
        mid = (lo + hi) // 2
        if xs[mid] == target:
            return f"found at {mid}"
        elif xs[mid] < target:
            lo = mid + 1
        else:
            hi = mid - 1
    return "not found"


def collatz(n: int) -> int:
    steps = 0
    while n != 1:
        n = n // 2 if n % 2 == 0 else 3 * n + 1
        steps += 1
    return steps


def classify(n: int) -> str:
    if n < 0:
        kind = "negative"
    elif n == 0:
        kind = "zero"
    else:
        kind = "positive"
    return kind


def main() -> None:
    xs = [1, 3, 5, 7, 9, 11]
    print(find(xs, 7), find(xs, 4))
    print(search(xs, 9), search(xs, 2))
    print([collatz(n) for n in range(1, 11)])
    print(classify(-3), classify(0), classify(8))
    for i in range(10, 0, -3):
        print(i, end=" ")
    print()
    for i in reversed(range(3)):
        print(i, end=" ")
    print()
    count = 0
    i = 0
    while True:
        i += 1
        if i % 3 == 0:
            continue
        if i > 10:
            break
        count += i
    print(count)
    n = 0
    while n < 3:
        n += 1
    else:
        print("loop done", n)
    print(1 < 2 < 3, 3 > 2 > 2, 0 <= n <= 3)


main()
