class AppError(Exception):
    pass


class ConfigError(AppError):
    pass


class MissingKey(ConfigError):
    pass


class BadValue(ConfigError):
    pass


def read_value(cfg: dict[str, str], key: str) -> str:
    if key not in cfg:
        raise MissingKey(f"missing key {key!r}")
    return cfg[key]


def read_int(cfg: dict[str, str], key: str) -> int:
    raw = read_value(cfg, key)
    try:
        return int(raw)
    except ValueError:
        raise BadValue(f"{key} is not an int: {raw!r}")


def read_port(cfg: dict[str, str]) -> int:
    p = read_int(cfg, "port")
    if p < 1 or p > 65535:
        raise BadValue(f"port out of range: {p}")
    return p


def load(cfg: dict[str, str]) -> str:
    port = read_port(cfg)
    host = read_value(cfg, "host")
    return f"{host}:{port}"


def describe(cfg: dict[str, str]) -> str:
    try:
        return "ok " + load(cfg)
    except MissingKey as e:
        return f"missing: {e}"
    except BadValue as e:
        return f"bad: {e}"


def strict(cfg: dict[str, str]) -> str:
    try:
        return load(cfg)
    except ConfigError as e:
        print("  logging:", e)
        raise


def main() -> None:
    cfgs = [{"host": "localhost", "port": "8080"}, {"host": "a"}, {"port": "x"},
            {"port": "70000", "host": "b"}, {"port": "22"}, {"port": " 443 ", "host": "c.org"}]
    for c in cfgs:
        print(describe(c))
    for c in cfgs:
        try:
            print("strict", strict(c))
        except AppError as e:
            print("strict failed:", e)
    errors = 0
    for c in cfgs:
        try:
            load(c)
        except AppError:
            errors += 1
    print("errors", errors)


main()
