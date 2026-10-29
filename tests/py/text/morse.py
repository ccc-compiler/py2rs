class MorseError(Exception):
    pass


def table() -> dict[str, str]:
    letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
    codes = (".- -... -.-. -.. . ..-. --. .... .. .--- -.- .-.. -- -. --- .--. --.- .-. ... - ..- "
             "...- .-- -..- -.-- --.. ----- .---- ..--- ...-- ....- ..... -.... --... ---.. ----.").split()
    t: dict[str, str] = {}
    for i in range(len(letters)):
        t[letters[i]] = codes[i]
    return t


def encode(text: str, t: dict[str, str]) -> str:
    words: list[str] = []
    for w in text.upper().split():
        parts: list[str] = []
        for c in w:
            if c not in t:
                raise MorseError(f"cannot encode {c!r}")
            parts.append(t[c])
        words.append(" ".join(parts))
    return " / ".join(words)


def decode(code: str, t: dict[str, str]) -> str:
    rev: dict[str, str] = {}
    for k, v in t.items():
        rev[v] = k
    out: list[str] = []
    for w in code.split(" / "):
        letters = ""
        for sym in w.split():
            if sym not in rev:
                raise MorseError(f"unknown symbol {sym}")
            letters += rev[sym]
        out.append(letters)
    return " ".join(out)


def main() -> None:
    t = table()
    for msg in ["SOS", "Hello World", "py2rs 2024", "The quick brown fox", "naïve", "a-b"]:
        try:
            e = encode(msg, t)
            print(f"{msg:<20} {e}")
            print(f"{'':<20} {decode(e, t)}")
        except MorseError as err:
            print(f"{msg:<20} error: {err}")
    for code in ["... --- ...", ".... .. / - .... . .-. .", "...---..."]:
        try:
            print(code, "=>", decode(code, t))
        except MorseError as err:
            print(code, "=> error:", err)
    dots = sum(v.count(".") for v in t.values())
    dashes = sum(v.count("-") for v in t.values())
    print("dots", dots, "dashes", dashes)
    longest = sorted(t.items(), key=lambda kv: (-len(kv[1]), kv[0]))[:4]
    print(longest)


main()
