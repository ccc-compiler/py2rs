def word_count(text: str) -> dict[str, int]:
    counts: dict[str, int] = {}
    for w in text.lower().split():
        w = w.strip(".,!?")
        counts[w] = counts.get(w, 0) + 1
    return counts


def main() -> None:
    text = "The quick brown fox. The lazy dog! The end?"
    counts = word_count(text)
    for word, n in counts.items():
        print(word, n)
    words = sorted(counts.keys())
    print(", ".join(words))
    s = "hello"
    print(s.upper(), s[0], s[-1], s[1:3], s[::-1], len(s))
    print("ell" in s, s.startswith("he"), s.find("l"), s.replace("l", "L"))
    name = "py2rs"
    print(f"{name!r} has {len(name)} chars")
    line = "-" * 10
    print(line)
    parts = "a,b,,c".split(",")
    print(parts, len(parts))


main()
