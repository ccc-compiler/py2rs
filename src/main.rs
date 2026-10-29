use std::path::{Path, PathBuf};
use std::process::Command;

use py2rs::project;

const HELP: &str = "\
py2rs: translate a typed subset of Python into readable Rust

usage:
  py2rs emit  FILE.py              print the Rust program
  py2rs check FILE.py              check only (types, ownership)
  py2rs explain FILE.py            ownership decisions: parameter modes, references, copies
  py2rs build FILE.py [-o DIR]     write a Cargo project (default FILE_rs/) and build it
  py2rs run   FILE.py [ARGS...]    build (cached) and run
  py2rs test  [FILES|DIRS...]      differential test against CPython (default tests/)
  py2rs fuzz  [N] [SEED]           N random programs (default 200) against CPython;
                                   failures are kept in target/py2rs-cache/fuzz-fail/

The reference Python is PY2RS_PYTHON, else python3.14 / 3.13 / 3.12 / python3.";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = match args.first().map(|s| s.as_str()) {
        Some("emit") if args.len() == 2 => emit(&args[1]),
        Some("check") if args.len() == 2 => check(&args[1]),
        Some("explain") if args.len() == 2 => explain(&args[1]),
        Some("build") if args.len() >= 2 => {
            let out = match args.iter().position(|a| a == "-o") {
                Some(i) if i + 1 < args.len() => Some(PathBuf::from(&args[i + 1])),
                _ => None,
            };
            build(&args[1], out)
        }
        Some("run") if args.len() >= 2 => run(&args[1], &args[2..]),
        Some("test") => test(&args[1..]),
        Some("fuzz") => fuzz(
            args.get(1).and_then(|n| n.parse().ok()).unwrap_or(200),
            args.get(2).and_then(|n| n.parse().ok()).unwrap_or_else(|| {
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
            }),
        ),
        _ => {
            println!("{}", HELP);
            if args.is_empty() || ["help", "-h", "--help"].contains(&args[0].as_str()) {
                0
            } else {
                2
            }
        }
    };
    std::process::exit(code);
}

fn translate(file: &str) -> Option<String> {
    let src = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}: {}", file, e);
            return None;
        }
    };
    match py2rs::translate(&src) {
        Ok(r) => Some(r),
        Err(e) => {
            eprint!("{}", e.render(file, &src));
            None
        }
    }
}

fn emit(file: &str) -> i32 {
    match translate(file) {
        Some(r) => {
            print!("{}", r);
            0
        }
        None => 1,
    }
}

fn explain(file: &str) -> i32 {
    let src = match std::fs::read_to_string(file) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}: {}", file, e);
            return 1;
        }
    };
    match py2rs::explain(&src) {
        Ok(r) => {
            print!("{}", r);
            0
        }
        Err(e) => {
            eprint!("{}", e.render(file, &src));
            1
        }
    }
}

fn check(file: &str) -> i32 {
    match translate(file) {
        Some(_) => {
            println!("{}: ok", file);
            0
        }
        None => 1,
    }
}

fn cache_target() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join("py2rs-cache")
}

fn build(file: &str, out: Option<PathBuf>) -> i32 {
    let Some(rust) = translate(file) else { return 1 };
    let path = Path::new(file);
    let name = project::crate_name(path);
    let dir = out.unwrap_or_else(|| path.with_file_name(format!("{}_rs", name)));
    if let Err(e) = project::write_project(&dir, &name, &rust) {
        eprintln!("{}: {}", dir.display(), e);
        return 1;
    }
    match project::build(&dir, &name, &dir.join("target"), false) {
        Ok(bin) => {
            println!("{}", bin.display());
            0
        }
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
}

fn run(file: &str, args: &[String]) -> i32 {
    let Some(rust) = translate(file) else { return 1 };
    let name = project::crate_name(Path::new(file));
    let dir = cache_target().join("src").join(&name);
    if let Err(e) = project::write_project(&dir, &name, &rust) {
        eprintln!("{}", e);
        return 1;
    }
    match project::build(&dir, &name, &cache_target(), true) {
        Ok(bin) => Command::new(bin).args(args).status().map(|s| s.code().unwrap_or(1)).unwrap_or(1),
        Err(e) => {
            eprintln!("{}", e);
            1
        }
    }
}

/// Python files under the arguments (default: tests/).
fn files(args: &[String]) -> Vec<PathBuf> {
    let roots: Vec<PathBuf> = if args.is_empty() {
        vec![Path::new(env!("CARGO_MANIFEST_DIR")).join("tests")]
    } else {
        args.iter().map(PathBuf::from).collect()
    };
    fn walk(p: &Path, out: &mut Vec<PathBuf>) {
        if p.is_dir() {
            let mut es: Vec<PathBuf> = std::fs::read_dir(p).unwrap().flatten().map(|e| e.path()).collect();
            es.sort();
            for e in es {
                walk(&e, out);
            }
        } else if p.extension().map(|e| e == "py").unwrap_or(false) {
            out.push(p.to_path_buf());
        }
    }
    let mut out = vec![];
    for r in roots {
        walk(&r, &mut out);
    }
    out
}

/// Differential test: every program must translate, build, and print exactly
/// what CPython prints (stdout, exit code, exception type); programs whose
/// first line is `# error: TEXT` must be rejected with a message containing TEXT.
fn test(args: &[String]) -> i32 {
    let py = project::python();
    let dir = match std::env::var("PY2RS_TEST_DIR") {
        Ok(d) => PathBuf::from(d),
        Err(_) => cache_target().join("difftest"),
    };
    let _ = std::fs::remove_dir_all(dir.join("src/bin"));
    std::fs::create_dir_all(dir.join("src/bin")).unwrap();
    let dir = dir.canonicalize().unwrap();
    let mut fails = 0;
    let mut rejected = 0;
    let mut bins: Vec<(PathBuf, String)> = vec![];
    for f in files(args) {
        let src = std::fs::read_to_string(&f).unwrap();
        let expect = src.lines().next().and_then(|l| l.strip_prefix("# error: ")).map(|s| s.trim().to_string());
        match (py2rs::translate(&src), expect) {
            (Ok(_), Some(want)) => {
                println!("FAIL {} (accepted, expected error: {})", f.display(), want);
                fails += 1;
            }
            (Err(e), Some(want)) => {
                if e.msg.contains(&want) || e.hint.as_deref().unwrap_or("").contains(&want) {
                    rejected += 1;
                } else {
                    println!("FAIL {} (error `{}`, expected `{}`)", f.display(), e.msg, want);
                    fails += 1;
                }
            }
            (Err(e), None) => {
                print!("FAIL {}\n{}", f.display(), e.render(&f.display().to_string(), &src));
                fails += 1;
            }
            (Ok(rust), None) => {
                // directory + file name: the same name may appear in two areas
                let dir_name = f.parent().and_then(|d| d.file_name()).map(|d| project::crate_name(Path::new(d))).unwrap_or_default();
                let name = format!("t_{}_{}", dir_name, project::crate_name(&f));
                std::fs::write(dir.join("src/bin").join(format!("{}.rs", name)), rust).unwrap();
                bins.push((f, name));
            }
        }
    }
    let names: Vec<String> = bins.iter().map(|b| b.1.clone()).collect();
    std::fs::write(dir.join("Cargo.toml"), project::cargo_toml("difftest", &names)).unwrap();
    if !bins.is_empty() {
        let out = Command::new("cargo")
            .args(["build", "--release", "--quiet"])
            .current_dir(&dir)
            .env("CARGO_TARGET_DIR", dir.join("target"))
            .output()
            .expect("cargo");
        if !out.status.success() {
            println!("FAIL: generated Rust does not compile\n{}", String::from_utf8_lossy(&out.stderr));
            return 1;
        }
    }
    let mut pass = 0;
    // run CPython and the Rust programs in parallel
    let results: Vec<(project::Run, project::Run)> = std::thread::scope(|sc| {
        let hs: Vec<_> = bins
            .iter()
            .map(|(f, name)| {
                let (py, dir) = (&py, &dir);
                sc.spawn(move || {
                    let input = std::fs::read(f.with_extension("in")).unwrap_or_default();
                    let a = project::run(Command::new(py).arg(f).env("PYTHONPATH", project::python_path()), &input);
                    let b = project::run(&mut Command::new(dir.join("target/release").join(name)), &input);
                    (a, b)
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for ((f, _), (a, b)) in bins.iter().zip(results) {
        if a.stdout == b.stdout && a.code == b.code && (a.code == 0 || a.exc == b.exc) {
            pass += 1;
            continue;
        }
        fails += 1;
        println!("FAIL {} (exit {} vs {}; {} vs {})", f.display(), a.code, b.code, a.exc, b.exc);
        let (sa, sb) = (String::from_utf8_lossy(&a.stdout), String::from_utf8_lossy(&b.stdout));
        for (i, (x, y)) in sa.lines().zip(sb.lines()).enumerate() {
            if x != y {
                println!("  line {}: python: {}\n  line {}: rust:   {}", i + 1, x, i + 1, y);
                break;
            }
        }
        if sa.lines().count() != sb.lines().count() {
            println!("  python printed {} lines, rust {}", sa.lines().count(), sb.lines().count());
        }
    }
    println!(
        "{} of {} programs match {}; {} rejected as expected; {} failed",
        pass,
        bins.len(),
        py,
        rejected,
        fails
    );
    if fails > 0 {
        1
    } else {
        0
    }
}

/// Random programs: CPython and the translation must agree (a program that
/// overflows 64-bit ints is skipped: Python ints are unbounded).
fn fuzz(n: u64, seed: u64) -> i32 {
    let py = project::python();
    let root = cache_target();
    let dir = root.join("fuzz");
    let fails = root.join("fuzz-fail");
    let _ = std::fs::remove_dir_all(dir.join("src/bin"));
    std::fs::create_dir_all(dir.join("src/bin")).unwrap();
    std::fs::create_dir_all(dir.join("py")).unwrap();
    std::fs::create_dir_all(&fails).unwrap();
    let mut names = vec![];
    let mut bad = 0;
    for i in 0..n {
        let s = seed.wrapping_add(i);
        let src = py2rs::fuzz::program(s);
        let pyf = dir.join("py").join(format!("f{}.py", s));
        std::fs::write(&pyf, &src).unwrap();
        match py2rs::translate(&src) {
            Ok(rust) => {
                let name = format!("f{}", s);
                std::fs::write(dir.join("src/bin").join(format!("{}.rs", name)), rust).unwrap();
                names.push((pyf, name));
            }
            Err(e) => {
                bad += 1;
                println!("REJECTED f{}.py: {}", s, e);
                let _ = std::fs::copy(&pyf, fails.join(format!("f{}.py", s)));
            }
        }
    }
    let bins: Vec<String> = names.iter().map(|x| x.1.clone()).collect();
    std::fs::write(dir.join("Cargo.toml"), project::cargo_toml("fuzz", &bins)).unwrap();
    let out = Command::new("cargo")
        .args(["build", "--release", "--quiet"])
        .current_dir(&dir)
        .env("CARGO_TARGET_DIR", dir.join("target"))
        .output()
        .expect("cargo");
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        println!("FAIL: generated Rust does not compile\n{}", err.lines().take(40).collect::<Vec<_>>().join("\n"));
        for (f, _) in &names {
            let _ = std::fs::copy(f, fails.join(f.file_name().unwrap()));
        }
        return 1;
    }
    let results: Vec<(project::Run, project::Run)> = std::thread::scope(|sc| {
        let hs: Vec<_> = names
            .iter()
            .map(|(f, name)| {
                let (py, dir) = (&py, &dir);
                sc.spawn(move || {
                    let a = project::run(Command::new(py).arg(f).env("PYTHONPATH", project::python_path()), b"");
                    let b = project::run(&mut Command::new(dir.join("target/release").join(name)), b"");
                    (a, b)
                })
            })
            .collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let mut skipped = 0;
    for ((f, _), (a, b)) in names.iter().zip(results) {
        if b.exc == "OverflowError" && a.exc != "OverflowError" {
            skipped += 1;
            continue;
        }
        if a.stdout == b.stdout && a.code == b.code && (a.code == 0 || a.exc == b.exc) {
            continue;
        }
        bad += 1;
        let (sa, sb) = (String::from_utf8_lossy(&a.stdout), String::from_utf8_lossy(&b.stdout));
        let line = sa.lines().zip(sb.lines()).position(|(x, y)| x != y);
        println!("FAIL {} (exit {} vs {}; {} vs {})", f.file_name().unwrap().to_string_lossy(), a.code, b.code, a.exc, b.exc);
        if let Some(i) = line {
            println!("  python: {}\n  rust:   {}", sa.lines().nth(i).unwrap(), sb.lines().nth(i).unwrap());
        }
        let _ = std::fs::copy(f, fails.join(f.file_name().unwrap()));
    }
    println!("{} random programs (seed {}): {} failed, {} skipped (64-bit overflow)", n, seed, bad, skipped);
    if bad > 0 {
        1
    } else {
        0
    }
}

