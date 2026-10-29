class InsufficientFunds(Exception):
    pass


class Account:
    def __init__(self, owner: str, number: int, balance: float) -> None:
        self.owner = owner
        self.number = number
        self.balance = balance
        self.log: list[str] = []

    def deposit(self, amount: float) -> None:
        if amount <= 0:
            raise ValueError(f"bad deposit {amount}")
        self.balance += amount
        self.log.append(f"+{amount:.2f}")

    def limit(self) -> float:
        return 0.0

    def withdraw(self, amount: float) -> None:
        if self.balance - amount < -self.limit():
            raise InsufficientFunds(f"account {self.number}: cannot withdraw {amount:.2f}")
        self.balance -= amount
        self.log.append(f"-{amount:.2f}")

    def month_end(self) -> None:
        pass

    def kind(self) -> str:
        return "account"

    def __str__(self) -> str:
        return f"[{self.number}] {self.owner:<6} {self.kind():<9} {self.balance:>10.2f}"


class Savings(Account):
    def __init__(self, owner: str, number: int, balance: float, rate: float) -> None:
        super().__init__(owner, number, balance)
        self.rate = rate
        self.withdrawals = 0

    def withdraw(self, amount: float) -> None:
        if self.withdrawals >= 3:
            raise InsufficientFunds(f"account {self.number}: withdrawal limit reached")
        super().withdraw(amount)
        self.withdrawals += 1

    def month_end(self) -> None:
        interest = round(self.balance * self.rate / 12.0, 2)
        self.balance += interest
        self.log.append(f"i{interest:.2f}")
        self.withdrawals = 0

    def kind(self) -> str:
        return "savings"


class Checking(Account):
    def __init__(self, owner: str, number: int, balance: float, overdraft: float) -> None:
        super().__init__(owner, number, balance)
        self.overdraft = overdraft

    def limit(self) -> float:
        return self.overdraft

    def month_end(self) -> None:
        if self.balance < 0:
            fee = 15.0
            self.balance -= fee
            self.log.append(f"fee{fee:.0f}")

    def kind(self) -> str:
        return "checking"


class Bank:
    def __init__(self, name: str) -> None:
        self.name = name
        self.accounts: list[Account] = []
        self.next_number = 1001

    def open_savings(self, owner: str, amount: float) -> int:
        n = self.next_number
        self.next_number += 1
        self.accounts.append(Savings(owner, n, amount, 0.03))
        return n

    def open_checking(self, owner: str, amount: float) -> int:
        n = self.next_number
        self.next_number += 1
        self.accounts.append(Checking(owner, n, amount, 200.0))
        return n

    def index_of(self, number: int) -> int:
        for i, a in enumerate(self.accounts):
            if a.number == number:
                return i
        raise KeyError(f"no account {number}")

    def transfer(self, src: int, dst: int, amount: float) -> bool:
        self.index_of(src)
        self.index_of(dst)
        try:
            for a in self.accounts:
                if a.number == src:
                    a.withdraw(amount)
        except InsufficientFunds as e:
            print("  refused:", e)
            return False
        for a in self.accounts:
            if a.number == dst:
                a.deposit(amount)
        return True

    def month_end(self) -> None:
        for a in self.accounts:
            a.month_end()

    def report(self) -> None:
        print(f"== {self.name} ==")
        for a in self.accounts:
            print(a)
        print(f"total {sum(a.balance for a in self.accounts):.2f}")


def main() -> None:
    bank = Bank("First Py Bank")
    a = bank.open_savings("ann", 1000.0)
    b = bank.open_checking("bob", 50.0)
    c = bank.open_checking("cy", 300.0)
    bank.report()
    moves = [(a, b, 100.0), (b, c, 300.0), (b, c, 100.0), (a, c, 50.0), (a, b, 10.0), (a, b, 10.0)]
    for src, dst, amt in moves:
        ok = bank.transfer(src, dst, amt)
        print(f"transfer {src}->{dst} {amt:.2f}: {'ok' if ok else 'failed'}")
    bank.month_end()
    bank.report()
    try:
        bank.accounts[0].deposit(-5.0)
    except ValueError as e:
        print("error:", e)
    try:
        bank.index_of(42)
    except KeyError as e:
        print("no such account", e)
    for acct in bank.accounts:
        print(acct.number, " ".join(acct.log))


main()
