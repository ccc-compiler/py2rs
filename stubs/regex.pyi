# crate: regex = "1"
"""py2rs stub for the `regex` crate: `from rust.regex import Regex`.

Each declaration maps to Rust: a path (`Regex::new`), a method name, or an
expression template where `{self}` is the receiver and `{0}`, `{1}` are the
arguments. `Result[T]` raises ValueError when the crate returns Err."""

from rust import Result


class Regex:
    @staticmethod
    def new(pattern: str) -> Result["Regex"]: ...

    def is_match(self, text: str) -> bool: ...

    @rust("{self}.find({0}).map(|m| m.as_str().to_string())")
    def find(self, text: str) -> str | None: ...

    @rust("{self}.find_iter({0}).map(|m| m.as_str().to_string()).collect::<Vec<String>>()")
    def find_all(self, text: str) -> list[str]: ...

    @rust("{self}.replace_all({0}, {1}).into_owned()")
    def replace_all(self, text: str, rep: str) -> str: ...

    @rust("{self}.split({0}).map(String::from).collect::<Vec<String>>()")
    def split(self, text: str) -> list[str]: ...

    @rust("{self}.captures({0}).map(|c| c.iter().map(|m| m.map(|m| m.as_str().to_string()).unwrap_or_default()).collect::<Vec<String>>())")
    def captures(self, text: str) -> list[str] | None: ...

    @rust("{self}.as_str().to_string()")
    def pattern(self) -> str: ...
