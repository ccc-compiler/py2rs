# regression: Rust compile error: elements of sorted(d.items()) keep reference types, so "*" * n gets &i64:
def main() -> None:
    counts = {"a": 2, "b": 3}
    items = sorted(counts.items(), key=lambda kv: kv[1])
    n = items[0][1]
    print("*" * n)


main()

#   totals = {"a": 1.0}
#   for d, amt in sorted(totals.items(), key=lambda kv: -kv[1]):
#       share = amt / sum(totals.values())
# error[E0308]: mismatched types: pyrt::div(amt, ...) expected `f64`, found `&f64`
