class BankError(Exception):
    pass


class InsufficientFunds(BankError):
    pass


def parse_int(s: str) -> int:
    try:
        return int(s)
    except ValueError:
        return -1


def withdraw(balance: int, amount: int) -> int:
    if amount <= 0:
        raise ValueError(f"bad amount {amount}")
    if amount > balance:
        raise InsufficientFunds(f"need {amount - balance} more")
    return balance - amount


def safe_div(a: int, b: int) -> float:
    try:
        return a / b
    except ZeroDivisionError as e:
        print("error:", e)
        return 0.0


def lookup(d: dict[str, int], keys: list[str]) -> int:
    total = 0
    for k in keys:
        try:
            total += d[k]
        except KeyError as e:
            print("missing", e)
    return total


def first_even(xs: list[int]) -> int:
    for i in range(len(xs)):
        if xs[i] % 2 == 0:
            return xs[i]
    raise LookupError("no even number")


def nested(s: str) -> str:
    try:
        try:
            n = int(s)
        except ValueError:
            raise KeyError(s)
        return f"ok {100 // n}"
    except KeyError as e:
        return f"key error {e}"
    except ArithmeticError:
        return "arithmetic"


def cleanup(xs: list[int]) -> None:
    try:
        print("value", xs[3])
    except IndexError:
        print("too short")
    else:
        print("long enough")
    finally:
        print("cleanup", len(xs))


def main() -> None:
    print(parse_int("42"), parse_int("x"), parse_int(" 7 "))
    bal = 100
    for amt in [30, -5, 500, 50]:
        try:
            bal = withdraw(bal, amt)
            print("ok", bal)
        except InsufficientFunds as e:
            print("insufficient:", e)
        except BankError:
            print("bank error")
        except ValueError as e:
            print("value error:", e)
    print(safe_div(7, 2), safe_div(1, 0))
    print(lookup({"a": 1, "b": 2}, ["a", "x", "b"]))
    try:
        print(first_even([1, 3, 4]))
        print(first_even([1, 3]))
    except LookupError as e:
        print("lookup:", e)
    print(nested("4"), nested("zero"), nested("0"))
    cleanup([1, 2])
    cleanup([1, 2, 3, 4])
    stack = [1, 2]
    while True:
        try:
            print(stack.pop())
        except IndexError:
            print("empty")
            break
    try:
        raise RuntimeError("boom")
    except Exception as e:
        print("caught", e)
    print(withdraw(10, 20))


main()
