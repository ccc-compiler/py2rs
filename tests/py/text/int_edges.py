def show_divmod(a: int, b: int) -> None:
    print(f"{a:>5} {b:>4} | // {a // b:>4} % {a % b:>4} | divmod {divmod(a, b)}")


def popcount(n: int) -> int:
    c = 0
    while n:
        c += n & 1
        n >>= 1
    return c


def main() -> None:
    for a in [7, -7, 0, 13, -13, 100]:
        for b in [2, -2, 3, -5]:
            show_divmod(a, b)
    print(2 ** 10, 2 ** 62, (-2) ** 3, (-2) ** 4, 3 ** 0, 0 ** 0, 2.0 ** -1, 10.0 ** -2)
    print(pow(2, 10, 1000), pow(7, 3, 5), pow(-3, 3, 7), pow(3, 200, 13), abs(-9223372036854775807))
    print(5 & 3, -5 & 3, 5 | -3, 5 ^ -1, ~0, ~-1, ~255)
    print(1 << 62, -1 << 3, 1024 >> 3, -1024 >> 3, -1 >> 10, 7 >> 1)
    for n in [0, 1, 7, 8, 255, 256, 1023, 12345, 2 ** 40 - 1]:
        print(n, popcount(n), f"{n:b}".count("1"))
    big = 9223372036854775807
    small = -9223372036854775807 - 1
    print(big, small, big // 2, small // 3, small % 7, big % -10)
    print(int(-7 / 2), -7 // 2, int(7 / -2), round(-3.5), round(4.5), int("-12") * 3)
    x = 0
    for i in range(1, 30):
        x = (x * 31 + i) % 1000000007
    print("hash", x)
    print(max(-3, 2, -10), min([5, -1, 3]), sum(range(-5, 6)), abs(-0), -(-5))
    print(10 // 3 * 3 + 10 % 3, -10 // 3 * 3 + -10 % 3, 17 % 1, -17 % 1)
    print(int(True) + int(True), int(False) - 1, 3 > 2 > 1, 1 < 3 < 2)
    print(divmod(-1, 7), divmod(1, -7), divmod(0, -3))
    try:
        print(5 // 0)
    except ZeroDivisionError:
        print("ZeroDivisionError for //")
    try:
        print(5 % 0)
    except ZeroDivisionError:
        print("ZeroDivisionError for %")
    try:
        print(5 / 0)
    except ZeroDivisionError as e:
        print("ZeroDivisionError:", e)


main()
