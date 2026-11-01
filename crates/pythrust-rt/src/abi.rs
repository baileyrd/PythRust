//! What the generated Rust binding shims call (M6b,
//! `docs/research/rust-interop.md#the-shim`). Safe code: a shim's
//! `#[unsafe(no_mangle)]` is its only `unsafe`.

use std::panic::{catch_unwind, AssertUnwindSafe};

/// Runs a binding's function `f`. Flushes stdout first, so the binding's
/// own output follows what the program printed before the call. A panic
/// must not unwind into generated code (it would abort and lose the
/// buffered output): it ends the program with status 1, as a fault does,
/// after Rust's panic hook has printed its message.
pub fn call<T>(f: impl FnOnce() -> T) -> T {
    crate::flush();
    // `f` cannot leave anything half-changed that is seen again: on a
    // panic the program ends.
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => value,
        Err(_) => {
            crate::flush();
            std::process::exit(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn call_gives_the_result() {
        assert_eq!(call(|| 21 * 2), 42);
    }
}
