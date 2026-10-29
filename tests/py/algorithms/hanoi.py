def hanoi(n: int, src: str, dst: str, via: str, moves: list[tuple[int, str, str]]) -> None:
    if n == 0:
        return
    hanoi(n - 1, src, via, dst, moves)
    moves.append((n, src, dst))
    hanoi(n - 1, via, dst, src, moves)


def simulate(n: int, moves: list[tuple[int, str, str]], verbose: bool) -> dict[str, list[int]]:
    pegs: dict[str, list[int]] = {"A": list(range(n, 0, -1)), "B": [], "C": []}
    for i in range(len(moves)):
        disk, s, d = moves[i]
        top = pegs[s].pop()
        assert top == disk, "wrong disk"
        if pegs[d]:
            assert pegs[d][-1] > disk, "larger on smaller"
        pegs[d].append(top)
        if verbose:
            state = "  ".join(f"{k}:{pegs[k]}" for k in pegs)
            print(f"{i + 1:>3}. disk {disk} {s}->{d}   {state}")
    return pegs


def main() -> None:
    for n in range(1, 4):
        moves: list[tuple[int, str, str]] = []
        hanoi(n, "A", "C", "B", moves)
        print(f"n={n}: {len(moves)} moves")
        final = simulate(n, moves, True)
        print("final:", final)
    for n in [5, 10, 16]:
        moves2: list[tuple[int, str, str]] = []
        hanoi(n, "A", "C", "B", moves2)
        final2 = simulate(n, moves2, False)
        per_disk: dict[int, int] = {}
        for m in moves2:
            per_disk[m[0]] = per_disk.get(m[0], 0) + 1
        print(f"n={n}: {len(moves2)} moves (2^n-1 = {2 ** n - 1}), C has {len(final2['C'])} disks")
        print("   moves per disk:", [per_disk[k] for k in sorted(per_disk)][:6])


main()
