# error: is not defined for these objects
class P:
    def __init__(self, x: int) -> None:
        self.x = x


print(P(1) < P(2))
