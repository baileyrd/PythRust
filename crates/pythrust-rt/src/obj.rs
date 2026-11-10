//! Bytes and files (M6a, Step 1b-5): `open(p, mode)`, a file's
//! methods, and `hashlib.sha256(b).hexdigest()` of bytes.
//!
//! One reference-counted object type, [`PtObj`], holds either immutable
//! bytes or a file ([`textio::File`]); generated code holds a `*mut
//! PtObj` per reference, owned as for strings ([`crate::str`]). A
//! sequence slot holding one has descriptor `y`. Dropping a file's last
//! reference closes it, as in CPython, and every exit flushes the files
//! still open for writing (D98). Mirrors the interpreter
//! (`interp/stdlib.rs`); the corpus holds the two to one output.

#![allow(unsafe_code)] // The C ABI below.

use crate::sync::Count;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, Weak};

use crate::exc::{UNICODE_DECODE, VALUE_ERROR};
use crate::os::{site, Site};
use crate::sha256;
use crate::str::{self as pstr, PtStr};
use crate::textio::{self, Data, FileError, Mode};

/// A bytes or file object.
#[derive(Debug)]
pub struct PtObj {
    pub(crate) rc: Count,
    obj: Obj,
}

#[derive(Debug)]
enum Obj {
    Bytes(Box<[u8]>),
    /// A file; [`WRITERS`] holds a weak reference to a writer. No file
    /// reaches a task (D97), so its lock is never contended.
    File(Arc<Mutex<textio::File>>),
}

/// The files opened for writing, flushed by every exit (D98). Entries
/// of files already dropped are pruned as files open.
static WRITERS: Mutex<Vec<Weak<Mutex<textio::File>>>> = Mutex::new(Vec::new());

/// Locks `m`; a panic while it was held cannot leave a file half-changed
/// in a way that matters more than losing its buffer.
fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Flushes every file still open for writing, dropping any error, as
/// CPython's shutdown does but for its "Exception ignored" note (D98).
pub(crate) fn flush_files() {
    let writers: Vec<_> = lock(&WRITERS).iter().filter_map(Weak::upgrade).collect();
    for w in writers {
        let _ = lock(&w).flush();
    }
}

fn into_ptr(obj: Obj) -> *mut PtObj {
    crate::live_inc();
    Box::into_raw(Box::new(PtObj {
        rc: Count::new(),
        obj,
    }))
}

/// Marks object `o` as reachable from another thread
/// ([`crate::sync`]); its bytes hold no references.
pub(crate) fn mark(o: &PtObj) {
    o.rc.mark();
}

/// A new bytes object holding `data`.
pub(crate) fn new_bytes(data: Vec<u8>) -> *mut PtObj {
    into_ptr(Obj::Bytes(data.into_boxed_slice()))
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
            Obj::File(f) => lock(f).repr(pstr::repr),
        }
    }

    /// `a == b`: bytes by content, a file only to itself.
    pub(crate) fn same(&self, other: &PtObj) -> bool {
        match (&self.obj, &other.obj) {
            (Obj::Bytes(a), Obj::Bytes(b)) => a == b,
            _ => std::ptr::eq(self, other),
        }
    }

    /// A bytes object's data; `None` for a file.
    pub(crate) fn bytes(&self) -> Option<&[u8]> {
        match &self.obj {
            Obj::Bytes(data) => Some(data),
            Obj::File(_) => None,
        }
    }

    fn len(&self) -> usize {
        match &self.obj {
            Obj::Bytes(data) => data.len(),
            Obj::File(_) => 0,
        }
    }
}

/// CPython's message for a path holding a NUL: Windows converts the
/// path to wide characters, POSIX to bytes.
const NUL: &str = if cfg!(windows) {
    "embedded null character"
} else {
    "embedded null byte"
};

/// Runs `op` on file `f`, or leaves the exception its failure is
/// pending at `at`: `None` then.
fn on_file<T>(
    f: &PtObj,
    at: Site,
    op: impl FnOnce(&mut textio::File) -> Result<T, FileError>,
) -> Option<T> {
    let Obj::File(file) = &f.obj else {
        at.raise_text(VALUE_ERROR, "I/O operation on closed file.".to_string());
        return None;
    };
    match op(&mut lock(file)) {
        Ok(v) => Some(v),
        Err(FileError::Os(e)) => {
            at.raise(&e, None);
            None
        }
        Err(FileError::Decode(msg)) => {
            at.raise_text(UNICODE_DECODE, msg);
            None
        }
        Err(FileError::Closed(msg)) => {
            at.raise_text(VALUE_ERROR, msg.to_string());
            None
        }
    }
}

/// A new string holding `text`.
fn new_str(text: String) -> *mut PtStr {
    pstr::into_ptr(PtStr::new(text.into_boxed_str()))
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
    crate::live_dec();
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

/// `open(p, mode, encoding=encoding)`: a new file, or null with an
/// `OSError` pending. `mode` is one inference accepted (D91).
///
/// # Safety
///
/// `p`, `mode` and `encoding` are live strings; `head` and `tail` are
/// as for [`crate::os::pt_os_listdir`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_open(
    p: *const PtStr,
    mode: *const PtStr,
    encoding: *const PtStr,
    head: *const u8,
    head_len: usize,
    tail: *const u8,
    tail_len: usize,
) -> *mut PtObj {
    // SAFETY: guaranteed by the caller.
    let (p, mode, encoding, at) = unsafe {
        (
            pstr::borrow(p).text(),
            pstr::borrow(mode).text(),
            pstr::borrow(encoding).text(),
            site(head, head_len, tail, tail_len),
        )
    };
    if p.contains('\0') {
        at.raise_text(VALUE_ERROR, NUL.to_string());
        return std::ptr::null_mut();
    }
    let Some(mode) = Mode::parse(mode) else {
        at.raise_text(VALUE_ERROR, format!("invalid mode: '{mode}'"));
        return std::ptr::null_mut();
    };
    let file = match textio::File::open(p, mode, encoding) {
        Ok(file) => Arc::new(Mutex::new(file)),
        Err(e) => {
            at.raise(&e, Some(p));
            return std::ptr::null_mut();
        }
    };
    if mode.kind().writes() {
        let mut writers = lock(&WRITERS);
        writers.retain(|w| w.strong_count() > 0);
        writers.push(Arc::downgrade(&file));
    }
    into_ptr(Obj::File(file))
}

/// The arguments of a file method: the file and the site of the call.
///
/// # Safety
///
/// `f` is a live file; `head` and `tail` are as for [`pt_open`].
unsafe fn file_site<'a>(
    f: *const PtObj,
    head: *const u8,
    head_len: usize,
    tail: *const u8,
    tail_len: usize,
) -> (&'a PtObj, Site) {
    // SAFETY: guaranteed by the caller.
    unsafe { (borrow(f), site(head, head_len, tail, tail_len)) }
}

/// `f.read()` of a binary file: new bytes, or null with an exception
/// pending.
///
/// # Safety
///
/// As for [`file_site`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_file_read(
    f: *const PtObj,
    head: *const u8,
    head_len: usize,
    tail: *const u8,
    tail_len: usize,
) -> *mut PtObj {
    // SAFETY: guaranteed by the caller.
    let (f, at) = unsafe { file_site(f, head, head_len, tail, tail_len) };
    match on_file(f, at, textio::File::read) {
        Some(Data::Bytes(data)) => new_bytes(data),
        Some(Data::Text(text)) => new_bytes(text.into_bytes()),
        None => std::ptr::null_mut(),
    }
}

/// `f.read()` of a text file: a new string, or null with an exception
/// pending.
///
/// # Safety
///
/// As for [`file_site`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_file_read_text(
    f: *const PtObj,
    head: *const u8,
    head_len: usize,
    tail: *const u8,
    tail_len: usize,
) -> *mut PtStr {
    // SAFETY: guaranteed by the caller.
    let (f, at) = unsafe { file_site(f, head, head_len, tail, tail_len) };
    match on_file(f, at, textio::File::read) {
        Some(Data::Text(text)) => new_str(text),
        Some(Data::Bytes(data)) => new_str(String::from_utf8_lossy(&data).into_owned()),
        None => std::ptr::null_mut(),
    }
}

/// `f.readline()` of a text file, and each step of `for line in f`: a
/// new string, `""` at the end, or null with an exception pending.
///
/// # Safety
///
/// As for [`file_site`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_file_readline(
    f: *const PtObj,
    head: *const u8,
    head_len: usize,
    tail: *const u8,
    tail_len: usize,
) -> *mut PtStr {
    // SAFETY: guaranteed by the caller.
    let (f, at) = unsafe { file_site(f, head, head_len, tail, tail_len) };
    on_file(f, at, textio::File::readline).map_or(std::ptr::null_mut(), new_str)
}

/// `f.write(s)` of a text file: how many code points it wrote, with an
/// exception pending on failure.
///
/// # Safety
///
/// `s` is a live string; the rest as for [`file_site`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_file_write_str(
    f: *const PtObj,
    s: *const PtStr,
    head: *const u8,
    head_len: usize,
    tail: *const u8,
    tail_len: usize,
) -> i64 {
    // SAFETY: guaranteed by the caller.
    let ((f, at), s) = unsafe {
        (
            file_site(f, head, head_len, tail, tail_len),
            pstr::borrow(s).text(),
        )
    };
    let n = on_file(f, at, |file| file.write_text(s)).unwrap_or(0);
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// `f.write(b)` of a binary file: how many bytes it wrote, with an
/// exception pending on failure.
///
/// # Safety
///
/// `b` is live bytes; the rest as for [`file_site`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_file_write_bytes(
    f: *const PtObj,
    b: *const PtObj,
    head: *const u8,
    head_len: usize,
    tail: *const u8,
    tail_len: usize,
) -> i64 {
    // SAFETY: guaranteed by the caller.
    let ((f, at), b) = unsafe { (file_site(f, head, head_len, tail, tail_len), borrow(b)) };
    let data = b.bytes().unwrap_or(&[]);
    let n = on_file(f, at, |file| file.write_bytes(data)).unwrap_or(0);
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// `f.flush()`, with an exception pending on failure.
///
/// # Safety
///
/// As for [`file_site`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_file_flush(
    f: *const PtObj,
    head: *const u8,
    head_len: usize,
    tail: *const u8,
    tail_len: usize,
) {
    // SAFETY: guaranteed by the caller.
    let (f, at) = unsafe { file_site(f, head, head_len, tail, tail_len) };
    on_file(f, at, textio::File::flush);
}

/// `f.close()`: flushes a writer and closes `f` now, with an `OSError`
/// pending if the flush failed; closing it again does nothing.
///
/// # Safety
///
/// As for [`file_site`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_file_close(
    f: *const PtObj,
    head: *const u8,
    head_len: usize,
    tail: *const u8,
    tail_len: usize,
) {
    // SAFETY: guaranteed by the caller.
    let (f, at) = unsafe { file_site(f, head, head_len, tail, tail_len) };
    on_file(f, at, |file| file.close().map_err(FileError::Os));
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
        Obj::File(_) => &[],
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
    fn a_writer_left_open_is_flushed_by_the_exit_flush() {
        let path = std::env::temp_dir().join(format!("pythrust-obj-{}", std::process::id()));
        let p = path.to_string_lossy().into_owned();
        let Ok(file) = textio::File::open(&p, Mode::W, "utf-8") else {
            panic!("opens")
        };
        let file = Arc::new(Mutex::new(file));
        lock(&WRITERS).push(Arc::downgrade(&file));
        let f = PtObj {
            rc: Count::new(),
            obj: Obj::File(Arc::clone(&file)),
        };
        assert_eq!(lock(&file).write_text("hi\n").ok(), Some(3));
        assert_eq!(std::fs::read(&path).ok(), Some(Vec::new()));
        flush_files();
        assert_eq!(
            std::fs::read(&path).ok(),
            Some(textio::encode("hi\n").as_bytes().to_vec())
        );
        assert!(f.repr().starts_with("<_io.TextIOWrapper name="));
        drop(f);
        let _ = std::fs::remove_file(&path);
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
