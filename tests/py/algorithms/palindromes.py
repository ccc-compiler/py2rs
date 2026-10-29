def is_palindrome(s: str) -> bool:
    cleaned = "".join(c.lower() for c in s if c.isalnum())
    return cleaned == cleaned[::-1]


def longest_palindrome(s: str) -> str:
    best_lo = 0
    best_len = 0
    for center in range(2 * len(s) - 1):
        lo = center // 2
        hi = lo + center % 2
        while lo >= 0 and hi < len(s) and s[lo] == s[hi]:
            lo -= 1
            hi += 1
        length = hi - lo - 1
        if length > best_len:
            best_len = length
            best_lo = lo + 1
    return s[best_lo:best_lo + best_len]


def count_palindromic_substrings(s: str) -> int:
    n = len(s)
    pal: set[tuple[int, int]] = set()
    count = 0
    for length in range(1, n + 1):
        for i in range(n - length + 1):
            j = i + length - 1
            if s[i] == s[j] and (length <= 2 or (i + 1, j - 1) in pal):
                pal.add((i, j))
                count += 1
    return count


def min_cuts(s: str) -> int:
    n = len(s)
    cuts = list(range(n))
    for i in range(n):
        if s[:i + 1] == s[:i + 1][::-1]:
            cuts[i] = 0
            continue
        for j in range(i):
            part = s[j + 1:i + 1]
            if part == part[::-1] and cuts[j] + 1 < cuts[i]:
                cuts[i] = cuts[j] + 1
    return cuts[n - 1] if n > 0 else 0


def palindromic_numbers(lo: int, hi: int) -> list[int]:
    return [n for n in range(lo, hi) if str(n) == str(n)[::-1]]


def main() -> None:
    phrases = ["A man, a plan, a canal: Panama", "racecar", "hello", "No 'x' in Nixon", "", "ab"]
    for p in phrases:
        print(f"{p!r:40} -> {is_palindrome(p)}")
    for s in ["babad", "cbbd", "forgeeksskeegfor", "a", "abacdfgdcaba", "aaaabaaa"]:
        lp = longest_palindrome(s)
        print(f"{s}: longest={lp!r} substrings={count_palindromic_substrings(s)} cuts={min_cuts(s)}")
    print(palindromic_numbers(100, 200))
    print(palindromic_numbers(1, 30))


main()
