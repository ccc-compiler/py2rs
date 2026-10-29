import random


def main() -> None:
    random.seed(2024)
    print(random.random())
    print([random.randint(1, 6) for _ in range(10)])
    print(random.randrange(10), random.randrange(5, 50, 5), random.uniform(1.5, 2.5))
    xs = list(range(10))
    random.shuffle(xs)
    print(xs)
    print(random.choice(["red", "green", "blue"]))
    print(random.sample(range(100), 5))
    print(random.sample(xs, 3))
    random.seed(7)
    a = random.random()
    random.seed(7)
    print(a == random.random())
    big = random.sample(range(1000), 30)
    print(big)
    hits = 0
    for _ in range(1000):
        x = random.random()
        y = random.random()
        if x * x + y * y < 1.0:
            hits += 1
    print(hits)
    try:
        random.choice([])
    except IndexError as e:
        print("error:", e)


if __name__ == "__main__":
    main()
