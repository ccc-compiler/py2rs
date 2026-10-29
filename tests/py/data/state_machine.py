def tokenize(src: str) -> list[str]:
    tokens: list[str] = []
    state = "start"
    cur = ""
    for ch in src + " ":
        if state == "start":
            if ch.isdigit():
                state = "num"
                cur = ch
            elif ch.isalpha() or ch == "_":
                state = "ident"
                cur = ch
            elif ch == '"':
                state = "str"
                cur = ""
            elif ch in "+-*/=(),":
                tokens.append("OP:" + ch)
            elif ch != " ":
                tokens.append("BAD:" + ch)
        elif state == "num":
            if ch.isdigit() or ch == ".":
                cur += ch
            else:
                tokens.append("NUM:" + cur)
                state = "start"
                if ch in "+-*/=(),":
                    tokens.append("OP:" + ch)
        elif state == "ident":
            if ch.isalnum() or ch == "_":
                cur += ch
            else:
                tokens.append("ID:" + cur)
                state = "start"
                if ch in "+-*/=(),":
                    tokens.append("OP:" + ch)
        elif state == "str":
            if ch == '"':
                tokens.append("STR:" + cur)
                state = "start"
            else:
                cur += ch
    if state == "str":
        tokens.append("UNTERMINATED:" + cur)
    return tokens


def csv_split(line: str) -> list[str]:
    fields: list[str] = []
    cur = ""
    quoted = False
    i = 0
    while i < len(line):
        ch = line[i]
        if quoted:
            if ch == '"':
                if i + 1 < len(line) and line[i + 1] == '"':
                    cur += '"'
                    i += 1
                else:
                    quoted = False
            else:
                cur += ch
        elif ch == '"':
            quoted = True
        elif ch == ",":
            fields.append(cur)
            cur = ""
        else:
            cur += ch
        i += 1
    fields.append(cur)
    return fields


def main() -> None:
    progs = ['x = 3.14 * (r + 2)', 'print("hi there", n_1)', 'a=b-7/c', 'oops $ "open']
    for p in progs:
        toks = tokenize(p)
        print(f"{p!r}: {len(toks)} tokens")
        print("   ", " ".join(toks))
    lines = ['a,b,c', '"x, y",2,"say ""hi"""', ',,', 'one', '"",x']
    for l in lines:
        f = csv_split(l)
        print(len(f), f)


main()
