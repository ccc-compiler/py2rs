class RetryableError(Exception):
    pass


class FatalError(Exception):
    pass


def flaky(attempt: int, fail_until: int) -> str:
    if attempt < fail_until:
        raise RetryableError(f"attempt {attempt} failed")
    if fail_until > 4:
        raise FatalError("gave up")
    return f"success on attempt {attempt}"


def with_retry(fail_until: int, tries: int) -> str:
    last = ""
    for attempt in range(tries):
        try:
            return flaky(attempt, fail_until)
        except RetryableError as e:
            last = str(e)
            print("  retry:", e)
    raise FatalError(f"exhausted after {tries}: {last}")


def audited(fail_until: int) -> str:
    try:
        return with_retry(fail_until, 3)
    except FatalError as e:
        print("  audit saw:", e)
        raise


def main() -> None:
    for fu in [0, 2, 3, 6]:
        print("case", fu)
        try:
            print(" ", audited(fu))
        except FatalError as e:
            print("  fatal:", e)
    try:
        try:
            int("nope")
        except ValueError:
            print("inner handler, re-raising")
            raise
    except ValueError as e:
        print("outer got:", e)
    try:
        try:
            raise KeyError("k")
        except KeyError as e:
            raise RuntimeError(f"wrapped {e}")
    except RuntimeError as e:
        print("wrapped:", e)


main()
