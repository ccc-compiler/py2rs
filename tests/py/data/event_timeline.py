from dataclasses import dataclass


@dataclass
class Event:
    start: int
    end: int
    room: str
    title: str


def t(s: str) -> int:
    return int(s[:2]) * 60 + int(s[3:])


def clock(m: int) -> str:
    return f"{m // 60:02d}:{m % 60:02d}"


def overlaps(a: Event, b: Event) -> bool:
    return a.room == b.room and a.start < b.end and b.start < a.end


def main() -> None:
    raw = """09:00-10:30 A Kickoff
09:30-10:00 B Standup
10:15-11:00 A Design review
10:00-12:00 B Workshop
11:00-11:30 A Coffee chat
13:00-14:30 A Planning
13:30-14:00 A Interview
14:00-15:00 C Demo
16:00-17:15 B Retro"""
    events: list[Event] = []
    for line in raw.splitlines():
        span = line.split()[0]
        room = line.split()[1]
        title = " ".join(line.split()[2:])
        events.append(Event(t(span[:5]), t(span[6:]), room, title))
    events.sort(key=lambda e: (e.start, e.room))
    day_start = min(e.start for e in events)
    day_end = max(e.end for e in events)
    scale = 15
    for e in events:
        lead = (e.start - day_start) // scale
        width = max(1, (e.end - e.start) // scale)
        print(f"{clock(e.start)} {e.room} {e.title:<14}|{' ' * lead}{'#' * width}")
    conflicts: list[str] = []
    for i in range(len(events)):
        for j in range(i + 1, len(events)):
            if overlaps(events[i], events[j]):
                conflicts.append(f"{events[i].title} / {events[j].title} in {events[i].room}")
    print("conflicts:", len(conflicts))
    for c in conflicts:
        print("  ", c)
    booked: dict[str, int] = {}
    for e in events:
        booked[e.room] = booked.get(e.room, 0) + e.end - e.start
    for r in sorted(booked):
        used = booked[r]
        print(f"room {r}: {used:>4} min booked, {used * 100 / (day_end - day_start):5.1f}% of the day")
    gaps: list[tuple[int, int]] = []
    cursor = day_start
    for e in events:
        if e.start > cursor:
            gaps.append((cursor, e.start))
        cursor = max(cursor, e.end)
    print("free slots:", [f"{clock(a)}-{clock(b)}" for a, b in gaps])


main()
