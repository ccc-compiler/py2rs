def permute(items: list[int], used: list[bool], path: list[int], out: list[list[int]]) -> None:
    if len(path) == len(items):
        out.append(path.copy())
        return
    for i in range(len(items)):
        if used[i]:
            continue
        if i > 0 and items[i] == items[i - 1] and not used[i - 1]:
            continue
        used[i] = True
        path.append(items[i])
        permute(items, used, path, out)
        path.pop()
        used[i] = False


def combinations(n: int, k: int, start: int, path: list[int], out: list[list[int]]) -> None:
    if len(path) == k:
        out.append(path.copy())
        return
    for i in range(start, n + 1):
        path.append(i)
        combinations(n, k, i + 1, path, out)
        path.pop()


def next_permutation(a: list[int]) -> list[int] | None:
    i = len(a) - 2
    while i >= 0 and a[i] >= a[i + 1]:
        i -= 1
    if i < 0:
        return None
    j = len(a) - 1
    while a[j] <= a[i]:
        j -= 1
    b = a[:i] + [a[j]] + a[i + 1:j] + [a[i]] + a[j + 1:]
    return b[:i + 1] + b[i + 1:][::-1]


def perm_strings(s: str) -> list[str]:
    if len(s) <= 1:
        return [s]
    out: list[str] = []
    for i in range(len(s)):
        for rest in perm_strings(s[:i] + s[i + 1:]):
            out.append(s[i] + rest)
    return out


def main() -> None:
    items = [1, 2, 3]
    out: list[list[int]] = []
    permute(items, [False] * len(items), [], out)
    print(f"{len(out)} permutations of {items}: {out}")
    dup = [1, 1, 2, 2]
    out2: list[list[int]] = []
    permute(dup, [False] * len(dup), [], out2)
    print(f"{len(out2)} distinct permutations of {dup}")
    for p in out2:
        print("  ", p)
    combs: list[list[int]] = []
    combinations(5, 3, 1, [], combs)
    print(f"C(5,3) = {len(combs)}: {combs}")
    cur: list[int] | None = [1, 2, 3, 4]
    count = 0
    while cur is not None:
        count += 1
        if count <= 5 or count > 22:
            print(f"#{count}: {cur}")
        cur = next_permutation(cur)
    print("total via next_permutation:", count)
    words = perm_strings("abc")
    print(words, sorted(set(perm_strings("aab"))))


main()
