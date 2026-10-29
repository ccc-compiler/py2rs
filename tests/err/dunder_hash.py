# error: special method `__hash__` is not supported
class K:
    def __init__(self, x: int) -> None:
        self.x = x

    def __hash__(self) -> int:
        return self.x
