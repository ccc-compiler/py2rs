from dataclasses import dataclass


@dataclass
class Txn:
    date: str
    account: str
    amount: int
    memo: str


def parse_amount(s: str) -> int:
    neg = s.startswith("-") or (s.startswith("(") and s.endswith(")"))
    s = s.strip("-()").replace(",", "")
    if "." in s:
        whole = s.split(".")[0]
        frac = (s.split(".")[1] + "00")[:2]
    else:
        whole = s
        frac = "00"
    v = int(whole) * 100 + int(frac)
    return -v if neg else v


def show(c: int) -> str:
    s = f"{abs(c) // 100:,}.{abs(c) % 100:02d}"
    return f"({s})" if c < 0 else f" {s} "


def main() -> None:
    raw = """2026-01-03|checking|1,250.00|paycheck
2026-01-04|checking|-45.5|groceries
2026-01-04|savings|500|transfer in
2026-01-04|checking|(500.00)|transfer out
2026-01-10|credit|-89.99|books
2026-01-15|checking|-1,100|rent
2026-01-17|credit|-12.3|coffee
2026-01-20|checking|1,250.00|paycheck
2026-02-01|credit|102.29|card payment
2026-02-01|checking|-102.29|card payment
2026-02-03|savings|1.07|interest"""
    txns: list[Txn] = []
    for line in raw.splitlines():
        p = line.split("|")
        txns.append(Txn(p[0], p[1], parse_amount(p[2]), p[3]))
    balances: dict[str, int] = {}
    print(f"{'date':<11}{'account':<10}{'amount':>12}{'balance':>12}  memo")
    for t in txns:
        balances[t.account] = balances.get(t.account, 0) + t.amount
        print(f"{t.date:<11}{t.account:<10}{show(t.amount):>12}{show(balances[t.account]):>12}  {t.memo}")
    print()
    for acc in sorted(balances):
        print(f"{acc:<10}{show(balances[acc]):>14}")
    net = sum(balances.values())
    print(f"{'net worth':<10}{show(net):>14}")
    months: dict[str, list[int]] = {}
    for t in txns:
        m = t.date[:7]
        if m not in months:
            months[m] = [0, 0]
        if t.amount >= 0:
            months[m][0] += t.amount
        else:
            months[m][1] += t.amount
    for m in months:
        inc = months[m][0]
        out = months[m][1]
        print(f"{m}: in {show(inc)} out {show(out)} net {show(inc + out)}")
    memos = sorted({t.memo for t in txns if t.amount < 0}, key=lambda s: (len(s), s))
    print(memos)


main()
