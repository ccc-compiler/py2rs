# error: @shared classes cannot be in a class hierarchy
from rust import shared


class Base:
    pass


@shared
class Leaf(Base):
    pass
