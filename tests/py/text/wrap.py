def wrap(text: str, width: int) -> list[str]:
    lines: list[str] = []
    cur = ""
    for w in text.split():
        if cur == "":
            cur = w
        elif len(cur) + 1 + len(w) <= width:
            cur = cur + " " + w
        else:
            lines.append(cur)
            cur = w
    if cur != "":
        lines.append(cur)
    return lines


def justify_line(words: list[str], width: int) -> str:
    if len(words) == 1:
        return words[0].ljust(width)
    total = sum(len(w) for w in words)
    gaps = len(words) - 1
    spaces = width - total
    base = spaces // gaps
    extra = spaces % gaps
    out = ""
    for i in range(gaps):
        out += words[i] + " " * (base + (1 if i < extra else 0))
    return out + words[-1]


def justify(text: str, width: int) -> list[str]:
    result: list[str] = []
    for line in wrap(text, width):
        result.append(line)
    out: list[str] = []
    for i in range(len(result)):
        ws = result[i].split()
        if i == len(result) - 1:
            out.append(" ".join(ws).ljust(width))
        else:
            out.append(justify_line(ws, width))
    return out


def main() -> None:
    text = ("This is an example of text justification. The quick brown fox jumps over "
            "the lazy dog, and supercalifragilisticexpialidocious words may overflow. "
            "Ünïcödé wörds count by characters: naïve café résumé.")
    for width in [16, 24, 40]:
        print(f"--- width {width} ---")
        for line in wrap(text, width):
            print("|" + line.ljust(width) + "|")
        print(f"=== justified {width} ===")
        for line in justify(text, width):
            print(f"|{line}|")
    for line in wrap(text, 30):
        print(f"[{line:^30}]")
        print(f"[{line:>30}]")
    print(len(wrap("", 10)), wrap("a b c", 1))


main()
