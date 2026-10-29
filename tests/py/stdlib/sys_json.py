import sys
import json
import time


def main() -> None:
    start = time.perf_counter()
    n = int(sys.stdin.readline())
    total = 0
    rows: list[list[int]] = []
    for _ in range(n):
        row = [int(x) for x in sys.stdin.readline().split()]
        rows.append(row)
        total += sum(row)
    rest = sys.stdin.read()
    print(total, len(rest.split()))
    print(json.dumps(rows))
    data = {"name": ["py2rs"], "tags": ["a", "b"], "empty": []}
    print(json.dumps(data))
    print(json.dumps({"version": 6, "ok": 1}), json.dumps([True, False]), json.dumps(0.1 + 0.2))
    print(json.dumps({1: None, 2: "x"}))
    print(json.dumps("quote \" backslash \\ tab \t é"))
    print(json.dumps((1, 2.0)))
    print(sys.maxsize)
    print(len(sys.argv))
    sys.stdout.write("no newline")
    sys.stdout.write("\n")
    elapsed = time.perf_counter() - start
    print(elapsed >= 0.0)
    sys.exit(0)


if __name__ == "__main__":
    main()
