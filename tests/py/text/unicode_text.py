def reverse_words(s: str) -> str:
    return " ".join(w[::-1] for w in s.split())


def count_kinds(s: str) -> tuple[int, int, int, int]:
    letters = 0
    digits = 0
    spaces = 0
    other = 0
    for c in s:
        if c.isalpha():
            letters += 1
        elif c.isdigit():
            digits += 1
        elif c.isspace():
            spaces += 1
        else:
            other += 1
    return (letters, digits, spaces, other)


def main() -> None:
    words = ["héllo", "naïve", "日本語", "Ελληνικά", "Straße", "über", "ça", "мир", "😀ok", "café"]
    for w in words:
        print(f"{w:<10}|{len(w):>3}|{w[0]}|{w[-1]}|{w[1:3]}|{w[::-1]}|{w.upper()}|{w.lower()}|")
    s = "Grüße aus Zürich: 42 Äpfel, 3 Öfen — 日本 2024!"
    print(len(s), s.find("Zürich"), s.find("日本"), s.count("ü"), s.index("4"))
    print(count_kinds(s))
    print(s.split(), s.split(":"))
    print(reverse_words(s))
    print(s.replace("ü", "ue").replace("ß", "ss"))
    print(s.title())
    print(s.swapcase())
    print([c for c in "añb€c" if c.isalpha()], [ord(c) for c in "añ€"], chr(8364) + chr(241))
    print("ÄÖÜ".lower(), "äöü".upper(), "ǆ".upper(), "ﬁ".upper())
    print(f"[{'日本':^8}] [{'é':>4}] [{'ñandú':<7}]")
    t = "αβγδε"
    for i in range(len(t)):
        print(i, t[i], t[:i], t[i:], end=" | ")
    print()
    print(sorted(["éclair", "apple", "Zebra", "über", "ápple"]))
    print("é" < "f", "Z" < "a", max("hello wörld"), min("hello wörld".replace(" ", "")))
    print("İ".lower() == "i̇", len("İ".lower()), "straße".upper(), len("straße".upper()))
    print("日本語テキスト".startswith("日本"), "テキスト" in "日本語テキスト", "x".join("日本語"))
    print("  ünïcode\t\n".strip() + "|", "--ü--".strip("-"), "ab€".center(7))


main()
