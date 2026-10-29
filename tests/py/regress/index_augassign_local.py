# regression: Rust compile error: xs[i] += 1 with a non-literal, non-range index (also on a list parameter, and for xs[i] = xs[i] + 1):
def main() -> None:
    counts = [0] * 3
    for v in [0, 1, 1, 2]:
        counts[v] += 1
    print(counts)


main()

# def bump(xs: list[int], i: int) -> None:
#     xs[i] += 1
# 
# 
# def main() -> None:
#     counts = [0] * 3
#     for v in [0, 1, 1, 2]:
#         bump(counts, v)
#     print(counts)
# 
# 
# main()
