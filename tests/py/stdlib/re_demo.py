import re


def parse_dates(text: str) -> list[tuple[str, str, str]]:
    return re.findall(r"(\d{4})-(\d{2})-(\d{2})", text)


def main() -> None:
    text = "Release 2024-03-15, patch 2024-04-01; next: 2025-01-20."
    print(re.findall(r"\d+", text))
    print(parse_dates(text))
    print(re.findall(r"(\w+):", "key: value, other: thing"))
    print(re.sub(r"\s+", " ", "a   lot    of   space"))
    print(re.sub(r"(\w+)@(\w+)", r"\2 at \1", "alice@home bob@work"))
    print(re.sub(r"(?P<word>\b\w{4}\b)", r"<\g<word>>", "this is some text here", count=2))
    print(re.split(r"[,;]\s*", "a, b;c,   d"))
    m = re.search(r"(\w+)@(\w+)\.com", "mail: jo@example.com now")
    if m:
        print(m.group(), m.group(1), m.group(2), m.start(), m.end(), m.span())
        print(m.groups())
        print(m)
    if re.match(r"\d+", "abc123") is None:
        print("no match at start")
    print(re.fullmatch(r"[a-z]+\d*", "abc123") is not None, re.fullmatch(r"[a-z]+", "abc123") is not None)
    word = re.compile(r"[A-Z][a-z]+")
    print(word)
    print(word.findall("Alice and Bob met Carol"))
    for mm in word.finditer("Dan, Eve"):
        print(mm.group(), mm.start())
    date = re.compile(r"(?P<y>\d{4})/(?P<m>\d{2})")
    d = date.search("on 2023/07 and")
    if d is not None:
        print(d.group("y"), d.group("m"))
    print(re.findall(r"hello", "Hello HELLO hello", re.I))
    print(re.findall(r"^\w+", "one two\nthree four", flags=re.M))
    print(re.escape("a.b*c"))
    print(re.sub(r"\d", "#", "phone 555-1234", 3))
    print(len(re.findall(r"é\w*", "école été")))
    s = re.search(r"é(t)é", "un été")
    if s:
        print(s.span(1))


if __name__ == "__main__":
    main()
