import math


def sentences(text: str) -> list[str]:
    out: list[str] = []
    cur = ""
    for c in text:
        cur += c
        if c in ".!?":
            s = cur.strip()
            if s:
                out.append(s)
            cur = ""
    if cur.strip():
        out.append(cur.strip())
    return out


def words_of(s: str) -> list[str]:
    ws: list[str] = []
    for raw in s.split():
        w = raw.strip(".,;:!?\"'()").lower()
        if w:
            ws.append(w)
    return ws


def syllables(word: str) -> int:
    vowels = "aeiouy"
    count = 0
    prev = False
    for c in word:
        v = c in vowels
        if v and not prev:
            count += 1
        prev = v
    if word.endswith("e") and count > 1:
        count -= 1
    return max(count, 1)


def main() -> None:
    text = ("It was the best of times, it was the worst of times. It was the age of wisdom! "
            "Was it the age of foolishness? Nobody knows; perhaps everybody does. "
            "The end")
    ss = sentences(text)
    for i, s in enumerate(ss, 1):
        print(f"{i:2}. ({len(words_of(s)):2} words) {s}")
    ws = words_of(text)
    n = len(ws)
    lens = [len(w) for w in ws]
    mean = sum(lens) / n
    var = sum((x - mean) ** 2 for x in lens) / n
    print(f"words={n} unique={len(set(ws))} mean_len={mean:.3f} std={math.sqrt(var):.3f}")
    srt = sorted(lens)
    median = (srt[n // 2 - 1] + srt[n // 2]) / 2 if n % 2 == 0 else float(srt[n // 2])
    print("median", median, "min", srt[0], "max", srt[-1])
    syl = sum(syllables(w) for w in ws)
    fk = 206.835 - 1.015 * (n / len(ss)) - 84.6 * (syl / n)
    print(f"syllables={syl} flesch={fk:.2f} raw={fk}")
    freq: dict[str, int] = {}
    for w in ws:
        freq[w] = freq.get(w, 0) + 1
    for w, c in sorted(freq.items(), key=lambda kv: (-kv[1], kv[0]))[:6]:
        print(f"{w:>8} {c:>3} {'#' * (c + 0)}")
    hist: dict[int, int] = {}
    for x in lens:
        hist[x] = hist.get(x, 0) + 1
    for k in sorted(hist):
        print(f"len {k:2}: {hist[k] / n:6.1%}")


main()
