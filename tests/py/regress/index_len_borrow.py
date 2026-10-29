# regression: Bug: assignment to a local list element whose index is not a range loop
def main() -> None:
    xs = [0, 0, 0]
    for k in [2, 0, 2]:
        xs[k] += 1
    print(xs)


main()
