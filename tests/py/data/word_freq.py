TEXT = """It was the best of times, it was the worst of times, it was the age of
wisdom, it was the age of foolishness, it was the epoch of belief, it was the
epoch of incredulity, it was the season of Light, it was the season of Darkness,
it was the spring of hope, it was the winter of despair."""


def clean(word: str) -> str:
    out = ""
    for ch in word.lower():
        if ch.isalpha():
            out += ch
    return out


def count_words(text: str) -> dict[str, int]:
    counts: dict[str, int] = {}
    for raw in text.split():
        w = clean(raw)
        if w:
            counts[w] = counts.get(w, 0) + 1
    return counts


def main() -> None:
    counts = count_words(TEXT)
    print("distinct words:", len(counts))
    print("total words:", sum(counts.values()))
    items = sorted(counts.items(), key=lambda kv: (-kv[1], kv[0]))
    width = max(len(w) for w in counts)
    for i in range(10):
        w = items[i][0]
        n = counts[w] + 0
        print(f"{w.rjust(width)} {n:3d} {'*' * n}")
    by_len: dict[int, list[str]] = {}
    for w in counts:
        by_len.setdefault(len(w), []).append(w)
    for k in sorted(by_len):
        print(k, sorted(by_len[k]))
    singles = [w for w, n in counts.items() if n == 1]
    print("hapax:", len(singles), sorted(singles)[:5])
    letters: dict[str, int] = {}
    for ch in TEXT.lower():
        if ch.isalpha():
            letters[ch] = letters.get(ch, 0) + 1
    top = sorted(letters.items(), key=lambda kv: kv[1], reverse=True)[:5]
    for ch, n in top:
        print(f"{ch}: {n:4} {n * 100 / sum(letters.values()):5.1f}%")


main()
