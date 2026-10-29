class DecodeError(Exception):
    pass


def rle_encode(s: str) -> str:
    if s == "":
        return ""
    out: list[str] = []
    prev = s[0]
    n = 1
    for c in s[1:]:
        if c == prev:
            n += 1
        else:
            out.append(f"{n}{prev}")
            prev = c
            n = 1
    out.append(f"{n}{prev}")
    return "".join(out)


def rle_decode(s: str) -> str:
    out = ""
    num = ""
    for c in s:
        if c.isdigit():
            num += c
        else:
            if num == "":
                raise DecodeError(f"missing count before {c!r}")
            out += c * int(num)
            num = ""
    if num != "":
        raise DecodeError(f"dangling count {num}")
    return out


def main() -> None:
    samples = ["aaabccdddd", "", "abc", "zzzzzzzzzzzzz", "ééé日日x", "aabbaa", "W" * 12 + "B" + "W" * 3]
    for s in samples:
        e = rle_encode(s)
        d = rle_decode(e)
        ratio = len(e) / len(s) if s else 0.0
        print(f"{s!r:>20} -> {e!r:<14} ok={len(d) == len(s) and d.startswith(s)} ratio={ratio:.2f}")
    for bad in ["3a2", "a3", "10x2y", "", "0a1b"]:
        try:
            print(repr(bad), "->", repr(rle_decode(bad)))
        except DecodeError as e:
            print(repr(bad), "-> error:", e)


main()
