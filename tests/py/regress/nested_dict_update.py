# regression: Rust compile error: d[k1][k2] = d[k1].get(k2, 0) + 1 on a nested dict:
def main() -> None:
    cells: dict[str, dict[str, int]] = {"a": {}}
    for c in ["x", "y", "x"]:
        cells["a"][c] = cells["a"].get(c, 0) + 1
    print(cells)


main()
