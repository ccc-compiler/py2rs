class Event:
    def __init__(self, topic: str, payload: int) -> None:
        self.topic = topic
        self.payload = payload

    def __repr__(self) -> str:
        return f"Event({self.topic}, {self.payload})"


class Handler:
    def __init__(self, name: str) -> None:
        self.name = name
        self.seen = 0

    def handle(self, e: Event) -> str:
        self.seen += 1
        return f"{self.name} got {e.topic}"

    def summary(self) -> str:
        return f"{self.name}: {self.seen} events"


class Counter(Handler):
    def __init__(self, name: str) -> None:
        super().__init__(name)
        self.total = 0

    def handle(self, e: Event) -> str:
        super().handle(e)
        self.total += e.payload
        return f"{self.name} total={self.total}"

    def summary(self) -> str:
        return super().summary() + f", total {self.total}"


class Threshold(Handler):
    def __init__(self, name: str, limit: int) -> None:
        super().__init__(name)
        self.limit = limit
        self.alerts: list[int] = []

    def handle(self, e: Event) -> str:
        super().handle(e)
        if e.payload > self.limit:
            self.alerts.append(e.payload)
            return f"{self.name} ALERT {e.payload} > {self.limit}"
        return f"{self.name} ok"

    def summary(self) -> str:
        return super().summary() + f", alerts {self.alerts}"


class Logger(Handler):
    def __init__(self, name: str) -> None:
        super().__init__(name)
        self.lines: list[str] = []

    def handle(self, e: Event) -> str:
        super().handle(e)
        self.lines.append(f"{len(self.lines) + 1:03d} {e.topic}={e.payload}")
        return f"{self.name} logged"


class Bus:
    def __init__(self) -> None:
        self.handlers: list[Handler] = []
        self.routes: dict[str, list[int]] = {}

    def subscribe(self, topic: str, h: Handler) -> None:
        idx = -1
        for i, existing in enumerate(self.handlers):
            if existing.name == h.name:
                idx = i
        if idx < 0:
            idx = len(self.handlers)
            self.handlers.append(h)
        if topic not in self.routes:
            self.routes[topic] = []
        self.routes[topic].append(idx)

    def publish(self, e: Event) -> list[str]:
        out: list[str] = []
        targets = self.routes.get(topic_key(e.topic), [])
        for i, h in enumerate(self.handlers):
            if i in targets:
                out.append(h.handle(e))
        return out


def topic_key(t: str) -> str:
    return t.split(".")[0]


def main() -> None:
    bus = Bus()
    bus.subscribe("temp", Counter("sum"))
    bus.subscribe("temp", Threshold("hot", 30))
    bus.subscribe("load", Threshold("busy", 80))
    bus.subscribe("temp", Logger("log"))
    bus.subscribe("load", Logger("log"))
    bus.subscribe("door", Handler("plain"))
    events = [Event("temp.kitchen", 21), Event("load.cpu", 55), Event("temp.attic", 34),
              Event("door.front", 1), Event("load.cpu", 91), Event("noise", 3), Event("temp.cellar", 12)]
    for e in events:
        res = bus.publish(e)
        print(e, "->", str(res) if res else "nobody listening")
    for h in bus.handlers:
        print(h.summary())
    for h in bus.handlers:
        if isinstance(h, Logger):
            for line in h.lines:
                print("  ", line)
    print(bus.routes)


main()
