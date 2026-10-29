import math


def main() -> None:
    vals = [0.1, 0.2, 0.1 + 0.2, 1.0, -1.0, 100.0, 1e15, 1e16, 1e17, 1.23e-4, 1e-4, 1e-5,
            123456789012345678.0, 0.000123, 1.5, 2.5, 1 / 7, 22 / 7, 1e300 * 10, -1e-300,
            5e-324, 1.7976931348623157e308, 3.0e10, 12345.678, 0.3, 1 - 0.9, 9.95, 2.675]
    for v in vals:
        print(v, repr(v), str(v))
    print([0.5, 0.25, 1e-10], (1.0, 2.0))
    for a in range(-3, 4):
        for b in [1, 2, 3, 7, -4]:
            print(f"{a}/{b}={a / b}", end=" ")
        print()
    for x in [0.5, 1.5, 2.5, -0.5, -1.5, 0.125, 0.375, 2.675, 1.005, 123.456, -7.25]:
        print(x, round(x), round(x, 1), round(x, 2), round(x, 0), round(-x, 1))
    print(round(1234.5678, -2), round(1250.0, -2), round(-15.0, -1))
    print(math.floor(2.7), math.ceil(-2.7), math.trunc(-2.7), int(1e10), float(2 ** 53))
    print(math.sqrt(16), math.sqrt(2) ** 2, math.hypot(3, 4), math.fabs(-3))
    print(0.1 + 0.2 == 0.3, abs(math.sqrt(2) ** 2 - 2.0) < 1e-9)
    inf = float("inf")
    print(inf, -inf, inf > 1e308, math.isinf(inf), math.isnan(inf - inf))
    print(math.log(100, 10), math.log10(1000), math.log2(1024), math.exp(1), math.log(math.e))
    print(math.sin(0.0), math.cos(0.0), math.atan2(1, 1) * 4, math.pi / 180)
    print(7.0 // 2, -7.0 // 2, 7.0 % -2, -0.0 == 0.0, 3 * 0.1, 0.1 * 3, 1e22, 1e21, 1e-7)
    total = 0.0
    for i in range(1, 11):
        total += 1 / i
        print(i, total)
    print(sum([0.1] * 10), max(0.1, 0.10000000000000001), min(-0.0, 0.0))
    print(float("  3.5  "), float("-0"), float("1e5"), float(10 ** 15), 10 ** 15 / 1)


main()
