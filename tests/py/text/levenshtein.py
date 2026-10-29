def distance(a: str, b: str) -> int:
    prev = list(range(len(b) + 1))
    for i in range(1, len(a) + 1):
        cur = [i] + [0] * len(b)
        for j in range(1, len(b) + 1):
            cost = 0 if a[i - 1] == b[j - 1] else 1
            cur[j] = min(prev[j] + 1, cur[j - 1] + 1, prev[j - 1] + cost)
        prev = cur
    return prev[len(b)]


def suggest(word: str, vocab: list[str], limit: int) -> list[str]:
    scored = [(distance(word, v), v) for v in vocab]
    scored.sort()
    return [v for d, v in scored[:limit] if d <= 3]


def similarity(a: str, b: str) -> float:
    m = max(len(a), len(b))
    if m == 0:
        return 1.0
    return 1.0 - distance(a, b) / m


def main() -> None:
    pairs = [("kitten", "sitting"), ("flaw", "lawn"), ("", "abc"), ("abc", ""), ("same", "same"),
             ("café", "cafe"), ("日本語", "日本"), ("intention", "execution"), ("a", "b")]
    for a, b in pairs:
        print(f"{a!r:>12} {b!r:<12} d={distance(a, b)} sim={similarity(a, b):.3f} {similarity(a, b)}")
    vocab = ["python", "rust", "translate", "transmit", "typhoon", "trust", "rusty", "pythons", "crust"]
    for w in ["pyhton", "rsut", "translte", "xyz", "trusts"]:
        print(w, "->", suggest(w, vocab, 3))


main()
