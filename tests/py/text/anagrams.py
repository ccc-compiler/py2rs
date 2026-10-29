def key_of(w: str) -> str:
    return "".join(sorted(w.lower()))


def is_palindrome(s: str) -> bool:
    cleaned = [c.lower() for c in s if c.isalnum()]
    return cleaned == cleaned[::-1]


def longest_pal_sub(s: str) -> str:
    best = ""
    for center in range(len(s)):
        for odd in [0, 1]:
            lo = center
            hi = center + odd
            while lo >= 0 and hi < len(s) and s[lo] == s[hi]:
                lo -= 1
                hi += 1
            cand = s[lo + 1:hi]
            if len(cand) > len(best):
                best = cand
    return best


def main() -> None:
    words = ["listen", "silent", "enlist", "google", "gooegl", "cat", "act", "tac", "dog",
             "God", "inlets", "banana", "Tinsel"]
    groups: dict[str, list[str]] = {}
    for w in words:
        k = key_of(w)
        if k not in groups:
            groups[k] = []
        groups[k].append(w)
    for k, g in groups.items():
        print(f"{k:<8} {len(g)} {', '.join(sorted(g))}")
    multi = [g.copy() for g in groups.values() if len(g) > 1]
    print(len(multi), sorted(len(g) for g in multi))
    for s in ["A man, a plan, a canal: Panama", "racecar", "hello", "", "No 'x' in Nixon",
              "Was it a car or a cat I saw?", "ésé", "ab"]:
        print(f"{s!r:<34} {is_palindrome(s)}")
    for s in ["babad", "cbbd", "forgeeksskeegfor", "abacdfgdcaba", "x", "日本日"]:
        print(s, "->", longest_pal_sub(s))


main()
