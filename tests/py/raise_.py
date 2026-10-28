def check(n: int) -> int:
    if n < 0:
        raise ValueError(f"negative: {n}")
    return n * 2


print(check(4))
print(check(-1))
