//! The `re` module on the `regex` crate. py2rs checks every pattern when it
//! translates (it must be a literal the regex crate accepts) and passes it on
//! already in regex syntax, with the flags as `(?i)` / `(?m)` / `(?s)`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use regex::Regex;

use crate::Py;

thread_local! {
    static CACHE: RefCell<HashMap<String, Rc<Regex>>> = RefCell::new(HashMap::new());
}

/// The compiled regex (once per thread and pattern).
fn regex(p: &str) -> Rc<Regex> {
    CACHE.with(|c| {
        if let Some(r) = c.borrow().get(p) {
            return r.clone();
        }
        let r = Rc::new(Regex::new(p).expect("pattern checked by py2rs"));
        c.borrow_mut().insert(p.to_string(), r.clone());
        r
    })
}

/// Character index of byte offset `b` in `s` (Python counts characters).
fn chars_at(s: &str, b: usize) -> i64 {
    s[..b].chars().count() as i64
}

/// `re.compile(p)`: `py` is the Python pattern (for its repr).
#[derive(Clone)]
pub struct Pattern {
    src: &'static str,
    py: &'static str,
}

impl Py for Pattern {
    fn fmt_repr(&self, out: &mut String) {
        out.push_str("re.compile(");
        self.py.to_string().fmt_repr(out);
        out.push(')');
    }
}

/// A match: the subject and the group positions (bytes).
#[derive(Clone)]
pub struct Match {
    subject: String,
    groups: Vec<Option<(usize, usize)>>,
    names: Rc<Regex>,
}

impl Match {
    fn new(re: &Rc<Regex>, s: &str, c: regex::Captures) -> Match {
        Match {
            subject: s.to_string(),
            groups: (0..c.len()).map(|i| c.get(i).map(|m| (m.start(), m.end()))).collect(),
            names: re.clone(),
        }
    }

    fn span_of(&self, i: usize) -> (usize, usize) {
        match self.groups.get(i) {
            Some(Some(s)) => *s,
            Some(None) => crate::raise(
                "ValueError",
                &format!("group {} did not take part in the match (Python gives None; not in the typed subset)", i),
            ),
            None => crate::raise("IndexError", "no such group"),
        }
    }

    /// `m.group(i)`
    pub fn group(&self, i: i64) -> String {
        let (a, b) = self.span_of(i as usize);
        self.subject[a..b].to_string()
    }

    /// `m.group("name")`
    pub fn group_named(&self, name: &str) -> String {
        let i = self.names.capture_names().position(|n| n == Some(name));
        match i {
            Some(i) => self.group(i as i64),
            None => crate::raise("IndexError", "no such group"),
        }
    }

    /// `m.groups()` (n groups)
    pub fn groups(&self) -> Vec<String> {
        (1..self.groups.len()).map(|i| self.group(i as i64)).collect()
    }

    pub fn start(&self, i: i64) -> i64 {
        let (a, _) = self.span_of(i as usize);
        chars_at(&self.subject, a)
    }

    pub fn end(&self, i: i64) -> i64 {
        let (_, b) = self.span_of(i as usize);
        chars_at(&self.subject, b)
    }

    pub fn span(&self, i: i64) -> (i64, i64) {
        (self.start(i), self.end(i))
    }
}

impl Py for Match {
    fn fmt_repr(&self, out: &mut String) {
        let (a, b) = self.span(0);
        out.push_str(&format!("<re.Match object; span=({}, {}), match=", a, b));
        self.group(0).fmt_repr(out);
        out.push('>');
    }
}

/// `re.compile(p)`
pub fn compile(src: &'static str, py: &'static str) -> Pattern {
    Pattern { src, py }
}

fn find(re: Rc<Regex>, s: &str) -> Option<Match> {
    re.captures(s).map(|c| Match::new(&re, s, c))
}

impl Pattern {
    fn re(&self) -> Rc<Regex> {
        regex(self.src)
    }

    pub fn search(&self, s: &str) -> Option<Match> {
        find(self.re(), s)
    }

    /// `match`: at the start
    pub fn match_(&self, s: &str) -> Option<Match> {
        find(regex(&format!("\\A(?:{})", self.src)), s)
    }

    /// `fullmatch`: the whole string
    pub fn fullmatch(&self, s: &str) -> Option<Match> {
        find(regex(&format!("\\A(?:{})\\z", self.src)), s)
    }

    pub fn findall(&self, s: &str) -> Vec<String> {
        self.re().captures_iter(s).map(|c| c.get(c.len().min(2) - 1).map_or(String::new(), |m| m.as_str().to_string())).collect()
    }

    /// `findall` with two or more groups
    pub fn findall_groups(&self, s: &str) -> Vec<Vec<String>> {
        self.re()
            .captures_iter(s)
            .map(|c| (1..c.len()).map(|i| c.get(i).map_or(String::new(), |m| m.as_str().to_string())).collect())
            .collect()
    }

    pub fn finditer(&self, s: &str) -> Vec<Match> {
        let re = self.re();
        re.captures_iter(s).map(|c| Match::new(&re, s, c)).collect()
    }

    /// `sub(repl, s, count)`: `repl` is in regex syntax (`${1}`)
    pub fn sub(&self, repl: &str, s: &str, count: i64) -> String {
        self.re().replacen(s, count.max(0) as usize, repl).into_owned()
    }

    pub fn split(&self, s: &str) -> Vec<String> {
        self.re().split(s).map(String::from).collect()
    }
}

/// The module functions: the pattern is compiled once per thread.
pub fn search(p: &'static str, s: &str) -> Option<Match> {
    compile(p, p).search(s)
}

pub fn match_(p: &'static str, s: &str) -> Option<Match> {
    compile(p, p).match_(s)
}

pub fn fullmatch(p: &'static str, s: &str) -> Option<Match> {
    compile(p, p).fullmatch(s)
}

pub fn findall(p: &'static str, s: &str) -> Vec<String> {
    compile(p, p).findall(s)
}

pub fn findall_groups(p: &'static str, s: &str) -> Vec<Vec<String>> {
    compile(p, p).findall_groups(s)
}

pub fn finditer(p: &'static str, s: &str) -> Vec<Match> {
    compile(p, p).finditer(s)
}

pub fn sub(p: &'static str, repl: &str, s: &str, count: i64) -> String {
    compile(p, p).sub(repl, s, count)
}

pub fn split(p: &'static str, s: &str) -> Vec<String> {
    compile(p, p).split(s)
}

/// `re.escape(s)` (Python escapes everything but ASCII letters, digits and `_`... and
/// since 3.7 only the special characters)
pub fn escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if "()[]{}?*+-|^$\\.&~# \t\n\r\x0b\x0c".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}
