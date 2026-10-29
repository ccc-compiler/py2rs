def shift_char(c: str, k: int) -> str:
    if "a" <= c <= "z":
        return chr((ord(c) - ord("a") + k) % 26 + ord("a"))
    if "A" <= c <= "Z":
        return chr((ord(c) - ord("A") + k) % 26 + ord("A"))
    return c


def caesar(text: str, k: int) -> str:
    return "".join(shift_char(c, k) for c in text)


def vigenere(text: str, key: str, decrypt: bool = False) -> str:
    out: list[str] = []
    j = 0
    for c in text:
        if c.lower() >= "a" and c.lower() <= "z":
            k = ord(key[j % len(key)].lower()) - ord("a")
            if decrypt:
                k = -k
            out.append(shift_char(c, k))
            j += 1
        else:
            out.append(c)
    return "".join(out)


def letter_freq(text: str) -> dict[str, int]:
    f: dict[str, int] = {}
    for c in text.lower():
        if c.isalpha():
            f[c] = f.get(c, 0) + 1
    return f


def crack(cipher: str) -> int:
    f = letter_freq(cipher)
    best = max(f.items(), key=lambda kv: (kv[1], kv[0]))
    return (ord(best[0]) - ord("e")) % 26


def main() -> None:
    msg = "Meet me at the secret entrance, near the eastern tree! Café 2024."
    for k in [0, 1, 3, 13, 25, -1, 52, -27]:
        e = caesar(msg, k)
        print(f"{k:>4}: {e}")
        assert caesar(e, -k) == msg
    enc = caesar(msg, 7)
    k = crack(enc)
    print("cracked key", k, caesar(enc, -k)[:20])
    v = vigenere(msg, "Lemon")
    print(v)
    print(vigenere(v, "Lemon", decrypt=True))
    print(vigenere("ATTACKATDAWN", "LEMON"))
    freq = letter_freq(msg)
    top = sorted(freq.items(), key=lambda kv: (-kv[1], kv[0]))[:5]
    print(top)
    print(caesar("xyz XYZ abc", 3), caesar("Hello, World", 13) == "Uryyb, Jbeyq")


main()
