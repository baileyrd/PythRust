//! Exceptions (D6): the one pending exception, raised and caught.
//!
//! An exception is a sequence object ([`crate::seq`]) whose descriptor
//! is `x`, its kind's code as one byte, `'0'` plus the code (the order
//! of [`NAMES`], which is the compiler's `ExcKind::ALL`), and its
//! argument's type, if
//! it has one, which its one slot holds.
//!
//! Raising makes an exception *pending*, with the diagnostic of the
//! place it was raised at, rendered at compile time around the line
//! [`pt_uncaught`] fills in. Generated code reads [`PENDING_FLAG`] inline
//! after whatever can raise and branches to a handler, which takes the
//! exception with [`pt_catch`], or returns to its caller with it still
//! pending. Compiled programs are single-threaded; the lock is
//! uncontended and only makes the static safe.

#![allow(unsafe_code)] // Exceptions and diagnostics cross the C ABI as pointers.

use std::io::{self, Write};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Mutex, PoisonError};

use crate::seq::{self, PtSeq};
use crate::str::{self as pstr, PtStr};

/// The builtin exception classes, by kind code (the compiler's
/// `ExcKind::ALL`).
const NAMES: [&str; 15] = [
    "ValueError",
    "IndexError",
    "ZeroDivisionError",
    "OverflowError",
    "RecursionError",
    "ArithmeticError",
    "LookupError",
    "RuntimeError",
    "Exception",
    "KeyError",
    "OSError",
    "FileNotFoundError",
    "PermissionError",
    "IsADirectoryError",
    "NotADirectoryError",
];

/// The kind code of `KeyError`, whose message is its argument's `repr`.
pub(crate) const KEY_ERROR: u8 = 9;

/// The kind codes of `OSError` and its subclasses.
pub(crate) const OS_ERROR: u8 = 10;
pub(crate) const FILE_NOT_FOUND: u8 = 11;
pub(crate) const PERMISSION: u8 = 12;
pub(crate) const IS_A_DIRECTORY: u8 = 13;
pub(crate) const NOT_A_DIRECTORY: u8 = 14;

/// The descriptors of an exception holding a string, by kind code.
const WITH_MESSAGE: [&[u8]; 15] = [
    b"x0s", b"x1s", b"x2s", b"x3s", b"x4s", b"x5s", b"x6s", b"x7s", b"x8s", b"x9s", b"x:s", b"x;s",
    b"x<s", b"x=s", b"x>s",
];

/// The descriptors of an `OSError` the runtime raised, by kind code
/// less [`OS_ERROR`]: `E` and its `(errno, strerror, filename)` tuple
/// ([`seq::errno_form`]).
const WITH_ERRNO: [&[u8]; 5] = [
    b"x:Et\x03isos",
    b"x;Et\x03isos",
    b"x<Et\x03isos",
    b"x=Et\x03isos",
    b"x>Et\x03isos",
];

/// A raised exception not yet caught.
struct Pending {
    /// The exception, whose reference this holds.
    exc: usize,
    /// The rendered diagnostic before and after its uncaught line.
    head: &'static [u8],
    tail: &'static [u8],
}

static PENDING: Mutex<Option<Pending>> = Mutex::new(None);

/// Whether an exception is pending, 1 or 0. Generated code reads this
/// byte inline instead of calling into the runtime; it is set and
/// cleared with [`PENDING`], under its lock.
#[unsafe(export_name = "pt_pending_flag")]
pub static PENDING_FLAG: AtomicU8 = AtomicU8::new(0);

/// The name of the exception kind `code`.
pub(crate) fn name(code: u8) -> &'static str {
    NAMES.get(usize::from(code)).copied().unwrap_or("Exception")
}

/// The message of a `repr` nested deeper than `run` allows.
const TOO_DEEP: &str =
    "RecursionError: maximum recursion depth exceeded while getting the repr of an object";

/// The last line Python prints for an uncaught exception: `ValueError:
/// bad`, or only the name when the message is empty. A message too deep
/// to render is `run`'s `RecursionError` instead, at the same place.
fn uncaught_line(e: u64) -> String {
    let name = name(seq::exc_kind(e));
    match seq::exc_message(e) {
        Ok(m) if m.is_empty() => name.to_string(),
        Ok(m) => format!("{name}: {m}"),
        Err(seq::TooDeep) => TOO_DEEP.to_string(),
    }
}

fn pending() -> std::sync::MutexGuard<'static, Option<Pending>> {
    PENDING.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Makes `exc`, whose reference it takes, pending. One already pending
/// (never, in generated code) is dropped.
fn set_pending(exc: *mut PtSeq, head: &'static [u8], tail: &'static [u8]) {
    let mut slot = pending();
    let old = slot.replace(Pending {
        exc: exc as usize,
        head,
        tail,
    });
    PENDING_FLAG.store(1, Ordering::Relaxed);
    drop(slot);
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
    // SAFETY: guaranteed by the caller.
    let (head, tail) = unsafe { (part(head, head_len), part(tail, tail_len)) };
    raise_text(0, message, head, tail); // ValueError
}

/// An exception of kind `code` that no `raise` wrote, holding the
/// message `msg`: a division by zero, an index out of range, a wrong
/// unpacking arity. Raised like [`pt_raise`].
///
/// # Safety
///
/// `msg` points to `msg_len` bytes of UTF-8, and `head` and `tail` are
/// as for [`pt_raise`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_raise_msg(
    code: u8,
    msg: *const u8,
    msg_len: usize,
    head: *const u8,
    head_len: usize,
    tail: *const u8,
    tail_len: usize,
) {
    // SAFETY: guaranteed by the caller.
    let text = String::from_utf8_lossy(unsafe { part(msg, msg_len) }).into_owned();
    // SAFETY: guaranteed by the caller.
    let (head, tail) = unsafe { (part(head, head_len), part(tail, tail_len)) };
    raise_text(code, text, head, tail);
}

/// Makes pending a new exception of kind `code` holding the string
/// `text`, raised where `head` and `tail` point (see [`pt_raise`]).
pub(crate) fn raise_text(code: u8, text: String, head: &'static [u8], tail: &'static [u8]) {
    let arg = pstr::into_ptr(PtStr::new(text.into()));
    let desc = WITH_MESSAGE
        .get(usize::from(code))
        .copied()
        .unwrap_or(b"x8s");
    // SAFETY: the descriptor is static and the slot a live string, whose
    // reference the exception adds before this one is dropped.
    unsafe {
        let exc = seq::pt_seq_new(desc.as_ptr(), desc.len(), 1);
        seq::pt_seq_push(exc, arg as u64);
        pstr::pt_str_dec(arg);
        set_pending(exc, head, tail);
    }
}

/// Makes pending a new `OSError` of kind `code` (one of the family)
/// holding CPython's `errno`, `strerror` and `filename`, raised where
/// `head` and `tail` point (see [`pt_raise`]). `str(e)` is
/// `[Errno 2] No such file or directory: 'p'`, its `repr`
/// `FileNotFoundError(2, 'No such file or directory')`.
pub(crate) fn raise_errno(
    code: u8,
    errno: i32,
    strerror: &str,
    filename: Option<&str>,
    head: &'static [u8],
    tail: &'static [u8],
) {
    let desc = WITH_ERRNO
        .get(usize::from(code.wrapping_sub(OS_ERROR)))
        .copied()
        .unwrap_or(WITH_ERRNO[0]);
    let tuple_desc = desc.get(3..).unwrap_or_default();
    let strerror = pstr::into_ptr(PtStr::new(strerror.into()));
    let filename = filename.map_or(std::ptr::null_mut(), |p| {
        pstr::into_ptr(PtStr::new(p.into()))
    });
    // The new tuple and exception take the strings' and the tuple's
    // references.
    let fields = vec![i64::from(errno) as u64, strerror as u64, filename as u64];
    let args = seq::new_seq(tuple_desc, fields);
    let exc = seq::new_seq(desc, vec![args as u64]);
    set_pending(exc, head, tail);
}

/// Whether the pending exception's kind is in `codes`, a bit per kind
/// code (the compiler sets a class's bit and those of the classes derived
/// from it): 1 or 0.
#[unsafe(no_mangle)]
pub extern "C" fn pt_pending_is(codes: u32) -> u8 {
    let guard = pending();
    let found = guard.as_ref().map(|p| seq::exc_kind(p.exc as u64));
    u8::from(found.is_some_and(|code| codes.checked_shr(u32::from(code)).unwrap_or(0) & 1 == 1))
}

/// Takes the pending exception and its reference, leaving none pending;
/// null if there is none.
#[unsafe(no_mangle)]
pub extern "C" fn pt_catch() -> *mut PtSeq {
    let mut slot = pending();
    PENDING_FLAG.store(0, Ordering::Relaxed);
    slot.take()
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

/// `str(e)`, a new string; null when it nests deeper than `run` allows.
///
/// # Safety
///
/// `e` is a live exception.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_exc_str(e: *const PtSeq) -> *mut PtStr {
    match seq::exc_message(e as u64) {
        Ok(m) => pstr::into_ptr(PtStr::new(m.into())),
        Err(seq::TooDeep) => std::ptr::null_mut(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serializes the tests: the pending exception is process-wide.
    static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

    fn raised() -> u8 {
        PENDING_FLAG.load(Ordering::Relaxed)
    }

    #[test]
    fn raising_and_catching_move_one_reference() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
        let (head, tail) = (b"at: ", b"!");
        // SAFETY: the exception is live until its last dec; the parts are
        // static.
        unsafe {
            let e = seq::pt_seq_new(b"x4".as_ptr(), 2, 0);
            assert_eq!(raised(), 0);
            pt_raise(e, head.as_ptr(), 4, tail.as_ptr(), 1);
            seq::pt_seq_dec(e);
            let (recursion, runtime, value) = (1 << 4, 1 << 4 | 1 << 7, 1);
            let tests = (
                pt_pending_is(recursion),
                pt_pending_is(runtime),
                pt_pending_is(value),
            );
            assert_eq!((raised(), tests), (1, (1, 1, 0)));
            assert_eq!(uncaught_line(e as u64), "RecursionError");
            let caught = pt_catch();
            assert_eq!((caught, raised()), (e, 0));
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
            assert_eq!(pt_pending_is(1), 1); // ValueError's bit
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

    #[test]
    fn faults_raise_their_class_with_the_message() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
        let msg = b"list index out of range";
        let none = b"";
        // SAFETY: as above.
        unsafe {
            let n = none.as_ptr();
            pt_raise_msg(1, msg.as_ptr(), msg.len(), n, 0, n, 0);
            let lookup = 1 << 1 | 1 << 6;
            assert_eq!(
                (raised(), pt_pending_is(lookup), pt_pending_is(1)),
                (1, 1, 0)
            );
            let e = pt_catch();
            assert_eq!(
                uncaught_line(e as u64),
                "IndexError: list index out of range"
            );
            seq::pt_seq_dec(e);
        }
    }

    #[test]
    fn an_os_error_keeps_errno_strerror_and_filename() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
        let strerror = "No such file or directory";
        raise_errno(FILE_NOT_FOUND, 2, strerror, Some("it's"), b"", b"");
        let e = pt_catch();
        let line = "FileNotFoundError: [Errno 2] No such file or directory: \"it's\"";
        assert_eq!(uncaught_line(e as u64), line);
        raise_errno(OS_ERROR, 5, "Input/output error", None, b"", b"");
        let bare = pt_catch();
        assert_eq!(
            uncaught_line(bare as u64),
            "OSError: [Errno 5] Input/output error"
        );
        // SAFETY: each object is live until its last dec; the descriptor
        // is static.
        unsafe {
            let list = seq::pt_seq_new(b"lx".as_ptr(), 2, 2);
            seq::pt_seq_push(list, e as u64);
            seq::pt_seq_push(list, bare as u64);
            seq::pt_seq_dec(e);
            seq::pt_seq_dec(bare);
            let shown = seq::pt_str_from_seq(list);
            let want = "[FileNotFoundError(2, 'No such file or directory'), \
                        OSError(5, 'Input/output error')]";
            assert_eq!(pstr::borrow(shown).text(), want);
            pstr::pt_str_dec(shown);
            seq::pt_seq_dec(list);
        }
    }

    #[test]
    fn a_key_error_message_is_its_arguments_repr() {
        let _one = ONE_AT_A_TIME.lock().unwrap_or_else(PoisonError::into_inner);
        // SAFETY: as above; each object is live until its last dec.
        unsafe {
            let s = pstr::into_ptr(PtStr::new("k".into()));
            let e = seq::pt_seq_new(WITH_MESSAGE[9].as_ptr(), 3, 1);
            seq::pt_seq_push(e, s as u64);
            assert_eq!(uncaught_line(e as u64), "KeyError: 'k'");
            let text = pt_exc_str(e);
            assert_eq!(pstr::borrow(text).text(), "'k'");
            pstr::pt_str_dec(text);
            seq::pt_seq_dec(e);
            let empty = seq::pt_seq_new(b"x9".as_ptr(), 2, 0);
            assert_eq!(uncaught_line(empty as u64), "KeyError");
            seq::pt_seq_dec(empty);
        }
    }
}
