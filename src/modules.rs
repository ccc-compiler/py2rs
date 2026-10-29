//! Programs of several files: the local modules a program imports (`import m`,
//! `from m import x`, with `m.py` next to it) are appended to its source, so
//! the main file keeps its line numbers; errors are mapped back to the file.

use std::path::{Path, PathBuf};

use ruff_python_ast::{Expr as AExpr, Stmt as AStmt};
use ruff_source_file::LineIndex;
use ruff_text_size::{Ranged, TextRange};

use crate::diag::Error;

/// The combined source and where each file starts.
pub struct Combined {
    pub src: String,
    /// (first line in `src`, file, its text)
    pub parts: Vec<(u32, PathBuf, String)>,
    /// names of the local modules
    pub modules: Vec<String>,
}

/// An error in file `.0` (whose text is `.1`).
pub type FileError = (PathBuf, String, Error);

impl Combined {
    /// The file, its text and the line within it of a line of `src`.
    pub fn locate(&self, line: u32) -> (PathBuf, String, u32) {
        let mut best = &self.parts[0];
        for p in &self.parts {
            if p.0 <= line {
                best = p;
            }
        }
        (best.1.clone(), best.2.clone(), line - best.0 + 1)
    }
}

/// Spaces in place of a statement (newlines kept).
fn blank(text: &mut String, r: TextRange) {
    let (a, b) = (r.start().to_usize(), r.end().to_usize());
    let rep: String = text[a..b].chars().map(|c| if c == '\n' { '\n' } else { ' ' }).collect();
    text.replace_range(a..b, &rep);
}

fn is_main_guard(s: &AStmt) -> bool {
    let AStmt::If(i) = s else { return false };
    let AExpr::Compare(c) = &*i.test else { return false };
    matches!(c.operands.first(), Some(AExpr::Name(n)) if n.id.as_str() == "__name__")
}

fn err_at(path: &Path, text: &str, r: TextRange, msg: String) -> FileError {
    let line = LineIndex::from_source_text(text).line_index(r.start()).get() as u32;
    (path.to_path_buf(), text.to_string(), Error::new(line, msg))
}

/// Reads `path` and the local modules it imports.
pub fn combine(path: &Path) -> Result<Combined, FileError> {
    let dir = path.parent().map(Path::to_path_buf).unwrap_or_default();
    let read = |p: &Path| std::fs::read_to_string(p).map_err(|e| (p.to_path_buf(), String::new(), Error::new(0, e.to_string())));
    let main = read(path)?;
    let mut out = Combined { src: String::new(), parts: vec![], modules: vec![] };
    let mut queue: Vec<(PathBuf, String, bool)> = vec![(path.to_path_buf(), main, true)];
    let mut texts: Vec<(PathBuf, String)> = vec![];
    while let Some((p, text, is_main)) = queue.pop() {
        let parsed = match ruff_python_parser::parse_module(&text) {
            Ok(m) => m.into_syntax(),
            // syntax errors are reported by the translator itself
            Err(_) => {
                texts.push((p, text));
                continue;
            }
        };
        let mut t = text.clone();
        for s in parsed.body.iter() {
            let local = |m: &str| {
                let f = dir.join(format!("{}.py", m));
                f.exists() && f != path
            };
            match s {
                AStmt::Import(i) => {
                    let names: Vec<&str> = i.names.iter().map(|a| a.name.id.as_str()).collect();
                    if names.iter().any(|n| local(n)) {
                        if names.iter().any(|n| !local(n)) || i.names.iter().any(|a| a.asname.is_some()) {
                            return Err(err_at(&p, &text, s.range(), "import local modules on their own line, without `as`".into()));
                        }
                        for n in names {
                            if !out.modules.iter().any(|m| m == n) {
                                out.modules.push(n.to_string());
                                let f = dir.join(format!("{}.py", n));
                                queue.push((f.clone(), read(&f)?, false));
                            }
                        }
                        blank(&mut t, s.range());
                    }
                }
                AStmt::ImportFrom(i) if i.level == 0 && i.module.as_ref().is_some_and(|m| local(m.id.as_str())) => {
                    let n = i.module.as_ref().unwrap().id.to_string();
                    if i.names.iter().any(|a| a.asname.is_some() || a.name.id.as_str() == "*") {
                        return Err(err_at(&p, &text, s.range(), "`from module import x as y` / `*` is not supported for local modules".into()));
                    }
                    if !out.modules.contains(&n) {
                        out.modules.push(n.clone());
                        let f = dir.join(format!("{}.py", n));
                        queue.push((f.clone(), read(&f)?, false));
                    }
                    blank(&mut t, s.range());
                }
                _ if is_main => {}
                // an imported module only defines things
                AStmt::FunctionDef(_) | AStmt::ClassDef(_) | AStmt::ImportFrom(_) | AStmt::Pass(_) => {}
                AStmt::Assign(_) | AStmt::AnnAssign(_) => {}
                AStmt::Expr(e) if matches!(&*e.value, AExpr::StringLiteral(_)) => {}
                _ if is_main_guard(s) => blank(&mut t, s.range()),
                _ => {
                    return Err(err_at(&p, &text, s.range(), "an imported module may only define functions, classes and constants".into()))
                }
            }
        }
        texts.push((p, t));
    }
    // the main file first, then the modules in the order they were found
    let main_idx = texts.iter().position(|(p, _)| p == path).unwrap();
    let main = texts.remove(main_idx);
    let mut line = 1u32;
    for (p, t) in std::iter::once(main).chain(texts) {
        let original = std::fs::read_to_string(&p).unwrap_or_default();
        out.parts.push((line, p, original));
        out.src += &t;
        if !t.ends_with('\n') {
            out.src.push('\n');
        }
        line = out.src.lines().count() as u32 + 1;
    }
    Ok(out)
}
