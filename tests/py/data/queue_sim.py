from dataclasses import dataclass


@dataclass
class Customer:
    cid: int
    arrive: int
    service: int


def rng(state: list[int]) -> int:
    state[0] = (state[0] * 1103515245 + 12345) % 2147483648
    return (state[0] >> 8) % 1000


def main() -> None:
    state = [2026]
    customers: list[Customer] = []
    t = 0
    for i in range(25):
        t += rng(state) % 6
        customers.append(Customer(i, t, 2 + rng(state) % 7))
    n_tellers = 2
    free_at = [0] * n_tellers
    waits: list[int] = []
    busy = [0] * n_tellers
    print(f"{'id':>3} {'arr':>4} {'svc':>4} {'tel':>4} {'start':>6} {'end':>5} {'wait':>5}")
    for c in customers:
        teller = min(range(n_tellers), key=lambda k: free_at[k])
        start = max(c.arrive, free_at[teller])
        end = start + c.service
        if teller == 0:
            free_at = [end, free_at[1]]
            busy = [busy[0] + c.service, busy[1]]
        else:
            free_at = [free_at[0], end]
            busy = [busy[0], busy[1] + c.service]
        waits.append(start - c.arrive)
        print(f"{c.cid:>3} {c.arrive:>4} {c.service:>4} {teller:>4} {start:>6} {end:>5} {start - c.arrive:>5}")
    horizon = max(free_at)
    print(f"finished at t={horizon}")
    print(f"mean wait {sum(waits) / len(waits):.2f}, max wait {max(waits)}")
    for k in range(n_tellers):
        print(f"teller {k}: busy {busy[k]} of {horizon} ({busy[k] * 100 / horizon:.1f}%)")
    waited = [w for w in waits if w > 0]
    print(f"{len(waited)} of {len(waits)} customers waited")


main()
