class StepError(Exception):
    pass


def step(n: int, log: list[str]) -> int:
    log.append(f"enter {n}")
    result = -1
    try:
        if n % 3 == 0:
            raise StepError(f"step {n} failed")
        if n % 5 == 0:
            raise ValueError(f"step {n} bad value")
        log.append(f"work {n}")
        result = n * 10
    finally:
        log.append(f"leave {n}")
    return result


def run(ns: list[int]) -> list[str]:
    log: list[str] = []
    for n in ns:
        try:
            r = step(n, log)
            log.append(f"result {r}")
        except StepError as e:
            log.append(f"caught {e}")
        except ValueError as e:
            log.append(f"value error {e}")
        else:
            log.append("no error")
        finally:
            log.append("---")
    return log


def outer(n: int) -> str:
    log: list[str] = []
    status = ""
    try:
        try:
            step(n, log)
            status = "inner ok"
        except StepError:
            status = "inner caught"
            raise ValueError("converted")
        log.append("inner done")
    except ValueError as e:
        status = f"outer caught {e}"
    finally:
        log.append("outer finally")
    return status + " / " + ", ".join(log)


def main() -> None:
    for line in run([1, 3, 5, 7, 15]):
        print(line)
    for n in [1, 3, 5]:
        print(outer(n))
    count = 0
    for i in range(6):
        try:
            if i % 2 != 0:
                count += 1
        finally:
            count += 100
    print("count", count)


main()
