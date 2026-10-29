def prefix_function(p: str) -> list[int]:
    pi = [0] * len(p)
    k = 0
    for i in range(1, len(p)):
        while k > 0 and p[i] != p[k]:
            k = pi[k - 1]
        if p[i] == p[k]:
            k += 1
        pi[i] = k
    return pi


def kmp_search(text: str, pat: str) -> list[int]:
    if not pat:
        return []
    pi = prefix_function(pat)
    hits: list[int] = []
    k = 0
    comparisons = 0
    for i in range(len(text)):
        while k > 0 and text[i] != pat[k]:
            k = pi[k - 1]
            comparisons += 1
        comparisons += 1
        if text[i] == pat[k]:
            k += 1
        if k == len(pat):
            hits.append(i - k + 1)
            k = pi[k - 1]
    print(f"  {comparisons} comparisons")
    return hits


def naive_search(text: str, pat: str) -> list[int]:
    return [i for i in range(len(text) - len(pat) + 1) if text[i:i + len(pat)] == pat]


def z_function(s: str) -> list[int]:
    n = len(s)
    z = [0] * n
    left = 0
    right = 0
    for i in range(1, n):
        if i < right:
            z[i] = min(right - i, z[i - left])
        while i + z[i] < n and s[z[i]] == s[i + z[i]]:
            z[i] += 1
        if i + z[i] > right:
            left = i
            right = i + z[i]
    return z


def main() -> None:
    for p in ["abcabd", "aabaaab", "aaaa", "abab"]:
        print(f"pi({p}) = {prefix_function(p)}  z = {z_function(p)}")
    text = "abababcabababcababab"
    for pat in ["abab", "ababc", "c", "xyz", "abababcabababcababab"]:
        hits = kmp_search(text, pat)
        print(f"{pat!r}: {hits} naive agrees: {hits == naive_search(text, pat)}")
        if hits:
            marks = "".join("^" if i in hits else " " for i in range(len(text)))
            print("   " + text)
            print("   " + marks.rstrip())
    s = "abcab"
    pi = prefix_function(s)
    period = len(s) - pi[-1]
    print(f"smallest period of {s!r}: {period}")


main()
