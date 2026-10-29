//! The `re` module: patterns are string literals, checked here with the regex
//! crate (the engine the program runs on) and passed on in its syntax.

use super::*;

/// `re.I`, `re.M`, `re.S` (CPython's values).
pub fn re_flag(name: &str) -> Option<u32> {
    Some(match name {
        "I" | "IGNORECASE" => 2,
        "M" | "MULTILINE" => 8,
        "S" | "DOTALL" => 16,
        _ => return None,
    })
}

/// A Python pattern in regex-crate syntax and its number of groups.
pub fn convert(py: &str, flags: u32) -> Result<(String, usize), String> {
    let c: Vec<char> = py.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let mut class = false;
    while i < c.len() {
        let ch = c[i];
        if ch == '\\' && i + 1 < c.len() {
            let n = c[i + 1];
            if n.is_ascii_digit() && n != '0' && !class {
                return Err("backreferences (\\1) are not supported by Rust's regex engine".into());
            }
            if n == 'Z' {
                out.push_str("\\z");
            } else {
                out.push(ch);
                out.push(n);
            }
            i += 2;
            continue;
        }
        if class {
            if ch == ']' {
                class = false;
            }
            // a literal `[` inside a class needs escaping in the regex crate
            if ch == '[' {
                out.push('\\');
            }
            out.push(ch);
            i += 1;
            continue;
        }
        match ch {
            '[' => {
                class = true;
                out.push(ch);
                // `[]...]` and `[^]...]`: a leading `]` is literal
                if c.get(i + 1) == Some(&'^') {
                    out.push('^');
                    i += 1;
                }
                if c.get(i + 1) == Some(&']') {
                    out.push_str("\\]");
                    i += 1;
                }
            }
            '(' if c.get(i + 1) == Some(&'?') => {
                let rest: String = c[i + 2..].iter().take(3).collect();
                if rest.starts_with('=') || rest.starts_with('!') || rest.starts_with("<=") || rest.starts_with("<!") {
                    return Err("lookahead / lookbehind are not supported by Rust's regex engine".into());
                }
                if rest.starts_with("P=") {
                    return Err("backreferences (?P=name) are not supported by Rust's regex engine".into());
                }
                if rest.starts_with('#') {
                    return Err("(?#...) comments are not supported".into());
                }
                out.push(ch);
            }
            '$' if flags & 8 == 0 => {
                return Err("`$` also matches before a final newline in Python; write \\Z, use re.fullmatch, or re.MULTILINE".into());
            }
            _ => out.push(ch),
        }
        i += 1;
    }
    let mut fl = String::new();
    for (bit, f) in [(2, 'i'), (8, 'm'), (16, 's')] {
        if flags & bit != 0 {
            fl.push(f);
        }
    }
    let src = if fl.is_empty() { out } else { format!("(?{}){}", fl, out) };
    match regex::Regex::new(&src) {
        Ok(r) => Ok((src, r.captures_len() - 1)),
        Err(e) => Err(format!("invalid pattern: {}", e.to_string().lines().last().unwrap_or("").trim())),
    }
}

/// A `re.sub` replacement template in regex-crate syntax.
pub fn convert_repl(py: &str) -> Result<String, String> {
    let c: Vec<char> = py.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < c.len() {
        match c[i] {
            '$' => out.push_str("$$"),
            '\\' if i + 1 < c.len() => {
                let n = c[i + 1];
                i += 1;
                match n {
                    d if d.is_ascii_digit() => {
                        let mut num = d.to_string();
                        if c.get(i + 1).is_some_and(|x| x.is_ascii_digit()) {
                            i += 1;
                            num.push(c[i]);
                        }
                        out.push_str(&format!("${{{}}}", num));
                    }
                    'g' if c.get(i + 1) == Some(&'<') => {
                        let end = c[i..].iter().position(|&x| x == '>').ok_or("missing > in \\g<...>")? + i;
                        let name: String = c[i + 2..end].iter().collect();
                        out.push_str(&format!("${{{}}}", name));
                        i = end;
                    }
                    'n' => out.push('\n'),
                    't' => out.push('\t'),
                    'r' => out.push('\r'),
                    '\\' => out.push('\\'),
                    x if x.is_ascii_alphabetic() => return Err(format!("bad escape \\{} in the replacement", x)),
                    x => {
                        out.push('\\');
                        out.push(x);
                    }
                }
            }
            ch => out.push(ch),
        }
        i += 1;
    }
    Ok(out)
}

impl Front {
    /// Literal flags: `re.I | re.M`
    fn re_flags(&self, e: &AExpr, line: u32) -> Res<u32> {
        match e {
            AExpr::BinOp(b) if b.op == ast::Operator::BitOr => Ok(self.re_flags(&b.left, line)? | self.re_flags(&b.right, line)?),
            AExpr::NumberLiteral(n) => match &n.value {
                ast::Number::Int(i) => Ok(i.as_u64().unwrap_or(0) as u32),
                _ => err(line, "flags must be re.I / re.M / re.S"),
            },
            _ => match self.std_name(e).as_deref().and_then(|q| q.strip_prefix("re.")).and_then(re_flag) {
                Some(f) => Ok(f),
                None => err(line, "flags must be literal: re.I, re.M, re.S or a combination with |"),
            },
        }
    }

    /// A literal pattern (and flags) in regex syntax: (source, Python text, groups).
    fn re_pattern(&self, e: &AExpr, flags: Option<&AExpr>, line: u32) -> Res<(String, String, usize)> {
        let AExpr::StringLiteral(s) = e else {
            return err(line, "the pattern must be a string literal (it is checked when translating)");
        };
        let py = s.value.to_str().to_string();
        let fl = match flags {
            Some(f) => self.re_flags(f, line)?,
            None => 0,
        };
        if fl & !(2 | 8 | 16) != 0 {
            return err(line, "only the flags re.I, re.M and re.S are supported");
        }
        let (src, n) = convert(&py, fl).map_err(|m| Error::new(line, m))?;
        Ok((src, py, n))
    }

    fn str_lit(&mut self, line: u32, s: String) -> Expr {
        self.mk(line, Ty::Str, ExprKind::Str(s))
    }

    /// `findall` with n groups: list[str] or list[tuple[str, ...]]
    fn findall_ty(n: usize) -> Ty {
        if n >= 2 {
            Ty::list(Ty::Tuple(vec![Ty::Str; n]))
        } else {
            Ty::list(Ty::Str)
        }
    }

    pub(super) fn re_call(&mut self, f: &str, args: &ast::Arguments, line: u32) -> Res<Expr> {
        use ExprKind as K;
        fn pos<'a>(args: &'a ast::Arguments, name: &str, i: usize, kws: &HashMap<&str, &'a AExpr>) -> Option<&'a AExpr> {
            args.args.get(i).or_else(|| kws.get(name).copied())
        }
        let (t, n, v): (Ty, &'static str, Vec<Expr>) = match f {
            "compile" => {
                let kws = self.kw(args, &["flags"], line)?;
                self.nargs(args, 1, 2, "compile", line)?;
                let (src, py, n) = self.re_pattern(&args.args[0], pos(args, "flags", 1, &kws), line)?;
                (Ty::Pattern(n), "re.compile", vec![self.str_lit(line, src), self.str_lit(line, py)])
            }
            "search" | "match" | "fullmatch" | "findall" | "finditer" | "split" => {
                let kws = self.kw(args, &["flags"], line)?;
                self.nargs(args, 2, 3, f, line)?;
                let (src, _, n) = self.re_pattern(&args.args[0], pos(args, "flags", 2, &kws), line)?;
                let s = self.arg(args, 1, &Ty::Str, "string")?;
                let p = self.str_lit(line, src);
                match f {
                    "findall" if n >= 2 => (Self::findall_ty(n), "re.findall_groups", vec![p, s]),
                    "findall" => (Self::findall_ty(n), "re.findall", vec![p, s]),
                    "finditer" => (Ty::list(Ty::Match(n)), "re.finditer", vec![p, s]),
                    "split" if n > 0 => return err(line, "re.split() with groups in the pattern is not supported"),
                    "split" => (Ty::list(Ty::Str), "re.split", vec![p, s]),
                    "search" => (Ty::opt(Ty::Match(n)), "re.search", vec![p, s]),
                    "match" => (Ty::opt(Ty::Match(n)), "re.match", vec![p, s]),
                    _ => (Ty::opt(Ty::Match(n)), "re.fullmatch", vec![p, s]),
                }
            }
            "sub" => {
                let kws = self.kw(args, &["count", "flags"], line)?;
                self.nargs(args, 3, 5, "sub", line)?;
                let (src, _, _) = self.re_pattern(&args.args[0], pos(args, "flags", 4, &kws), line)?;
                let AExpr::StringLiteral(r) = &args.args[1] else {
                    return err(line, "the replacement must be a string literal (functions are not supported)");
                };
                let repl = convert_repl(r.value.to_str()).map_err(|m| Error::new(line, m))?;
                let s = self.arg(args, 2, &Ty::Str, "string")?;
                let count = match pos(args, "count", 3, &kws) {
                    Some(c) => {
                        let x = self.expr(c, Some(&Ty::Int))?;
                        self.coerce(x, &Ty::Int, "count")?
                    }
                    None => self.mk(line, Ty::Int, K::Int(0)),
                };
                let p = self.str_lit(line, src);
                let rp = self.str_lit(line, repl);
                (Ty::Str, "re.sub", vec![p, rp, s, count])
            }
            "escape" => {
                self.nargs(args, 1, 1, "escape", line)?;
                let s = self.arg(args, 0, &Ty::Str, "string")?;
                (Ty::Str, "re.escape", vec![s])
            }
            _ => return err(line, format!("re.{} is not supported", f)),
        };
        Ok(self.mk(line, t, K::Builtin(n, v)))
    }

    pub(super) fn re_method(&mut self, recv: Expr, m: &str, args: &ast::Arguments, line: u32) -> Res<Expr> {
        use ExprKind as K;
        let rt = self.norm(&recv.ty);
        let b = |s: &mut Self, n: &'static str, t: Ty, mut v: Vec<Expr>, recv: Expr| {
            v.insert(0, recv);
            Ok(s.mk(line, t, K::Builtin(n, v)))
        };
        match rt {
            Ty::Pattern(n) => match m {
                "search" | "match" | "fullmatch" | "findall" | "finditer" | "split" => {
                    self.kw(args, &[], line)?;
                    self.nargs(args, 1, 1, m, line)?;
                    let s = self.arg(args, 0, &Ty::Str, "string")?;
                    match m {
                        "findall" if n >= 2 => b(self, "re.p_findall_groups", Self::findall_ty(n), vec![s], recv),
                        "findall" => b(self, "re.p_findall", Self::findall_ty(n), vec![s], recv),
                        "finditer" => b(self, "re.p_finditer", Ty::list(Ty::Match(n)), vec![s], recv),
                        "split" if n > 0 => err(line, "split() with groups in the pattern is not supported"),
                        "split" => b(self, "re.p_split", Ty::list(Ty::Str), vec![s], recv),
                        "search" => b(self, "re.p_search", Ty::opt(Ty::Match(n)), vec![s], recv),
                        "match" => b(self, "re.p_match", Ty::opt(Ty::Match(n)), vec![s], recv),
                        _ => b(self, "re.p_fullmatch", Ty::opt(Ty::Match(n)), vec![s], recv),
                    }
                }
                "sub" => {
                    let kws = self.kw(args, &["count"], line)?;
                    self.nargs(args, 2, 3, "sub", line)?;
                    let AExpr::StringLiteral(r) = &args.args[0] else {
                        return err(line, "the replacement must be a string literal (functions are not supported)");
                    };
                    let repl = convert_repl(r.value.to_str()).map_err(|m| Error::new(line, m))?;
                    let s = self.arg(args, 1, &Ty::Str, "string")?;
                    let count = match args.args.get(2).or_else(|| kws.get("count").copied()) {
                        Some(c) => {
                            let x = self.expr(c, Some(&Ty::Int))?;
                            self.coerce(x, &Ty::Int, "count")?
                        }
                        None => self.mk(line, Ty::Int, K::Int(0)),
                    };
                    let rp = self.str_lit(line, repl);
                    b(self, "re.p_sub", Ty::Str, vec![rp, s, count], recv)
                }
                _ => err(line, format!("re.Pattern has no method `{}` in the subset", m)),
            },
            Ty::Match(n) => {
                self.kw(args, &[], line)?;
                match m {
                    "group" | "start" | "end" | "span" => {
                        self.nargs(args, 0, 1, m, line)?;
                        let k = match args.args.first() {
                            Some(AExpr::StringLiteral(s)) if m == "group" => {
                                let name = s.value.to_str().to_string();
                                let ne = self.str_lit(line, name);
                                return b(self, "re.m_group_named", Ty::Str, vec![ne], recv);
                            }
                            Some(_) => self.arg(args, 0, &Ty::Int, "group number")?,
                            None => self.mk(line, Ty::Int, K::Int(0)),
                        };
                        if let K::Int(i) = k.kind {
                            if i < 0 || i as usize > n {
                                return err(line, format!("the pattern has {} group(s)", n));
                            }
                        }
                        let t = match m {
                            "group" => Ty::Str,
                            "span" => Ty::Tuple(vec![Ty::Int, Ty::Int]),
                            _ => Ty::Int,
                        };
                        let name: &'static str = match m {
                            "group" => "re.m_group",
                            "start" => "re.m_start",
                            "end" => "re.m_end",
                            _ => "re.m_span",
                        };
                        b(self, name, t, vec![k], recv)
                    }
                    "groups" => {
                        self.nargs(args, 0, 0, m, line)?;
                        if n == 0 || n > 6 {
                            return err(line, "groups() needs 1 to 6 groups in the pattern");
                        }
                        b(self, "re.m_groups", Ty::Tuple(vec![Ty::Str; n]), vec![], recv)
                    }
                    _ => err(line, format!("re.Match has no method `{}` in the subset", m)),
                }
            }
            _ => unreachable!(),
        }
    }
}
