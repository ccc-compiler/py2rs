import math


def main() -> None:
    print(abs(-5), abs(-2.5), min(3, 1, 2), max(1.5, 2.0), round(2.5), round(3.5), round(2.675, 2))
    print(divmod(17, 5), divmod(-17, 5), pow(3, 200, 1000))
    print(int("42"), int(" -7 "), int(3.99), int(-3.99), float("1.5"), float(3))
    print(str(12) + str(3.0), repr("a'b"), repr('q"'))
    print(math.sqrt(2), math.floor(-2.5), math.ceil(2.1), math.pi, math.gcd(12, 18))
    print(1e16, 1.5e-7, 123456789.0, 0.1, -0.0, 1 / 3, 2 / 3, 10 / 4)
    print(f"{3.14159:.2f} {42:05d} {7:>4} {'x':<3}| {1234567:,} {0.5:.1%} {255:x} {1e-5:g}")
    print(f"{math.e:10.4f}|{-1:+d}|{2.5:e}|{100:^9}|")
    print(7 // -2, 7 % -2, -7.5 // 2, -7.5 % 2, 2 ** 0.5, 10 ** 18)
    print(5 & 3, 5 | 3, 5 ^ 3, ~5, 1 << 10, -16 >> 2)
    total = 0.0
    for i in range(10):
        total += 0.1
    print(total, sum([0.1] * 10))
    print(ord("A"), chr(97), bool(0), bool(3))
    big = 2 ** 62
    print(big + (big - 1))


main()
