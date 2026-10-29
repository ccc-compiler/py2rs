//! Random well-typed programs of the subset, for differential testing.
//!
//! The programs stress semantics that are easy to get subtly wrong: integer
//! division and modulo signs, float printing, format specs, string slicing by
//! character, list and dict operations, exceptions caught by `try`.

pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E3779B97F4A7C15) | 1)
    }
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn range(&mut self, lo: i64, hi: i64) -> i64 {
        lo + self.below((hi - lo + 1) as u64) as i64
    }
    fn chance(&mut self, pct: u64) -> bool {
        self.below(100) < pct
    }
    fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len() as u64) as usize]
    }
}

#[derive(Clone, Copy, PartialEq)]
enum T {
    Int,
    Float,
    Str,
    Bool,
    List,
    Dict,
}

struct Gen {
    r: Rng,
    /// variables in scope: (name, type)
    vars: Vec<(String, T)>,
    n: u32,
}

const WORDS: &[&str] = &["a", "bc", "Hello", "  pad ", "x-y", "ÄÖü", "日本", "", "42", "-7", "3.5", "abc def"];
const INT_SPECS: &[&str] = &["", "5", "<6", "^7", "+d", "05d", ",", "x", "b", "o", "_"];
const FLOAT_SPECS: &[&str] = &["", ".2f", "10.3f", "+.1f", ".0f", "e", ".3e", "g", ".4g", "%", ".1%", "12", ","];
const STR_SPECS: &[&str] = &["", "8", ">8", "^9", ".2", "*<6"];

impl Gen {
    fn fresh(&mut self, t: T) -> String {
        self.n += 1;
        let n = format!("v{}", self.n);
        self.vars.push((n.clone(), t));
        n
    }

    fn var(&mut self, t: T) -> Option<String> {
        let vs: Vec<&String> = self.vars.iter().filter(|(_, u)| *u == t).map(|(n, _)| n).collect();
        if vs.is_empty() {
            None
        } else {
            Some(vs[self.r.below(vs.len() as u64) as usize].clone())
        }
    }

    fn lit(&mut self, t: T) -> String {
        match t {
            T::Int => self.r.range(-30, 30).to_string(),
            T::Float => {
                let v = self.r.range(-2000, 2000) as f64 / *self.r.pick(&[1.0, 4.0, 10.0, 7.0, 1000.0]);
                let s = format!("{:?}", v);
                if s.contains('.') || s.contains('e') { s } else { s + ".0" }
            }
            T::Str => format!("{:?}", self.r.pick(WORDS)),
            T::Bool => self.r.pick(&["True", "False"]).to_string(),
            T::List => {
                let n = self.r.range(0, 5);
                let xs: Vec<String> = (0..n).map(|_| self.r.range(-9, 9).to_string()).collect();
                format!("[{}]", xs.join(", "))
            }
            T::Dict => {
                let n = self.r.range(0, 3);
                let xs: Vec<String> = (0..n).map(|i| format!("{:?}: {}", WORDS[i as usize + 1], self.r.range(-5, 5))).collect();
                format!("{{{}}}", xs.join(", "))
            }
        }
    }

    fn expr(&mut self, t: T, d: u32) -> String {
        if d == 0 || self.r.chance(25) {
            if self.r.chance(60) {
                if let Some(v) = self.var(t) {
                    return v;
                }
            }
            return self.lit(t);
        }
        let d = d - 1;
        match t {
            T::Int => match self.r.below(16) {
                0 => format!("({} + {})", self.expr(T::Int, d), self.expr(T::Int, d)),
                1 => format!("({} - {})", self.expr(T::Int, d), self.expr(T::Int, d)),
                2 => format!("({} * {})", self.expr(T::Int, d), self.lit(T::Int)),
                3 => format!("({} // {})", self.expr(T::Int, d), self.expr(T::Int, d)),
                4 => format!("({} % {})", self.expr(T::Int, d), self.expr(T::Int, d)),
                5 => format!("abs({})", self.expr(T::Int, d)),
                6 => format!("min({}, {})", self.expr(T::Int, d), self.expr(T::Int, d)),
                7 => format!("len({})", self.expr(T::Str, d)),
                8 => format!("len({})", self.expr(T::List, d)),
                9 => format!("round({})", self.expr(T::Float, d)),
                10 => format!("int({})", self.expr(T::Float, d)),
                11 => format!("({} ** {})", self.expr(T::Int, d), self.r.range(0, 3)),
                12 => format!("sum({})", self.expr(T::List, d)),
                13 => format!("({} & {} | {} ^ {})", self.lit(T::Int), self.expr(T::Int, d), self.lit(T::Int), self.expr(T::Int, d)),
                14 => format!("({} >> {})", self.expr(T::Int, d), self.r.range(0, 4)),
                _ => format!("{}.count({:?})", self.expr(T::Str, d), self.r.pick(&["a", "b", " ", "日"])),
            },
            T::Float => match self.r.below(9) {
                0 => format!("({} + {})", self.expr(T::Float, d), self.expr(T::Float, d)),
                1 => format!("({} * {})", self.expr(T::Float, d), self.expr(T::Float, d)),
                2 => format!("({} / {})", self.expr(T::Int, d), self.expr(T::Int, d)),
                3 => format!("({} / {})", self.expr(T::Float, d), self.expr(T::Float, d)),
                4 => format!("float({})", self.expr(T::Int, d)),
                5 => format!("math.sqrt(abs({}))", self.expr(T::Float, d)),
                6 => format!("round({}, {})", self.expr(T::Float, d), self.r.range(0, 3)),
                7 => format!("({} // {})", self.expr(T::Float, d), self.expr(T::Float, d)),
                _ => format!("({} % {})", self.expr(T::Float, d), self.expr(T::Float, d)),
            },
            T::Str => match self.r.below(13) {
                0 => format!("({} + {})", self.expr(T::Str, d), self.expr(T::Str, d)),
                1 => format!("({} * {})", self.expr(T::Str, d), self.r.range(-1, 3)),
                2 => format!("{}.upper()", self.expr(T::Str, d)),
                3 => format!("{}[{}:{}]", self.expr(T::Str, d), self.r.range(-4, 4), self.r.range(-4, 6)),
                4 => format!("{}[::{}]", self.expr(T::Str, d), self.r.pick(&["-1", "2", "-2"])),
                5 => format!("str({})", self.expr(T::Int, d)),
                6 => format!("str({})", self.expr(T::Float, d)),
                7 => format!("f\"<{{{}:{}}}>\"", self.expr(T::Int, d), self.r.pick(INT_SPECS)),
                8 => format!("f\"[{{{}:{}}}]\"", self.expr(T::Float, d), self.r.pick(FLOAT_SPECS)),
                9 => format!("f\"({{{}:{}}})\"", self.expr(T::Str, d), self.r.pick(STR_SPECS)),
                10 => format!("{}.strip()", self.expr(T::Str, d)),
                11 => format!("\",\".join({}.split())", self.expr(T::Str, d)),
                _ => format!("{}.replace({:?}, {:?})", self.expr(T::Str, d), self.r.pick(&["a", "l", " "]), self.r.pick(&["", "-", "ZZ"])),
            },
            T::Bool => match self.r.below(7) {
                0 => format!("({} < {})", self.expr(T::Int, d), self.expr(T::Int, d)),
                1 => format!("({} == {})", self.expr(T::Str, d), self.expr(T::Str, d)),
                2 => format!("({} and {})", self.expr(T::Bool, d), self.expr(T::Bool, d)),
                3 => format!("(not {})", self.expr(T::Bool, d)),
                4 => format!("({} in {})", self.expr(T::Int, d), self.expr(T::List, d)),
                5 => format!("{}.startswith({:?})", self.expr(T::Str, d), self.r.pick(&["a", "H", ""])),
                _ => format!("({} >= {})", self.expr(T::Float, d), self.expr(T::Float, d)),
            },
            T::List => match self.r.below(6) {
                0 => format!("sorted({})", self.expr(T::List, d)),
                1 => format!("({} + {})", self.expr(T::List, d), self.expr(T::List, d)),
                2 => format!("[x * {} for x in {} if x % 2 == {}]", self.lit(T::Int), self.expr(T::List, d), self.r.range(0, 1)),
                3 => format!("{}[{}:{}]", self.expr(T::List, d), self.r.range(-3, 3), self.r.range(-3, 5)),
                4 => format!("list(range({}, {}, {}))", self.r.range(-5, 5), self.r.range(-5, 9), self.r.pick(&["1", "2", "-1", "3"])),
                _ => format!("sorted({}, reverse=True)", self.expr(T::List, d)),
            },
            T::Dict => self.lit(T::Dict),
        }
    }

    fn stmt(&mut self, out: &mut Vec<String>, ind: &str, d: u32) {
        match self.r.below(9) {
            0..=2 => {
                let t = *self.r.pick(&[T::Int, T::Float, T::Str, T::Bool, T::List]);
                let e = self.expr(t, 3);
                let v = self.fresh(t);
                out.push(format!("{}{} = {}", ind, v, e));
            }
            3 => {
                let t = *self.r.pick(&[T::Int, T::Float, T::Str, T::List, T::Bool]);
                let e = self.expr(t, 3);
                out.push(format!("{}print({})", ind, e));
            }
            4 if d > 0 => {
                let c = self.expr(T::Bool, 2);
                out.push(format!("{}if {}:", ind, c));
                let saved = self.vars.len();
                self.stmt(out, &format!("{}    ", ind), d - 1);
                self.vars.truncate(saved);
                out.push(format!("{}else:", ind));
                self.stmt(out, &format!("{}    ", ind), d - 1);
                self.vars.truncate(saved);
            }
            5 if d > 0 => {
                let l = self.expr(T::List, 2);
                self.n += 1;
                let x = format!("i{}", self.n);
                out.push(format!("{}for {} in {}:", ind, x, l));
                let saved = self.vars.len();
                self.vars.push((x.clone(), T::Int));
                out.push(format!("{}    print({}, {})", ind, x, self.expr(T::Int, 2)));
                self.vars.truncate(saved);
            }
            6 if d > 0 => {
                // a caught exception
                let e = self.expr(T::Int, 2);
                let s = self.expr(T::Str, 1);
                out.push(format!("{}try:", ind));
                out.push(format!("{}    print(100 // {}, int({}))", ind, e, s));
                out.push(format!("{}except ZeroDivisionError as e:", ind));
                out.push(format!("{}    print(\"zero\", e)", ind));
                out.push(format!("{}except ValueError:", ind));
                out.push(format!("{}    print(\"value\")", ind));
            }
            7 => {
                let k = format!("{:?}", self.r.pick(WORDS));
                let d0 = self.lit(T::Dict);
                let v = self.fresh(T::Dict);
                out.push(format!("{}{}: dict[str, int] = {}", ind, v, d0));
                out.push(format!("{}{}[{}] = {}.get({}, 0) + {}", ind, v, k, v, k, self.expr(T::Int, 1)));
                out.push(format!("{}print({}, len({}), {} in {})", ind, v, v, k, v));
            }
            _ => {
                let e = self.expr(T::Str, 3);
                out.push(format!("{}print(repr({}), len({}))", ind, e, e));
            }
        }
    }
}

/// A random program (deterministic for a seed).
pub fn program(seed: u64) -> String {
    let mut g = Gen { r: Rng::new(seed), vars: vec![], n: 0 };
    let mut out = vec!["import math".to_string(), String::new(), String::new(), "def main() -> None:".to_string()];
    for _ in 0..g.r.range(5, 14) {
        g.stmt(&mut out, "    ", 2);
    }
    out.push(String::new());
    out.push(String::new());
    out.push("main()".to_string());
    out.join("\n") + "\n"
}
