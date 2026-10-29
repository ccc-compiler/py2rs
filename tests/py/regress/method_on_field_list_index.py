# regression: BUG (Rust compile error, same cause as field_list_index_store): a mutating method call on
class Acc:
    def __init__(self, n: int) -> None:
        self.n = n

    def add(self, k: int) -> None:
        self.n += k


class Bank:
    def __init__(self) -> None:
        self.accs: list[Acc] = [Acc(1), Acc(2)]

    def bump(self, i: int) -> None:
        self.accs[i].add(5)


def main() -> None:
    b = Bank()
    b.bump(1)
    print(b.accs[1].n)


main()
