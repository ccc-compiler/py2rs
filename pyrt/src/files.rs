//! Text files (`open`) and `os.path`: UTF-8, read at open, written through a buffer.

use std::io::Write;

use crate::raise;

pub struct File {
    path: String,
    /// read mode: the contents and the read position (bytes)
    text: Option<(String, usize)>,
    out: Option<std::io::BufWriter<std::fs::File>>,
    closed: bool,
}

fn os_error(e: std::io::Error, path: &str) -> ! {
    let (kind, no, what) = match e.kind() {
        std::io::ErrorKind::NotFound => ("FileNotFoundError", 2, "No such file or directory"),
        std::io::ErrorKind::PermissionDenied => ("PermissionError", 13, "Permission denied"),
        _ if std::path::Path::new(path).is_dir() => ("IsADirectoryError", 21, "Is a directory"),
        _ => ("OSError", 5, "Input/output error"),
    };
    raise(kind, &format!("[Errno {}] {}: '{}'", no, what, path))
}

/// `open(path, mode)`: mode is r, w or a (checked by py2rs).
pub fn open(path: &str, mode: &str) -> File {
    let mut f = File { path: path.to_string(), text: None, out: None, closed: false };
    match mode.trim_end_matches('t') {
        "r" => match std::fs::read(path) {
            Ok(b) => match String::from_utf8(b) {
                Ok(s) => f.text = Some((s, 0)),
                Err(_) => raise("UnicodeDecodeError", "'utf-8' codec can't decode the file"),
            },
            Err(e) => os_error(e, path),
        },
        m => {
            let r = std::fs::OpenOptions::new().write(true).create(true).truncate(m == "w").append(m == "a").open(path);
            match r {
                Ok(h) => f.out = Some(std::io::BufWriter::new(h)),
                Err(e) => os_error(e, path),
            }
        }
    }
    f
}

impl File {
    fn reading(&mut self) -> &mut (String, usize) {
        if self.closed {
            raise("ValueError", "I/O operation on closed file.");
        }
        match &mut self.text {
            Some(t) => t,
            None => raise("UnsupportedOperation", "not readable"),
        }
    }

    pub fn read(&mut self) -> String {
        let (s, pos) = self.reading();
        let r = s[*pos..].to_string();
        *pos = s.len();
        r
    }

    pub fn readline(&mut self) -> String {
        let (s, pos) = self.reading();
        let end = s[*pos..].find('\n').map(|i| *pos + i + 1).unwrap_or(s.len());
        let r = s[*pos..end].to_string();
        *pos = end;
        r
    }

    /// `readlines()` / `for line in f`
    pub fn readlines(&mut self) -> Vec<String> {
        let mut out = vec![];
        loop {
            let l = self.readline();
            if l.is_empty() {
                return out;
            }
            out.push(l);
        }
    }

    /// `write(s)`: the number of characters
    pub fn write(&mut self, s: &str) -> i64 {
        if self.closed {
            raise("ValueError", "I/O operation on closed file.");
        }
        match &mut self.out {
            Some(w) => {
                if let Err(e) = w.write_all(s.as_bytes()) {
                    os_error(e, &self.path.clone());
                }
                s.chars().count() as i64
            }
            None => raise("UnsupportedOperation", "not writable"),
        }
    }

    pub fn writelines(&mut self, xs: &[String]) {
        for x in xs {
            self.write(x);
        }
    }

    pub fn close(&mut self) {
        if let Some(mut w) = self.out.take() {
            let _ = w.flush();
        }
        self.closed = true;
    }
}

impl Drop for File {
    fn drop(&mut self) {
        self.close();
    }
}

/// `os.path.exists(p)`
pub fn exists(p: &str) -> bool {
    std::path::Path::new(p).exists()
}

/// `os.remove(p)`
pub fn remove(p: &str) {
    if let Err(e) = std::fs::remove_file(p) {
        os_error(e, p);
    }
}

/// `os.path.join(a, b, ...)`
pub fn join(parts: &[&str]) -> String {
    let mut out = String::new();
    for p in parts {
        if p.starts_with('/') {
            out = p.to_string();
        } else if out.is_empty() || out.ends_with('/') {
            out += p;
        } else {
            out.push('/');
            out += p;
        }
    }
    out
}
