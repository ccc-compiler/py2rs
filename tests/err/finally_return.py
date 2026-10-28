# error: inside try ... finally
def f(s: str) -> int:
    try:
        return int(s)
    finally:
        print("done")
