def total(*nums: int) -> int:
    s = 0
    for n in nums:
        s += n
    return s


def log(prefix: str, *parts: str, sep: str = " ") -> str:
    return prefix + ": " + sep.join(parts)


def main() -> None:
    print(total(), total(1), total(1, 2, 3))
    print(log("info"), log("warn", "disk", "full"), log("x", "a", "b", sep="-"))


if __name__ == "__main__":
    main()
