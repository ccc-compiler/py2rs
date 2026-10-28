# error: comprehension
def parse(items: list[str]) -> list[int]:
    try:
        return [int(s) for s in items]
    except ValueError:
        return []
