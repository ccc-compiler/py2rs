class State:
    def name(self) -> str:
        return "state"

    def insert(self, m: "Machine", cents: int) -> str:
        return "ignored coin"

    def select(self, m: "Machine", code: str) -> str:
        return "ignored selection"

    def cancel(self, m: "Machine") -> str:
        return "nothing to cancel"


class Idle(State):
    def name(self) -> str:
        return "idle"

    def insert(self, m: "Machine", cents: int) -> str:
        m.credit += cents
        m.next_state = "has-credit"
        return f"credit {m.credit}"


class HasCredit(State):
    def name(self) -> str:
        return "has-credit"

    def insert(self, m: "Machine", cents: int) -> str:
        m.credit += cents
        return f"credit {m.credit}"

    def select(self, m: "Machine", code: str) -> str:
        if code not in m.prices:
            return f"no product {code}"
        if m.stock[code] == 0:
            return f"{code} sold out"
        price = m.prices[code]
        if m.credit < price:
            return f"{code} costs {price}, credit {m.credit}"
        m.stock[code] -= 1
        m.credit -= price
        m.sales += price
        change = m.make_change()
        if m.total_stock() > 0:
            m.next_state = "idle"
        else:
            m.next_state = "sold-out"
        return f"vend {code}, change {change}"

    def cancel(self, m: "Machine") -> str:
        change = m.make_change()
        m.next_state = "idle"
        return f"refund {change}"


class SoldOut(State):
    def name(self) -> str:
        return "sold-out"

    def insert(self, m: "Machine", cents: int) -> str:
        return f"sold out, returning {cents}"


class Machine:
    def __init__(self) -> None:
        self.state_name = "idle"
        self.next_state = "idle"
        self.credit = 0
        self.sales = 0
        self.prices: dict[str, int] = {"A1": 65, "A2": 100, "B1": 135}
        self.stock: dict[str, int] = {"A1": 2, "A2": 1, "B1": 1}

    def make_change(self) -> list[int]:
        coins: list[int] = []
        for c in [100, 25, 10, 5]:
            while self.credit >= c:
                coins.append(c)
                self.credit -= c
        return coins

    def total_stock(self) -> int:
        return sum(self.stock.values())

    def handle(self, cmd: str) -> str:
        parts = cmd.split()
        st = make_state(self.state_name)
        self.next_state = self.state_name
        if parts[0] == "coin":
            out = st.insert(self, int(parts[1]))
        elif parts[0] == "sel":
            out = st.select(self, parts[1])
        else:
            out = st.cancel(self)
        self.state_name = self.next_state
        return out


def make_state(name: str) -> State:
    if name == "idle":
        return Idle()
    if name == "has-credit":
        return HasCredit()
    return SoldOut()


def main() -> None:
    m = Machine()
    script = [
        "sel A1", "coin 25", "coin 25", "sel A1", "coin 25", "sel A1", "coin 100", "sel C9",
        "sel A2", "coin 100", "coin 10", "cancel", "cancel", "coin 100", "coin 100", "sel A1",
        "coin 100", "coin 25", "coin 10", "sel B1", "coin 5", "sel A1",
    ]
    for cmd in script:
        before = make_state(m.state_name).name()
        out = m.handle(cmd)
        print(f"{cmd:<10} [{before:>10} -> {make_state(m.state_name).name():<10}] {out}")
    print("sales", m.sales, "stock", m.stock, "credit", m.credit)


main()
