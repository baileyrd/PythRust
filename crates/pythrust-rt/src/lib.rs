//! Pythrust runtime: what compiled programs call that is not worth
//! generating inline. Output buffering, `print` formatting, runtime
//! error exits, and strings ([`str`]).
//!
//! The C ABI surface is the only `unsafe` in the project. Each exported
//! item (or module, for [`str`]) carries `allow(unsafe_code)` and a
//! `# Safety` note on what the caller guarantees, so the audit is
//! `grep allow(unsafe_code)`. The logic behind the ABI is safe Rust,
//! tested.

#![deny(unsafe_code)]

pub mod str;

use std::io::{self, Write};
use std::sync::{Mutex, PoisonError};

/// Flush threshold for buffered stdout.
const FLUSH_AT: usize = 1 << 16;

/// Buffered stdout. Compiled programs are single-threaded; the lock is
/// uncontended and only makes the static safe.
static OUT: Mutex<Vec<u8>> = Mutex::new(Vec::new());

/// Appends `bytes` to stdout, flushing when the buffer is full.
fn write(bytes: &[u8]) {
    let mut out = OUT.lock().unwrap_or_else(PoisonError::into_inner);
    out.extend_from_slice(bytes);
    if out.len() >= FLUSH_AT {
        drain(&mut out);
    }
}

/// Writes `out` to stdout and clears it. A write error ends the program:
/// output the program believes it printed must not vanish silently.
fn drain(out: &mut Vec<u8>) {
    let mut stdout = io::stdout().lock();
    let result = stdout.write_all(out).and_then(|()| stdout.flush());
    out.clear();
    if let Err(err) = result {
        let _ = writeln!(io::stderr(), "pythrust: cannot write to stdout: {err}");
        std::process::exit(1);
    }
}

fn flush() {
    drain(&mut OUT.lock().unwrap_or_else(PoisonError::into_inner));
}

/// `str(b)` for a bool.
fn bool_text(b: bool) -> &'static [u8] {
    if b {
        b"True"
    } else {
        b"False"
    }
}

/// Writes `n` in decimal.
#[allow(unsafe_code)] // SAFETY: plain value argument, no pointers.
#[unsafe(no_mangle)]
pub extern "C" fn pt_write_int(n: i64) {
    write(n.to_string().as_bytes());
}

/// Writes `True` or `False`. Any nonzero byte is true.
#[allow(unsafe_code)] // SAFETY: plain value argument, no pointers.
#[unsafe(no_mangle)]
pub extern "C" fn pt_write_bool(b: u8) {
    write(bool_text(b != 0));
}

/// Writes `None`.
#[allow(unsafe_code)] // SAFETY: no arguments.
#[unsafe(no_mangle)]
pub extern "C" fn pt_write_none() {
    write(b"None");
}

/// Writes one byte: `print`'s separator and line end.
#[allow(unsafe_code)] // SAFETY: plain value argument, no pointers.
#[unsafe(no_mangle)]
pub extern "C" fn pt_write_byte(b: u8) {
    write(&[b]);
}

/// Ends the program on a runtime error: flushes stdout, prints the
/// already-rendered diagnostic `msg[..len]` to stderr, exits 1.
///
/// # Safety
///
/// `msg` points to `len` readable bytes that live for the call. Generated
/// code passes a pointer and length into its own read-only data.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_fail(msg: *const u8, len: usize) -> ! {
    // SAFETY: guaranteed by the caller, see above.
    let msg = unsafe { std::slice::from_raw_parts(msg, len) };
    flush();
    let _ = io::stderr().write_all(msg);
    std::process::exit(1)
}

/// Process entry. Not built for unit tests, whose harness has its own.
#[cfg(not(test))]
mod entry {
    #[allow(unsafe_code)]
    unsafe extern "C" {
        /// The compiled module's top-level code, emitted by the compiler.
        fn pythrust_main();
    }

    /// Runs the module, then flushes stdout.
    #[allow(unsafe_code)]
    #[unsafe(no_mangle)]
    pub extern "C" fn main() -> i32 {
        // SAFETY: `pythrust_main` takes and returns nothing; the compiler
        // emits it with exactly that signature.
        unsafe { pythrust_main() };
        super::flush();
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bools_print_as_python_does() {
        assert_eq!(bool_text(true), b"True");
        assert_eq!(bool_text(false), b"False");
    }
}
