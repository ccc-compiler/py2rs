//! `py2rs pyo3 FILE.py`: a Python extension module (PyO3) exposing the
//! module-level functions of FILE.py. Arguments are converted by PyO3 (int,
//! float, bool, str, list, tuple, dict, None); an exception the function raises
//! becomes the same Python exception.

use std::path::Path;

use crate::exc::Exc;
use crate::gen::{ident, rty};
use crate::ir::*;
use crate::own::{Mode, Own};

/// Can PyO3 convert this type (both ways)?
fn convertible(t: &Ty) -> bool {
    match t {
        Ty::Int | Ty::Float | Ty::Bool | Ty::Str | Ty::None => true,
        Ty::List(e) | Ty::Optional(e) => convertible(e),
        Ty::Tuple(ts) => ts.iter().all(convertible),
        Ty::Dict(k, v, DK::Plain) => convertible(k) && convertible(v),
        _ => false,
    }
}

/// The wrapper functions and the `#[pymodule]`; the names exported and those skipped (with why).
pub fn wrappers(p: &Program, o: &Own, x: &Exc, module: &str) -> (String, Vec<String>, Vec<(String, String)>) {
    let mut out = String::new();
    let mut names = vec![];
    let mut skipped = vec![];
    for (fid, f) in p.funcs.iter().enumerate() {
        if fid == p.main || f.class.is_some() || f.ext.is_some() || f.name.starts_with('_') || !f.captures.is_empty() {
            continue;
        }
        if f.is_gen {
            skipped.push((f.name.clone(), "generators are not exported".into()));
            continue;
        }
        let mut why = None;
        let mut params = vec![];
        let mut args = vec![];
        for (i, q) in f.params.iter().enumerate() {
            let v = &f.vars[q.var];
            if !convertible(&v.ty) {
                why = Some(format!("parameter `{}` has type {}", v.name, TyName(&v.ty, p)));
                break;
            }
            let n = ident(&v.name);
            params.push(format!("{}: {}", n, rty(&v.ty, p)));
            args.push(match o.params[fid][i] {
                Mode::Copy | Mode::Owned => n,
                Mode::Ref => format!("&{}", n),
                Mode::Mut => {
                    why = Some(format!("it modifies its argument `{}` (Python would see the change)", v.name));
                    break;
                }
            });
        }
        if why.is_none() && !convertible(&f.ret) {
            why = Some(format!("it returns {}", TyName(&f.ret, p)));
        }
        if let Some(w) = why {
            skipped.push((f.name.clone(), w));
            continue;
        }
        let rust_name = if f.name == "main" { "py_main".to_string() } else { ident(&f.name) };
        let ret = if f.ret == Ty::None { "()".to_string() } else { rty(&f.ret, p) };
        let call = format!("{}({})", rust_name, args.join(", "));
        let body = if x.fallible[fid] {
            format!("pyrt::catching(|| {}).and_then(|r| r)", call)
        } else {
            format!("pyrt::catching(|| {})", call)
        };
        out += &format!(
            "#[pyfunction]\n#[pyo3(name = {:?})]\nfn py_{}({}) -> PyResult<{}> {{\n    let r = {};\n    pyrt::flush();\n    r.map_err(to_python)\n}}\n\n",
            f.name,
            f.name,
            params.join(", "),
            ret,
            body
        );
        names.push(f.name.clone());
    }
    out += "/// A pyrt exception as the Python exception of the same (or the nearest builtin) class.\n";
    out += "fn to_python(e: pyrt::PyErr) -> PyErr {\n    use pyo3::exceptions::*;\n";
    out += "    let msg = e.msg.clone();\n    for k in e.chain {\n        let err = match *k {\n";
    for k in [
        "ValueError", "TypeError", "IndexError", "KeyError", "ZeroDivisionError", "OverflowError", "ArithmeticError",
        "LookupError", "RuntimeError", "NotImplementedError", "AssertionError", "AttributeError", "EOFError", "NameError",
        "FileNotFoundError", "OSError", "StopIteration",
    ] {
        out += &format!("            {:?} => Py{}::new_err(msg.clone()),\n", k, k);
    }
    out += "            _ => continue,\n        };\n        return err;\n    }\n    PyException::new_err(msg)\n}\n\n";
    out += &format!(
        "#[pymodule]\nfn {}(m: &Bound<'_, PyModule>) -> PyResult<()> {{\n    pyrt::init();\n    \
         // print() writes to Python's sys.stdout, in order with Python's own output\n    \
         pyrt::out::set_sink(|s| {{\n        Python::attach(|py| {{\n            \
         let _ = py.import(\"sys\").and_then(|sys| sys.getattr(\"stdout\")).and_then(|o| o.call_method1(\"write\", (s,)));\n        \
         }});\n    }});\n",
        module
    );
    for n in &names {
        out += &format!("    m.add_function(wrap_pyfunction!(py_{}, m)?)?;\n", n);
    }
    out += "    Ok(())\n}\n";
    (out, names, skipped)
}

/// The crate: `Cargo.toml` and `src/lib.rs`.
pub fn crate_files(path: &Path) -> Result<(String, String, String, Vec<(String, String)>), crate::modules::FileError> {
    let c = crate::modules::combine(path)?;
    let module = crate::project::crate_name(path);
    let r = (|| {
        let prog = crate::front::lower_modules(&c.src, &crate::front::default_stub_dirs(), &c.modules)?;
        let own = crate::own::analyze(&prog)?;
        let exc = crate::exc::analyze(&prog)?;
        let code = crate::gen::generate(&prog, &own, &exc)?;
        Ok((prog, own, exc, code))
    })();
    let (prog, own, exc, code) = r.map_err(|mut e: crate::diag::Error| {
        let (f, src, line) = c.locate(e.line);
        e.line = line;
        (f, src, e)
    })?;
    // the program without its `main`: module-level code does not run on import
    let body = match code.find("\nfn main() {") {
        Some(i) => code[..i].to_string(),
        None => code,
    };
    let body: String = body.lines().filter(|l| *l != "#![allow(dead_code)]").map(|l| format!("{}\n", l)).collect();
    let (w, _, skipped) = wrappers(&prog, &own, &exc, &module);
    let lib = format!(
        "//! Python extension module `{}`, translated by py2rs.\n#![allow(dead_code)]\n\nuse pyo3::prelude::*;\n\n{}\n// ---- Python bindings\n\n{}",
        module, body, w
    );
    let crates = crate::project::crates_of(&lib);
    let toml = crate::project::cargo_toml_lib(&module, &crates);
    Ok((module, toml, lib, skipped))
}
