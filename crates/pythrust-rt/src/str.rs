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

use std::cmp::Ordering;
use std::fmt::Write;

use crate::text;

/// A string object. Immutable; its count is atomic once marked (see
/// `sync.rs`), when a task may reach it.
#[derive(Debug)]
pub struct PtStr {
    pub(crate) rc: crate::sync::Count,
    chars: usize,
    text: Box<str>,
}

impl PtStr {
    pub(crate) fn new(text: Box<str>) -> PtStr {
        let chars = text.chars().count();
        PtStr {
            rc: crate::sync::Count::new(),
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
        rc: crate::sync::Count::new(),
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
        rc: crate::sync::Count::new(),
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

/// Python's `repr` of a string: single quotes unless the text holds a
/// single quote and no double quote.
pub(crate) fn repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if c.is_control() => out.push_str(&control_escape(c)),
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

fn control_escape(c: char) -> String {
    let n = u32::from(c);
    if n <= 0xff {
        format!("\\x{n:02x}")
    } else {
        format!("\\u{n:04x}")
    }
}

pub(crate) fn into_ptr(s: PtStr) -> *mut PtStr {
    crate::live_inc();
    Box::into_raw(Box::new(s))
}

/// Borrows a live string.
///
/// # Safety
///
/// `p` came from `into_ptr` and its count is at least 1 for the borrow.
pub(crate) unsafe fn borrow<'a>(p: *const PtStr) -> &'a PtStr {
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
    s.rc.inc();
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
    if !unsafe { borrow(s) }.rc.dec() {
        return;
    }
    // SAFETY: the last reference: `s` came from `Box::into_raw` and no
    // other reference remains.
    drop(unsafe { Box::from_raw(s) });
    crate::live_dec();
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

/// The text of `p`, or `None` for null.
///
/// # Safety
///
/// `p` is null or a live string.
unsafe fn text_or_none<'a>(p: *const PtStr) -> Option<&'a str> {
    if p.is_null() {
        return None;
    }
    // SAFETY: guaranteed by the caller.
    Some(unsafe { borrow(p) }.text())
}

/// `s.split()` (`sep` null) or `s.split(sep)`: a new list of type
/// `desc` (`list[str]`), or null with CPython's `ValueError: empty
/// separator` pending.
///
/// # Safety
///
/// `s` is a live string and `sep` null or one; `desc` points to `len`
/// bytes of the compiler's descriptor of `list[str]` and `head`/`tail`
/// as for [`crate::os::site`], all living for the whole program.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)] // The site's four words, as `pt_os_listdir` takes them.
pub unsafe extern "C" fn pt_str_split(
    s: *const PtStr,
    sep: *const PtStr,
    desc: *const u8,
    len: usize,
    head: *const u8,
    head_len: usize,
    tail: *const u8,
    tail_len: usize,
) -> *mut crate::seq::PtSeq {
    // SAFETY: guaranteed by the caller.
    let (s, sep, desc, at) = unsafe {
        (
            borrow(s).text(),
            text_or_none(sep),
            std::slice::from_raw_parts(desc, len),
            crate::os::site(head, head_len, tail, tail_len),
        )
    };
    let parts = match sep {
        None => text::split_space(s),
        Some(sep) => match text::split(s, sep) {
            Some(parts) => parts,
            None => {
                at.raise_text(crate::exc::VALUE_ERROR, text::EMPTY_SEPARATOR.to_string());
                return std::ptr::null_mut();
            }
        },
    };
    let items = parts
        .into_iter()
        .map(|p| into_ptr(PtStr::new(p.into())) as u64)
        .collect();
    crate::seq::new_seq(desc, items)
}

/// `s.strip(chars)`, `lstrip` or `rstrip` (`side` 0, 1 or 2): a new
/// string. A null `chars` strips spaces.
///
/// # Safety
///
/// `s` is a live string and `chars` null or one.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_str_strip(
    s: *const PtStr,
    chars: *const PtStr,
    side: u8,
) -> *mut PtStr {
    // SAFETY: guaranteed by the caller.
    let (s, chars) = unsafe { (borrow(s).text(), text_or_none(chars)) };
    let side = match side {
        1 => text::Side::Left,
        2 => text::Side::Right,
        _ => text::Side::Both,
    };
    into_ptr(PtStr::new(text::strip(s, chars, side).into()))
}

/// `s.replace(old, new)`: a new string.
///
/// # Safety
///
/// `s`, `old` and `new` are live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_str_replace(
    s: *const PtStr,
    old: *const PtStr,
    new: *const PtStr,
) -> *mut PtStr {
    // SAFETY: guaranteed by the caller.
    let (s, old, new) = unsafe { (borrow(s).text(), borrow(old).text(), borrow(new).text()) };
    into_ptr(PtStr::new(text::replace(s, old, new).into()))
}

/// `sep.join(xs)` of a `list[str]`: a new string.
///
/// # Safety
///
/// `sep` is a live string and `xs` a live list of live strings.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_str_join(
    sep: *const PtStr,
    xs: *const crate::seq::PtSeq,
) -> *mut PtStr {
    // SAFETY: guaranteed by the caller.
    let (sep, xs) = unsafe { (borrow(sep).text(), &*xs) };
    let items = xs.items.borrow();
    // SAFETY: each item of a `list[str]` is a live string.
    let parts: Vec<&str> = items
        .iter()
        .map(|&p| unsafe { borrow(p as *const PtStr) }.text())
        .collect();
    into_ptr(PtStr::new(parts.join(sep).into()))
}

/// `str(n)` for an int.
#[unsafe(no_mangle)]
pub extern "C" fn pt_str_from_int(n: i64) -> *mut PtStr {
    into_ptr(PtStr::new(decimal(n).into_boxed_str()))
}

/// `n` in decimal, allocated once at its exact length, so boxing it does
/// not reallocate. `to_string` then `into` would shrink by `realloc`, which in glibc takes its arena's
/// lock where `malloc` and `free` mostly do not; tasks building strings
/// at once wait on it (M6c step 4c, `docs/benchmarks.md`).
fn decimal(n: i64) -> String {
    let digits = n
        .unsigned_abs()
        .checked_ilog10()
        .map_or(1, |d| d as usize + 1);
    let mut text = String::with_capacity(digits + usize::from(n < 0));
    // `String`'s `fmt::Write` never errors, and `i64`'s `Display` fails
    // only if the writer does.
    let _ = write!(text, "{n}");
    text
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

/// `repr(s)`, a new string.
///
/// # Safety
///
/// `s` is a live string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_str_repr(s: *const PtStr) -> *mut PtStr {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { borrow(s) };
    into_ptr(PtStr::new(repr(s.text()).into()))
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
    fn repr_quotes_and_escapes_as_python_does() {
        assert_eq!(repr("x"), "'x'");
        assert_eq!(repr("it's"), "\"it's\"");
        assert_eq!(repr("a\n\\'\""), "'a\\n\\\\\\'\"'");
        assert_eq!(repr("\u{7}"), "'\\x07'");
    }

    #[test]
    fn str_from_int_at_the_bounds() {
        let cases = [
            (0, "0"),
            (7, "7"),
            (-7, "-7"),
            (1_000_000_000_000_000_000, "1000000000000000000"),
            (-1_000_000_000_000_000_000, "-1000000000000000000"),
            (i64::MAX, "9223372036854775807"),
            (i64::MIN, "-9223372036854775808"),
        ];
        for (n, want) in cases {
            let p = pt_str_from_int(n);
            // SAFETY: `p` is a new string, freed by the dec.
            unsafe {
                assert_eq!(borrow(p).text(), want);
                assert_eq!(borrow(p).chars, want.len());
                pt_str_dec(p);
            }
        }
    }

    #[test]
    fn decimal_is_exact_at_the_bounds() {
        let bounds = [
            0,
            9,
            10,
            99,
            100,
            999_999_999_999_999_999,
            1_000_000_000_000_000_000,
        ];
        let negated = bounds.map(|n: i64| -n);
        let extremes = [i64::MAX, i64::MIN, i64::MIN + 1];
        for n in bounds.into_iter().chain(negated).chain(extremes) {
            let text = decimal(n);
            assert_eq!(text, n.to_string(), "{n}");
            assert_eq!(text.capacity(), text.len(), "{n}");
        }
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
