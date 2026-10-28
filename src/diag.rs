//! Errors reported to the user, in Python terms.

use std::fmt;

#[derive(Clone, Debug)]
pub struct Error {
    pub line: u32,
    pub msg: String,
    pub hint: Option<String>,
}

pub type Res<T> = Result<T, Error>;

impl Error {
    pub fn new(line: u32, msg: impl Into<String>) -> Error {
        Error { line, msg: msg.into(), hint: None }
    }
    pub fn hint(mut self, h: impl Into<String>) -> Error {
        self.hint = Some(h.into());
        self
    }
    /// `file:line: error: msg` plus the source line and the hint.
    pub fn render(&self, file: &str, src: &str) -> String {
        let mut s = format!("{}:{}: error: {}\n", file, self.line, self.msg);
        if let Some(l) = src.lines().nth(self.line.saturating_sub(1) as usize) {
            s += &format!("  {:>4} | {}\n", self.line, l);
        }
        if let Some(h) = &self.hint {
            s += &format!("  hint: {}\n", h);
        }
        s
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.msg)?;
        if let Some(h) = &self.hint {
            write!(f, " (hint: {})", h)?;
        }
        Ok(())
    }
}

pub fn err<T>(line: u32, msg: impl Into<String>) -> Res<T> {
    Err(Error::new(line, msg))
}
