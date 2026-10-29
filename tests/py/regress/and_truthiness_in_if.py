# regression: Rejected although only used as an if condition (plain 'if cur:' works); maybe by design:
def main() -> None:
    cur: list[str] = []
    for w in ["a", "b"]:
        if cur and len(w) > 0:
            print("nonempty")
        cur.append(w)
    print(cur)


main()
