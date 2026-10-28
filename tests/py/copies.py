def subsets(xs: list[int]) -> list[list[int]]:
    out: list[list[int]] = []
    path: list[int] = []
    for mask in range(1 << len(xs)):
        path.clear()
        for j in range(len(xs)):
            if mask >> j & 1:
                path.append(xs[j])
        out.append(path.copy())
    return out


a = [1, 2]
b = a.copy()
b.append(3)
print(a, b)
print(subsets([1, 2, 3]))
