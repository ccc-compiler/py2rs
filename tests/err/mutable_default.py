# error: default values must be constants
def f(xs: list[int] = []) -> int:
    return len(xs)
