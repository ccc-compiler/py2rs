class CalcError(Exception):
    pass


class ParseError(CalcError):
    pass


class EvalError(CalcError):
    pass


class Token:
    def __init__(self, kind: str, text: str) -> None:
        self.kind = kind
        self.text = text

    def __repr__(self) -> str:
        return f"Token({self.kind}, {self.text!r})"


def tokenize(src: str) -> list[Token]:
    toks: list[Token] = []
    i = 0
    while i < len(src):
        c = src[i]
        if c == " ":
            i += 1
        elif c.isdigit():
            j = i
            while j < len(src) and (src[j].isdigit() or src[j] == "."):
                j += 1
            toks.append(Token("num", src[i:j]))
            i = j
        elif c in "+-*/()%^":
            toks.append(Token("op", c))
            i += 1
        else:
            raise ParseError(f"unexpected character {c!r} at {i}")
    toks.append(Token("end", ""))
    return toks


class Parser:
    def __init__(self, toks: list[Token]) -> None:
        self.toks = toks.copy()
        self.pos = 0

    def peek(self) -> str:
        return self.toks[self.pos].text

    def advance(self) -> Token:
        t = self.toks[self.pos]
        self.pos += 1
        return t

    def expr(self) -> float:
        v = self.term()
        while self.peek() == "+" or self.peek() == "-":
            op = self.advance().text
            r = self.term()
            if op == "+":
                v = v + r
            else:
                v = v - r
        return v

    def term(self) -> float:
        v = self.power()
        while self.peek() == "*" or self.peek() == "/" or self.peek() == "%":
            op = self.advance().text
            r = self.power()
            if op == "*":
                v = v * r
            elif r == 0.0:
                raise EvalError("division by zero")
            elif op == "/":
                v = v / r
            else:
                v = v % r
        return v

    def power(self) -> float:
        b = self.unary()
        if self.peek() == "^":
            self.advance()
            e = self.power()
            return b ** e
        return b

    def unary(self) -> float:
        if self.peek() == "-":
            self.advance()
            return -self.unary()
        return self.atom()

    def atom(self) -> float:
        t = self.advance()
        if t.kind == "num":
            try:
                return float(t.text)
            except ValueError:
                raise ParseError(f"bad number {t.text!r}")
        if t.text == "(":
            v = self.expr()
            if self.advance().text != ")":
                raise ParseError("expected )")
            return v
        raise ParseError(f"unexpected token {t!r}")


def evaluate(src: str) -> float:
    p = Parser(tokenize(src))
    v = p.expr()
    if p.peek() != "":
        raise ParseError(f"trailing input at token {p.pos}")
    return v


def main() -> None:
    exprs = ["1 + 2 * 3", "(1 + 2) * 3", "2 ^ 3 ^ 2", "-4 + 10 / 4", "7 % 3 + 0.5",
             "1 / 0", "2 * (3 + ", "4 $ 4", "1.2.3 + 1", "3 4", "((2))", "-(-(-1))",
             "10 / 3", "0.1 + 0.2", "2 ^ 0.5", "100 % 7 * 2"]
    ok = 0
    for s in exprs:
        try:
            v = evaluate(s)
            print(f"{s:>14} = {v}")
            ok += 1
        except ParseError as e:
            print(f"{s:>14} : parse error: {e}")
        except EvalError as e:
            print(f"{s:>14} : eval error: {e}")
    print("ok", ok, "of", len(exprs))
    print(tokenize("(1+22)*3"))


main()
