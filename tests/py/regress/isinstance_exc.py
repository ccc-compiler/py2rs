# regression: Rejected: "`MyErr` is not a class of this program" for isinstance on a caught exception
class MyErr(ValueError):
    pass


try:
    raise MyErr("boom")
except ValueError as e:
    print("MyErr" if isinstance(e, MyErr) else "ValueError", e)
