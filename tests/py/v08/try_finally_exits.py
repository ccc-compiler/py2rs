def find(xs: list[int], target: int) -> int:
    try:
        for i, x in enumerate(xs):
            if x == target:
                return i
        return -1
    finally:
        print("searched", len(xs))


def first_even(xs: list[int]) -> int:
    found = -1
    for x in xs:
        try:
            if x % 2 == 0:
                found = x
                break
            if x > 100:
                continue
            print("odd", x)
        finally:
            print("checked", x)
    return found


def parse_all(items: list[str]) -> list[int]:
    out: list[int] = []
    for s in items:
        try:
            out.append(int(s))
        except ValueError:
            print("skip", s)
            continue
        finally:
            print("done", s)
    return out


def nested(n: int) -> str:
    try:
        try:
            if n > 0:
                return "positive"
        finally:
            print("inner finally")
        return "non-positive"
    finally:
        print("outer finally")


def main() -> None:
    print(find([4, 5, 6], 5), find([1], 9))
    print(first_even([1, 101, 3, 8, 9]))
    print(parse_all(["1", "x", "3"]))
    print(nested(1), nested(0))


if __name__ == "__main__":
    main()
