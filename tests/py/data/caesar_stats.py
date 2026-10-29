def shift(text: str, k: int) -> str:
    out: list[str] = []
    for ch in text:
        if "a" <= ch <= "z":
            out.append(chr((ord(ch) - ord("a") + k) % 26 + ord("a")))
        elif "A" <= ch <= "Z":
            out.append(chr((ord(ch) - ord("A") + k) % 26 + ord("A")))
        else:
            out.append(ch)
    return "".join(out)


def freqs(text: str) -> list[float]:
    counts = [text.lower().count(chr(ord("a") + i)) for i in range(26)]
    total = sum(counts)
    return [c / total if total else 0.0 for c in counts]


def score(text: str) -> float:
    english = [8.2, 1.5, 2.8, 4.3, 12.7, 2.2, 2.0, 6.1, 7.0, 0.15, 0.77, 4.0, 2.4,
               6.7, 7.5, 1.9, 0.095, 6.0, 6.3, 9.1, 2.8, 0.98, 2.4, 0.15, 2.0, 0.074]
    f = freqs(text)
    return sum((f[i] * 100 - english[i]) ** 2 / english[i] for i in range(26))


def crack(cipher: str) -> int:
    return min(range(26), key=lambda k: score(shift(cipher, -k)))


def main() -> None:
    plain = "Data processing is mostly about reading records, grouping them, and printing tidy reports."
    for k in [1, 3, 13, 25]:
        c = shift(plain, k)
        print(f"k={k:2}: {c[:40]}...")
        assert shift(c, -k) == plain
    secret = shift(plain, 17)
    guess = crack(secret)
    print("cracked key:", guess, "->", shift(secret, -guess)[:30])
    ranked = sorted(range(26), key=lambda k: score(shift(secret, -k)))
    for k in ranked[:5]:
        print(f"  key {k:2} chi2 {score(shift(secret, -k)):10.2f}")
    f = freqs(plain)
    letters = sorted(range(26), key=lambda i: -f[i])[:6]
    for i in letters:
        print(f"{chr(ord('a') + i)} {f[i] * 100:5.2f}% {'|' * int(f[i] * 200)}")


main()
