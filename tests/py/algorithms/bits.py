def popcount(x: int) -> int:
    c = 0
    while x:
        x &= x - 1
        c += 1
    return c


def gray(n: int) -> int:
    return n ^ (n >> 1)


def from_gray(g: int) -> int:
    n = 0
    while g:
        n ^= g
        g >>= 1
    return n


def reverse_bits(x: int, width: int) -> int:
    r = 0
    for _ in range(width):
        r = (r << 1) | (x & 1)
        x >>= 1
    return r


def subsets_of(mask: int) -> list[int]:
    out: list[int] = []
    sub = mask
    while True:
        out.append(sub)
        if sub == 0:
            break
        sub = (sub - 1) & mask
    return out


def single_number(xs: list[int]) -> int:
    acc = 0
    for x in xs:
        acc ^= x
    return acc


def main() -> None:
    print(" n  bin       pop  gray      back rev8")
    for n in [0, 1, 2, 3, 5, 7, 8, 13, 100, 255]:
        g = gray(n)
        print(f"{n:>3} {n:08b}  {popcount(n):>3}  {g:08b}  {from_gray(g):>4} {reverse_bits(n, 8):>4}")
    print("subsets of 0b1011:", [f"{s:04b}" for s in subsets_of(0b1011)])
    print("single number:", single_number([4, 1, 2, 1, 2, 7, 4]))
    print("powers of two up to 1000:", [x for x in range(1, 1001) if x & (x - 1) == 0])
    total = sum(popcount(i) for i in range(1 << 10))
    print("bits set in 0..1023:", total)
    x = 0b10110000
    print(f"lowest set bit of {x:b}: {x & -x:b}, trailing zeros {popcount((x & -x) - 1)}")
    print("shifts:", 1 << 40, -17 >> 2, -17 & 0xFF, ~5, 0xF0 | 0x0F, 0b1100 ^ 0b1010)


main()
