from itertools import permutations, combinations, product, chain, accumulate, pairwise
import itertools
from functools import reduce


def main() -> None:
    for p in permutations([1, 2, 3]):
        print(p)
    print(list(permutations("abc", 2)))
    print(list(combinations([1, 2, 3, 4], 2)))
    print(len(list(combinations(range(10), 3))))
    for a, b in product([1, 2], ["x", "y"]):
        print(a, b, end="; ")
    print()
    print(list(itertools.product([0, 1], repeat=3)))
    print(list(product([1], [2, 3], [4, 5])))
    print(list(chain([1, 2], [3], range(4, 6))))
    print(list(accumulate([1, 2, 3, 4, 5])))
    print(list(accumulate([0.5, 0.25])))
    print(list(pairwise([1, 4, 9, 16])))
    print(reduce(lambda a, b: a * b, [1, 2, 3, 4, 5]))
    print(reduce(lambda acc, s: acc + len(s), ["ab", "cde"], 0))
    print(reduce(lambda a, b: a + b, ["x", "y", "z"], ""))
    best = max(permutations([3, 1, 2]), key=lambda t: t[0] * 100 + t[1] * 10 + t[2])
    print(best)
    try:
        empty: list[int] = []
        print(reduce(lambda a, b: a + b, empty))
    except TypeError as e:
        print("error:", e)


if __name__ == "__main__":
    main()
