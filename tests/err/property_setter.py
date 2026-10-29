# error: property setters are not supported
class T:
    def __init__(self) -> None:
        self._c = 0.0

    @property
    def c(self) -> float:
        return self._c

    @c.setter
    def c(self, v: float) -> None:
        self._c = v
