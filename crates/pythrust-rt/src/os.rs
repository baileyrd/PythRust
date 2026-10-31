//! The `os` module (M6a): `os.listdir` and `os.path`, over `std::fs`.
//! Mirrors the interpreter (`interp/stdlib.rs`); the corpus holds the two
//! to one output. A function that fails raises the `OSError` CPython
//! raises, made pending at the call's site, and returns a placeholder.

#![allow(unsafe_code)] // The C ABI below.

use std::{fs, io};

use crate::exc::{
    raise_errno, raise_text, FILE_NOT_FOUND, IS_A_DIRECTORY, NOT_A_DIRECTORY, OS_ERROR, PERMISSION,
};
use crate::seq::{new_seq, PtSeq};
use crate::str::{borrow, into_ptr, repr, PtStr};

/// What CPython raises for `e` on path `p`: the `OSError` subclass
/// code its kind names, its errno if the system gave one, and the
/// system's text (`No such file or directory`).
pub(crate) fn os_error(e: &io::Error) -> (u8, Option<i32>, String) {
    let code = match e.kind() {
        io::ErrorKind::NotFound => FILE_NOT_FOUND,
        io::ErrorKind::PermissionDenied => PERMISSION,
        io::ErrorKind::IsADirectory => IS_A_DIRECTORY,
        io::ErrorKind::NotADirectory => NOT_A_DIRECTORY,
        _ => OS_ERROR,
    };
    let text = e.to_string();
    let detail = text
        .rsplit_once(" (os error ")
        .map_or(text.as_str(), |(d, _)| d);
    (code, e.raw_os_error(), detail.to_string())
}

/// Where a failing call raises: the rendered diagnostic around its
/// uncaught line, as for `pt_raise`.
#[derive(Clone, Copy)]
pub(crate) struct Site {
    head: &'static [u8],
    tail: &'static [u8],
}

impl Site {
    /// Raises CPython's `OSError` for `e` on `p`, if it names one: with
    /// `errno`, `strerror` and `filename` when the system gave an errno,
    /// else a message alone (`text: 'p'`).
    pub(crate) fn raise(self, e: &io::Error, p: Option<&str>) {
        let (code, errno, detail) = os_error(e);
        match (errno, p) {
            (Some(n), _) => raise_errno(code, n, &detail, p, self.head, self.tail),
            (None, Some(p)) => self.raise_text(code, format!("{detail}: {}", repr(p))),
            (None, None) => self.raise_text(code, detail),
        }
    }

    /// Makes pending an exception of kind `code` holding `text`.
    pub(crate) fn raise_text(self, code: u8, text: String) {
        raise_text(code, text, self.head, self.tail);
    }
}

/// # Safety
///
/// `head` and `tail` point to `head_len` and `tail_len` bytes that live
/// for the whole program.
pub(crate) unsafe fn site(
    head: *const u8,
    head_len: usize,
    tail: *const u8,
    tail_len: usize,
) -> Site {
    // SAFETY: guaranteed by the caller.
    unsafe {
        Site {
            head: std::slice::from_raw_parts(head, head_len),
            tail: std::slice::from_raw_parts(tail, tail_len),
        }
    }
}

/// The entry names of directory `p`, in the order the system gives them.
/// A name that is not UTF-8 is decoded lossily (a listed deviation).
fn names(p: &str) -> io::Result<Vec<String>> {
    fs::read_dir(p)?
        .map(|e| e.map(|e| e.file_name().to_string_lossy().into_owned()))
        .collect()
}

/// `posixpath.join(a, b)`: an absolute `b` replaces `a`; a `/` joins
/// them unless `a` is empty or already ends in one.
fn join(a: &str, b: &str) -> String {
    if b.starts_with('/') {
        return b.to_string();
    }
    let sep = if a.is_empty() || a.ends_with('/') {
        ""
    } else {
        "/"
    };
    format!("{a}{sep}{b}")
}

/// `os.listdir(p)`: a new list of type `desc` (`list[str]`), or null
/// with an `OSError` pending.
///
/// # Safety
///
/// `p` is a live string; `desc` points to `len` bytes of the compiler's
/// descriptor of `list[str]` and `head`/`tail` as for [`site`], all
/// living for the whole program.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_os_listdir(
    p: *const PtStr,
    desc: *const u8,
    len: usize,
    head: *const u8,
    head_len: usize,
    tail: *const u8,
    tail_len: usize,
) -> *mut PtSeq {
    // SAFETY: guaranteed by the caller.
    let (p, desc, at) = unsafe {
        (
            borrow(p).text(),
            std::slice::from_raw_parts(desc, len),
            site(head, head_len, tail, tail_len),
        )
    };
    match names(p) {
        Ok(names) => {
            let items = names
                .into_iter()
                .map(|n| into_ptr(PtStr::new(n.into_boxed_str())) as u64)
                .collect();
            new_seq(desc, items)
        }
        Err(e) => {
            at.raise(&e, Some(p));
            std::ptr::null_mut()
        }
    }
}

/// `os.path.join(a, b)`: a new string.
///
/// # Safety
///
/// `a` and `b` are live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_os_path_join(a: *const PtStr, b: *const PtStr) -> *mut PtStr {
    // SAFETY: guaranteed by the caller.
    let (a, b) = unsafe { (borrow(a).text(), borrow(b).text()) };
    into_ptr(PtStr::new(join(a, b).into_boxed_str()))
}

/// What `os.path.isdir`, `isfile` and `islink` test: `which` is 0, 1 or
/// 2. Any error is false, as in CPython.
fn is(p: &str, which: u8) -> bool {
    match which {
        0 => fs::metadata(p).is_ok_and(|m| m.is_dir()),
        1 => fs::metadata(p).is_ok_and(|m| m.is_file()),
        _ => fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink()),
    }
}

/// `os.path.isdir(p)` (`which` 0), `isfile` (1) or `islink` (2): 1 or 0.
///
/// # Safety
///
/// `p` is a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_os_path_is(p: *const PtStr, which: u8) -> u8 {
    // SAFETY: guaranteed by the caller.
    u8::from(is(unsafe { borrow(p) }.text(), which))
}

/// `os.path.getsize(p)`, or 0 with an `OSError` pending. A size beyond
/// `i64` reads as `i64::MAX`; no file system holds one.
///
/// # Safety
///
/// `p` is a live string; `head` and `tail` are as for [`site`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_os_path_getsize(
    p: *const PtStr,
    head: *const u8,
    head_len: usize,
    tail: *const u8,
    tail_len: usize,
) -> i64 {
    // SAFETY: guaranteed by the caller.
    let (p, at) = unsafe { (borrow(p).text(), site(head, head_len, tail, tail_len)) };
    match fs::metadata(p) {
        Ok(m) => i64::try_from(m.len()).unwrap_or(i64::MAX),
        Err(e) => {
            at.raise(&e, Some(p));
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_follows_posixpath() {
        assert_eq!(join("a", "b"), "a/b");
        assert_eq!(join("a/", "b"), "a/b");
        assert_eq!(join("", "b"), "b");
        assert_eq!(join("a", ""), "a/");
        assert_eq!(join("a", "/b"), "/b");
    }

    #[cfg(unix)]
    #[test]
    fn os_errors_read_as_cpython_on_posix() {
        let missing = io::Error::from_raw_os_error(2);
        let text = "No such file or directory".to_string();
        assert_eq!(os_error(&missing), (FILE_NOT_FOUND, Some(2), text));
        let codes = [
            (13, PERMISSION),
            (20, NOT_A_DIRECTORY),
            (21, IS_A_DIRECTORY),
            (36, OS_ERROR),
        ];
        for (errno, code) in codes {
            assert_eq!(os_error(&io::Error::from_raw_os_error(errno)).0, code);
        }
        let synthetic = io::Error::from(io::ErrorKind::InvalidInput);
        assert_eq!(os_error(&synthetic).1, None);
    }

    #[test]
    fn tests_are_false_on_a_missing_path() {
        for which in 0..3 {
            assert!(!is("/no/such/path/pythrust", which));
        }
        assert!(is(".", 0) && !is(".", 1));
    }
}
