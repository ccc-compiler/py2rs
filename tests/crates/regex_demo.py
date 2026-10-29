from rust.regex import Regex


def words(text: str) -> list[str]:
    r = Regex.new(r"[a-z]+")
    return r.find_all(text.lower())


def check(pattern: str) -> str:
    try:
        Regex.new(pattern)
        return "ok"
    except ValueError:
        return "invalid"


def main() -> None:
    date = Regex.new(r"(\d{4})-(\d{2})-(\d{2})")
    text = "released 2024-05-17, patched 2025-01-02"
    print(date.is_match(text), date.find(text), date.find_all(text))
    print(date.replace_all(text, "$3/$2/$1"))
    print(date.captures(text), date.captures("none"))
    print(Regex.new(r"\s*,\s*").split("a , b,c ,d"))
    print(words("Hello, big World!"), check("a+"), check("(unclosed"))


main()
