def signature(w: str) -> str:
    return "".join(sorted(w.lower()))


def letter_counts(w: str) -> dict[str, int]:
    counts: dict[str, int] = {}
    for c in w:
        if c.isalpha():
            c = c.lower()
            counts[c] = counts.get(c, 0) + 1
    return counts


def is_anagram(a: str, b: str) -> bool:
    return letter_counts(a) == letter_counts(b)


def group_anagrams(words: list[str]) -> list[list[str]]:
    groups: dict[str, list[str]] = {}
    for w in words:
        groups.setdefault(signature(w), []).append(w)
    out = [sorted(g) for g in groups.values() if len(g) > 1]
    out.sort(key=lambda g: (-len(g), g[0]))
    return out


def find_anagram_positions(text: str, pat: str) -> list[int]:
    k = len(pat)
    target = letter_counts(pat)
    out: list[int] = []
    window: dict[str, int] = {}
    for i in range(len(text)):
        c = text[i]
        window[c] = window.get(c, 0) + 1
        if i >= k:
            old = text[i - k]
            window[old] -= 1
            if window[old] == 0:
                del window[old]
        if i >= k - 1 and window == target:
            out.append(i - k + 1)
    return out


def main() -> None:
    words = ["listen", "silent", "enlist", "google", "gooegl", "cat", "act", "tac",
             "dog", "god", "odg", "bird", "inlets", "tinsel", "banana"]
    for g in group_anagrams(words):
        print(f"{len(g)}: {', '.join(g)}")
    tests = [("Dormitory", "Dirty room"), ("The eyes", "They see"), ("Hello", "World"), ("Astronomer", "Moon starer")]
    for a, b in tests:
        print(f"{a!r} ~ {b!r}: {is_anagram(a, b)}")
    print(find_anagram_positions("cbaebabacd", "abc"))
    print(find_anagram_positions("abab", "ab"))
    lc = letter_counts("Mississippi")
    for k in sorted(lc, key=lambda c: (-lc[c], c)):
        print(f"  {k}: {'*' * lc[k]}")


main()
