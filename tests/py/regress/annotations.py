from rust import Own, Ref, Mut


def total(xs: Ref[list[int]]) -> int:
    return sum(xs)


def fill(xs: Mut[list[int]], n: int) -> None:
    for i in range(n):
        xs.append(i)


def keep(store: Mut[list[list[int]]], xs: Own[list[int]]) -> None:
    store.append(xs)


data: list[int] = []
fill(data, 4)
print(total(data))
kept: list[list[int]] = []
keep(kept, [1, 2])
print(kept)
