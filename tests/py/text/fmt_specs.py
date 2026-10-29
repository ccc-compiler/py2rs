def show(label: str, s: str) -> None:
    print(f"{label:<12}[{s}]")


def main() -> None:
    n = 1234567
    neg = -42
    x = 3.14159265
    show("int", f"{n}")
    show("width", f"{n:12}")
    show("left", f"{n:<12}")
    show("center", f"{n:^12}")
    show("fill", f"{n:*^12}")
    show("zero", f"{neg:08d}")
    show("plus", f"{n:+d} {neg:+d}")
    show("space", f"{n: d} {neg: d}")
    show("group", f"{n:,} {neg * 1000000:,}")
    show("underscore", f"{n:_}")
    show("hex", f"{n:x} {n:X} {n:#x} {255:#x}")
    show("bin", f"{10:b} {10:08b} {10:#b}")
    show("oct", f"{64:o} {64:#o}")
    show("float", f"{x:.3f} {x:10.2f} {x:<10.1f}| {x:+.4f}")
    show("exp", f"{x:e} {x:.2e} {123456.789:E}")
    show("general", f"{x:g} {1e20:g} {0.0001:g} {0.00001:g} {123456789.0:g}")
    show("percent", f"{0.256:%} {0.256:.1%} {1.5:.0%}")
    show("fgroup", f"{1234567.891:,.2f} {-0.5:,.3f}")
    show("zfloat", f"{-3.5:08.2f} {3.5:08.2f}")
    show("str", f"{'abc':>6}|{'abc':<6}|{'abc':^7}|{'abc':.2}")
    show("strfill", f"{'mid':-^11} {'r':#>4}")
    show("bool", f"{True} {str(False):>6}")
    show("repr", f"{'q'!r} {'it' + chr(39) + 's'!r}")
    show("braces", f"{{literal}} {n}")
    show("zero0", f"{0:05} {0.0:05.1f} {-0.0}")
    show("big", f"{2 ** 62:,} {-(2 ** 62):x}")
    for v in [0, 1, -1, 7, 100, -100, 65535]:
        print(f"{v:>7d}|{v:<7x}|{v:^9b}|{v:+07}|")
    for f in [0.0, 1.0, -1.5, 2.675, 1e-7, 1e16, 12345.6789]:
        print(f"{f:>12.3f}|{f:<12g}|{f:^14.2e}|")


main()
