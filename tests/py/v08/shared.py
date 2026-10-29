from __future__ import annotations
from dataclasses import dataclass
from rust import shared


@shared
class Node:
    def __init__(self, name: str) -> None:
        self.name = name
        self.children: list[Node] = []
        self.parent: Node | None = None
        self.visits = 0

    def add(self, child: Node) -> None:
        child.parent = self
        self.children.append(child)

    def path(self) -> str:
        p = self.parent
        if p is None:
            return self.name
        return p.path() + "/" + self.name


@shared
@dataclass
class Account:
    owner: str
    balance: int


def deposit(acct: Account, n: int) -> None:
    acct.balance += n


def main() -> None:
    root = Node("root")
    usr = Node("usr")
    local = Node("local")
    root.add(usr)
    usr.add(local)
    print(local.path(), len(root.children), len(usr.children))
    alias = usr
    alias.visits += 1
    usr.visits += 2
    print(usr.visits, alias.visits, alias is usr, alias is local)
    for c in root.children:
        c.visits += 10
    print(usr.visits)
    a = Account("ann", 10)
    accounts = [a, Account("bob", 5)]
    deposit(a, 7)
    deposit(accounts[1], 1)
    print(a, accounts[0], accounts[1].balance)
    same = accounts[0]
    same.owner = "anna"
    print(a.owner, a == Account("anna", 17), a is same)


if __name__ == "__main__":
    main()
