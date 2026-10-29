class LedgerError(Exception):
    pass


class Overdraft(LedgerError):
    pass


def apply(balance: int, tx: str) -> int:
    i = tx.find(":")
    kind = tx[:i]
    amount_s = tx[i + 1:]
    amount = int(amount_s)
    if kind == "dep":
        return balance + amount
    if kind == "wd":
        if amount > balance:
            raise Overdraft(f"withdraw {amount} with balance {balance}")
        return balance - amount
    raise LedgerError(f"unknown kind {kind}")


def process(txs: list[str]) -> int:
    b = 0
    for t in txs:
        try:
            b = apply(b, t)
            print(f"{t:<10} -> {b:>6}")
        except ValueError as e:
            print(f"{t:<10} skipped: {e}")
    return b


def main() -> None:
    print("final", process(["dep:100", "wd:30", "dep:x", "dep:5"]))
    try:
        process(["dep:1", "xfer:5"])
    except LedgerError as e:
        print("ledger error:", e)
    print("final", process(["dep:10", "wd:20", "dep:1000"]))
    print("not reached")


main()
