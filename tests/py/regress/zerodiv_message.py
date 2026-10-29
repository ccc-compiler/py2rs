# regression: Wrong output: CPython 3.14 says "division by zero" for int // 0 and int % 0
def f(a: int, b: int) -> None:
    try:
        print(a // b)
    except ZeroDivisionError as e:
        print(e)
    try:
        print(a % b)
    except ZeroDivisionError as e:
        print(e)


f(5, 0)
