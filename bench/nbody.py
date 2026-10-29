from dataclasses import dataclass


@dataclass
class Body:
    x: float
    y: float
    z: float
    vx: float
    vy: float
    vz: float
    m: float


def energy(bs: list[Body]) -> float:
    e = 0.0
    for i in range(len(bs)):
        b = bs[i]
        e += 0.5 * b.m * (b.vx * b.vx + b.vy * b.vy + b.vz * b.vz)
        for j in range(i + 1, len(bs)):
            c = bs[j]
            dx = b.x - c.x
            dy = b.y - c.y
            dz = b.z - c.z
            e -= b.m * c.m / (dx * dx + dy * dy + dz * dz) ** 0.5
    return e


def advance(bs: list[Body], dt: float, steps: int) -> None:
    n = len(bs)
    for _ in range(steps):
        for i in range(n):
            for j in range(i + 1, n):
                dx = bs[i].x - bs[j].x
                dy = bs[i].y - bs[j].y
                dz = bs[i].z - bs[j].z
                d2 = dx * dx + dy * dy + dz * dz
                mag = dt / (d2 * d2 ** 0.5)
                mi = bs[i].m * mag
                mj = bs[j].m * mag
                bs[i].vx -= dx * mj
                bs[i].vy -= dy * mj
                bs[i].vz -= dz * mj
                bs[j].vx += dx * mi
                bs[j].vy += dy * mi
                bs[j].vz += dz * mi
        for b in bs:
            b.x += dt * b.vx
            b.y += dt * b.vy
            b.z += dt * b.vz


def main() -> None:
    bs = [
        Body(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 39.47),
        Body(4.84, -1.16, -0.10, 0.60, 2.81, -0.02, 0.037),
        Body(8.34, 4.12, -0.40, -1.01, 1.82, 0.008, 0.011),
        Body(12.89, -15.11, -0.22, 1.08, 0.86, -0.01, 0.0017),
        Body(15.37, -25.91, 0.17, 0.97, 0.59, -0.03, 0.002),
    ]
    print(f"{energy(bs):.9f}")
    advance(bs, 0.01, 200000)
    print(f"{energy(bs):.9f}")


if __name__ == "__main__":
    main()
