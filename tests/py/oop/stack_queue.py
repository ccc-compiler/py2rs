class Stack:
    def __init__(self) -> None:
        self.items: list[str] = []

    def push(self, x: str) -> None:
        self.items.append(x)

    def pop(self) -> str:
        if not self.items:
            raise IndexError("pop from empty stack")
        return self.items.pop()

    def peek(self) -> str | None:
        if self.items:
            return self.items[-1]
        return None

    def size(self) -> int:
        return len(self.items)

    def __repr__(self) -> str:
        return f"Stack({self.items})"


class Queue:
    def __init__(self, capacity: int) -> None:
        self.capacity = capacity
        self.buf: dict[int, int] = {}
        self.head = 0
        self.count = 0

    def enqueue(self, x: int) -> bool:
        if self.count == self.capacity:
            return False
        self.buf[(self.head + self.count) % self.capacity] = x
        self.count += 1
        return True

    def dequeue(self) -> int | None:
        if self.count == 0:
            return None
        x = self.buf[self.head]
        self.head = (self.head + 1) % self.capacity
        self.count -= 1
        return x

    def items(self) -> list[int]:
        return [self.buf[(self.head + i) % self.capacity] for i in range(self.count)]

    def __str__(self) -> str:
        return f"Queue{self.items()} ({self.count}/{self.capacity})"


def balanced(text: str) -> bool:
    pairs = {")": "(", "]": "[", "}": "{"}
    st = Stack()
    for ch in text:
        if ch in "([{":
            st.push(ch)
        elif ch in pairs:
            if st.size() == 0 or st.pop() != pairs[ch]:
                return False
    return st.size() == 0


def rpn(expr: str) -> float:
    st: list[float] = []
    for tok in expr.split():
        if tok in ["+", "-", "*", "/"]:
            b = st.pop()
            a = st.pop()
            if tok == "+":
                st.append(a + b)
            elif tok == "-":
                st.append(a - b)
            elif tok == "*":
                st.append(a * b)
            else:
                st.append(a / b)
        else:
            st.append(float(tok))
    return st[0]


def main() -> None:
    s = Stack()
    for w in "the quick brown fox".split():
        s.push(w)
    print(s, s.peek(), s.size())
    while s.size() > 0:
        print("pop", s.pop())
    try:
        s.pop()
    except IndexError as e:
        print("error:", e)
    for t in ["(a[b]{c})", "(]", "((", "{[()()]}", ""]:
        print(repr(t), balanced(t))
    for e in ["3 4 +", "5 1 2 + 4 * + 3 -", "2 3 4 * +", "10 4 /"]:
        print(e, "=", rpn(e))
    q = Queue(4)
    for i in range(6):
        print("enqueue", i * 10, q.enqueue(i * 10), q)
    print("dequeue", q.dequeue(), q.dequeue())
    q.enqueue(99)
    q.enqueue(100)
    print(q)
    while True:
        x = q.dequeue()
        if x is None:
            break
        print("got", x)
    print(q)


main()
