class Employee:
    def __init__(self, name: str, emp_id: int) -> None:
        self.name = name
        self.emp_id = emp_id

    def monthly_pay(self) -> float:
        return 0.0

    def title(self) -> str:
        return "employee"

    def __str__(self) -> str:
        return f"#{self.emp_id} {self.name} ({self.title()})"


class Salaried(Employee):
    def __init__(self, name: str, emp_id: int, salary: float) -> None:
        super().__init__(name, emp_id)
        self.salary = salary

    def monthly_pay(self) -> float:
        return self.salary / 12.0

    def title(self) -> str:
        return "salaried"


class Manager(Salaried):
    def __init__(self, name: str, emp_id: int, salary: float, bonus: float) -> None:
        super().__init__(name, emp_id, salary)
        self.bonus = bonus
        self.reports: list[int] = []

    def monthly_pay(self) -> float:
        return super().monthly_pay() + self.bonus + 50.0 * len(self.reports)

    def title(self) -> str:
        return f"manager of {len(self.reports)}"


class Hourly(Employee):
    def __init__(self, name: str, emp_id: int, rate: float) -> None:
        super().__init__(name, emp_id)
        self.rate = rate
        self.hours: list[float] = []

    def log(self, h: float) -> None:
        self.hours.append(h)

    def monthly_pay(self) -> float:
        total = 0.0
        for h in self.hours:
            if h > 8.0:
                total += 8.0 * self.rate + (h - 8.0) * self.rate * 1.5
            else:
                total += h * self.rate
        return total

    def title(self) -> str:
        return "hourly"


class Contractor(Employee):
    def __init__(self, name: str, emp_id: int, fee: float, done: bool) -> None:
        super().__init__(name, emp_id)
        self.fee = fee
        self.done = done

    def monthly_pay(self) -> float:
        return self.fee if self.done else self.fee * 0.25

    def title(self) -> str:
        return "contractor"


def tax(amount: float) -> float:
    if amount <= 2000.0:
        return amount * 0.1
    if amount <= 5000.0:
        return 200.0 + (amount - 2000.0) * 0.2
    return 800.0 + (amount - 5000.0) * 0.3


def make_hourly(name: str, emp_id: int, rate: float, shift: int) -> Hourly:
    h = Hourly(name, emp_id, rate)
    for x in [8.0, 9.5, 7.0, 10.0, 8.0, 6.5]:
        h.log(x + shift * 0.5)
    return h


def main() -> None:
    boss = Manager("alice", 1, 120000.0, 500.0)
    staff: list[Employee] = [
        Salaried("bob", 2, 54000.0),
        make_hourly("carol", 3, 22.5, 1),
        Contractor("dave", 4, 3000.0, True),
        Contractor("erin", 5, 4000.0, False),
        make_hourly("frank", 6, 30.0, 5),
    ]
    for e in staff:
        boss.reports.append(e.emp_id)
    staff.insert(0, boss)
    print(f"{'who':<40}{'gross':>10}{'tax':>10}{'net':>10}")
    total_gross = 0.0
    total_tax = 0.0
    for e in staff:
        g = e.monthly_pay()
        t = tax(g)
        total_gross += g
        total_tax += t
        print(f"{str(e):<40}{g:>10.2f}{t:>10.2f}{g - t:>10.2f}")
    print(f"{'TOTAL':<40}{total_gross:>10.2f}{total_tax:>10.2f}{total_gross - total_tax:>10.2f}")
    top = max(staff, key=lambda e: e.monthly_pay())
    print("top earner:", top.name)
    kinds: dict[str, list[str]] = {}
    for e in staff:
        k = e.title().split()[0]
        if k not in kinds:
            kinds[k] = []
        kinds[k].append(e.name)
    print(kinds)


main()
