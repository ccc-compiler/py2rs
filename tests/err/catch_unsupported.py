# error: is not supported yet
def f(x: float) -> int:
    try:
        return round(x)
    except ValueError:
        return 0
