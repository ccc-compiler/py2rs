from dataclasses import dataclass

LOG = """2026-10-01 08:00:01 INFO  server started port=8080
2026-10-01 08:00:05 DEBUG config loaded keys=12
2026-10-01 08:01:17 WARN  slow request path=/api/users ms=1250
2026-10-01 08:02:44 ERROR db timeout path=/api/orders ms=5000
2026-10-01 08:03:10 INFO  request path=/api/users ms=35
2026-10-01 08:03:11 INFO  request path=/api/orders ms=120
2026-10-01 08:04:00 ERROR db timeout path=/api/orders ms=5000
2026-10-01 08:05:30 WARN  slow request path=/api/items ms=980
2026-10-01 08:06:02 INFO  request path=/api/items ms=40
2026-10-01 09:00:00 INFO  shutdown"""


@dataclass
class Entry:
    time: str
    level: str
    message: str
    fields: dict[str, str]


def parse_line(line: str) -> Entry:
    parts = line.split()
    fields: dict[str, str] = {}
    words: list[str] = []
    for p in parts[3:]:
        if "=" in p:
            eq = p.find("=")
            fields[p[:eq]] = p[eq + 1:]
        else:
            words.append(p)
    return Entry(parts[1], parts[2], " ".join(words), fields)


def seconds(t: str) -> int:
    h, m, s = t.split(":")[0], t.split(":")[1], t.split(":")[2]
    return int(h) * 3600 + int(m) * 60 + int(s)


def main() -> None:
    entries = [parse_line(l) for l in LOG.split("\n")]
    levels: dict[str, int] = {}
    for e in entries:
        levels[e.level] = levels.get(e.level, 0) + 1
    for lv in ["DEBUG", "INFO", "WARN", "ERROR"]:
        print(f"{lv:<6}{levels.get(lv, 0):>3}")
    paths: dict[str, list[int]] = {}
    for e in entries:
        if "path" in e.fields and "ms" in e.fields:
            paths.setdefault(e.fields["path"], []).append(int(e.fields["ms"]))
    print(f"{'path':<14}{'n':>3}{'avg ms':>10}{'max':>6}")
    for p in sorted(paths):
        ms = paths[p]
        print(f"{p:<14}{len(ms):>3}{sum(ms) / len(ms):>10.1f}{max(ms):>6}")
    for e in entries:
        if e.level != "ERROR":
            continue
        print("ERROR at", e.time, "-", e.message, e.fields)
    span = seconds(entries[-1].time) - seconds(entries[0].time)
    print(f"span {span // 60} min {span % 60} s")
    msgs = sorted({e.message for e in entries})
    print(msgs)


main()
