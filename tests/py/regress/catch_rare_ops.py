import math


def safe_round(x: float) -> int:
    try:
        return round(x)
    except ValueError:
        return -1
    except OverflowError:
        return -2


def safe_chr(i: int) -> str:
    try:
        return chr(i)
    except ValueError as e:
        return f"bad: {e}"


def nested(x: float, d: int) -> str:
    try:
        return str(math.floor(x / d))
    except ZeroDivisionError:
        return "div"
    except (ValueError, OverflowError) as e:
        return f"floor: {e}"


print(safe_round(2.5), safe_round(float("nan")), safe_round(float("inf")))
print(safe_chr(65), safe_chr(-1))
print(nested(7.0, 2), nested(1.0, 0), nested(float("inf"), 1))
