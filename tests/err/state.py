# error: would modify
class State:
    def insert(self, m: "Machine", cents: int) -> str:
        return "ignored"


class Idle(State):
    def insert(self, m: "Machine", cents: int) -> str:
        m.credit += cents
        m.state = Busy()
        return "ok"


class Busy(State):
    pass


class Machine:
    def __init__(self) -> None:
        self.state: State = Idle()
        self.credit = 0

    def handle(self, cents: int) -> str:
        return self.state.insert(self, cents)


def main() -> None:
    m = Machine()
    print(m.handle(5), m.handle(10), m.credit)


main()
