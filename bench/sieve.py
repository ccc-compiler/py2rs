def primes(n: int) -> list[int]:
    is_p = [True] * (n + 1)
    is_p[0] = False
    is_p[1] = False
    i = 2
    while i * i <= n:
        if is_p[i]:
            for j in range(i * i, n + 1, i):
                is_p[j] = False
        i += 1
    return [k for k in range(n + 1) if is_p[k]]


def main() -> None:
    ps = primes(5_000_000)
    print(len(ps), ps[-1], sum(ps) % 1_000_007)


if __name__ == "__main__":
    main()
