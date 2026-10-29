# error: mapping patterns are not supported
d = {"a": 1}
match d:
    case {"a": v}:
        print(v)
