# regression: BUG (Rust compile error): a user variable named `e` read inside an `except ... as err`
class Thing:
    def show(self) -> str:
        return "thing"


def risky(n: int) -> int:
    if n == 0:
        raise ValueError("zero")
    return 10 // n


def main() -> None:
    e = Thing()
    for n in [2, 0]:
        try:
            print(risky(n))
        except ValueError as err:
            print(e.show(), "failed:", err)


main()
