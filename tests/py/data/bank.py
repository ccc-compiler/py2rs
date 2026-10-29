from dataclasses import dataclass


class InsufficientFunds(Exception):
    pass


@dataclass
class Account:
    number: str
    owner: str
    balance: int


class Bank:
    def __init__(self) -> None:
        self.accounts: dict[str, Account] = {}
        self.history: list[str] = []

    def open(self, number: str, owner: str, cents: int) -> None:
        self.accounts[number] = Account(number, owner, cents)
        self.history.append(f"open {number} {cents}")

    def deposit(self, number: str, cents: int) -> None:
        if cents <= 0:
            raise ValueError("deposit must be positive")
        self.accounts[number].balance += cents
        self.history.append(f"dep {number} {cents}")

    def withdraw(self, number: str, cents: int) -> None:
        have = self.accounts[number].balance
        if have < cents:
            raise InsufficientFunds(f"{number} has {have}, needs {cents}")
        self.accounts[number].balance -= cents
        self.history.append(f"wd {number} {cents}")

    def transfer(self, src: str, dst: str, cents: int) -> None:
        self.withdraw(src, cents)
        self.deposit(dst, cents)


def money(cents: int) -> str:
    sign = "-" if cents < 0 else ""
    c = abs(cents)
    return f"{sign}${c // 100:,}.{c % 100:02d}"


def main() -> None:
    bank = Bank()
    n = int(input())
    for _ in range(n):
        parts = input().split()
        op = parts[0]
        try:
            if op == "open":
                bank.open(parts[1], parts[2], int(parts[3]))
            elif op == "dep":
                bank.deposit(parts[1], int(parts[2]))
            elif op == "wd":
                bank.withdraw(parts[1], int(parts[2]))
            elif op == "xfer":
                bank.transfer(parts[1], parts[2], int(parts[3]))
            else:
                print("unknown op", op)
        except InsufficientFunds as e:
            print("declined:", e)
        except KeyError as e:
            print("no such account:", e)
        except ValueError as e:
            print("invalid:", e)
    print(f"{'Acct':<6}{'Owner':<10}{'Balance':>14}")
    for acc in sorted(bank.accounts.values(), key=lambda a: a.number):
        print(f"{acc.number:<6}{acc.owner:<10}{money(acc.balance):>14}")
    total = sum(a.balance for a in bank.accounts.values())
    print("total", money(total))
    print(len(bank.history), "operations logged")
    for h in bank.history[-3:]:
        print("  ", h)


main()
