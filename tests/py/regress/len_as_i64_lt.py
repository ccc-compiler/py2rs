# regression: BUG (Rust parse error): `len(xs) < n` is emitted as `xs.len() as i64 < n`, which rustc
def main() -> None:
    xs = [1, 2, 3]
    cap = 5
    if len(xs) < cap:
        print("room")


main()
