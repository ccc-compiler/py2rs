def caesar(s: str, k: int) -> str:
    out: list[str] = []
    for ch in s:
        if "a" <= ch <= "z":
            out.append(chr((ord(ch) - 97 + k) % 26 + 97))
        else:
            out.append(ch)
    return "".join(out)


def count_vowels(s: str) -> int:
    n = 0
    for i in range(len(s)):
        if s[i] in "aeiou":
            n += 1
    return n


def main() -> None:
    base = "the quick brown fox jumps over the lazy dog "
    s = base * 20000
    t = caesar(s, 3)
    print(t[:44], count_vowels(t), len(t))
    print(t.upper().count("WKH"), len(t.split()))


if __name__ == "__main__":
    main()
