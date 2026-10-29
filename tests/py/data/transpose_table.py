TABLE = """name,mon,tue,wed,thu,fri
ann,8,7,9,6,8
ben,5,6,5,7,4
cat,9,9,8,9,10
dov,3,4,6,5,7"""


def parse(text: str) -> list[list[str]]:
    return [line.split(",") for line in text.split("\n")]


def transpose(rows: list[list[str]]) -> list[list[str]]:
    out: list[list[str]] = []
    for c in range(len(rows[0])):
        out.append([rows[r][c] for r in range(len(rows))])
    return out


def render(rows: list[list[str]]) -> None:
    widths = [max(len(rows[r][c]) for r in range(len(rows))) for c in range(len(rows[0]))]
    sep = "+" + "+".join("-" * (w + 2) for w in widths) + "+"
    print(sep)
    for i, row in enumerate(rows):
        cells = [f" {row[c].ljust(widths[c])} " if c == 0 else f" {row[c].rjust(widths[c])} " for c in range(len(row))]
        print("|" + "|".join(cells) + "|")
        if i == 0:
            print(sep)
    print(sep)


def with_totals(rows: list[list[str]]) -> list[list[str]]:
    out: list[list[str]] = [rows[0].copy() + ["total"]]
    for row in rows[1:]:
        nums = [int(x) for x in row[1:]]
        out.append(row.copy() + [str(sum(nums))])
    footer = ["sum"]
    for c in range(1, len(out[0])):
        footer.append(str(sum(int(out[r][c]) for r in range(1, len(out)))))
    out.append(footer)
    return out


def main() -> None:
    rows = parse(TABLE)
    render(rows)
    t = transpose(rows)
    render(t)
    full = with_totals(rows)
    render(full)
    back = transpose(transpose(rows))
    print("round trip ok:", back == rows)
    nums = [[int(x) for x in r[1:]] for r in rows[1:]]
    flat = [x for r in nums for x in r]
    print("cells:", len(flat), "avg:", f"{sum(flat) / len(flat):.3f}")
    best_day = max(range(len(nums[0])), key=lambda c: sum(r[c] for r in nums))
    print("busiest day:", rows[0][best_day + 1])


main()
