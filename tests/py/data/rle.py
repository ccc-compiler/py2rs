def encode(s: str) -> list[tuple[str, int]]:
    out: list[tuple[str, int]] = []
    i = 0
    while i < len(s):
        j = i
        while j < len(s) and s[j] == s[i]:
            j += 1
        out.append((s[i], j - i))
        i = j
    return out


def to_text(runs: list[tuple[str, int]]) -> str:
    parts: list[str] = []
    for ch, n in runs:
        if n == 1:
            parts.append(ch)
        else:
            parts.append(f"{n}{ch}")
    return "".join(parts)


def decode(code: str) -> str:
    out = ""
    num = 0
    for ch in code:
        if ch.isdigit():
            num = num * 10 + int(ch)
        else:
            out += ch * (num if num > 0 else 1)
            num = 0
    return out


def main() -> None:
    samples = [
        "aaabbbcccd",
        "wwwwwwwwwwwwbwwwwwwwwwwwwbbbwwwwwwwwwwwwwwwwwwwwwwwwbwwwwwwwwwwwwww",
        "abc",
        "",
        "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
        "aabbaabb",
    ]
    for s in samples:
        runs = encode(s)
        code = to_text(runs)
        back = decode(code)
        ratio = len(code) / len(s) if s else 0.0
        print(f"{s[:20]!r:<24} -> {code!r:<22} ok={len(back) == len(s) and back in samples} ratio={ratio:.2f}")
        longest = max(runs, key=lambda r: r[1]) if runs else ("-", 0)
        print("   runs:", len(runs), "longest:", longest)
    image = ["....##....", "...####...", "..######..", "...####...", "....##...."]
    for row in image:
        print(row, "|", to_text(encode(row)))


main()
