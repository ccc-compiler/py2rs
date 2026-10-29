# regression: Rust compile error E0277: can't compare `String` with `&String` (generated `d == s`
def up(s: str) -> str:
    return s.upper().lower()


def main() -> None:
    samples = ["ab", "cd"]
    for s in samples:
        d = up(s)
        print(d == s)


main()
