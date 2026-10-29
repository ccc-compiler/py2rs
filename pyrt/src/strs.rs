//! `str` methods with Python's semantics (indices count characters).

use crate::raise;

/// `str.isspace()` for one character
pub fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\x1c'..='\x1f').contains(&c)
}

/// `len(s)`
pub fn len(s: &str) -> i64 {
    s.chars().count() as i64
}

/// `s[i]`
pub fn str_at(s: &str, i: i64) -> String {
    let n = len(s);
    let k = if i < 0 { i + n } else { i };
    if k < 0 || k >= n {
        raise("IndexError", "string index out of range");
    }
    s.chars().nth(k as usize).unwrap().to_string()
}

/// `s[a:b:c]`
pub fn str_slice(s: &str, lo: Option<i64>, hi: Option<i64>, step: Option<i64>) -> String {
    let cs: Vec<char> = s.chars().collect();
    crate::slice_indices(cs.len(), lo, hi, step).into_iter().map(|i| cs[i]).collect()
}

/// `s * n`
pub fn repeat_str(s: &str, n: i64) -> String {
    if n <= 0 {
        String::new()
    } else {
        s.repeat(n as usize)
    }
}

pub fn strip(s: &str) -> String {
    s.trim_matches(is_space).to_string()
}

pub fn lstrip(s: &str) -> String {
    s.trim_start_matches(is_space).to_string()
}

pub fn rstrip(s: &str) -> String {
    s.trim_end_matches(is_space).to_string()
}

pub fn strip_chars(s: &str, chars: &str) -> String {
    s.trim_matches(|c| chars.contains(c)).to_string()
}

pub fn lstrip_chars(s: &str, chars: &str) -> String {
    s.trim_start_matches(|c| chars.contains(c)).to_string()
}

pub fn rstrip_chars(s: &str, chars: &str) -> String {
    s.trim_end_matches(|c| chars.contains(c)).to_string()
}

/// `s.split()`
pub fn split_ws(s: &str) -> Vec<String> {
    s.split(is_space).filter(|w| !w.is_empty()).map(String::from).collect()
}

/// `s.split(sep)`
pub fn split(s: &str, sep: &str) -> Vec<String> {
    if sep.is_empty() {
        raise("ValueError", "empty separator");
    }
    s.split(sep).map(String::from).collect()
}

/// `s.splitlines()`
pub fn splitlines(s: &str) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    let mut it = s.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\r' => {
                if it.peek() == Some(&'\n') {
                    it.next();
                }
                out.push(std::mem::take(&mut cur));
            }
            '\n' | '\x0b' | '\x0c' | '\x1c' | '\x1d' | '\x1e' | '\u{85}' | '\u{2028}' | '\u{2029}' => {
                out.push(std::mem::take(&mut cur))
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn char_index(s: &str, byte: usize) -> i64 {
    s[..byte].chars().count() as i64
}

/// `s.find(sub)`
pub fn find(s: &str, sub: &str) -> i64 {
    match s.find(sub) {
        Some(b) => char_index(s, b),
        None => -1,
    }
}

/// `s.rfind(sub)`
pub fn rfind(s: &str, sub: &str) -> i64 {
    match s.rfind(sub) {
        Some(b) => char_index(s, b),
        None => -1,
    }
}

/// `s.index(sub)`
pub fn index_str(s: &str, sub: &str) -> i64 {
    match s.find(sub) {
        Some(b) => char_index(s, b),
        None => raise("ValueError", "substring not found"),
    }
}

/// `s.rindex(sub)`
pub fn rindex_str(s: &str, sub: &str) -> i64 {
    match s.rfind(sub) {
        Some(b) => char_index(s, b),
        None => raise("ValueError", "substring not found"),
    }
}

/// `s.count(sub)`
pub fn count_str(s: &str, sub: &str) -> i64 {
    if sub.is_empty() {
        return len(s) + 1;
    }
    s.matches(sub).count() as i64
}

fn all_nonempty(s: &str, f: impl Fn(char) -> bool) -> bool {
    !s.is_empty() && s.chars().all(f)
}

/// Digits that are not decimal: superscripts, subscripts, circled (Numeric_Type=Digit).
fn digit_only(c: char) -> bool {
    matches!(c as u32, 0xB2 | 0xB3 | 0xB9 | 0x2070 | 0x2074..=0x2079 | 0x2080..=0x2089 | 0x2460..=0x2468
        | 0x2474..=0x247C | 0x2488..=0x2490 | 0x24EA | 0x24F5..=0x24FD | 0x24FF | 0x2776..=0x277E
        | 0x2780..=0x2788 | 0x278A..=0x2792 | 0x1369..=0x1371 | 0x19DA)
}

/// Numeric characters that are not digits: fractions, roman numerals, larger numbers.
fn numeric_only(c: char) -> bool {
    matches!(c as u32, 0xBC..=0xBE | 0x2150..=0x218B | 0x2469..=0x2473 | 0x247D..=0x2487 | 0x2491..=0x249B
        | 0x24EB..=0x24F4 | 0x24FE | 0x277F | 0x2789 | 0x2793 | 0x2CFD | 0x3007 | 0x3021..=0x3029 | 0x3038..=0x303A
        | 0x3192..=0x3195 | 0x3220..=0x3229 | 0x3248..=0x324F | 0x3251..=0x325F | 0x3280..=0x3289 | 0x32B1..=0x32BF
        | 0x16EE..=0x16F0 | 0x1372..=0x137C | 0x0BF0..=0x0BF2 | 0x09F4..=0x09F9)
}

/// `str.isdecimal()` (Unicode Nd)
pub fn isdecimal(s: &str) -> bool {
    all_nonempty(s, |c| c.is_ascii_digit() || (!c.is_ascii() && c.is_numeric() && !digit_only(c) && !numeric_only(c)))
}

/// `str.isdigit()` (decimal digits and superscripts and the like)
pub fn isdigit(s: &str) -> bool {
    all_nonempty(s, |c| c.is_ascii_digit() || (!c.is_ascii() && c.is_numeric() && !numeric_only(c)))
}

/// `str.isnumeric()`
pub fn isnumeric(s: &str) -> bool {
    all_nonempty(s, |c| c.is_numeric())
}

pub fn isalpha(s: &str) -> bool {
    all_nonempty(s, |c| c.is_alphabetic())
}

pub fn isalnum(s: &str) -> bool {
    all_nonempty(s, |c| c.is_alphanumeric())
}

pub fn isspace(s: &str) -> bool {
    all_nonempty(s, is_space)
}

pub fn isupper(s: &str) -> bool {
    s.chars().any(|c| c.is_uppercase()) && !s.chars().any(|c| c.is_lowercase())
}

pub fn islower(s: &str) -> bool {
    s.chars().any(|c| c.is_lowercase()) && !s.chars().any(|c| c.is_uppercase())
}

pub fn capitalize(s: &str) -> String {
    let mut it = s.chars();
    match it.next() {
        Some(c) => c.to_uppercase().collect::<String>() + &it.as_str().to_lowercase(),
        None => String::new(),
    }
}

pub fn title(s: &str) -> String {
    let mut out = String::new();
    let mut prev_cased = false;
    for c in s.chars() {
        if prev_cased {
            out.extend(c.to_lowercase());
        } else {
            out.extend(c.to_uppercase());
        }
        prev_cased = c.is_uppercase() || c.is_lowercase();
    }
    out
}

pub fn swapcase(s: &str) -> String {
    s.chars()
        .flat_map(|c| -> Box<dyn Iterator<Item = char>> {
            if c.is_uppercase() {
                Box::new(c.to_lowercase())
            } else if c.is_lowercase() {
                Box::new(c.to_uppercase())
            } else {
                Box::new(std::iter::once(c))
            }
        })
        .collect()
}

pub fn zfill(s: &str, width: i64) -> String {
    let n = len(s);
    if width <= n {
        return s.to_string();
    }
    let pad = "0".repeat((width - n) as usize);
    match s.chars().next() {
        Some(c @ ('+' | '-')) => format!("{}{}{}", c, pad, &s[1..]),
        _ => format!("{}{}", pad, s),
    }
}

pub fn center(s: &str, width: i64) -> String {
    let n = len(s);
    if width <= n {
        return s.to_string();
    }
    let marg = width - n;
    let left = marg / 2 + (marg & width & 1);
    format!("{}{}{}", " ".repeat(left as usize), s, " ".repeat((marg - left) as usize))
}

pub fn ljust(s: &str, width: i64) -> String {
    let n = len(s);
    if width <= n {
        return s.to_string();
    }
    format!("{}{}", s, " ".repeat((width - n) as usize))
}

pub fn rjust(s: &str, width: i64) -> String {
    let n = len(s);
    if width <= n {
        return s.to_string();
    }
    format!("{}{}", " ".repeat((width - n) as usize), s)
}

/// `int(s)`
pub fn int_str(s: &str) -> i64 {
    let bad = || -> ! { raise("ValueError", &format!("invalid literal for int() with base 10: {}", crate::str_repr(s))) };
    let t = s.trim_matches(is_space);
    let (neg, digits) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    if digits.is_empty() || digits.starts_with('_') || digits.ends_with('_') || digits.contains("__") {
        bad();
    }
    let mut v: i128 = 0;
    for c in digits.chars() {
        if c == '_' {
            continue;
        }
        let d = c.to_digit(10).unwrap_or_else(|| bad());
        v = v * 10 + d as i128;
        if v > i64::MAX as i128 + 1 {
            panic!("attempt to convert with overflow");
        }
    }
    let v = if neg { -v } else { v };
    i64::try_from(v).unwrap_or_else(|_| panic!("attempt to convert with overflow"))
}

/// `float(s)`
pub fn float_str(s: &str) -> f64 {
    let t = s.trim_matches(is_space);
    let low = t.to_ascii_lowercase();
    let body = low.trim_start_matches(['+', '-']);
    let neg = low.starts_with('-');
    let special = match body {
        "inf" | "infinity" => Some(f64::INFINITY),
        "nan" => Some(f64::NAN),
        _ => None,
    };
    if let Some(v) = special {
        return if neg { -v } else { v };
    }
    let ok = !t.is_empty()
        && !t.contains("__")
        && t.chars().all(|c| c.is_ascii_digit() || "+-.eE_".contains(c));
    match (ok, t.replace('_', "").parse::<f64>()) {
        (true, Ok(v)) => v,
        _ => raise("ValueError", &format!("could not convert string to float: {}", crate::str_repr(s))),
    }
}

/// `ord(c)`
pub fn ord(s: &str) -> i64 {
    let mut it = s.chars();
    match (it.next(), it.next()) {
        (Some(c), None) => c as i64,
        _ => raise("TypeError", &format!("ord() expected a character, but string of length {} found", len(s))),
    }
}

/// `chr(i)`
pub fn chr(i: i64) -> String {
    match u32::try_from(i).ok().and_then(char::from_u32) {
        Some(c) => c.to_string(),
        None => raise("ValueError", "chr() arg not in range(0x110000)"),
    }
}

/// `int(s, base)`
pub fn int_str_base(s: &str, base: i64) -> i64 {
    let bad = || -> ! { raise("ValueError", &format!("invalid literal for int() with base {}: {}", base, crate::str_repr(s))) };
    if !(base == 0 || (2..=36).contains(&base)) {
        raise("ValueError", "int() base must be >= 2 and <= 36, or 0");
    }
    let t = s.trim_matches(is_space).replace('_', "");
    let (neg, body) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, &t[..]),
    };
    let low = body.to_ascii_lowercase();
    let (b, digits) = match (base, low.get(..2)) {
        (16 | 0, Some("0x")) => (16, &low[2..]),
        (8 | 0, Some("0o")) => (8, &low[2..]),
        (2 | 0, Some("0b")) => (2, &low[2..]),
        (0, _) => (10, &low[..]),
        (b, _) => (b, &low[..]),
    };
    if digits.is_empty() {
        bad();
    }
    let mut v: i128 = 0;
    for c in digits.chars() {
        let d = c.to_digit(36).filter(|d| (*d as i64) < b).unwrap_or_else(|| bad()) as i128;
        v = v * b as i128 + d;
        if v > i64::MAX as i128 + 1 {
            panic!("attempt to convert with overflow");
        }
    }
    let v = if neg { -v } else { v };
    i64::try_from(v).unwrap_or_else(|_| panic!("attempt to convert with overflow"))
}

fn radix(n: i64, prefix: &str, f: fn(u64) -> String) -> String {
    let m = f(n.unsigned_abs());
    if n < 0 {
        format!("-{}{}", prefix, m)
    } else {
        format!("{}{}", prefix, m)
    }
}

/// `bin(n)`
pub fn bin(n: i64) -> String {
    radix(n, "0b", |m| format!("{:b}", m))
}

/// `oct(n)`
pub fn oct(n: i64) -> String {
    radix(n, "0o", |m| format!("{:o}", m))
}

/// `hex(n)`
pub fn hex(n: i64) -> String {
    radix(n, "0x", |m| format!("{:x}", m))
}

/// `s.split(sep, maxsplit)`
pub fn split_max(s: &str, sep: &str, max: i64) -> Vec<String> {
    if sep.is_empty() {
        raise("ValueError", "empty separator");
    }
    if max < 0 {
        return split(s, sep);
    }
    s.splitn(max as usize + 1, sep).map(String::from).collect()
}

/// `s.rsplit(sep, maxsplit)`
pub fn rsplit_max(s: &str, sep: &str, max: i64) -> Vec<String> {
    if sep.is_empty() {
        raise("ValueError", "empty separator");
    }
    let mut v: Vec<String> = if max < 0 {
        s.rsplit(sep).map(String::from).collect()
    } else {
        s.rsplitn(max as usize + 1, sep).map(String::from).collect()
    };
    v.reverse();
    v
}

/// `s.split(None, maxsplit)` / `s.split(maxsplit=n)`
pub fn split_ws_max(s: &str, max: i64) -> Vec<String> {
    if max < 0 {
        return split_ws(s);
    }
    let mut out = vec![];
    let mut rest = s.trim_start_matches(is_space);
    while !rest.is_empty() {
        if out.len() as i64 == max {
            out.push(rest.to_string());
            break;
        }
        match rest.find(is_space) {
            Some(i) => {
                out.push(rest[..i].to_string());
                rest = rest[i..].trim_start_matches(is_space);
            }
            None => {
                out.push(rest.to_string());
                break;
            }
        }
    }
    out
}

/// `s.replace(a, b, count)`
pub fn replace_n(s: &str, a: &str, b: &str, count: i64) -> String {
    if count < 0 {
        return s.replace(a, b);
    }
    s.replacen(a, b, count as usize)
}

/// `s.partition(sep)`
pub fn partition(s: &str, sep: &str) -> (String, String, String) {
    if sep.is_empty() {
        raise("ValueError", "empty separator");
    }
    match s.split_once(sep) {
        Some((a, b)) => (a.to_string(), sep.to_string(), b.to_string()),
        None => (s.to_string(), String::new(), String::new()),
    }
}

/// `s.rpartition(sep)`
pub fn rpartition(s: &str, sep: &str) -> (String, String, String) {
    if sep.is_empty() {
        raise("ValueError", "empty separator");
    }
    match s.rsplit_once(sep) {
        Some((a, b)) => (a.to_string(), sep.to_string(), b.to_string()),
        None => (String::new(), String::new(), s.to_string()),
    }
}

pub fn removeprefix(s: &str, p: &str) -> String {
    s.strip_prefix(p).unwrap_or(s).to_string()
}

pub fn removesuffix(s: &str, p: &str) -> String {
    s.strip_suffix(p).unwrap_or(s).to_string()
}

fn one_char(fill: &str) -> char {
    let mut it = fill.chars();
    match (it.next(), it.next()) {
        (Some(c), None) => c,
        _ => raise("TypeError", "The fill character must be exactly one character long"),
    }
}

/// `s.center(w, fill)`
pub fn center_fill(s: &str, width: i64, fill: &str) -> String {
    let c = one_char(fill);
    let n = len(s);
    if width <= n {
        return s.to_string();
    }
    let marg = width - n;
    let left = marg / 2 + (marg & width & 1);
    let f = |k: i64| std::iter::repeat(c).take(k as usize).collect::<String>();
    format!("{}{}{}", f(left), s, f(marg - left))
}

pub fn ljust_fill(s: &str, width: i64, fill: &str) -> String {
    let c = one_char(fill);
    let n = len(s);
    if width <= n {
        return s.to_string();
    }
    format!("{}{}", s, std::iter::repeat(c).take((width - n) as usize).collect::<String>())
}

pub fn rjust_fill(s: &str, width: i64, fill: &str) -> String {
    let c = one_char(fill);
    let n = len(s);
    if width <= n {
        return s.to_string();
    }
    format!("{}{}", std::iter::repeat(c).take((width - n) as usize).collect::<String>(), s)
}

/// `s.find(sub, start)`
pub fn find_from(s: &str, sub: &str, start: i64) -> i64 {
    let n = len(s);
    let st = if start < 0 { (start + n).max(0) } else { start.min(n) } as usize;
    let off: usize = s.char_indices().nth(st).map(|(b, _)| b).unwrap_or(s.len());
    match s[off..].find(sub) {
        Some(b) => st as i64 + s[off..off + b].chars().count() as i64,
        None => -1,
    }
}

/// `s.count(sub, start)`
pub fn count_from(s: &str, sub: &str, start: i64) -> i64 {
    let n = len(s);
    let st = if start < 0 { (start + n).max(0) } else { start.min(n) } as usize;
    let off: usize = s.char_indices().nth(st).map(|(b, _)| b).unwrap_or(s.len());
    count_str(&s[off..], sub)
}

pub fn isascii(s: &str) -> bool {
    s.is_ascii()
}

/// `a, b = xs` for a list: Python's length errors
pub fn unpack_len(n: usize, want: usize) {
    if n < want {
        raise("ValueError", &format!("not enough values to unpack (expected {}, got {})", want, n));
    }
    if n > want {
        raise("ValueError", &format!("too many values to unpack (expected {}, got {})", want, n));
    }
}
