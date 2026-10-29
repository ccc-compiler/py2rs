class RPNError(Exception):
    pass


class StackUnderflow(RPNError):
    pass


class UnknownOp(RPNError):
    pass


def pop2(stack: list[float]) -> tuple[float, float]:
    if 2 > len(stack):
        raise StackUnderflow(f"need 2 values, have {len(stack)}")
    b = stack.pop()
    a = stack.pop()
    return (a, b)


def apply_op(op: str, stack: list[float]) -> None:
    a, b = pop2(stack)
    if op == "+":
        stack.append(a + b)
    elif op == "-":
        stack.append(a - b)
    elif op == "*":
        stack.append(a * b)
    elif op == "/":
        stack.append(a / b)
    elif op == "^":
        stack.append(a ** b)
    else:
        raise UnknownOp(f"unknown operator {op!r}")


def run(src: str) -> float:
    stack: list[float] = []
    for tok in src.split():
        try:
            stack.append(float(tok))
        except ValueError:
            apply_op(tok, stack)
    if len(stack) != 1:
        raise RPNError(f"stack has {len(stack)} values at end")
    return stack[0]


def main() -> None:
    progs = ["3 4 +", "5 1 2 + 4 * + 3 -", "2 3 ^ 2 ^", "1 2 3", "+", "4 0 /", "2 x *",
             "1.5 2.25 *", "10 3 /", "2 -0.5 ^", "1e3 7 -", "0.1 0.2 +"]
    for p in progs:
        try:
            print(f"{p:<22} = {run(p)}")
        except StackUnderflow as e:
            print(f"{p:<22} underflow: {e}")
        except UnknownOp as e:
            print(f"{p:<22} unknown: {e}")
        except RPNError as e:
            print(f"{p:<22} rpn error: {e}")
        except ZeroDivisionError as e:
            print(f"{p:<22} zero division: {e}")


main()
