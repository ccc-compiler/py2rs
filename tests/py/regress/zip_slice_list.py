# regression: Rust compile error: list(zip(slice, list)) yields (f64, &f64):
def main() -> None:
    xs = [1.0, 2.0, 3.0]
    ys = [4.0, 5.0]
    pairs = list(zip(xs[:2], ys))
    print(pairs)


main()
