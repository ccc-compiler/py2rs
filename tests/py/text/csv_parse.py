class CSVError(Exception):
    pass


def parse_line(line: str, lineno: int) -> list[str]:
    fields: list[str] = []
    cur = ""
    in_q = False
    i = 0
    while i < len(line):
        c = line[i]
        if in_q:
            if c == '"':
                if i + 1 < len(line) and line[i + 1] == '"':
                    cur += '"'
                    i += 1
                else:
                    in_q = False
            else:
                cur += c
        elif c == '"':
            if cur != "":
                raise CSVError(f"line {lineno}: quote inside field at column {i}")
            in_q = True
        elif c == ",":
            fields.append(cur)
            cur = ""
        else:
            cur += c
        i += 1
    if in_q:
        raise CSVError(f"line {lineno}: unterminated quote")
    fields.append(cur)
    return fields


def parse(text: str) -> list[list[str]]:
    rows: list[list[str]] = []
    for n, line in enumerate(text.splitlines(), 1):
        if line.strip() == "" or line.startswith("#"):
            continue
        rows.append(parse_line(line, n))
    return rows


def main() -> None:
    text = 'name,age,city,score\n"Smith, John",42,"New York",88.5\n# comment\nAnna,31,Zürich,92\n"Quote ""Q"" Man",,Paris,70.25\n\nBob,x,Oslo,n/a\n'
    rows = parse(text)
    header = rows[0]
    widths = [max(len(r[i]) for r in rows) for i in range(len(header))]
    for r in rows:
        print(" | ".join(r[i].ljust(widths[i]) for i in range(len(r))))
    total = 0.0
    ages: list[int] = []
    for r in rows[1:]:
        try:
            ages.append(int(r[1]))
        except ValueError:
            print(f"bad age {r[1]!r} for {r[0]}")
        try:
            total += float(r[3])
        except ValueError:
            print(f"bad score {r[3]!r} for {r[0]}")
    print("ages", ages, "score total", total, "avg", round(total / 3, 3))
    for bad in ['a,"b', 'x"y",z', '"ok",""', ',,']:
        try:
            print(repr(bad), "->", parse_line(bad, 99))
        except CSVError as e:
            print(repr(bad), "->", e)


main()
