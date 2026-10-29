from collections import Counter


def make_text(n: int) -> str:
    words = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta", "iota", "kappa"]
    parts: list[str] = []
    x = 12345
    for _ in range(n):
        x = (x * 1103515245 + 12345) % 2147483648
        parts.append(words[x % 10] + str(x % 7))
    return " ".join(parts)


def main() -> None:
    text = make_text(1_000_000)
    counts: dict[str, int] = {}
    for w in text.split():
        counts[w] = counts.get(w, 0) + 1
    top = sorted(counts.items(), key=lambda kv: (-kv[1], kv[0]))[:5]
    print(top)
    c = Counter(w[0] for w in text.split())
    print(c.most_common(3))


if __name__ == "__main__":
    main()
