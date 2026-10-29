# regression: Rust compile error E0381: used binding `result` is possibly-uninitialized.
def step(n: int) -> int:
    try:
        if n < 0:
            raise ValueError("neg")
        result = n * 10
    finally:
        print("leave")
    return result


try:
    print(step(3))
except ValueError:
    print("caught")
