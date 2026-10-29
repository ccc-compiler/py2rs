//! Python standard library modules, with CPython's exact algorithms where the
//! result is visible (heap layout, random numbers, Counter order).

use std::cmp::Ordering;
use std::collections::VecDeque;
use std::hash::Hash;
use std::ops::{Deref, DerefMut};

use crate::{raise, IndexMap, Py, PyOrd};

// ------------------------------------------------------------- collections

/// `collections.defaultdict`: a missing key reads as (and becomes) the default.
#[derive(Clone, Debug, PartialEq)]
pub struct DefaultDict<K: Hash + Eq, V> {
    pub map: IndexMap<K, V>,
    /// `int`, `list`, ... (for repr)
    pub factory: &'static str,
}

impl<K: Hash + Eq, V: Default> DefaultDict<K, V> {
    pub fn new(factory: &'static str) -> Self {
        DefaultDict { map: IndexMap::new(), factory }
    }
    /// `d[k]`: inserts the default when missing
    pub fn at(&mut self, k: K) -> &mut V {
        self.map.entry(k).or_default()
    }
    /// `d[k]` of a Copy value
    pub fn val(&mut self, k: K) -> V
    where
        V: Copy,
    {
        *self.at(k)
    }
}

impl<K: Hash + Eq, V> Deref for DefaultDict<K, V> {
    type Target = IndexMap<K, V>;
    fn deref(&self) -> &IndexMap<K, V> {
        &self.map
    }
}

impl<K: Hash + Eq, V> DerefMut for DefaultDict<K, V> {
    fn deref_mut(&mut self) -> &mut IndexMap<K, V> {
        &mut self.map
    }
}

impl<K: Hash + Eq + Py, V: Py> Py for DefaultDict<K, V> {
    fn fmt_repr(&self, out: &mut String) {
        out.push_str(&format!("defaultdict(<class '{}'>, ", self.factory));
        self.map.fmt_repr(out);
        out.push(')');
    }
}

/// `collections.Counter`
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Counter<K: Hash + Eq> {
    pub map: IndexMap<K, i64>,
}

impl<K: Hash + Eq + Clone> Counter<K> {
    pub fn new() -> Self {
        Counter { map: IndexMap::new() }
    }
    /// `Counter(iterable)`
    pub fn of<I: IntoIterator<Item = K>>(it: I) -> Self {
        let mut c = Counter::new();
        c.update(it);
        c
    }
    /// `c.update(iterable)`
    pub fn update<I: IntoIterator<Item = K>>(&mut self, it: I) {
        for k in it {
            *self.map.entry(k).or_insert(0) += 1;
        }
    }
    /// `c[k]`: 0 when missing (not inserted)
    pub fn get_count<Q: Hash + indexmap::Equivalent<K> + ?Sized>(&self, k: &Q) -> i64 {
        self.map.get(k).copied().unwrap_or(0)
    }
    /// `c[k] += n`
    pub fn at(&mut self, k: K) -> &mut i64 {
        self.map.entry(k).or_insert(0)
    }
    /// `c.most_common(n)`: by count, ties in insertion order
    pub fn most_common(&self, n: Option<i64>) -> Vec<(K, i64)> {
        let mut v: Vec<(K, i64)> = self.map.iter().map(|(k, c)| (k.clone(), *c)).collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        if let Some(n) = n {
            v.truncate(n.max(0) as usize);
        }
        v
    }
    /// `c.total()`
    pub fn total(&self) -> i64 {
        self.map.values().sum()
    }
    /// `list(c.elements())`
    pub fn elements(&self) -> Vec<K> {
        let mut v = vec![];
        for (k, c) in &self.map {
            for _ in 0..(*c).max(0) {
                v.push(k.clone());
            }
        }
        v
    }
}

impl<K: Hash + Eq> Deref for Counter<K> {
    type Target = IndexMap<K, i64>;
    fn deref(&self) -> &IndexMap<K, i64> {
        &self.map
    }
}

impl<K: Hash + Eq> DerefMut for Counter<K> {
    fn deref_mut(&mut self) -> &mut IndexMap<K, i64> {
        &mut self.map
    }
}

impl<K: Hash + Eq + Py + Clone> Py for Counter<K> {
    fn fmt_repr(&self, out: &mut String) {
        if self.map.is_empty() {
            out.push_str("Counter()");
            return;
        }
        out.push_str("Counter({");
        for (i, (k, c)) in self.most_common(None).iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            k.fmt_repr(out);
            out.push_str(": ");
            c.fmt_repr(out);
        }
        out.push_str("})");
    }
}

impl<T: Py> Py for VecDeque<T> {
    fn fmt_repr(&self, out: &mut String) {
        out.push_str("deque([");
        for (i, x) in self.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            x.fmt_repr(out);
        }
        out.push_str("])");
    }
}

pub fn popleft<T>(d: &mut VecDeque<T>) -> T {
    d.pop_front().unwrap_or_else(|| raise("IndexError", "pop from an empty deque"))
}

pub fn pop_deque<T>(d: &mut VecDeque<T>) -> T {
    d.pop_back().unwrap_or_else(|| raise("IndexError", "pop from an empty deque"))
}

// ------------------------------------------------------------------ heapq

fn lt<T: PyOrd>(a: &T, b: &T) -> bool {
    a.py_cmp(b) == Ordering::Less
}

/// CPython's `_siftdown`
fn siftdown<T: PyOrd>(h: &mut [T], start: usize, mut pos: usize) {
    while pos > start {
        let parent = (pos - 1) >> 1;
        if lt(&h[pos], &h[parent]) {
            h.swap(pos, parent);
            pos = parent;
        } else {
            break;
        }
    }
}

/// CPython's `_siftup`
fn siftup<T: PyOrd>(h: &mut [T], mut pos: usize) {
    let end = h.len();
    let start = pos;
    let mut child = 2 * pos + 1;
    while child < end {
        let right = child + 1;
        if right < end && !lt(&h[child], &h[right]) {
            child = right;
        }
        h.swap(pos, child);
        pos = child;
        child = 2 * pos + 1;
    }
    siftdown(h, start, pos);
}

pub fn heappush<T: PyOrd>(h: &mut Vec<T>, x: T) {
    h.push(x);
    let n = h.len() - 1;
    siftdown(h, 0, n);
}

pub fn heappop<T: PyOrd>(h: &mut Vec<T>) -> T {
    let last = h.pop().unwrap_or_else(|| raise("IndexError", "index out of range"));
    if h.is_empty() {
        return last;
    }
    let top = std::mem::replace(&mut h[0], last);
    siftup(h, 0);
    top
}

pub fn heapify<T: PyOrd>(h: &mut [T]) {
    let n = h.len();
    for i in (0..n / 2).rev() {
        siftup(h, i);
    }
}

pub fn heappushpop<T: PyOrd>(h: &mut [T], x: T) -> T {
    if !h.is_empty() && lt(&h[0], &x) {
        let top = std::mem::replace(&mut h[0], x);
        siftup(h, 0);
        return top;
    }
    x
}

pub fn heapreplace<T: PyOrd>(h: &mut [T], x: T) -> T {
    if h.is_empty() {
        raise("IndexError", "index out of range");
    }
    let top = std::mem::replace(&mut h[0], x);
    siftup(h, 0);
    top
}

/// `heapq.nlargest(n, it)` (stable: equal items keep their order)
pub fn nlargest<T: PyOrd + Clone>(n: i64, it: impl IntoIterator<Item = T>) -> Vec<T> {
    let mut v: Vec<T> = it.into_iter().collect();
    v.sort_by(|a, b| b.py_cmp(a));
    v.truncate(n.max(0) as usize);
    v
}

/// `heapq.nsmallest(n, it)`
pub fn nsmallest<T: PyOrd + Clone>(n: i64, it: impl IntoIterator<Item = T>) -> Vec<T> {
    let mut v: Vec<T> = it.into_iter().collect();
    v.sort_by(|a, b| a.py_cmp(b));
    v.truncate(n.max(0) as usize);
    v
}

// ----------------------------------------------------------------- bisect

pub fn bisect_left<T: PyOrd>(a: &[T], x: &T) -> i64 {
    let (mut lo, mut hi) = (0, a.len());
    while lo < hi {
        let mid = (lo + hi) / 2;
        if lt(&a[mid], x) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo as i64
}

pub fn bisect_right<T: PyOrd>(a: &[T], x: &T) -> i64 {
    let (mut lo, mut hi) = (0, a.len());
    while lo < hi {
        let mid = (lo + hi) / 2;
        if lt(x, &a[mid]) {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    lo as i64
}

pub fn insort_left<T: PyOrd>(a: &mut Vec<T>, x: T) {
    let i = bisect_left(a, &x) as usize;
    a.insert(i, x);
}

pub fn insort_right<T: PyOrd>(a: &mut Vec<T>, x: T) {
    let i = bisect_right(a, &x) as usize;
    a.insert(i, x);
}

// -------------------------------------------------------------- itertools

/// `itertools.permutations(xs, r)` in CPython's order
pub fn permutations<T: Clone>(xs: &[T], r: i64) -> Vec<Vec<T>> {
    let (n, r) = (xs.len(), r as usize);
    if r > n {
        return vec![];
    }
    let mut idx: Vec<usize> = (0..n).collect();
    let mut cycles: Vec<usize> = (n - r + 1..=n).rev().collect();
    let mut out = vec![idx[..r].iter().map(|&i| xs[i].clone()).collect()];
    if n == 0 {
        return out;
    }
    loop {
        let mut done = true;
        for i in (0..r).rev() {
            cycles[i] -= 1;
            if cycles[i] == 0 {
                let x = idx.remove(i);
                idx.push(x);
                cycles[i] = n - i;
            } else {
                let j = cycles[i];
                idx.swap(i, n - j);
                out.push(idx[..r].iter().map(|&k| xs[k].clone()).collect());
                done = false;
                break;
            }
        }
        if done {
            return out;
        }
    }
}

/// `itertools.combinations(xs, r)`
pub fn combinations<T: Clone>(xs: &[T], r: i64) -> Vec<Vec<T>> {
    let (n, r) = (xs.len(), r as usize);
    if r > n {
        return vec![];
    }
    let mut idx: Vec<usize> = (0..r).collect();
    let mut out = vec![idx.iter().map(|&i| xs[i].clone()).collect()];
    loop {
        let mut i = r;
        let mut found = false;
        while i > 0 {
            i -= 1;
            if idx[i] != i + n - r {
                found = true;
                break;
            }
        }
        if !found {
            return out;
        }
        idx[i] += 1;
        for j in i + 1..r {
            idx[j] = idx[j - 1] + 1;
        }
        out.push(idx.iter().map(|&k| xs[k].clone()).collect());
    }
}

/// `itertools.product(xs, repeat=r)`
pub fn product_repeat<T: Clone>(xs: &[T], r: i64) -> Vec<Vec<T>> {
    let mut out: Vec<Vec<T>> = vec![vec![]];
    for _ in 0..r {
        let mut next = vec![];
        for p in &out {
            for x in xs {
                let mut q = p.clone();
                q.push(x.clone());
                next.push(q);
            }
        }
        out = next;
    }
    out
}

/// `itertools.accumulate(xs)` for numbers
pub fn accumulate<T: Copy + std::ops::Add<Output = T>>(xs: impl IntoIterator<Item = T>) -> Vec<T> {
    let mut out: Vec<T> = vec![];
    for x in xs {
        let v = match out.last() {
            Some(&l) => l + x,
            None => x,
        };
        out.push(v);
    }
    out
}

// ----------------------------------------------------------------- random

/// CPython's Mersenne Twister (`random` module): same seed, same numbers.
pub struct Random {
    mt: [u32; 624],
    index: usize,
}

impl Random {
    fn init_genrand(s: u32) -> Random {
        let mut mt = [0u32; 624];
        mt[0] = s;
        for i in 1..624 {
            mt[i] = 1812433253u32.wrapping_mul(mt[i - 1] ^ (mt[i - 1] >> 30)).wrapping_add(i as u32);
        }
        Random { mt, index: 624 }
    }

    /// `random.seed(n)` for an int
    pub fn seeded(n: i64) -> Random {
        // init_by_array with the 32-bit words of |n|
        let mut a = n.unsigned_abs();
        let mut key = vec![];
        while a > 0 {
            key.push(a as u32);
            a >>= 32;
        }
        if key.is_empty() {
            key.push(0);
        }
        let mut r = Random::init_genrand(19650218);
        let (mut i, mut j) = (1usize, 0usize);
        let n = 624;
        let k = n.max(key.len());
        for _ in 0..k {
            r.mt[i] = (r.mt[i] ^ (r.mt[i - 1] ^ (r.mt[i - 1] >> 30)).wrapping_mul(1664525)).wrapping_add(key[j]).wrapping_add(j as u32);
            i += 1;
            j += 1;
            if i >= n {
                r.mt[0] = r.mt[n - 1];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
        }
        for _ in 0..n - 1 {
            r.mt[i] = (r.mt[i] ^ (r.mt[i - 1] ^ (r.mt[i - 1] >> 30)).wrapping_mul(1566083941)).wrapping_sub(i as u32);
            i += 1;
            if i >= n {
                r.mt[0] = r.mt[n - 1];
                i = 1;
            }
        }
        r.mt[0] = 0x80000000;
        r
    }

    fn genrand(&mut self) -> u32 {
        if self.index >= 624 {
            for i in 0..624 {
                let y = (self.mt[i] & 0x80000000) | (self.mt[(i + 1) % 624] & 0x7fffffff);
                let mut v = self.mt[(i + 397) % 624] ^ (y >> 1);
                if y & 1 != 0 {
                    v ^= 0x9908b0df;
                }
                self.mt[i] = v;
            }
            self.index = 0;
        }
        let mut y = self.mt[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c5680;
        y ^= (y << 15) & 0xefc60000;
        y ^ (y >> 18)
    }

    /// `random.random()`
    pub fn random(&mut self) -> f64 {
        let a = (self.genrand() >> 5) as f64;
        let b = (self.genrand() >> 6) as f64;
        (a * 67108864.0 + b) * (1.0 / 9007199254740992.0)
    }

    /// `getrandbits(k)`, k ≤ 64
    fn getrandbits(&mut self, k: u32) -> u64 {
        if k <= 32 {
            return (self.genrand() >> (32 - k)) as u64;
        }
        let lo = self.genrand() as u64;
        let hi = (self.genrand() >> (64 - k)) as u64;
        lo | (hi << 32)
    }

    /// `_randbelow(n)`
    fn randbelow(&mut self, n: u64) -> u64 {
        if n == 0 {
            return 0;
        }
        let k = 64 - n.leading_zeros();
        let mut r = self.getrandbits(k);
        while r >= n {
            r = self.getrandbits(k);
        }
        r
    }

    /// `random.randrange(start, stop, step)`
    pub fn randrange(&mut self, start: i64, stop: i64, step: i64) -> i64 {
        if step == 1 {
            if start >= stop {
                raise("ValueError", &format!("empty range in randrange({}, {})", start, stop));
            }
            return start + self.randbelow((stop - start) as u64) as i64;
        }
        if step == 0 {
            raise("ValueError", "zero step for randrange()");
        }
        let n = if step > 0 { (stop - start + step - 1) / step } else { (stop - start + step + 1) / step };
        if n <= 0 {
            raise("ValueError", "empty range in randrange()");
        }
        start + step * self.randbelow(n as u64) as i64
    }

    /// `random.randint(a, b)`
    pub fn randint(&mut self, a: i64, b: i64) -> i64 {
        self.randrange(a, b + 1, 1)
    }

    /// `random.uniform(a, b)`
    pub fn uniform(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.random()
    }

    /// `random.choice(seq)`
    pub fn choice<T: Clone>(&mut self, xs: &[T]) -> T {
        if xs.is_empty() {
            raise("IndexError", "Cannot choose from an empty sequence");
        }
        xs[self.randbelow(xs.len() as u64) as usize].clone()
    }

    /// `random.shuffle(xs)`
    pub fn shuffle<T>(&mut self, xs: &mut [T]) {
        for i in (1..xs.len()).rev() {
            let j = self.randbelow(i as u64 + 1) as usize;
            xs.swap(i, j);
        }
    }

    /// `random.sample(xs, k)` (CPython's set-or-pool selection)
    pub fn sample<T: Clone>(&mut self, xs: &[T], k: i64) -> Vec<T> {
        let n = xs.len();
        if k < 0 || k as usize > n {
            raise("ValueError", "Sample larger than population or is negative");
        }
        let k = k as usize;
        let mut setsize = 21usize;
        if k > 5 {
            let mut p = 1usize;
            let mut e = 0;
            while p < k * 3 {
                p *= 4;
                e += 1;
            }
            setsize += 4usize.pow(e);
        }
        let mut out = Vec::with_capacity(k);
        if n <= setsize {
            let mut pool: Vec<T> = xs.to_vec();
            for i in 0..k {
                let j = self.randbelow((n - i) as u64) as usize;
                out.push(pool[j].clone());
                pool[j] = pool[n - i - 1].clone();
            }
        } else {
            let mut selected = std::collections::HashSet::new();
            for _ in 0..k {
                let mut j = self.randbelow(n as u64) as usize;
                while selected.contains(&j) {
                    j = self.randbelow(n as u64) as usize;
                }
                selected.insert(j);
                out.push(xs[j].clone());
            }
        }
        out
    }
}

thread_local! {
    static RNG: std::cell::RefCell<Random> = std::cell::RefCell::new(Random::seeded(
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as i64).unwrap_or(0)
    ));
}

/// The module-level generator of `random`.
pub fn rng<R>(f: impl FnOnce(&mut Random) -> R) -> R {
    RNG.with(|r| f(&mut r.borrow_mut()))
}

/// `random.seed(n)`
pub fn seed(n: i64) {
    RNG.with(|r| *r.borrow_mut() = Random::seeded(n));
}

// ------------------------------------------------------------- sys / time

/// `sys.argv`
pub fn argv() -> Vec<String> {
    let mut v: Vec<String> = std::env::args().collect();
    if v.is_empty() {
        v.push(String::new());
    }
    v
}

/// `sys.exit(code)`
pub fn exit(code: i64) -> ! {
    use std::io::Write;
    let _ = std::io::stdout().flush();
    std::process::exit(code as i32)
}

/// `sys.stdin.readline()` (with the newline; "" at the end)
pub fn readline() -> String {
    use std::io::{BufRead, Write};
    let _ = std::io::stdout().flush();
    let mut s = String::new();
    let _ = std::io::stdin().lock().read_line(&mut s);
    s
}

/// `sys.stdin.read()`
pub fn read_all() -> String {
    use std::io::Read;
    let mut s = String::new();
    let _ = std::io::stdin().read_to_string(&mut s);
    s
}

/// `for line in sys.stdin` / `sys.stdin.readlines()`
pub fn lines() -> Vec<String> {
    let all = read_all();
    let mut out = vec![];
    let mut cur = String::new();
    for c in all.chars() {
        cur.push(c);
        if c == '\n' {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Runs the program on a thread with a 1 GB stack (`sys.setrecursionlimit`).
pub fn with_stack(f: fn()) {
    let t = std::thread::Builder::new().stack_size(1 << 30).spawn(f).expect("thread");
    if t.join().is_err() {
        std::process::exit(1);
    }
}

/// `time.time()`
pub fn time() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

thread_local! {
    static START: std::time::Instant = std::time::Instant::now();
}

/// `time.perf_counter()`
pub fn perf_counter() -> f64 {
    START.with(|s| s.elapsed().as_secs_f64())
}

/// `time.sleep(s)`
pub fn sleep(s: f64) {
    std::thread::sleep(std::time::Duration::from_secs_f64(s.max(0.0)));
}

// ------------------------------------------------------------------- json

/// `json.dumps(x)`: values of the subset.
pub trait Json {
    fn json(&self, out: &mut String);
}

pub fn dumps<T: Json + ?Sized>(x: &T) -> String {
    let mut s = String::new();
    x.json(&mut s);
    s
}

impl Json for i64 {
    fn json(&self, out: &mut String) {
        out.push_str(&self.to_string())
    }
}

impl Json for f64 {
    fn json(&self, out: &mut String) {
        if self.is_nan() {
            out.push_str("NaN")
        } else if self.is_infinite() {
            out.push_str(if *self > 0.0 { "Infinity" } else { "-Infinity" })
        } else {
            out.push_str(&crate::float_repr(*self))
        }
    }
}

impl Json for bool {
    fn json(&self, out: &mut String) {
        out.push_str(if *self { "true" } else { "false" })
    }
}

impl Json for str {
    fn json(&self, out: &mut String) {
        out.push('"');
        for c in self.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\r' => out.push_str("\\r"),
                '\t' => out.push_str("\\t"),
                '\u{8}' => out.push_str("\\b"),
                '\u{c}' => out.push_str("\\f"),
                c if (c as u32) < 0x20 || (c as u32) > 0x7e => {
                    let mut buf = [0u16; 2];
                    for u in c.encode_utf16(&mut buf) {
                        out.push_str(&format!("\\u{:04x}", u));
                    }
                }
                c => out.push(c),
            }
        }
        out.push('"');
    }
}

impl Json for String {
    fn json(&self, out: &mut String) {
        self.as_str().json(out)
    }
}

impl<T: Json> Json for Option<T> {
    fn json(&self, out: &mut String) {
        match self {
            Some(x) => x.json(out),
            None => out.push_str("null"),
        }
    }
}

impl<T: Json> Json for Vec<T> {
    fn json(&self, out: &mut String) {
        out.push('[');
        for (i, x) in self.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            x.json(out);
        }
        out.push(']');
    }
}

/// dict keys: JSON object keys are strings
pub trait JsonKey {
    fn key(&self) -> String;
}

impl JsonKey for String {
    fn key(&self) -> String {
        self.clone()
    }
}

impl JsonKey for i64 {
    fn key(&self) -> String {
        self.to_string()
    }
}

impl JsonKey for bool {
    fn key(&self) -> String {
        (if *self { "true" } else { "false" }).into()
    }
}

impl<K: JsonKey + Hash + Eq, V: Json> Json for IndexMap<K, V> {
    fn json(&self, out: &mut String) {
        out.push('{');
        for (i, (k, v)) in self.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            k.key().json(out);
            out.push_str(": ");
            v.json(out);
        }
        out.push('}');
    }
}

macro_rules! tuple_json {
    ($($n:tt $t:ident),+) => {
        impl<$($t: Json),+> Json for ($($t,)+) {
            fn json(&self, out: &mut String) {
                out.push('[');
                $(
                    if $n > 0 { out.push_str(", "); }
                    self.$n.json(out);
                )+
                out.push(']');
            }
        }
    };
}
tuple_json!(0 A);
tuple_json!(0 A, 1 B);
tuple_json!(0 A, 1 B, 2 C);
tuple_json!(0 A, 1 B, 2 C, 3 D);

// ------------------------------------------------- module-level functions

/// `random.random()`
pub fn random() -> f64 {
    rng(|r| r.random())
}

pub fn randint(a: i64, b: i64) -> i64 {
    rng(|r| r.randint(a, b))
}

pub fn randrange(a: i64, b: i64, step: i64) -> i64 {
    rng(|r| r.randrange(a, b, step))
}

pub fn uniform(a: f64, b: f64) -> f64 {
    rng(|r| r.uniform(a, b))
}

pub fn choice<T: Clone>(xs: &[T]) -> T {
    rng(|r| r.choice(xs))
}

pub fn shuffle<T>(xs: &mut [T]) {
    rng(|r| r.shuffle(xs))
}

pub fn sample<T: Clone>(xs: &[T], k: i64) -> Vec<T> {
    rng(|r| r.sample(xs, k))
}

/// `itertools.product(a, b)`
pub fn product2<A: Clone, B: Clone>(a: &[A], b: &[B]) -> Vec<(A, B)> {
    let mut out = vec![];
    for x in a {
        for y in b {
            out.push((x.clone(), y.clone()));
        }
    }
    out
}

/// `itertools.product(a, b, c)`
pub fn product3<A: Clone, B: Clone, C: Clone>(a: &[A], b: &[B], c: &[C]) -> Vec<(A, B, C)> {
    let mut out = vec![];
    for x in a {
        for y in b {
            for z in c {
                out.push((x.clone(), y.clone(), z.clone()));
            }
        }
    }
    out
}

/// `itertools.pairwise(xs)`
pub fn pairwise<T: Clone>(xs: &[T]) -> Vec<(T, T)> {
    xs.windows(2).map(|w| (w[0].clone(), w[1].clone())).collect()
}
