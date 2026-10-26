//! Strings: immutable, reference-counted, UTF-8.
//!
//! Generated code holds a `*mut PtStr` per string reference. Every
//! reference is owned: a new string starts at count 1, `pt_str_inc` adds
//! one, `pt_str_dec` drops one and frees the string at zero. Null means
//! "no string" (an unbound variable); inc and dec ignore it. The other
//! functions borrow their arguments, which must be live strings.
//!
//! Indexing and `len` count chars, as Python does. The char count is
//! kept, so ASCII strings index in O(1); others walk the bytes.

#![allow(unsafe_code)] // The C ABI below; the logic above it is safe.

use std::cell::Cell;
use std::cmp::Ordering;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};

/// Strings allocated and not yet freed. A debug runtime checks it is 0
/// when the program ends normally, so a leak fails the corpus tests.
static LIVE: AtomicUsize = AtomicUsize::new(0);

/// How many strings are live.
pub fn live() -> usize {
    LIVE.load(Relaxed)
}

/// A string object. Never shared across threads: compiled programs are
/// single-threaded.
#[derive(Debug)]
pub struct PtStr {
    rc: Cell<usize>,
    chars: usize,
    text: Box<str>,
}

impl PtStr {
    fn new(text: Box<str>) -> PtStr {
        let chars = text.chars().count();
        PtStr {
            rc: Cell::new(1),
            chars,
            text,
        }
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    fn is_ascii(&self) -> bool {
        self.chars == self.text.len()
    }
}

fn concat(a: &PtStr, b: &PtStr) -> PtStr {
    let text: String = [a.text(), b.text()].concat();
    PtStr {
        rc: Cell::new(1),
        chars: a.chars + b.chars,
        text: text.into_boxed_str(),
    }
}

/// `s * n`. A count below 1 gives `""`; `None` when the result could
/// not be allocated (Python's `MemoryError`).
fn repeat(s: &PtStr, n: i64) -> Option<PtStr> {
    let n = usize::try_from(n).unwrap_or(0);
    let n = if s.text.is_empty() { 0 } else { n };
    let bytes = s.text.len().checked_mul(n)?;
    let mut out = String::new();
    out.try_reserve_exact(bytes).ok()?;
    (0..n).for_each(|_| out.push_str(s.text()));
    Some(PtStr {
        rc: Cell::new(1),
        chars: s.chars * n,
        text: out.into_boxed_str(),
    })
}

/// `s[i]` with Python's negative indices; `None` when out of range.
fn index(s: &PtStr, i: i64) -> Option<PtStr> {
    let len = i64::try_from(s.chars).ok()?;
    let i = if i < 0 { i.checked_add(len)? } else { i };
    let i = usize::try_from(i).ok().filter(|&i| i < s.chars)?;
    let ch = match s.is_ascii() {
        true => s.text.get(i..=i)?,
        false => {
            let (start, c) = s.text.char_indices().nth(i)?;
            &s.text[start..start + c.len_utf8()]
        }
    };
    Some(PtStr::new(ch.into()))
}

fn compare(a: &PtStr, b: &PtStr) -> i64 {
    // UTF-8 byte order is code point order, which is Python's order.
    match a.text.as_bytes().cmp(b.text.as_bytes()) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

fn into_ptr(s: PtStr) -> *mut PtStr {
    LIVE.fetch_add(1, Relaxed);
    Box::into_raw(Box::new(s))
}

/// Borrows a live string.
///
/// # Safety
///
/// `p` came from `into_ptr` and its count is at least 1 for the borrow.
unsafe fn borrow<'a>(p: *const PtStr) -> &'a PtStr {
    // SAFETY: guaranteed by the caller.
    unsafe { &*p }
}

/// A new string holding a copy of `bytes[..len]`, which generated code
/// points into its read-only data. Invalid UTF-8 is replaced, never
/// trusted.
///
/// # Safety
///
/// `bytes` points to `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_str_new(bytes: *const u8, len: usize) -> *mut PtStr {
    // SAFETY: guaranteed by the caller.
    let bytes = unsafe { std::slice::from_raw_parts(bytes, len) };
    into_ptr(PtStr::new(String::from_utf8_lossy(bytes).into()))
}

/// Adds a reference. Null is ignored.
///
/// # Safety
///
/// `s` is null or a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_str_inc(s: *const PtStr) {
    if s.is_null() {
        return;
    }
    // SAFETY: non-null, so live by the caller's guarantee.
    let s = unsafe { borrow(s) };
    s.rc.set(s.rc.get() + 1);
}

/// Drops a reference, freeing the string at zero. Null is ignored.
///
/// # Safety
///
/// `s` is null or a live string whose reference the caller gives up.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_str_dec(s: *mut PtStr) {
    if s.is_null() {
        return;
    }
    // SAFETY: non-null, so live by the caller's guarantee.
    let count = unsafe { borrow(s) }.rc.get() - 1;
    if count > 0 {
        // SAFETY: as above.
        unsafe { borrow(s) }.rc.set(count);
        return;
    }
    // SAFETY: the last reference: `s` came from `Box::into_raw` and no
    // other reference remains.
    drop(unsafe { Box::from_raw(s) });
    LIVE.fetch_sub(1, Relaxed);
}

/// `a + b`.
///
/// # Safety
///
/// `a` and `b` are live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_str_concat(a: *const PtStr, b: *const PtStr) -> *mut PtStr {
    // SAFETY: guaranteed by the caller.
    let (a, b) = unsafe { (borrow(a), borrow(b)) };
    into_ptr(concat(a, b))
}

/// `s * n`, or null when too large to allocate.
///
/// # Safety
///
/// `s` is a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_str_repeat(s: *const PtStr, n: i64) -> *mut PtStr {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { borrow(s) };
    repeat(s, n).map_or(std::ptr::null_mut(), into_ptr)
}

/// `s[i]`, or null when out of range.
///
/// # Safety
///
/// `s` is a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_str_index(s: *const PtStr, i: i64) -> *mut PtStr {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { borrow(s) };
    index(s, i).map_or(std::ptr::null_mut(), into_ptr)
}

/// `len(s)`, in chars.
///
/// # Safety
///
/// `s` is a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_str_len(s: *const PtStr) -> i64 {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { borrow(s) };
    i64::try_from(s.chars).unwrap_or(i64::MAX)
}

/// -1, 0 or 1 as `a` sorts before, with or after `b`.
///
/// # Safety
///
/// `a` and `b` are live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_str_cmp(a: *const PtStr, b: *const PtStr) -> i64 {
    // SAFETY: guaranteed by the caller.
    let (a, b) = unsafe { (borrow(a), borrow(b)) };
    compare(a, b)
}

/// `needle in hay`: 1 or 0.
///
/// # Safety
///
/// `hay` and `needle` are live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_str_contains(hay: *const PtStr, needle: *const PtStr) -> u8 {
    // SAFETY: guaranteed by the caller.
    let (hay, needle) = unsafe { (borrow(hay), borrow(needle)) };
    u8::from(hay.text.contains(needle.text()))
}

/// `str(n)` for an int.
#[unsafe(no_mangle)]
pub extern "C" fn pt_str_from_int(n: i64) -> *mut PtStr {
    into_ptr(PtStr::new(n.to_string().into()))
}

/// `str(b)` for a bool: any nonzero byte is true.
#[unsafe(no_mangle)]
pub extern "C" fn pt_str_from_bool(b: u8) -> *mut PtStr {
    let text = if b != 0 { "True" } else { "False" };
    into_ptr(PtStr::new(text.into()))
}

/// `str(None)`.
#[unsafe(no_mangle)]
pub extern "C" fn pt_str_none() -> *mut PtStr {
    into_ptr(PtStr::new("None".into()))
}

/// Writes `s` to stdout.
///
/// # Safety
///
/// `s` is a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_write_str(s: *const PtStr) {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { borrow(s) };
    crate::write(s.text().as_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(text: &str) -> PtStr {
        PtStr::new(text.into())
    }

    fn text(p: Option<PtStr>) -> Option<String> {
        p.map(|p| p.text().to_string())
    }

    #[test]
    fn concat_and_repeat_track_chars() {
        let c = concat(&s("hé"), &s("llo"));
        assert_eq!((c.text(), c.chars), ("héllo", 5));
        assert_eq!(text(repeat(&s("ab"), 3)).as_deref(), Some("ababab"));
        assert_eq!(repeat(&s("é"), 2).map(|r| r.chars), Some(2));
        assert_eq!(text(repeat(&s("ab"), -2)).as_deref(), Some(""));
        assert_eq!(text(repeat(&s(""), i64::MAX)).as_deref(), Some(""));
        assert!(repeat(&s("ab"), i64::MAX).is_none());
    }

    #[test]
    fn index_counts_chars_and_wraps_negatives() {
        let ascii = s("rust");
        assert_eq!(text(index(&ascii, 0)).as_deref(), Some("r"));
        assert_eq!(text(index(&ascii, -1)).as_deref(), Some("t"));
        assert!(index(&ascii, 4).is_none());
        assert!(index(&ascii, -5).is_none());
        assert!(index(&ascii, i64::MIN).is_none());
        let wide = s("héllo");
        assert_eq!(text(index(&wide, 1)).as_deref(), Some("é"));
        assert_eq!(text(index(&wide, -1)).as_deref(), Some("o"));
        assert!(index(&s(""), 0).is_none());
    }

    #[test]
    fn compare_is_code_point_order() {
        assert_eq!(compare(&s("apple"), &s("banana")), -1);
        assert_eq!(compare(&s("Zebra"), &s("apple")), -1);
        assert_eq!(compare(&s("é"), &s("z")), 1);
        assert_eq!(compare(&s("a"), &s("a")), 0);
        assert_eq!(compare(&s("ab"), &s("a")), 1);
    }

    #[test]
    fn counts_free_at_zero() {
        let p = pt_str_from_int(-42);
        // SAFETY: `p` is live until the last dec.
        unsafe {
            assert_eq!(borrow(p).text(), "-42");
            pt_str_inc(p);
            pt_str_dec(p);
            assert_eq!(borrow(p).rc.get(), 1);
            pt_str_dec(p);
            pt_str_inc(std::ptr::null());
            pt_str_dec(std::ptr::null_mut());
        }
    }
}
