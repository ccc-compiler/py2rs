# regression: Rust compile error: `back == s` where s iterates a list[str] variable (s is &String):
def main() -> None:
    samples = ["ab", "c"]
    for s in samples:
        back = s + ""
        print(back == s)


main()

#   for r in regions:            # regions: list[str]
#       sel = [a.rating for a in answers if a.region == r]
# error[E0277]: can't compare `String` with `&String`   (a.region == r)
