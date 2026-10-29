# regression: Rejected: "catching the ValueError this operation may raise is not supported yet" at the
def parse(tx: str) -> int:
    parts = tx.split(":")
    return int(parts[1])


for t in ["a:5", "b:x"]:
    try:
        print(parse(t))
    except ValueError:
        print("skipped")
