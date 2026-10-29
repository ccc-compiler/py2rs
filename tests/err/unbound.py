# error: might be used before it is assigned
def f(x: int) -> int:
    if x > 0:
        y = 1
    return y
