def fib(n: int) -> int:
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a


def is_prime(n: int) -> bool:
    if n < 2:
        return False
    i = 2
    while i * i <= n:
        if n % i == 0:
            return False
        i += 1
    return True


def main() -> None:
    print(fib(10), fib(50))
    primes = [p for p in range(50) if is_prime(p)]
    print(primes)
    print(len(primes), sum(primes), max(primes))
    total = 0
    for p in primes:
        total += p * p
    print("sum of squares:", total)
    print(7 // 2, -7 // 2, 7 % 3, -7 % 3, 7 / 2, 2 ** 10)
    print(1.5 * 2, 0.1 + 0.2, 1e20, 1 / 3)
    x = 10
    x -= 3
    print(f"x = {x}, half = {x / 2:.3f}, padded = [{x:>5}]")


if __name__ == "__main__":
    main()
