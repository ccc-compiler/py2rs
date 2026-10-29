def slugify(title: str, maxlen: int = 30) -> str:
    repl = {"ä": "ae", "ö": "oe", "ü": "ue", "ß": "ss", "é": "e", "è": "e", "à": "a", "ç": "c", "ñ": "n"}
    out = ""
    dash = False
    for c in title.lower():
        if c in repl:
            out += repl[c]
            dash = False
        elif ("a" <= c and c <= "z") or ("0" <= c and c <= "9"):
            out += c
            dash = False
        elif not dash and out != "":
            out += "-"
            dash = True
    out = out.strip("-")
    if len(out) > maxlen:
        cut = out[:maxlen]
        k = cut.rfind("-")
        out = cut[:k] if k > 0 else cut
    return out


def camel_to_snake(name: str) -> str:
    out = ""
    for i, c in enumerate(name):
        if c.isupper():
            if i > 0 and (not name[i - 1].isupper() or (i + 1 < len(name) and name[i + 1].islower())):
                out += "_"
            out += c.lower()
        else:
            out += c
    return out


def snake_to_camel(name: str) -> str:
    parts = [p for p in name.split("_") if p]
    if not parts:
        return ""
    return parts[0].lower() + "".join(p.capitalize() for p in parts[1:])


def main() -> None:
    titles = ["Hello, World!", "  Über die Straße  ", "Crème brûlée à la française",
              "Rust & Python: 2 Languages, 1 Translator", "---", "año nuevo 2025",
              "A very long title that keeps going and going forever"]
    for t in titles:
        print(f"{t!r:<55} -> {slugify(t)!r}")
    print(slugify(titles[-1], 12), slugify(titles[-1], 200))
    for n in ["parseHTTPResponse", "simpleName", "XMLHttpRequest", "already_snake", "A", "getID"]:
        s = camel_to_snake(n)
        print(f"{n:<20} {s:<24} {snake_to_camel(s)}")
    print(snake_to_camel("__private__name__"), repr(snake_to_camel("___")))


main()
