# regression: Rust parse error: `<` is interpreted as a start of generic arguments for `i64`
def f(ds: str, xs: list[int]) -> None:
    if len(ds) < 2:
        print("short")
    if len(xs) < 2:
        print("short")


f("a", [1])
