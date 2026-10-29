def keywords() -> list[str]:
    return ["if", "else", "while", "return", "def"]


def classify(word: str) -> str:
    if word in keywords():
        return "KW"
    if word[0].isdigit():
        return "INT"
    return "ID"


def tokenize(src: str) -> list[tuple[str, str, int]]:
    out: list[tuple[str, str, int]] = []
    i = 0
    line = 1
    n = len(src)
    while i < n:
        c = src[i]
        if c == "\n":
            line += 1
            i += 1
        elif c.isspace():
            i += 1
        elif c == "#":
            while i < n and src[i] != "\n":
                i += 1
        elif c.isalnum() or c == "_":
            j = i
            while j < n and (src[j].isalnum() or src[j] == "_"):
                j += 1
            w = src[i:j]
            out.append((classify(w), w, line))
            i = j
        elif c == '"':
            k = i + 1
            while k < n and src[k] != '"':
                k += 1
            out.append(("STR", src[i + 1:k], line))
            i = k + 1
        else:
            two = src[i:i + 2]
            if two in ["==", "!=", "<=", ">=", "->"]:
                out.append(("OP", two, line))
                i += 2
            else:
                out.append(("OP", c, line))
                i += 1
    return out


def main() -> None:
    src = 'def f(x):\n    # comment here\n    if x >= 10:\n        return "big"\n    else:\n        return x_1 + 42\nwhile a != b: c = d -> e\n'
    toks = tokenize(src)
    for kind, text, line in toks:
        print(f"{line:3d} {kind:<4}|{text}|")
    counts: dict[str, int] = {}
    for t in toks:
        counts[t[0]] = counts.get(t[0], 0) + 1
    for k in sorted(counts):
        print(k, counts[k])
    ids = [t[1] for t in toks if t[0] == "ID"]
    print(ids)
    print(" ".join(t[1] for t in toks if t[0] == "OP"))
    print(max(toks, key=lambda t: len(t[1])))


main()
