def binary_search(xs: list[int], target: int) -> int:
    lo = 0
    hi = len(xs) - 1
    steps = 0
    while lo <= hi:
        steps += 1
        mid = (lo + hi) // 2
        if xs[mid] == target:
            print(f"  found {target} at {mid} in {steps} steps")
            return mid
        elif xs[mid] < target:
            lo = mid + 1
        else:
            hi = mid - 1
    print(f"  {target} not found after {steps} steps")
    return -1


def lower_bound(xs: list[int], target: int) -> int:
    lo = 0
    hi = len(xs)
    while lo < hi:
        mid = (lo + hi) // 2
        if xs[mid] < target:
            lo = mid + 1
        else:
            hi = mid
    return lo


def upper_bound(xs: list[int], target: int) -> int:
    lo = 0
    hi = len(xs)
    while lo < hi:
        mid = (lo + hi) // 2
        if xs[mid] <= target:
            lo = mid + 1
        else:
            hi = mid
    return lo


def isqrt(n: int) -> int:
    lo = 0
    hi = min(n, 3037000499)
    while lo < hi:
        mid = (lo + hi + 1) // 2
        if mid * mid <= n:
            lo = mid
        else:
            hi = mid - 1
    return lo


def sqrt_bisect(x: float) -> float:
    lo = 0.0
    hi = max(1.0, x)
    for _ in range(60):
        mid = (lo + hi) / 2
        if mid * mid < x:
            lo = mid
        else:
            hi = mid
    return lo


def main() -> None:
    xs = [1, 3, 3, 3, 5, 8, 13, 21, 34, 55, 89]
    for t in [3, 13, 89, 1, 4, 100, 0]:
        i = binary_search(xs, t)
        lb = lower_bound(xs, t)
        ub = upper_bound(xs, t)
        print(f"t={t}: idx={i} lower={lb} upper={ub} count={ub - lb}")
    for n in [0, 1, 15, 16, 17, 99, 1000000, 999999999999]:
        print(f"isqrt({n}) = {isqrt(n)}")
    for x in [2.0, 9.0, 0.25, 10.0]:
        print(f"sqrt({x}) ~ {sqrt_bisect(x):.6f}")


main()
