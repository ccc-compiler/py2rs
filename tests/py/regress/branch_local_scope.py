# regression: Rust compile error E0425: cannot find value `j` in this scope.
def f(src: str) -> int:
    i = 0
    if src[0] == "a":
        j = i + 3
        i = j
    elif src[0] == "b":
        j = i + 2
        i = j
    return i


print(f("abc"))
print(f("bcd"))
