"""CPython stand-in for `from rust.regex import Regex` (the Rust `regex` crate),
so that programs using it run under CPython too. Written for the common subset
of the two syntaxes; `$1` in replacements is the crate's group syntax."""

import re as _re


class Regex:
    def __init__(self, pattern: str) -> None:
        self._p = _re.compile(pattern)
        self._s = pattern

    @staticmethod
    def new(pattern: str) -> "Regex":
        try:
            return Regex(pattern)
        except _re.error as e:
            raise ValueError(str(e))

    def is_match(self, text: str) -> bool:
        return self._p.search(text) is not None

    def find(self, text: str) -> "str | None":
        m = self._p.search(text)
        return m.group(0) if m else None

    def find_all(self, text: str) -> "list[str]":
        return [m.group(0) for m in self._p.finditer(text)]

    def replace_all(self, text: str, rep: str) -> str:
        return self._p.sub(_re.sub(r"\$(\d+)", r"\\\1", rep), text)

    def split(self, text: str) -> "list[str]":
        out, last = [], 0
        for m in self._p.finditer(text):
            out.append(text[last:m.start()])
            last = m.end()
        out.append(text[last:])
        return out

    def captures(self, text: str) -> "list[str] | None":
        m = self._p.search(text)
        if not m:
            return None
        return [m.group(0)] + [g or "" for g in m.groups()]

    def pattern(self) -> str:
        return self._s
