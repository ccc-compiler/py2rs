# regression: BUG: `x is None or x != y` does not narrow x in the right operand of `or`:
def peek(xs: list[str]) -> str | None:
    if xs:
        return xs[-1]
    return None


def main() -> None:
    top = peek(["a", "b"])
    if top is None or top != "b":
        print("different")
    else:
        print("same")


main()
