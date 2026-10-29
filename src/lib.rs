//! py2rs: a typed subset of Python → readable Rust.
//!
//! `front` parses (ruff) and type-checks into the IR, `own` infers ownership
//! and rejects programs whose meaning depends on shared mutable objects,
//! `gen` prints Rust that uses the `pyrt` runtime crate.

pub mod diag;
pub mod exc;
pub mod front;
pub mod fuzz;
pub mod gen;
pub mod ir;
pub mod own;
pub mod project;
pub mod stubgen;

pub use diag::Error;

/// What py2rs decided about ownership, per function (`py2rs explain`).
pub fn explain(src: &str) -> Result<String, Error> {
    use ir::Ty;
    let prog = front::lower(src)?;
    let own = own::analyze(&prog)?;
    let exc = exc::analyze(&prog)?;
    gen::generate(&prog, &own, &exc)?;
    let clones = gen::clones();
    let mut out = String::new();
    for (fid, f) in prog.funcs.iter().enumerate() {
        if !f.dispatch.is_empty() {
            continue;
        }
        let name = match f.class {
            Some(c) => format!("{}.{}", prog.classes[c].name, f.name),
            None if fid == prog.main => "<module>".to_string(),
            None => f.name.clone(),
        };
        out += &format!("{} (line {})\n", name, f.line);
        if f.self_var.is_some() && !f.is_init {
            out += &format!("  self: {}\n", if own.self_mut[fid] { "&mut self (modifies the object)" } else { "&self (only reads)" });
        }
        for (i, q) in f.params.iter().enumerate() {
            let v = &f.vars[q.var];
            let t = gen::rty(&v.ty, &prog);
            let rt = match &v.ty {
                Ty::Str => "&str".to_string(),
                Ty::List(e) => format!("&[{}]", gen::rty(e, &prog)),
                _ => format!("&{}", t),
            };
            let how = match own.params[fid][i] {
                own::Mode::Copy => format!("{} (copied: a number / bool / tuple of them)", t),
                own::Mode::Ref => format!("{} (only read)", rt),
                own::Mode::Mut => format!("&mut {} (modified in place)", t),
                own::Mode::Owned if own.owned_local[fid][i] => format!("{} (rebound inside: callers may pass a copy)", t),
                own::Mode::Owned => format!("{} (kept or returned: callers move it, or it is copied)", t),
            };
            let forced = if q.force.is_some() { " [annotated]" } else { "" };
            out += &format!("  param {}: {}{}\n", v.name, how, forced);
        }
        if exc.fallible[fid] {
            let mut ks: Vec<&String> = exc.may[fid].iter().collect();
            ks.sort();
            out += &format!("  returns Result<_, PyErr>: a caller catches {}\n", ks.iter().map(|k| k.as_str()).collect::<Vec<_>>().join(", "));
        }
        let mut names = |set: &std::collections::HashSet<usize>, what: &str| {
            let mut v: Vec<&str> = set.iter().map(|x| f.vars[*x].name.as_str()).collect();
            v.sort();
            if !v.is_empty() {
                out += &format!("  {}: {}\n", what, v.join(", "));
            }
        };
        names(&own.views[fid], "references (&T, only read other objects)");
        names(&own.mut_views[fid], "mutable references (&mut T into a container)");
        names(&own.consumed[fid], "narrowed values that take the Optional's value");
        let mut cs: Vec<&(usize, u32, String)> = clones.iter().filter(|c| c.0 == fid).collect();
        cs.sort_by_key(|c| c.1);
        cs.dedup();
        for (_, line, what) in cs {
            out += &format!("  line {}: copies `{}` (the original is still used, or Python would share it)\n", line, what);
        }
    }
    Ok(out)
}

/// Translate Python source to the Rust source of a program.
pub fn translate(src: &str) -> Result<String, Error> {
    let prog = front::lower(src)?;
    let own = own::analyze(&prog)?;
    let exc = exc::analyze(&prog)?;
    gen::generate(&prog, &own, &exc)
}
