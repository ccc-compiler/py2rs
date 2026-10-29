# regression: BUG (Rust compile error): any store into a list field with a variable index:
class R:
    def __init__(self) -> None:
        self.buf: list[int] = [0, 0]

    def put(self, i: int, x: int) -> None:
        self.buf[i] = x


def main() -> None:
    r = R()
    r.put(1, 5)
    print(r.buf)


main()
