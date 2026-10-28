//! Exceptions (D6): the one pending exception, raised and caught.
//!
//! An exception is a sequence object ([`crate::seq`]) whose descriptor
//! is `x`, its kind's code as an ASCII digit (the order of [`NAMES`],
//! which is the compiler's `ExcKind::ALL`), and its argument's type, if
//! it has one, which its one slot holds.
//!
//! Raising makes an exception *pending*, with the diagnostic of the
//! place it was raised at, rendered at compile time around the line
//! [`pt_uncaught`] fills in. Generated code tests [`pt_raised`] after
//! whatever can raise and branches to a handler, which takes the
//! exception with [`pt_catch`], or returns to its caller with it still
//! pending. Compiled programs are single-threaded; the lock is
//! uncontended and only makes the static safe.

#![allow(unsafe_code)] // Exceptions and diagnostics cross the C ABI as pointers.

use std::io::{self, Write};
use std::sync::{Mutex, PoisonError};

use crate::seq::{self, PtSeq};
use crate::str::{self as pstr, PtStr};

/// The builtin exceptions, by kind code.
const NAMES: [&str; 5] = [
    "ValueError",
    "IndexError",
    "ZeroDivisionError",
    "OverflowError",
    "RecursionError",
];

/// The descriptor of a `ValueError` holding a string.
const VALUE_ERROR: &[u8] = b"x0s";

/// A raised exception not yet caught.
struct Pending {
    /// The exception, whose reference this holds.
    exc: usize,
    /// The rendered diagnostic before and after its uncaught line.
    head: &'static [u8],
    tail: &'static [u8],
}

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

/// The name of the exception kind `code`.
pub(crate) fn name(code: u8) -> &'static str {
    NAMES.get(usize::from(code)).copied().unwrap_or("Exception")
}

/// The last line Python prints for an uncaught exception: `ValueError:
/// bad`, or only the name when the message is empty.
fn uncaught_line(e: u64) -> String {
    let name = name(seq::exc_kind(e));
    match seq::exc_message(e) {
        m if m.is_empty() => name.to_string(),
        m => format!("{name}: {m}"),
    }
}

fn pending() -> std::sync::MutexGuard<'static, Option<Pending>> {
    PENDING.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Makes `exc`, whose reference it takes, pending. One already pending
/// (never, in generated code) is dropped.
fn set_pending(exc: *mut PtSeq, head: &'static [u8], tail: &'static [u8]) {
    let old = pending().replace(Pending {
        exc: exc as usize,
        head,
        tail,
    });
    if let Some(old) = old {
        // SAFETY: a pending exception is live and owns its reference.
        unsafe { seq::pt_seq_dec(old.exc as *mut PtSeq) };
    }
}

/// Reads a diagnostic part the compiler stored.
///
/// # Safety
///
/// `p` points to `len` bytes of read-only data that live for the whole
/// program.
unsafe fn part(p: *const u8, len: usize) -> &'static [u8] {
    // SAFETY: guaranteed by the caller.
    unsafe { std::slice::from_raw_parts(p, len) }
}

/// `raise e`: makes `e` pending with a new reference, raised where the
/// diagnostic `head`, its uncaught line, then `tail` points.
///
/// # Safety
///
/// `e` is a live exception; `head` and `tail` point to `head_len` and
/// `tail_len` bytes of the compiler's read-only data.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_raise(
    e: *const PtSeq,
    head: *const u8,
    head_len: usize,
    tail: *const u8,
    tail_len: usize,
) {
    // SAFETY: guaranteed by the caller.
    unsafe {
        seq::pt_seq_inc(e);
        set_pending(e.cast_mut(), part(head, head_len), part(tail, tail_len));
    }
}

/// The `ValueError` a malformed `int(s)` or `float(s)` raises: the
/// message is `prefix` then `repr(s)`, raised like [`pt_raise`].
///
/// # Safety
///
/// `prefix` points to `prefix_len` bytes of UTF-8, `s` is a live string,
/// and `head` and `tail` are as for [`pt_raise`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_raise_value(
    prefix: *const u8,
    prefix_len: usize,
    s: *const PtStr,
    head: *const u8,
    head_len: usize,
    tail: *const u8,
    tail_len: usize,
) {
    // SAFETY: guaranteed by the caller.
    let (prefix, text) = unsafe { (part(prefix, prefix_len), pstr::borrow(s).text()) };
    let message = format!("{}{}", String::from_utf8_lossy(prefix), pstr::repr(text));
    let arg = pstr::into_ptr(PtStr::new(message.into()));
    // SAFETY: the descriptor is static and the slot a live string, whose
    // reference the exception adds before this one is dropped.
    unsafe {
        let exc = seq::pt_seq_new(VALUE_ERROR.as_ptr(), VALUE_ERROR.len(), 1);
        seq::pt_seq_push(exc, arg as u64);
        pstr::pt_str_dec(arg);
        set_pending(exc, part(head, head_len), part(tail, tail_len));
    }
}

/// Whether an exception is pending: 1 or 0.
#[unsafe(no_mangle)]
pub extern "C" fn pt_raised() -> u8 {
    u8::from(pending().is_some())
}

/// Whether the pending exception is of kind `code`: 1 or 0.
#[unsafe(no_mangle)]
pub extern "C" fn pt_pending_is(code: u8) -> u8 {
    let guard = pending();
    let found = guard.as_ref().map(|p| seq::exc_kind(p.exc as u64));
    u8::from(found == Some(code))
}

/// Takes the pending exception and its reference, leaving none pending;
/// null if there is none.
#[unsafe(no_mangle)]
pub extern "C" fn pt_catch() -> *mut PtSeq {
    pending()
        .take()
        .map_or(std::ptr::null_mut(), |p| p.exc as *mut PtSeq)
}

/// Ends the program with the pending exception, as an uncaught one:
/// flushes stdout, prints its diagnostic to stderr, exits 1.
#[unsafe(no_mangle)]
pub extern "C" fn pt_uncaught() -> ! {
    crate::flush();
    if let Some(p) = pending().as_ref() {
        let line = uncaught_line(p.exc as u64);
        let mut err = io::stderr().lock();
        let _ = err
            .write_all(p.head)
            .and_then(|()| err.write_all(line.as_bytes()))
            .and_then(|()| err.write_all(p.tail));
    }
    std::process::exit(1)
}

/// `str(e)`, a new string.
///
/// # Safety
///
/// `e` is a live exception.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_exc_str(e: *const PtSeq) -> *mut PtStr {
    pstr::into_ptr(PtStr::new(seq::exc_message(e as u64).into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serializes the tests: the pending exception is process-wide.
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

    #[test]
    fn raising_and_catching_move_one_reference() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
        let (head, tail) = (b"at: ", b"!");
        // SAFETY: the exception is live until its last dec; the parts are
        // static.
        unsafe {
            let e = seq::pt_seq_new(b"x4".as_ptr(), 2, 0);
            assert_eq!(pt_raised(), 0);
            pt_raise(e, head.as_ptr(), 4, tail.as_ptr(), 1);
            seq::pt_seq_dec(e);
            assert_eq!((pt_raised(), pt_pending_is(4), pt_pending_is(0)), (1, 1, 0));
            assert_eq!(uncaught_line(e as u64), "RecursionError");
            let caught = pt_catch();
            assert_eq!((caught, pt_raised()), (e, 0));
            assert!(pt_catch().is_null());
            seq::pt_seq_dec(caught);
        }
    }

    #[test]
    fn parse_errors_raise_a_value_error_quoting_the_input() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
        let prefix = b"could not convert string to float: ";
        let s = pstr::into_ptr(PtStr::new("2.5.1".into()));
        let none = b"";
        // SAFETY: as above; `s` is live until dropped.
        unsafe {
            let n = none.as_ptr();
            pt_raise_value(prefix.as_ptr(), prefix.len(), s, n, 0, n, 0);
            pstr::pt_str_dec(s);
            assert_eq!(pt_pending_is(0), 1);
            let e = pt_catch();
            let line = uncaught_line(e as u64);
            assert_eq!(
                line,
                "ValueError: could not convert string to float: '2.5.1'"
            );
            let text = pt_exc_str(e);
            assert_eq!(
                pstr::borrow(text).text(),
                "could not convert string to float: '2.5.1'"
            );
            pstr::pt_str_dec(text);
            seq::pt_seq_dec(e);
        }
    }
}
