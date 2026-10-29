# regression: Rust compile error E0308: mismatched types in pyrt::repeat_str("#", c)
def main() -> None:
    freq: dict[str, int] = {"a": 2, "b": 1}
    for w, c in sorted(freq.items(), key=lambda kv: -kv[1]):
        print(w, "#" * c)


main()
