def payment(principal: float, annual_rate: float, months: int) -> float:
    if months <= 0:
        raise ValueError("months must be positive")
    r = annual_rate / 12
    if r == 0:
        return principal / months
    return principal * r / (1 - (1 + r) ** -months)


def schedule(principal: float, annual_rate: float, months: int) -> list[tuple[int, float, float, float]]:
    p = payment(principal, annual_rate, months)
    bal = principal
    rows: list[tuple[int, float, float, float]] = []
    for m in range(1, months + 1):
        interest = bal * annual_rate / 12
        princ = p - interest
        bal -= princ
        rows.append((m, interest, princ, max(bal, 0.0)))
    return rows


def main() -> None:
    p = payment(10000.0, 0.06, 12)
    print("payment", p, round(p, 2), f"{p:.2f}")
    for m, i, pr, b in schedule(10000.0, 0.06, 12):
        print(f"{m:>3} {i:>9.2f} {pr:>9.2f} {b:>11,.2f}")
    for rate in [0.0, 0.01, 0.035, 0.125]:
        for months in [12, 60, 360]:
            pay = payment(200000.0, rate, months)
            total = pay * months
            print(f"rate {rate:6.2%} months {months:>3}: {pay:>10.2f} total {total:>13,.2f} interest {total - 200000:>12,.2f}")
    try:
        payment(1.0, 0.1, 0)
    except ValueError as e:
        print("error:", e)
    bal = 1000.0
    for year in range(1, 6):
        bal *= 1.05
        print(year, bal, round(bal, 2), round(bal, 4), int(bal))


main()
