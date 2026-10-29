def main() -> None:
    for i in range(300_000):
        print(i, i * i, f"{i / 7:.3f}")


if __name__ == "__main__":
    main()
