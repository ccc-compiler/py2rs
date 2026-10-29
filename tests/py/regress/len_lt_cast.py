# regression: Bug: `len(a) < X` is emitted as `a.len() as i64 < X` (e.g. `while t.len() as i64 < n`),
def shorter(a: list[int], b: list[int]) -> bool:
    return len(a) < len(b)


def pad(t: list[int], n: int) -> list[int]:
    while len(t) < n:
        t.append(0)
    return t


print(shorter([1], [1, 2]), shorter([1, 2], [1]), pad([1], 3))
