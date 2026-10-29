def rle_encode(s: str) -> str:
    if not s:
        return ""
    out: list[str] = []
    cur = s[0]
    count = 1
    for ch in s[1:]:
        if ch == cur:
            count += 1
        else:
            out.append(f"{count}{cur}")
            cur = ch
            count = 1
    out.append(f"{count}{cur}")
    return "".join(out)


def rle_decode(s: str) -> str:
    out = ""
    num = 0
    for ch in s:
        if ch.isdigit():
            num = num * 10 + int(ch)
        else:
            out += ch * num
            num = 0
    return out


def caesar(s: str, k: int) -> str:
    out: list[str] = []
    for ch in s:
        if "a" <= ch <= "z":
            out.append(chr((ord(ch) - ord("a") + k) % 26 + ord("a")))
        elif "A" <= ch <= "Z":
            out.append(chr((ord(ch) - ord("A") + k) % 26 + ord("A")))
        else:
            out.append(ch)
    return "".join(out)


def vigenere(s: str, key: str, sign: int) -> str:
    out: list[str] = []
    j = 0
    for ch in s:
        if ch.isalpha():
            shift = ord(key[j % len(key)]) - ord("a")
            base = ord("a") if ch.islower() else ord("A")
            out.append(chr((ord(ch) - base + sign * shift) % 26 + base))
            j += 1
        else:
            out.append(ch)
    return "".join(out)


def crack_caesar(cipher: str) -> int:
    common = "etaoinshr"
    best_k = 0
    best_score = -1
    for k in range(26):
        plain = caesar(cipher, -k).lower()
        score = sum(1 for c in plain if c in common)
        if score > best_score:
            best_score = score
            best_k = k
    return best_k


def main() -> None:
    for s in ["WWWWWWWWWWWWBWWWWWWWWWWWWBBBWWWWWWWWWWWWWWWWWWWWWWWWBWWWWWWWWWWWWWW", "abc", "aaabccddd", ""]:
        e = rle_encode(s)
        print(f"{e!r} ({len(s)} -> {len(e)}) roundtrip={rle_decode(e) == s}")
    msg = "The Quick Brown Fox Jumps Over The Lazy Dog!"
    for k in [1, 3, 13, 25]:
        c = caesar(msg, k)
        print(f"k={k:>2}: {c}  back: {caesar(c, -k) == msg}")
    secret = caesar("meet me near the station at seven in the evening", 11)
    k = crack_caesar(secret)
    print(f"cracked k={k}: {caesar(secret, -k)}")
    v = vigenere("Attack at dawn!", "lemon", 1)
    print("vigenere:", v, "->", vigenere(v, "lemon", -1))


main()
