def ones() -> list[str]:
    return ["zero", "one", "two", "three", "four", "five", "six", "seven", "eight", "nine",
        "ten", "eleven", "twelve", "thirteen", "fourteen", "fifteen", "sixteen",
        "seventeen", "eighteen", "nineteen"]
def tens() -> list[str]:
    return ["", "", "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety"]
def scales() -> list[tuple[int, str]]:
    return [(1000000000000, "trillion"), (1000000000, "billion"), (1000000, "million"), (1000, "thousand")]


def below_thousand(n: int) -> str:
    parts: list[str] = []
    if n >= 100:
        parts.append(ones()[n // 100] + " hundred")
        n %= 100
    if n >= 20:
        t = tens()[n // 10]
        if n % 10:
            t += "-" + ones()[n % 10]
        parts.append(t)
    elif n > 0:
        parts.append(ones()[n])
    return " ".join(parts)


def to_words(n: int) -> str:
    if n == 0:
        return "zero"
    if n < 0:
        return "minus " + to_words(-n)
    parts: list[str] = []
    for value, name in scales():
        if n >= value:
            parts.append(below_thousand(n // value) + " " + name)
            n %= value
    if n > 0:
        parts.append(below_thousand(n))
    return " ".join(parts)


def ordinal(n: int) -> str:
    m = n % 100
    if 10 <= m <= 20:
        suf = "th"
    else:
        suf = {1: "st", 2: "nd", 3: "rd"}.get(n % 10, "th")
    return f"{n}{suf}"


def main() -> None:
    for n in [0, 7, 13, 20, 21, 99, 100, 101, 115, 999, 1000, 1001, 12345, 1000000,
              -42, 7000000007, 999999999999999, 2 ** 40]:
        w = to_words(n)
        print(f"{n:>16,}: {w}")
    print([ordinal(i) for i in [1, 2, 3, 4, 11, 12, 13, 21, 22, 101, 111, 112, 123]])
    lens = [len(to_words(i).replace(" ", "").replace("-", "")) for i in range(1, 1001)]
    print("letters 1..1000:", sum(lens))
    print(to_words(77).title(), to_words(88).upper(), to_words(1234).capitalize())


main()
