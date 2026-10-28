//! py2rs: a typed subset of Python → readable Rust.
//!
//! `front` parses (ruff) and type-checks into the IR, `own` infers ownership
//! and rejects programs whose meaning depends on shared mutable objects,
//! `gen` prints Rust that uses the `pyrt` runtime crate.

pub mod diag;
pub mod front;
pub mod gen;
pub mod ir;
pub mod own;
pub mod project;

pub use diag::Error;

/// Translate Python source to the Rust source of a program.
pub fn translate(src: &str) -> Result<String, Error> {
    let prog = front::lower(src)?;
    let own = own::analyze(&prog)?;
    gen::generate(&prog, &own)
}
