//! The `math` module with Python's domain and range errors.

use crate::raise;

fn domain() -> ! {
    raise("ValueError", "math domain error")
}

fn range_err() -> ! {
    raise("OverflowError", "math range error")
}

fn finite(x: f64, arg: f64) -> f64 {
    if x.is_infinite() && arg.is_finite() {
        range_err();
    }
    x
}

pub fn sqrt(x: f64) -> f64 {
    if x < 0.0 {
        domain();
    }
    x.sqrt()
}

pub fn log(x: f64) -> f64 {
    if x <= 0.0 {
        domain();
    }
    x.ln()
}

/// `math.log(x, base)`
pub fn log_base(x: f64, b: f64) -> f64 {
    if x <= 0.0 || b <= 0.0 {
        domain();
    }
    let d = b.ln();
    if d == 0.0 {
        raise("ZeroDivisionError", "float division by zero");
    }
    x.ln() / d
}

pub fn log2(x: f64) -> f64 {
    if x <= 0.0 {
        domain();
    }
    x.log2()
}

pub fn log10(x: f64) -> f64 {
    if x <= 0.0 {
        domain();
    }
    x.log10()
}

pub fn exp(x: f64) -> f64 {
    finite(x.exp(), x)
}

pub fn asin(x: f64) -> f64 {
    if !(-1.0..=1.0).contains(&x) {
        domain();
    }
    x.asin()
}

pub fn acos(x: f64) -> f64 {
    if !(-1.0..=1.0).contains(&x) {
        domain();
    }
    x.acos()
}

pub fn sinh(x: f64) -> f64 {
    finite(x.sinh(), x)
}

pub fn cosh(x: f64) -> f64 {
    finite(x.cosh(), x)
}

pub fn pow(x: f64, y: f64) -> f64 {
    if x == 0.0 && y < 0.0 {
        domain();
    }
    if x < 0.0 && x.is_finite() && y.is_finite() && y.fract() != 0.0 {
        domain();
    }
    finite(x.powf(y), x + y)
}

pub fn fmod(x: f64, y: f64) -> f64 {
    if y == 0.0 || x.is_infinite() {
        domain();
    }
    x % y
}

fn to_int(x: f64) -> i64 {
    crate::int_float(x)
}

pub fn floor(x: f64) -> i64 {
    to_int(x.floor())
}

pub fn ceil(x: f64) -> i64 {
    to_int(x.ceil())
}

pub fn trunc(x: f64) -> i64 {
    to_int(x.trunc())
}

pub fn gcd(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.unsigned_abs(), b.unsigned_abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    i64::try_from(a).unwrap_or_else(|_| panic!("attempt to negate with overflow"))
}

pub fn lcm(a: i64, b: i64) -> i64 {
    if a == 0 || b == 0 {
        return 0;
    }
    (a / gcd(a, b) * b).abs()
}

pub fn isqrt(n: i64) -> i64 {
    if n < 0 {
        raise("ValueError", "isqrt() argument must be nonnegative");
    }
    let mut x = (n as f64).sqrt() as i64;
    while x * x > n {
        x -= 1;
    }
    while (x + 1) * (x + 1) <= n {
        x += 1;
    }
    x
}

pub fn factorial(n: i64) -> i64 {
    if n < 0 {
        raise("ValueError", "factorial() not defined for negative values");
    }
    (1..=n).product()
}

pub fn comb(n: i64, k: i64) -> i64 {
    if n < 0 || k < 0 {
        raise("ValueError", "n must be a non-negative integer");
    }
    if k > n {
        return 0;
    }
    let k = k.min(n - k);
    let mut r: i128 = 1;
    for i in 0..k {
        r = r * (n - i) as i128 / (i + 1) as i128;
        if r > i64::MAX as i128 {
            panic!("attempt to multiply with overflow");
        }
    }
    r as i64
}

pub fn perm(n: i64, k: i64) -> i64 {
    if n < 0 || k < 0 {
        raise("ValueError", "n must be a non-negative integer");
    }
    if k > n {
        return 0;
    }
    (n - k + 1..=n).product()
}

pub fn radians(x: f64) -> f64 {
    x * (std::f64::consts::PI / 180.0)
}

pub fn degrees(x: f64) -> f64 {
    x * (180.0 / std::f64::consts::PI)
}

/// `math.isclose(a, b)` with the default tolerances
pub fn isclose(a: f64, b: f64) -> bool {
    if a == b {
        return true;
    }
    if a.is_infinite() || b.is_infinite() {
        return false;
    }
    let d = (a - b).abs();
    d <= 1e-09 * b.abs() || d <= 1e-09 * a.abs()
}
