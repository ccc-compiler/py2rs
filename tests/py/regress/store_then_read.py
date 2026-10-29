# a value stored in a list and only read afterwards: a copy is equivalent
def main() -> None:
    rows: list[list[int]] = []
    total = 0
    for i in range(3):
        row = [i, i + 1]
        rows.append(row)
        total += sum(row)
        rows.append([0])
    print(rows, total)
    names: list[str] = []
    best = "x"
    names.append(best)
    print(best.upper(), names)


main()
