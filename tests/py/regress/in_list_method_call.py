# regression: Rust compile error E0277: can't compare `&String` with `String`
class P:
    def __init__(self) -> None:
        self.s = "*"

    def peek(self) -> str:
        return self.s

    def check(self) -> bool:
        return self.peek() in ["*", "/"]


print(P().check())
