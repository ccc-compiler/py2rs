class TemplateError(Exception):
    pass


class MissingVar(TemplateError):
    pass


def render(tpl: str, env: dict[str, str]) -> str:
    out = ""
    i = 0
    while i < len(tpl):
        c = tpl[i]
        if c == "$" and i + 1 < len(tpl) and tpl[i + 1] == "$":
            out += "$"
            i += 2
        elif c == "$" and i + 1 < len(tpl) and tpl[i + 1] == "{":
            end = tpl[i:].find("}")
            if end >= 0:
                end += i
            if end < 0:
                raise TemplateError(f"unclosed ${{ at {i}")
            spec = tpl[i + 2:end]
            name = spec
            default = ""
            has_default = False
            if ":-" in spec:
                k = spec.find(":-")
                name = spec[:k]
                default = spec[k + 2:]
                has_default = True
            if name in env:
                out += env[name]
            elif has_default:
                out += default
            else:
                raise MissingVar(f"variable {name!r} is not set")
            i = end + 1
        else:
            out += c
            i += 1
    return out


def main() -> None:
    env = {"name": "Ada", "lang": "Rust", "n": "3", "empty": ""}
    tpls = ["Hello ${name}!", "${name} writes ${lang} (${n} years)", "cost: $$5", "${missing}",
            "${missing:-default value}", "x${empty}y", "broken ${name", "${name}${name}${name}",
            "unicode ${name}: ÄÖÜ ${lang:-?}", "$name stays", "end $"]
    for t in tpls:
        try:
            print(f"{t!r:<40} => {render(t, env)!r}")
        except MissingVar as e:
            print(f"{t!r:<40} => missing: {e}")
        except TemplateError as e:
            print(f"{t!r:<40} => error: {e}")


main()
