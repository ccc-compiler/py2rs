//! Python's format specification mini-language (`f"{x:>10.2f}"`).

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Spec {
    pub fill: Option<char>,
    pub align: Option<char>,
    pub sign: Option<char>,
    pub alt: bool,
    pub zero: bool,
    pub width: Option<usize>,
    pub grouping: Option<char>,
    pub precision: Option<usize>,
    pub ty: Option<char>,
}

impl Spec {
    pub fn parse(s: &str) -> Option<Spec> {
        let c: Vec<char> = s.chars().collect();
        let mut i = 0;
        let mut sp = Spec::default();
        let is_align = |ch: char| matches!(ch, '<' | '>' | '^' | '=');
        if c.len() >= 2 && is_align(c[1]) {
            sp.fill = Some(c[0]);
            sp.align = Some(c[1]);
            i = 2;
        } else if !c.is_empty() && is_align(c[0]) {
            sp.align = Some(c[0]);
            i = 1;
        }
        if i < c.len() && matches!(c[i], '+' | '-' | ' ') {
            sp.sign = Some(c[i]);
            i += 1;
        }
        if i < c.len() && c[i] == '#' {
            sp.alt = true;
            i += 1;
        }
        if i < c.len() && c[i] == '0' {
            sp.zero = true;
            i += 1;
        }
        let st = i;
        while i < c.len() && c[i].is_ascii_digit() {
            i += 1;
        }
        if i > st {
            sp.width = Some(c[st..i].iter().collect::<String>().parse().ok()?);
        }
        if i < c.len() && matches!(c[i], ',' | '_') {
            sp.grouping = Some(c[i]);
            i += 1;
        }
        if i < c.len() && c[i] == '.' {
            i += 1;
            let st = i;
            while i < c.len() && c[i].is_ascii_digit() {
                i += 1;
            }
            if i == st {
                return None;
            }
            sp.precision = Some(c[st..i].iter().collect::<String>().parse().ok()?);
        }
        if i < c.len() {
            if !"bcdeEfFgGosxX%".contains(c[i]) {
                return None;
            }
            sp.ty = Some(c[i]);
            i += 1;
        }
        if i != c.len() {
            return None;
        }
        Some(sp)
    }
}

fn group(int_part: &str, sep: char, every: usize) -> String {
    let digits: Vec<char> = int_part.chars().collect();
    let mut out = String::new();
    for (i, d) in digits.iter().enumerate() {
        if i > 0 && (digits.len() - i) % every == 0 {
            out.push(sep);
        }
        out.push(*d);
    }
    out
}

/// Pad `body` (without sign) to the width.
fn pad(sign: &str, body: &str, sp: &Spec, numeric: bool) -> String {
    let len = sign.chars().count() + body.chars().count();
    let width = sp.width.unwrap_or(0);
    let (fill, align) = if sp.zero && sp.align.is_none() && numeric {
        ('0', '=')
    } else {
        (sp.fill.unwrap_or(' '), sp.align.unwrap_or(if numeric { '>' } else { '<' }))
    };
    if len >= width {
        return format!("{}{}", sign, body);
    }
    let n = width - len;
    let f = |k: usize| std::iter::repeat(fill).take(k).collect::<String>();
    match align {
        '<' => format!("{}{}{}", sign, body, f(n)),
        '>' => format!("{}{}{}", f(n), sign, body),
        '^' => format!("{}{}{}{}", f(n / 2), sign, body, f(n - n / 2)),
        _ => format!("{}{}{}", sign, f(n), body),
    }
}

fn sign_str(neg: bool, sp: &Spec) -> &'static str {
    if neg {
        "-"
    } else {
        match sp.sign {
            Some('+') => "+",
            Some(' ') => " ",
            _ => "",
        }
    }
}

pub fn format_int(x: i64, sp: &Spec) -> String {
    match sp.ty {
        Some('e' | 'E' | 'f' | 'F' | 'g' | 'G' | '%') => return format_float(x as f64, sp),
        Some('c') => {
            let ch = char::from_u32(x as u32).map(|c| c.to_string()).unwrap_or_default();
            return pad("", &ch, sp, false);
        }
        _ => {}
    }
    let neg = x < 0;
    let mag = x.unsigned_abs();
    let (digits, prefix, every) = match sp.ty {
        Some('x') => (format!("{:x}", mag), "0x", 4),
        Some('X') => (format!("{:X}", mag), "0X", 4),
        Some('b') => (format!("{:b}", mag), "0b", 4),
        Some('o') => (format!("{:o}", mag), "0o", 4),
        _ => (mag.to_string(), "", 3),
    };
    let body = match sp.grouping {
        Some(g) => group(&digits, g, every),
        None => digits,
    };
    // `#06x`: zero padding goes between `0x` and the digits
    let sign = if sp.alt && !prefix.is_empty() {
        format!("{}{}", sign_str(neg, sp), prefix)
    } else {
        sign_str(neg, sp).to_string()
    };
    pad(&sign, &body, sp, true)
}

/// Digits and exponent of `x` rounded to `prec` significant digits.
fn sci_parts(x: f64, prec: usize) -> (String, i32) {
    let s = format!("{:.*e}", prec.saturating_sub(1), x);
    let (m, e) = s.split_once('e').unwrap();
    (m.replace('.', ""), e.parse().unwrap())
}

fn exp_str(e: i32) -> String {
    format!("{}{:02}", if e < 0 { '-' } else { '+' }, e.abs())
}

pub fn format_float(x: f64, sp: &Spec) -> String {
    let neg = x.is_sign_negative() && !(x.is_nan());
    let a = x.abs();
    let upper = matches!(sp.ty, Some('E' | 'F' | 'G'));
    let body = if a.is_nan() || a.is_infinite() {
        let s = if a.is_nan() { "nan" } else { "inf" };
        let s = if upper { s.to_uppercase() } else { s.to_string() };
        if sp.ty == Some('%') {
            s + "%"
        } else {
            s
        }
    } else {
        match sp.ty {
            Some('f' | 'F') => fixed(a, sp.precision.unwrap_or(6), sp),
            Some('%') => fixed(a * 100.0, sp.precision.unwrap_or(6), sp) + "%",
            Some('e' | 'E') => {
                let p = sp.precision.unwrap_or(6);
                let s = format!("{:.*e}", p, a);
                let (m, e) = s.split_once('e').unwrap();
                let mut m = m.to_string();
                if sp.alt && p == 0 {
                    m.push('.');
                }
                let r = format!("{}e{}", m, exp_str(e.parse().unwrap()));
                if upper {
                    r.to_uppercase()
                } else {
                    r
                }
            }
            Some('g' | 'G') => {
                let r = general(a, sp.precision.unwrap_or(6).max(1), sp.alt, sp, false);
                if upper {
                    r.to_uppercase()
                } else {
                    r
                }
            }
            None => match sp.precision {
                Some(p) => general(a, p.max(1), sp.alt, sp, true),
                None => {
                    let r = crate::float_repr(a);
                    match sp.grouping {
                        Some(g) => match r.split_once('.') {
                            Some((i, f)) if !r.contains('e') => format!("{}.{}", group(i, g, 3), f),
                            _ => r,
                        },
                        None => r,
                    }
                }
            },
            _ => crate::float_repr(a),
        }
    };
    pad(sign_str(neg, sp), &body, sp, true)
}

fn fixed(a: f64, p: usize, sp: &Spec) -> String {
    let s = format!("{:.*}", p, a);
    let s = if sp.alt && p == 0 { s + "." } else { s };
    match sp.grouping {
        Some(g) => match s.split_once('.') {
            Some((i, f)) => format!("{}.{}", group(i, g, 3), f),
            None => group(&s, g, 3),
        },
        None => s,
    }
}

/// 'g' formatting; `no_type` is the format with a precision but no type.
fn general(a: f64, p: usize, alt: bool, sp: &Spec, no_type: bool) -> String {
    if a == 0.0 {
        return if alt {
            format!("0.{}", "0".repeat(p - 1))
        } else if no_type {
            "0.0".into()
        } else {
            "0".into()
        };
    }
    let (digits, e) = sci_parts(a, p);
    let mut out;
    if -4 <= e && (e as i64) < p as i64 {
        let s = fixed(a, (p as i64 - 1 - e as i64).max(0) as usize, &Spec { grouping: sp.grouping, ..Spec::default() });
        out = s;
        if !alt && out.contains('.') {
            out = out.trim_end_matches('0').trim_end_matches('.').to_string();
        }
        if no_type && !out.contains('.') {
            out += ".0";
        }
    } else {
        let mut m = digits[..1].to_string();
        let rest = &digits[1..];
        let rest = if alt { rest.to_string() } else { rest.trim_end_matches('0').to_string() };
        if !rest.is_empty() || alt {
            m.push('.');
            m += &rest;
        }
        out = format!("{}e{}", m, exp_str(e));
    }
    out
}

pub fn format_str(s: &str, sp: &Spec) -> String {
    let s: String = match sp.precision {
        Some(p) => s.chars().take(p).collect(),
        None => s.to_string(),
    };
    pad("", &s, sp, false)
}
