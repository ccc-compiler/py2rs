TEXT = """Data processing programs read records, clean them up, group them by
some key and print a report. Most of the work is in the details: trimming
whitespace, handling missing fields, choosing column widths and making
numbers line up on the decimal point. A good report is boring to read
because nothing in it surprises you."""


def wrap(words: list[str], width: int) -> list[str]:
    lines: list[str] = []
    cur: list[str] = []
    length = 0
    for w in words:
        if len(cur) > 0 and length + 1 + len(w) > width:
            lines.append(" ".join(cur))
            cur = []
            length = 0
        if cur:
            length += 1
        cur.append(w)
        length += len(w)
    if cur:
        lines.append(" ".join(cur))
    return lines


def justify(line: str, width: int) -> str:
    words = line.split()
    if len(words) == 1:
        return line.ljust(width)
    gaps = len(words) - 1
    spaces = width - sum(len(w) for w in words)
    out = ""
    for i in range(gaps):
        n = spaces // gaps + (1 if i < spaces % gaps else 0)
        out += words[i] + " " * n
    return out + words[-1]


def main() -> None:
    words = TEXT.split()
    print(len(words), "words,", sum(len(w) for w in words), "letters")
    for width in [30, 45]:
        lines = wrap(words, width)
        print("+" + "-" * width + "+")
        for l in lines[:-1]:
            print("|" + justify(l, width) + "|")
        print("|" + lines[-1].ljust(width) + "|")
        print("+" + "-" * width + "+")
        print(f"width {width}: {len(lines)} lines, ragged {sum(width - len(l) for l in lines)}")
    for l in wrap(words, 38)[:4]:
        print(f"[{l:^38}]")
    for l in wrap(words, 38)[:3]:
        print(f"[{l:>38}]")
    longest = max(words, key=lambda w: len(w.strip(".,:")))
    print("longest word:", longest.strip(".,:"))


main()
