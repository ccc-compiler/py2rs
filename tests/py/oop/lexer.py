from dataclasses import dataclass


@dataclass(frozen=True)
class Token:
    kind: str
    text: str
    line: int
    col: int


class LexError(Exception):
    pass


class Lexer:
    def __init__(self, src: str) -> None:
        self.src = src
        self.pos = 0
        self.line = 1
        self.col = 1
        self.keywords = {"let", "if", "else", "while", "print"}

    def peek(self) -> str:
        if self.pos < len(self.src):
            return self.src[self.pos]
        return ""

    def advance(self) -> str:
        ch = self.src[self.pos]
        self.pos += 1
        if ch == "\n":
            self.line += 1
            self.col = 1
        else:
            self.col += 1
        return ch

    def next_token(self) -> Token | None:
        while self.peek() != "" and self.peek() in " \t\n":
            self.advance()
        if self.peek() == "#":
            while self.peek() != "" and self.peek() != "\n":
                self.advance()
            return self.next_token()
        ch = self.peek()
        if ch == "":
            return None
        line = self.line
        col = self.col
        if ch.isdigit():
            text = ""
            while self.peek() != "" and self.peek().isdigit():
                text += self.advance()
            return Token("num", text, line, col)
        if ch.isalpha() or ch == "_":
            text = ""
            while self.peek() != "" and (self.peek().isalnum() or self.peek() == "_"):
                text += self.advance()
            return Token("kw" if text in self.keywords else "ident", text, line, col)
        if ch == '"':
            self.advance()
            text = ""
            while self.peek() != '"':
                if self.peek() == "" or self.peek() == "\n":
                    raise LexError(f"unterminated string at {line}:{col}")
                text += self.advance()
            self.advance()
            return Token("str", text, line, col)
        two = self.src[self.pos:self.pos + 2]
        if two in ["==", "!=", "<=", ">="]:
            self.advance()
            self.advance()
            return Token("op", two, line, col)
        if ch in "+-*/=<>(){};":
            self.advance()
            return Token("op", ch, line, col)
        raise LexError(f"unexpected {ch!r} at {line}:{col}")

    def tokens(self) -> list[Token]:
        out: list[Token] = []
        while True:
            t = self.next_token()
            if t is None:
                return out
            out.append(t)


def main() -> None:
    program = 'let x = 10;\n# a comment\nwhile (x >= 1) {\n  print "x is" x;\n  x = x - 3;\n}\nif (x != 0) { print "odd_end"; }\n'
    toks = Lexer(program).tokens()
    for t in toks:
        print(f"{t.line:2}:{t.col:<3}{t.kind:<6}{t.text}")
    counts: dict[str, int] = {}
    for t in toks:
        counts[t.kind] = counts.get(t.kind, 0) + 1
    print(counts)
    print(toks[0], toks[0] == Token("kw", "let", 1, 1))
    for bad in ['let s = "oops\n;', "x = 3 @ 4", "ok_123 + 7"]:
        try:
            found = Lexer(bad).tokens()
            texts: list[str] = []
            for t in found:
                texts.append(t.text)
            print(texts)
        except LexError as err:
            print("lex error:", err)


main()
