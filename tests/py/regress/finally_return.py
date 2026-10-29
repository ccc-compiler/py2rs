def f(s: str) -> int:
    try:
        return int(s)
    finally:
        print("done")


print(f("42"))
