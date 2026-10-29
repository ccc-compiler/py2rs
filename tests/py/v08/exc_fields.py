class InsufficientFunds(Exception):
    def __init__(self, needed: int, balance: int) -> None:
        super().__init__(f"need {needed}, have {balance}")
        self.needed = needed
        self.balance = balance


class ParseError(ValueError):
    def __init__(self, line: int, text: str) -> None:
        super().__init__(f"line {line}: bad input {text!r}")
        self.line = line
        self.text = text


class Account:
    def __init__(self, balance: int) -> None:
        self.balance = balance

    def withdraw(self, n: int) -> None:
        if n > self.balance:
            raise InsufficientFunds(n, self.balance)
        self.balance -= n


def parse(lines: list[str]) -> list[int]:
    out = []
    for i, s in enumerate(lines):
        if not s.isdigit():
            raise ParseError(i + 1, s)
        out.append(int(s))
    return out


def main() -> None:
    a = Account(100)
    for n in [30, 50, 40]:
        try:
            a.withdraw(n)
            print("ok", a.balance)
        except InsufficientFunds as e:
            print("failed:", e, "short by", e.needed - e.balance)
    try:
        print(parse(["1", "2", "x3"]))
    except ParseError as e:
        print(e.line, e.text, str(e))
    try:
        parse(["7", "?"])
    except ValueError as e:
        print("as ValueError:", e)
    print(parse(["4", "5"]))
    a.withdraw(1000)


if __name__ == "__main__":
    main()
