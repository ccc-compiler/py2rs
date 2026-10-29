//! Cargo projects for generated code, building, and the differential test
//! against CPython.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The `pyrt` crate next to this one.
pub fn pyrt_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("pyrt")
}

/// Crate dependencies a generated program declares (`// py2rs-crate: regex = "1"`).
pub fn crates_of(rust: &str) -> Vec<String> {
    rust.lines().filter_map(|l| l.strip_prefix("// py2rs-crate: ")).map(String::from).collect()
}

pub fn cargo_toml(name: &str, bins: &[String]) -> String {
    cargo_toml_with(name, bins, &[])
}

pub fn cargo_toml_with(name: &str, bins: &[String], crates: &[String]) -> String {
    let mut deps = String::new();
    let mut features = vec![];
    for c in crates {
        // `pyrt/re`: a feature of pyrt
        match c.strip_prefix("pyrt/") {
            Some(f) => features.push(format!("{:?}", f)),
            None => deps += &format!("{}\n", c),
        }
    }
    features.sort();
    features.dedup();
    let feat = if features.is_empty() { String::new() } else { format!(", features = [{}]", features.join(", ")) };
    // next to the sources (a checkout), else the published crate (`cargo install py2rs`)
    let pyrt = if pyrt_dir().join("Cargo.toml").exists() {
        format!("pyrt = {{ path = {:?}{} }}", pyrt_dir(), feat)
    } else {
        format!("pyrt = {{ version = {:?}{} }}", env!("CARGO_PKG_VERSION"), feat)
    };
    let mut s = format!(
        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\n{}\n{}\n\
         # Python ints never wrap: overflow is reported instead\n[profile.release]\noverflow-checks = true\n",
        name,
        pyrt,
        deps
    );
    for b in bins {
        s += &format!("\n[[bin]]\nname = \"{}\"\npath = \"src/bin/{}.rs\"\n", b, b);
    }
    s += "\n[workspace]\n";
    s
}

/// Rust package name from a file name.
pub fn crate_name(path: &Path) -> String {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("prog");
    let mut n: String = stem.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' }).collect();
    if n.is_empty() || n.chars().next().unwrap().is_ascii_digit() {
        n = format!("p_{}", n);
    }
    n
}

/// Write `dir/Cargo.toml` and `dir/src/main.rs`.
/// `Cargo.toml` of a Python extension module (`py2rs pyo3`).
pub fn cargo_toml_lib(name: &str, crates: &[String]) -> String {
    let t = cargo_toml_with(name, &[], crates);
    let t = t.replacen("[dependencies]\n", "[lib]\ncrate-type = [\"cdylib\"]\n\n[dependencies]\npyo3 = { version = \"0.30\", features = [\"extension-module\", \"indexmap\"] }\n", 1);
    t
}

pub fn write_project(dir: &Path, name: &str, rust: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(dir.join("src"))?;
    std::fs::write(dir.join("Cargo.toml"), cargo_toml_with(name, &[], &crates_of(rust)))?;
    std::fs::write(dir.join("src/main.rs"), rust)?;
    Ok(())
}

/// `cargo build --release` in `dir`; returns the binary path.
pub fn build(dir: &Path, name: &str, target: &Path, quiet: bool) -> Result<PathBuf, String> {
    let mut c = Command::new("cargo");
    c.arg("build").arg("--release").current_dir(dir).env("CARGO_TARGET_DIR", target);
    if quiet {
        c.arg("--quiet");
    }
    let out = c.stderr(Stdio::piped()).output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).to_string());
    }
    Ok(target.join("release").join(name))
}

/// Where the `rust` annotation module lives (for running the Python side).
pub fn python_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("python")
}

/// The Python used as the reference (`PY2RS_PYTHON`, else python3).
pub fn python() -> String {
    if let Ok(p) = std::env::var("PY2RS_PYTHON") {
        return p;
    }
    for p in ["python3.14", "python3.13", "python3.12"] {
        if Command::new(p).arg("-c").arg("pass").output().map(|o| o.status.success()).unwrap_or(false) {
            return p.to_string();
        }
    }
    "python3".into()
}

pub struct Run {
    pub stdout: Vec<u8>,
    pub code: i32,
    pub exc: String,
}

pub fn run(cmd: &mut Command, input: &[u8]) -> Run {
    use std::io::Write;
    let mut child = cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().expect("spawn");
    if let Some(mut si) = child.stdin.take() {
        let _ = si.write_all(input);
    }
    let out = child.wait_with_output().expect("wait");
    let err = String::from_utf8_lossy(&out.stderr);
    let last = err.lines().last().unwrap_or("").to_string();
    let exc = last.split(':').next().unwrap_or("").to_string();
    Run { stdout: out.stdout, code: out.status.code().unwrap_or(-1), exc }
}
