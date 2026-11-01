//! Bytes and files (M6a): `open(p, "rb")`, `f.read()`, `f.close()`,
//! and `hashlib.sha256(b).hexdigest()` of bytes.
//!
//! One reference-counted object type, [`PtObj`], holds either immutable
//! bytes or a file open for reading; generated code holds a `*mut PtObj`
//! per reference, owned as for strings ([`crate::str`]). A sequence slot
//! holding one has descriptor `y`. Dropping a file's last reference
//! closes it, as in CPython. Mirrors the interpreter
//! (`interp/stdlib.rs`); the corpus holds the two to one output.

#![allow(unsafe_code)] // The C ABI below.

use crate::sync::{Count, Slot};
use std::fs;
use std::io::{self, Read};
use std::sync::atomic::Ordering::Relaxed;

use crate::exc::VALUE_ERROR;
use crate::os::{site, Site};
use crate::sha256;
use crate::str::{self as pstr, PtStr};
use crate::LIVE;

/// A bytes or file object.
#[derive(Debug)]
pub struct PtObj {
    pub(crate) rc: Count,
    obj: Obj,
}

#[derive(Debug)]
enum Obj {
    Bytes(Box<[u8]>),
    /// The path it was opened by, and the open file until `close`.
    File {
        name: Box<str>,
        file: Slot<Option<fs::File>>,
    },
}

fn into_ptr(obj: Obj) -> *mut PtObj {
    LIVE.fetch_add(1, Relaxed);
    Box::into_raw(Box::new(PtObj {
        rc: Count::new(),
        obj,
    }))
}

/// Marks object `o` as reachable from another thread
/// ([`crate::sync`]); its bytes hold no references.
pub(crate) fn mark(o: &PtObj) {
    o.rc.mark();
    if let Obj::File { file, .. } = &o.obj {
        file.mark();
    }
}

/// Borrows a live object.
///
/// # Safety
///
/// `p` came from [`into_ptr`] and its count is at least 1 for the borrow.
pub(crate) unsafe fn borrow<'a>(p: *const PtObj) -> &'a PtObj {
    // SAFETY: guaranteed by the caller.
    unsafe { &*p }
}

/// Python's `repr` of `bytes`: `b'...'`, quoted as `str` is, with
/// `\t`, `\n`, `\r`, `\\` and the quote escaped and other bytes outside
/// printable ASCII as `\xhh`.
fn bytes_repr(data: &[u8]) -> String {
    let quote = if data.contains(&b'\'') && !data.contains(&b'"') {
        b'"'
    } else {
        b'\''
    };
    let mut out = String::with_capacity(data.len() + 3);
    out.push('b');
    out.push(char::from(quote));
    for &b in data {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'\n' => out.push_str("\\n"),
            b'\r' => out.push_str("\\r"),
            b'\t' => out.push_str("\\t"),
            _ if b == quote => {
                out.push('\\');
                out.push(char::from(b));
            }
            0x20..=0x7e => out.push(char::from(b)),
            _ => out.push_str(&format!("\\x{b:02x}")),
        }
    }
    out.push(char::from(quote));
    out
}

impl PtObj {
    /// Python's `repr` (and `str`) of the object.
    pub(crate) fn repr(&self) -> String {
        match &self.obj {
            Obj::Bytes(data) => bytes_repr(data),
            Obj::File { name, .. } => {
                format!("<_io.BufferedReader name={}>", pstr::repr(name))
            }
        }
    }

    /// `a == b`: bytes by content, a file only to itself.
    pub(crate) fn same(&self, other: &PtObj) -> bool {
        match (&self.obj, &other.obj) {
            (Obj::Bytes(a), Obj::Bytes(b)) => a == b,
            _ => std::ptr::eq(self, other),
        }
    }

    fn len(&self) -> usize {
        match &self.obj {
            Obj::Bytes(data) => data.len(),
            Obj::File { .. } => 0,
        }
    }
}

/// `open(p, "rb")`. A directory fails as CPython's does, though Unix
/// opens one for reading.
fn open(p: &str) -> io::Result<fs::File> {
    let file = fs::File::open(p)?;
    if file.metadata()?.is_dir() {
        return Err(is_a_directory());
    }
    Ok(file)
}

/// CPython's message for a path holding a NUL: Windows converts the
/// path to wide characters, POSIX to bytes.
const NUL: &str = if cfg!(windows) {
    "embedded null character"
} else {
    "embedded null byte"
};

#[cfg(unix)]
fn is_a_directory() -> io::Error {
    io::Error::from_raw_os_error(21) // EISDIR on Linux and macOS.
}

#[cfg(not(unix))]
fn is_a_directory() -> io::Error {
    io::Error::from(io::ErrorKind::IsADirectory)
}

/// What `f.read()` gives: the rest of the file, an `OSError`, or `None`
/// when `f` is closed (or is not a file).
fn read(f: &PtObj) -> Option<io::Result<Vec<u8>>> {
    let Obj::File { file, .. } = &f.obj else {
        return None;
    };
    let mut file = file.borrow_mut();
    let file = file.as_mut()?;
    let mut data = Vec::new();
    Some(file.read_to_end(&mut data).map(|_| data))
}

/// Adds a reference. Null is ignored.
///
/// # Safety
///
/// `p` is null or a live object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_obj_inc(p: *const PtObj) {
    if p.is_null() {
        return;
    }
    // SAFETY: non-null, so live by the caller's guarantee.
    let o = unsafe { borrow(p) };
    o.rc.inc();
}

/// Drops a reference, freeing the object at zero, which closes a file.
/// Null is ignored.
///
/// # Safety
///
/// `p` is null or a live object whose reference the caller gives up.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_obj_dec(p: *mut PtObj) {
    if p.is_null() {
        return;
    }
    // SAFETY: non-null, so live by the caller's guarantee.
    let o = unsafe { borrow(p) };
    if !o.rc.dec() {
        return;
    }
    // SAFETY: the last reference: `p` came from `Box::into_raw` and no
    // other reference remains.
    drop(unsafe { Box::from_raw(p) });
    LIVE.fetch_sub(1, Relaxed);
}

/// `len(b)` of bytes.
///
/// # Safety
///
/// `p` is a live object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_obj_len(p: *const PtObj) -> i64 {
    // SAFETY: guaranteed by the caller.
    let n = unsafe { borrow(p) }.len();
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// `a == b`: 1 or 0.
///
/// # Safety
///
/// `a` and `b` are live objects.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_obj_eq(a: *const PtObj, b: *const PtObj) -> u8 {
    // SAFETY: guaranteed by the caller.
    u8::from(unsafe { borrow(a).same(borrow(b)) })
}

/// `str(o)`, a new string: its `repr`.
///
/// # Safety
///
/// `p` is a live object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_obj_str(p: *const PtObj) -> *mut PtStr {
    // SAFETY: guaranteed by the caller.
    let text = unsafe { borrow(p) }.repr();
    pstr::into_ptr(PtStr::new(text.into_boxed_str()))
}

/// Writes `str(o)` to stdout.
///
/// # Safety
///
/// `p` is a live object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_write_obj(p: *const PtObj) {
    // SAFETY: guaranteed by the caller.
    crate::write(unsafe { borrow(p) }.repr().as_bytes());
}

/// `open(p, "rb")`: a new file, or null with an `OSError` pending.
///
/// # Safety
///
/// `p` is a live string; `head` and `tail` are as for
/// [`crate::os::pt_os_listdir`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_open(
    p: *const PtStr,
    head: *const u8,
    head_len: usize,
    tail: *const u8,
    tail_len: usize,
) -> *mut PtObj {
    // SAFETY: guaranteed by the caller.
    let (p, at) = unsafe { (pstr::borrow(p).text(), site(head, head_len, tail, tail_len)) };
    if p.contains('\0') {
        at.raise_text(VALUE_ERROR, NUL.to_string());
        return std::ptr::null_mut();
    }
    match open(p) {
        Ok(file) => into_ptr(Obj::File {
            name: p.into(),
            file: Slot::new(Some(file)),
        }),
        Err(e) => {
            at.raise(&e, Some(p));
            std::ptr::null_mut()
        }
    }
}

/// `f.read()`: new bytes, or null with an `OSError` pending, or a
/// `ValueError` when `f` is closed.
///
/// # Safety
///
/// `f` is a live file; `head` and `tail` are as for [`pt_open`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_file_read(
    f: *const PtObj,
    head: *const u8,
    head_len: usize,
    tail: *const u8,
    tail_len: usize,
) -> *mut PtObj {
    // SAFETY: guaranteed by the caller.
    let (f, at): (&PtObj, Site) = unsafe { (borrow(f), site(head, head_len, tail, tail_len)) };
    match read(f) {
        Some(Ok(data)) => into_ptr(Obj::Bytes(data.into_boxed_slice())),
        Some(Err(e)) => {
            at.raise(&e, None);
            std::ptr::null_mut()
        }
        None => {
            at.raise_text(VALUE_ERROR, "read of closed file".to_string());
            std::ptr::null_mut()
        }
    }
}

/// `f.close()`: closes `f` now; closing it again does nothing.
///
/// # Safety
///
/// `f` is a live file.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_file_close(f: *const PtObj) {
    // SAFETY: guaranteed by the caller.
    if let Obj::File { file, .. } = &unsafe { borrow(f) }.obj {
        file.borrow_mut().take();
    }
}

/// `hashlib.sha256(b).hexdigest()` of bytes `b`: a new string.
///
/// # Safety
///
/// `b` is live bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_sha256_hex(b: *const PtObj) -> *mut PtStr {
    // SAFETY: guaranteed by the caller.
    let data: &[u8] = match &unsafe { borrow(b) }.obj {
        Obj::Bytes(data) => data,
        Obj::File { .. } => &[],
    };
    let hex = sha256::hex(&sha256::digest(data));
    pstr::into_ptr(PtStr::new(hex.into_boxed_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_repr_matches_cpython() {
        assert_eq!(bytes_repr(b"ab\x00\xff\"'\n"), r#"b'ab\x00\xff"\'\n'"#);
        assert_eq!(bytes_repr(b"it's"), r#"b"it's""#);
        assert_eq!(bytes_repr(b"\\\t\r"), r"b'\\\t\r'");
        assert_eq!(bytes_repr(b""), "b''");
    }

    #[test]
    fn a_file_reads_once_then_empty_and_fails_closed() {
        let path = std::env::temp_dir().join(format!("pythrust-obj-{}", std::process::id()));
        let Ok(()) = fs::write(&path, b"hi\n") else {
            panic!("temp file")
        };
        let Ok(file) = open(&path.to_string_lossy()) else {
            panic!("opens")
        };
        let f = PtObj {
            rc: Count::new(),
            obj: Obj::File {
                name: "x".into(),
                file: Slot::new(Some(file)),
            },
        };
        assert_eq!(read(&f).and_then(Result::ok), Some(b"hi\n".to_vec()));
        assert_eq!(read(&f).and_then(Result::ok), Some(Vec::new()));
        assert_eq!(f.repr(), "<_io.BufferedReader name='x'>");
        // SAFETY: `f` is live for the call.
        unsafe { pt_file_close(&f) };
        assert!(read(&f).is_none());
        let _ = fs::remove_file(&path);
    }

    #[cfg(unix)]
    #[test]
    fn opening_a_directory_or_a_missing_path_fails() {
        let dir = std::env::temp_dir();
        let kind = open(&dir.to_string_lossy())
            .map(|_| ())
            .map_err(|e| e.kind());
        assert_eq!(kind, Err(io::ErrorKind::IsADirectory));
        let missing = open("/no/such/pythrust").map(|_| ()).map_err(|e| e.kind());
        assert_eq!(missing, Err(io::ErrorKind::NotFound));
    }

    #[test]
    fn bytes_compare_by_content() {
        let a = PtObj {
            rc: Count::new(),
            obj: Obj::Bytes(Box::from(*b"ab")),
        };
        let b = PtObj {
            rc: Count::new(),
            obj: Obj::Bytes(Box::from(*b"ab")),
        };
        assert!(a.same(&b) && a.len() == 2);
    }
}
