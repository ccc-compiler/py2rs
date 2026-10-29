import os


def write_report(path: str, rows: list[tuple[str, int]]) -> None:
    with open(path, "w") as f:
        f.write("name,score\n")
        for name, score in rows:
            f.write(f"{name},{score}\n")


def read_scores(path: str) -> dict[str, int]:
    scores: dict[str, int] = {}
    with open(path) as f:
        header = f.readline()
        print("header:", header.strip())
        for line in f:
            name, score = line.strip().split(",")
            scores[name] = int(score)
    return scores


def main() -> None:
    path = os.path.join("py2rs_files_test.csv")
    write_report(path, [("ann", 90), ("bob", 72), ("cy", 85)])
    print(os.path.exists(path))
    print(read_scores(path))
    with open(path, "a") as f:
        n = f.write("dee,60\n")
        print("wrote", n)
    with open(path, encoding="utf-8") as f:
        text = f.read()
    print(len(text.splitlines()), text.count("\n"))
    with open(path) as f:
        lines = f.readlines()
    print(lines[-1].strip())
    os.remove(path)
    print(os.path.exists(path))
    try:
        with open(path) as f:
            print(f.read())
    except FileNotFoundError as e:
        print("error:", e)
    print(os.path.join("a", "b", "c.txt"), os.path.join("a/", "/abs", "x"))


if __name__ == "__main__":
    main()
