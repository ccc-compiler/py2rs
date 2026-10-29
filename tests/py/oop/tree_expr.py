import math


class Expr:
    def __init__(self, kind: str, text: str, value: float) -> None:
        self.kind = kind
        self.text = text
        self.value = value
        self.left: Expr | None = None
        self.right: Expr | None = None

    @staticmethod
    def num(v: float) -> "Expr":
        return Expr("num", "", v)

    @staticmethod
    def var(name: str) -> "Expr":
        return Expr("var", name, 0.0)


    def eval(self, env: dict[str, float]) -> float:
        if self.kind == "num":
            return self.value
        if self.kind == "var":
            if self.text not in env:
                raise KeyError(f"unbound variable {self.text}")
            return env[self.text]
        assert self.left is not None
        a = self.left.eval(env)
        if self.kind == "call":
            if self.text == "sqrt":
                return math.sqrt(a)
            if self.text == "abs":
                return abs(a)
            return math.floor(a)
        assert self.right is not None
        b = self.right.eval(env)
        if self.text == "+":
            return a + b
        if self.text == "-":
            return a - b
        if self.text == "*":
            return a * b
        if b == 0.0:
            raise ZeroDivisionError("division by zero in " + self.show())
        return a / b

    def show(self) -> str:
        if self.kind == "num":
            return f"{self.value:g}"
        if self.kind == "var":
            return self.text
        assert self.left is not None
        if self.kind == "call":
            return f"{self.text}({self.left.show()})"
        assert self.right is not None
        return f"({self.left.show()} {self.text} {self.right.show()})"

    def size(self) -> int:
        n = 1
        if self.left is not None:
            n += self.left.size()
        if self.right is not None:
            n += self.right.size()
        return n


def parse(tokens: list[str], pos: int) -> tuple[Expr, int]:
    tok = tokens[pos]
    if tok == "(":
        left, p = parse(tokens, pos + 1)
        op = tokens[p]
        right, q = parse(tokens, p + 1)
        node = Expr("bin", op, 0.0)
        node.left = left
        node.right = right
        return node, q + 1
    if tok in ["sqrt", "abs", "floor"]:
        arg, p = parse(tokens, pos + 2)
        call = Expr("call", tok, 0.0)
        call.left = arg
        return call, p + 1
    if tok[0].isalpha():
        return Expr.var(tok), pos + 1
    return Expr.num(float(tok)), pos + 1


def tokenize(s: str) -> list[str]:
    out: list[str] = []
    cur = ""
    for ch in s:
        if ch in "()+-*/ ":
            if cur:
                out.append(cur)
                cur = ""
            if ch != " ":
                out.append(ch)
        else:
            cur += ch
    if cur:
        out.append(cur)
    return out


def main() -> None:
    env = {"x": 3.0, "y": 4.0, "z": 0.0}
    sources = [
        "(x + y)",
        "sqrt(((x * x) + (y * y)))",
        "((x - 10) * (y / 8))",
        "abs((x - 10))",
        "floor((y / 3))",
        "(x / z)",
        "(w + 1)",
        "((1.5 * x) - (y * (x + 2)))",
    ]
    for src in sources:
        expr, _ = parse(tokenize(src), 0)
        try:
            v = expr.eval(env)
            print(f"{expr.show():<35} size={expr.size():2} = {v:.4f}")
        except ZeroDivisionError as err:
            print(f"{expr.show():<35} error: {err}")
        except KeyError as err:
            print(f"{expr.show():<35} error: {err}")


main()
