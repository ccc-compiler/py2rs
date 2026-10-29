# regression: Wrong output: str.isdigit / isdecimal use Rust's char::is_numeric (Unicode N*), not
def main() -> None:
    for t in ["½", "²", "Ⅻ", "٣"]:
        print(t, t.isdigit(), t.isdecimal(), t.isnumeric())


main()
