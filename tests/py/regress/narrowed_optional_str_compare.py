# regression: BUG (Rust compile error): a str | None narrowed by `if top is None: return` compared
class Stack:
    def __init__(self) -> None:
        self.items: list[str] = ["(", "["]

    def peek(self) -> str | None:
        if self.items:
            return self.items[-1]
        return None


def check(st: Stack, ch: str) -> bool:
    pairs = {")": "(", "]": "["}
    top = st.peek()
    if top is None:
        return False
    if top != pairs[ch]:
        return False
    return True


def main() -> None:
    print(check(Stack(), "]"), check(Stack(), ")"))


main()
