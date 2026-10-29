//! Buffered standard output. Generated code imports `pyrt::{print, println}`;
//! the buffer is flushed when the program ends, before input is read, and
//! before a traceback is written.

use std::io::Write;
use std::sync::Mutex;

static OUT: Mutex<Option<std::io::BufWriter<std::io::Stdout>>> = Mutex::new(None);

pub fn write_out(args: std::fmt::Arguments) {
    let mut g = OUT.lock().unwrap_or_else(|e| e.into_inner());
    let w = g.get_or_insert_with(|| std::io::BufWriter::with_capacity(1 << 16, std::io::stdout()));
    if w.write_fmt(args).is_err() {
        // a closed pipe: stop like CPython would (BrokenPipeError)
        std::process::exit(1);
    }
}

/// Writes out what is buffered.
pub fn flush() {
    let mut g = OUT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(w) = g.as_mut() {
        let _ = w.flush();
    }
    let _ = std::io::stdout().flush();
}

/// `print(..)` without a newline
#[macro_export]
macro_rules! print {
    ($($t:tt)*) => {
        $crate::out::write_out(format_args!($($t)*))
    };
}

/// `print(..)`
#[macro_export]
macro_rules! println {
    () => {
        $crate::out::write_out(format_args!("\n"))
    };
    ($($t:tt)*) => {
        $crate::out::write_out(format_args!("{}\n", format_args!($($t)*)))
    };
}
