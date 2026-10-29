//! Exceptions that a `try` can catch: `Result<T, PyErr>`.
//!
//! Operations whose exception nobody catches abort with a traceback instead
//! (`raise`); code inside a `try` (and functions called from it) uses the
//! `try_*` variants and `?` / `check!`.

use std::fmt;

/// An exception: its class and base classes (most specific first) and message.
#[derive(Clone, Debug, PartialEq)]
pub struct PyErr {
    pub chain: &'static [&'static str],
    pub msg: String,
}

pub const EXCEPTION: &[&str] = &["Exception", "BaseException"];
pub const VALUE_ERROR: &[&str] = &["ValueError", "Exception", "BaseException"];
pub const TYPE_ERROR: &[&str] = &["TypeError", "Exception", "BaseException"];
pub const INDEX_ERROR: &[&str] = &["IndexError", "LookupError", "Exception", "BaseException"];
pub const KEY_ERROR: &[&str] = &["KeyError", "LookupError", "Exception", "BaseException"];
pub const LOOKUP_ERROR: &[&str] = &["LookupError", "Exception", "BaseException"];
pub const ZERO_DIVISION_ERROR: &[&str] = &["ZeroDivisionError", "ArithmeticError", "Exception", "BaseException"];
pub const ARITHMETIC_ERROR: &[&str] = &["ArithmeticError", "Exception", "BaseException"];
pub const OVERFLOW_ERROR: &[&str] = &["OverflowError", "ArithmeticError", "Exception", "BaseException"];
pub const RUNTIME_ERROR: &[&str] = &["RuntimeError", "Exception", "BaseException"];
pub const NOT_IMPLEMENTED_ERROR: &[&str] = &["NotImplementedError", "RuntimeError", "Exception", "BaseException"];
pub const ASSERTION_ERROR: &[&str] = &["AssertionError", "Exception", "BaseException"];
pub const ATTRIBUTE_ERROR: &[&str] = &["AttributeError", "Exception", "BaseException"];
pub const EOF_ERROR: &[&str] = &["EOFError", "Exception", "BaseException"];
pub const NAME_ERROR: &[&str] = &["NameError", "Exception", "BaseException"];
pub const STOP_ITERATION: &[&str] = &["StopIteration", "Exception", "BaseException"];

impl PyErr {
    pub fn new(chain: &'static [&'static str], msg: impl Into<String>) -> PyErr {
        PyErr { chain, msg: msg.into() }
    }
    /// The exception's class name.
    pub fn kind(&self) -> &'static str {
        self.chain[0]
    }
    /// `isinstance(e, base)`
    pub fn is(&self, base: &str) -> bool {
        self.chain.contains(&base)
    }
}

impl fmt::Display for PyErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad(&self.msg)
    }
}

impl crate::Py for PyErr {
    fn fmt_repr(&self, out: &mut String) {
        out.push_str(self.kind());
        out.push('(');
        if !self.msg.is_empty() {
            out.push_str(&crate::str_repr(&self.msg));
        }
        out.push(')');
    }
    fn fmt_str(&self, out: &mut String) {
        out.push_str(&self.msg)
    }
}

/// An exception that reached the top: print it like CPython and exit.
pub fn uncaught(e: PyErr) -> ! {
    crate::raise(e.kind(), &e.msg)
}

/// `.or_raise()`: the exception is not caught anywhere, abort with it.
pub trait OrRaise<T> {
    fn or_raise(self) -> T;
}

impl<T> OrRaise<T> for Result<T, PyErr> {
    fn or_raise(self) -> T {
        match self {
            Ok(v) => v,
            Err(e) => uncaught(e),
        }
    }
}

/// Inside `let e = 'body: { ... };` of a try: an exception leaves the block.
#[macro_export]
macro_rules! check {
    ($e:expr, $l:lifetime) => {
        match $e {
            Ok(v) => v,
            Err(e) => break $l e,
        }
    };
}

pub type PyResult<T> = Result<T, PyErr>;

thread_local! {
    static CATCHING: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

pub(crate) fn catching_now() -> bool {
    CATCHING.with(|c| c.get() > 0)
}

/// The class chain of a builtin exception name.
pub fn builtin_chain(name: &str) -> &'static [&'static str] {
    match name {
        "ValueError" => VALUE_ERROR,
        "TypeError" => TYPE_ERROR,
        "IndexError" => INDEX_ERROR,
        "KeyError" => KEY_ERROR,
        "LookupError" => LOOKUP_ERROR,
        "ZeroDivisionError" => ZERO_DIVISION_ERROR,
        "ArithmeticError" => ARITHMETIC_ERROR,
        "OverflowError" => OVERFLOW_ERROR,
        "NotImplementedError" => NOT_IMPLEMENTED_ERROR,
        "AssertionError" => ASSERTION_ERROR,
        "AttributeError" => ATTRIBUTE_ERROR,
        "EOFError" => EOF_ERROR,
        "StopIteration" => STOP_ITERATION,
        "NameError" => NAME_ERROR,
        "RuntimeError" => RUNTIME_ERROR,
        _ => EXCEPTION,
    }
}

/// Run an operation whose exception a `try` catches: what it raises (or a
/// Rust panic such as an index out of range) becomes the `Err`.
pub fn catching<T>(f: impl FnOnce() -> T) -> PyResult<T> {
    CATCHING.with(|c| c.set(c.get() + 1));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    CATCHING.with(|c| c.set(c.get() - 1));
    match r {
        Ok(v) => Ok(v),
        Err(p) => match p.downcast::<PyErr>() {
            Ok(e) => Err(*e),
            Err(p) => {
                let msg = p
                    .downcast_ref::<&str>()
                    .map(|s| s.to_string())
                    .or_else(|| p.downcast_ref::<String>().cloned())
                    .unwrap_or_default();
                let text = crate::exception_of(&msg);
                let (k, m) = text.split_once(": ").unwrap_or((&text, ""));
                Err(PyErr::new(builtin_chain(k), m))
            }
        },
    }
}

// ---------------------------------------------------------------- fallible ops

/// A list index, IndexError when out of range.
pub fn try_idx(i: i64, len: usize) -> PyResult<usize> {
    let n = len as i64;
    let k = if i < 0 { i + n } else { i };
    if k < 0 || k >= n {
        return Err(PyErr::new(INDEX_ERROR, "list index out of range"));
    }
    Ok(k as usize)
}

/// `d[k]`
pub fn try_key<'a, K, Q, V>(d: &'a crate::IndexMap<K, V>, k: &Q) -> PyResult<&'a V>
where
    K: std::hash::Hash + Eq,
    Q: std::hash::Hash + indexmap::Equivalent<K> + crate::Py + ?Sized,
{
    d.get(k).ok_or_else(|| PyErr::new(KEY_ERROR, crate::repr(k)))
}

/// `d[k] op= v`
pub fn try_key_mut<'a, K, Q, V>(d: &'a mut crate::IndexMap<K, V>, k: &Q) -> PyResult<&'a mut V>
where
    K: std::hash::Hash + Eq,
    Q: std::hash::Hash + indexmap::Equivalent<K> + crate::Py + ?Sized,
{
    let r = crate::repr(k);
    d.get_mut(k).ok_or_else(|| PyErr::new(KEY_ERROR, r))
}

pub fn try_floordiv(a: i64, b: i64) -> PyResult<i64> {
    if b == 0 {
        return Err(PyErr::new(ZERO_DIVISION_ERROR, "division by zero"));
    }
    Ok(crate::floordiv(a, b))
}

pub fn try_modulo(a: i64, b: i64) -> PyResult<i64> {
    if b == 0 {
        return Err(PyErr::new(ZERO_DIVISION_ERROR, "division by zero"));
    }
    Ok(crate::modulo(a, b))
}

pub fn try_div(a: f64, b: f64) -> PyResult<f64> {
    if b == 0.0 {
        return Err(PyErr::new(ZERO_DIVISION_ERROR, "division by zero"));
    }
    Ok(a / b)
}

pub fn try_ffloordiv(a: f64, b: f64) -> PyResult<f64> {
    if b == 0.0 {
        return Err(PyErr::new(ZERO_DIVISION_ERROR, "division by zero"));
    }
    Ok(crate::ffloordiv(a, b))
}

pub fn try_fmod(a: f64, b: f64) -> PyResult<f64> {
    if b == 0.0 {
        return Err(PyErr::new(ZERO_DIVISION_ERROR, "division by zero"));
    }
    Ok(crate::fmod(a, b))
}

pub fn try_int_str(s: &str) -> PyResult<i64> {
    let t = s.trim_matches(crate::is_space);
    let body = t.strip_prefix(['-', '+']).unwrap_or(t);
    let ok = !body.is_empty()
        && !body.starts_with('_')
        && !body.ends_with('_')
        && !body.contains("__")
        && body.chars().all(|c| c.is_ascii_digit() || c == '_');
    if !ok {
        return Err(PyErr::new(VALUE_ERROR, format!("invalid literal for int() with base 10: {}", crate::str_repr(s))));
    }
    Ok(crate::int_str(s))
}

pub fn try_float_str(s: &str) -> PyResult<f64> {
    let t = s.trim_matches(crate::is_space);
    let low = t.to_ascii_lowercase();
    let body = low.trim_start_matches(['+', '-']);
    let special = matches!(body, "inf" | "infinity" | "nan");
    let ok = special
        || (!t.is_empty()
            && !t.contains("__")
            && t.chars().all(|c| c.is_ascii_digit() || "+-.eE_".contains(c))
            && t.replace('_', "").parse::<f64>().is_ok());
    if !ok {
        return Err(PyErr::new(VALUE_ERROR, format!("could not convert string to float: {}", crate::str_repr(s))));
    }
    Ok(crate::float_str(s))
}

pub fn try_pop<T>(xs: &mut Vec<T>) -> PyResult<T> {
    xs.pop().ok_or_else(|| PyErr::new(INDEX_ERROR, "pop from empty list"))
}

pub fn try_pop_at<T>(xs: &mut Vec<T>, i: i64) -> PyResult<T> {
    let n = xs.len() as i64;
    if n == 0 {
        return Err(PyErr::new(INDEX_ERROR, "pop from empty list"));
    }
    let k = if i < 0 { i + n } else { i };
    if k < 0 || k >= n {
        return Err(PyErr::new(INDEX_ERROR, "pop index out of range"));
    }
    Ok(xs.remove(k as usize))
}

pub fn try_remove<T: PartialEq>(xs: &mut Vec<T>, x: &T) -> PyResult<()> {
    match xs.iter().position(|e| e == x) {
        Some(i) => {
            xs.remove(i);
            Ok(())
        }
        None => Err(PyErr::new(VALUE_ERROR, "list.remove(x): x not in list")),
    }
}

pub fn try_index<T: PartialEq + crate::Py>(xs: &[T], x: &T) -> PyResult<i64> {
    match xs.iter().position(|e| e == x) {
        Some(i) => Ok(i as i64),
        None => Err(PyErr::new(VALUE_ERROR, format!("{} is not in list", crate::repr(x)))),
    }
}

pub fn try_dict_pop<K, Q, V>(d: &mut crate::IndexMap<K, V>, k: &Q) -> PyResult<V>
where
    K: std::hash::Hash + Eq,
    Q: std::hash::Hash + indexmap::Equivalent<K> + crate::Py + ?Sized,
{
    let r = crate::repr(k);
    d.shift_remove(k).ok_or_else(|| PyErr::new(KEY_ERROR, r))
}

pub fn try_set_remove<T, Q>(s: &mut crate::IndexSet<T>, x: &Q) -> PyResult<()>
where
    T: std::hash::Hash + Eq,
    Q: std::hash::Hash + indexmap::Equivalent<T> + crate::Py + ?Sized,
{
    if s.shift_remove(x) {
        Ok(())
    } else {
        Err(PyErr::new(KEY_ERROR, crate::repr(x)))
    }
}

pub fn try_str_at(s: &str, i: i64) -> PyResult<String> {
    let n = crate::len(s);
    let k = if i < 0 { i + n } else { i };
    if k < 0 || k >= n {
        return Err(PyErr::new(INDEX_ERROR, "string index out of range"));
    }
    Ok(s.chars().nth(k as usize).unwrap().to_string())
}

pub fn try_index_str(s: &str, sub: &str) -> PyResult<i64> {
    match crate::find(s, sub) {
        -1 => Err(PyErr::new(VALUE_ERROR, "substring not found")),
        i => Ok(i),
    }
}

pub fn try_sqrt(x: f64) -> PyResult<f64> {
    if x < 0.0 {
        return Err(PyErr::new(VALUE_ERROR, "math domain error"));
    }
    Ok(x.sqrt())
}

pub fn try_log(x: f64) -> PyResult<f64> {
    if x <= 0.0 {
        return Err(PyErr::new(VALUE_ERROR, "math domain error"));
    }
    Ok(x.ln())
}
