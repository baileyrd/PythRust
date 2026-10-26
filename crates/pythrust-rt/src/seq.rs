//! Lists and tuples: one reference-counted object, a vector of 64-bit
//! slots and a type descriptor saying what the slots hold.
//!
//! A slot holds an int, a float's bits, a bool or `None` (0 or 1), or a
//! pointer to a string or another sequence, which the sequence owns a
//! reference to. A list's slots are all its element type; a tuple's
//! follow its field types. Static types rule out a sequence containing
//! itself, so counts never form a cycle.
//!
//! Descriptors are a prefix code the compiler emits as read-only data:
//! `i` int, `f` float, `b` bool, `n` `None`, `s` str, `l` and the element
//! type for a list, `t`, a field count byte and the field types for a
//! tuple. Ownership follows strings: functions borrow their arguments,
//! and slots read out are new references.

#![allow(unsafe_code)] // Slots are pointers; the C ABI below.

use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::sync::atomic::Ordering::Relaxed;

use crate::str::{self as pstr, PtStr};
use crate::LIVE;

/// A list or tuple object.
#[derive(Debug)]
pub struct PtSeq {
    rc: Cell<usize>,
    /// The descriptor of this sequence's own type (`l…` or `t…`).
    desc: &'static [u8],
    items: RefCell<Vec<u64>>,
}

/// What a descriptor describes.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Int,
    Float,
    Bool,
    None,
    Str,
    List,
    Tuple,
}

fn kind(desc: &[u8]) -> Kind {
    match desc.first() {
        Some(b'f') => Kind::Float,
        Some(b'b') => Kind::Bool,
        Some(b'n') => Kind::None,
        Some(b's') => Kind::Str,
        Some(b'l') => Kind::List,
        Some(b't') => Kind::Tuple,
        _ => Kind::Int,
    }
}

/// The length of the one type `desc` starts with.
fn span(desc: &[u8]) -> usize {
    match kind(desc) {
        Kind::List => 1 + span(desc.get(1..).unwrap_or_default()),
        Kind::Tuple => {
            let n = desc.get(1).copied().unwrap_or(0);
            let fields = (0..n).fold(2, |at, _| at + span(desc.get(at..).unwrap_or_default()));
            fields.min(desc.len())
        }
        _ => 1,
    }
}

/// The descriptor of slot `k` of a sequence described by `desc`.
fn slot(desc: &'static [u8], k: usize) -> &'static [u8] {
    if kind(desc) == Kind::List {
        return desc.get(1..).unwrap_or_default();
    }
    let first = desc.get(2..).unwrap_or_default();
    (0..k).fold(first, |rest, _| rest.get(span(rest)..).unwrap_or_default())
}

/// Borrows the sequence a slot or argument points to.
///
/// # Safety
///
/// `p` is a live sequence for the borrow.
unsafe fn seq<'a>(p: u64) -> &'a PtSeq {
    // SAFETY: guaranteed by the caller.
    unsafe { &*(p as *const PtSeq) }
}

/// Borrows the string a slot points to.
///
/// # Safety
///
/// `p` is a live string for the borrow.
unsafe fn text<'a>(p: u64) -> &'a str {
    // SAFETY: guaranteed by the caller.
    unsafe { pstr::borrow(p as *const PtStr) }.text()
}

/// Adds a reference to what slot `v` of type `desc` holds, if anything.
fn inc(desc: &[u8], v: u64) {
    match kind(desc) {
        // SAFETY: a string slot holds a live string.
        Kind::Str => unsafe { pstr::pt_str_inc(v as *const PtStr) },
        // SAFETY: a sequence slot holds a live sequence.
        Kind::List | Kind::Tuple => unsafe { pt_seq_inc(v as *const PtSeq) },
        _ => {}
    }
}

/// Drops the reference slot `v` of type `desc` owns, if any.
fn dec(desc: &[u8], v: u64) {
    match kind(desc) {
        // SAFETY: a string slot owns a reference to a live string.
        Kind::Str => unsafe { pstr::pt_str_dec(v as *mut PtStr) },
        // SAFETY: a sequence slot owns a reference to a live sequence.
        Kind::List | Kind::Tuple => unsafe { pt_seq_dec(v as *mut PtSeq) },
        _ => {}
    }
}

/// `a == b` for two slots of type `desc`.
fn eq(desc: &[u8], a: u64, b: u64) -> bool {
    match kind(desc) {
        Kind::Float => f64::from_bits(a) == f64::from_bits(b),
        // SAFETY: string slots hold live strings.
        Kind::Str => unsafe { text(a) == text(b) },
        // SAFETY: sequence slots hold live sequences.
        Kind::List | Kind::Tuple => a == b || unsafe { seq_eq(seq(a), seq(b)) },
        _ => a == b,
    }
}

/// Elementwise `==`. A sequence equals itself without looking inside.
fn seq_eq(a: &PtSeq, b: &PtSeq) -> bool {
    let (x, y) = (a.items.borrow(), b.items.borrow());
    x.len() == y.len()
        && x.iter()
            .zip(y.iter())
            .enumerate()
            .all(|(k, (p, q))| eq(slot(a.desc, k), *p, *q))
}

/// Orders two slots of type `desc`; `None` when unordered (a NaN).
fn cmp(desc: &[u8], a: u64, b: u64) -> Option<Ordering> {
    match kind(desc) {
        Kind::Int => Some((a as i64).cmp(&(b as i64))),
        Kind::Float => f64::from_bits(a).partial_cmp(&f64::from_bits(b)),
        // SAFETY: string slots hold live strings.
        Kind::Str => unsafe { Some(text(a).as_bytes().cmp(text(b).as_bytes())) },
        // SAFETY: sequence slots hold live sequences.
        Kind::List | Kind::Tuple => unsafe { seq_cmp(seq(a), seq(b)) },
        Kind::Bool | Kind::None => Some(a.cmp(&b)),
    }
}

/// Lexicographic, as Python: the first unequal pair decides, else length.
fn seq_cmp(a: &PtSeq, b: &PtSeq) -> Option<Ordering> {
    let (x, y) = (a.items.borrow(), b.items.borrow());
    for (k, (p, q)) in x.iter().zip(y.iter()).enumerate() {
        let desc = slot(a.desc, k);
        if !eq(desc, *p, *q) {
            return cmp(desc, *p, *q);
        }
    }
    Some(x.len().cmp(&y.len()))
}

/// Appends Python's `repr` of slot `v` of type `desc` to `out`.
fn repr(desc: &[u8], v: u64, out: &mut String) {
    match kind(desc) {
        Kind::Int => out.push_str(&(v as i64).to_string()),
        Kind::Float => out.push_str(&crate::num::repr(f64::from_bits(v))),
        Kind::Bool => out.push_str(if v != 0 { "True" } else { "False" }),
        Kind::None => out.push_str("None"),
        // SAFETY: a string slot holds a live string.
        Kind::Str => out.push_str(&pstr::repr(unsafe { text(v) })),
        // SAFETY: a sequence slot holds a live sequence.
        Kind::List | Kind::Tuple => seq_repr(unsafe { seq(v) }, out),
    }
}

fn seq_repr(s: &PtSeq, out: &mut String) {
    let items = s.items.borrow();
    let tuple = kind(s.desc) == Kind::Tuple;
    out.push(if tuple { '(' } else { '[' });
    for (k, v) in items.iter().enumerate() {
        if k > 0 {
            out.push_str(", ");
        }
        repr(slot(s.desc, k), *v, out);
    }
    let close = match (tuple, items.len()) {
        (true, 1) => ",)",
        (true, _) => ")",
        (false, _) => "]",
    };
    out.push_str(close);
}

/// Resolves a Python index (negative counts from the end) into `0..len`.
fn position(i: i64, len: usize) -> Option<usize> {
    let len = i64::try_from(len).ok()?;
    let i = if i < 0 { i.checked_add(len)? } else { i };
    usize::try_from(i)
        .ok()
        .filter(|&i| i64::try_from(i).is_ok_and(|i| i < len))
}

/// Writes `status` for the caller.
///
/// # Safety
///
/// `status` points to a writable byte.
unsafe fn report(status: *mut u8, code: u8) {
    // SAFETY: guaranteed by the caller.
    unsafe { *status = code };
}

/// A new, empty sequence described by `desc[..len]`, with room for `cap`
/// slots.
///
/// # Safety
///
/// `desc` points to `len` bytes of read-only data that live for the
/// whole program: the compiler's descriptor.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_seq_new(desc: *const u8, len: usize, cap: i64) -> *mut PtSeq {
    // SAFETY: guaranteed by the caller.
    let desc: &'static [u8] = unsafe { std::slice::from_raw_parts(desc, len) };
    let cap = usize::try_from(cap).unwrap_or(0);
    LIVE.fetch_add(1, Relaxed);
    Box::into_raw(Box::new(PtSeq {
        rc: Cell::new(1),
        desc,
        items: RefCell::new(Vec::with_capacity(cap)),
    }))
}

/// Appends `v`, adding a reference to what it points to: list literals,
/// tuple construction and `append`.
///
/// # Safety
///
/// `s` is a live sequence; `v` is a live value of its next slot's type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_seq_push(s: *const PtSeq, v: u64) {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { seq(s as u64) };
    let mut items = s.items.borrow_mut();
    inc(slot(s.desc, items.len()), v);
    items.push(v);
}

/// `len(s)`.
///
/// # Safety
///
/// `s` is a live sequence.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_seq_len(s: *const PtSeq) -> i64 {
    // SAFETY: guaranteed by the caller.
    let n = unsafe { seq(s as u64) }.items.borrow().len();
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// `s[i]`, a new reference; `*status` is 1, and the result 0, when `i`
/// is out of range.
///
/// # Safety
///
/// `s` is a live sequence and `status` points to a writable byte.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_seq_get(s: *const PtSeq, i: i64, status: *mut u8) -> u64 {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { seq(s as u64) };
    let items = s.items.borrow();
    let Some(k) = position(i, items.len()) else {
        // SAFETY: guaranteed by the caller.
        unsafe { report(status, 1) };
        return 0;
    };
    let v = items[k];
    inc(slot(s.desc, k), v);
    // SAFETY: guaranteed by the caller.
    unsafe { report(status, 0) };
    v
}

/// `s[i] = v` on a list: adds a reference to `v`'s target and drops the
/// old slot's. Gives 1, changing nothing, when `i` is out of range, else
/// 0.
///
/// # Safety
///
/// `s` is a live list and `v` a live value of its element type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_list_set(s: *const PtSeq, i: i64, v: u64) -> u8 {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { seq(s as u64) };
    let desc = slot(s.desc, 0);
    let old = {
        let mut items = s.items.borrow_mut();
        let Some(k) = position(i, items.len()) else {
            return 1;
        };
        inc(desc, v);
        std::mem::replace(&mut items[k], v)
    };
    // Dropped after the borrow ends: freeing may reach other sequences.
    dec(desc, old);
    0
}

/// Adds a reference. Null is ignored.
///
/// # Safety
///
/// `s` is null or a live sequence.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_seq_inc(s: *const PtSeq) {
    if s.is_null() {
        return;
    }
    // SAFETY: non-null, so live by the caller's guarantee.
    let s = unsafe { seq(s as u64) };
    s.rc.set(s.rc.get() + 1);
}

/// Drops a reference, freeing the sequence and dropping its slots' at
/// zero. Null is ignored.
///
/// # Safety
///
/// `s` is null or a live sequence whose reference the caller gives up.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_seq_dec(s: *mut PtSeq) {
    if s.is_null() {
        return;
    }
    // SAFETY: non-null, so live by the caller's guarantee.
    let count = unsafe { seq(s as u64) }.rc.get() - 1;
    if count > 0 {
        // SAFETY: as above.
        unsafe { seq(s as u64) }.rc.set(count);
        return;
    }
    // SAFETY: the last reference: `s` came from `Box::into_raw` and no
    // other reference remains.
    let owned = unsafe { Box::from_raw(s) };
    LIVE.fetch_sub(1, Relaxed);
    for (k, v) in owned.items.into_inner().into_iter().enumerate() {
        dec(slot(owned.desc, k), v);
    }
}

/// `a == b`.
///
/// # Safety
///
/// `a` and `b` are live sequences of one type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_seq_eq(a: *const PtSeq, b: *const PtSeq) -> u8 {
    // SAFETY: guaranteed by the caller.
    let same = a == b || unsafe { seq_eq(seq(a as u64), seq(b as u64)) };
    u8::from(same)
}

/// Orders `a` and `b`: -1, 0 or 1, or 2 when unordered (a NaN decides).
///
/// # Safety
///
/// `a` and `b` are live sequences of one type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_seq_cmp(a: *const PtSeq, b: *const PtSeq) -> i64 {
    // SAFETY: guaranteed by the caller.
    match unsafe { seq_cmp(seq(a as u64), seq(b as u64)) } {
        Some(Ordering::Less) => -1,
        Some(Ordering::Equal) => 0,
        Some(Ordering::Greater) => 1,
        None => 2,
    }
}

/// `v in s` on a list.
///
/// # Safety
///
/// `s` is a live list and `v` a live value of its element type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_list_contains(s: *const PtSeq, v: u64) -> u8 {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { seq(s as u64) };
    let desc = slot(s.desc, 0);
    u8::from(s.items.borrow().iter().any(|x| eq(desc, *x, v)))
}

/// Writes Python's `repr` of `s`, as `print` does.
///
/// # Safety
///
/// `s` is a live sequence.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_write_seq(s: *const PtSeq) {
    let mut out = String::new();
    // SAFETY: guaranteed by the caller.
    seq_repr(unsafe { seq(s as u64) }, &mut out);
    crate::write(out.as_bytes());
}

/// `str(s)`, a new string.
///
/// # Safety
///
/// `s` is a live sequence.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_str_from_seq(s: *const PtSeq) -> *mut PtStr {
    let mut out = String::new();
    // SAFETY: guaranteed by the caller.
    seq_repr(unsafe { seq(s as u64) }, &mut out);
    pstr::into_ptr(PtStr::new(out.into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A new sequence of type `desc` holding `items`.
    fn make(desc: &'static [u8], items: &[u64]) -> *mut PtSeq {
        // SAFETY: `desc` is static; the items match it.
        unsafe {
            let s = pt_seq_new(desc.as_ptr(), desc.len(), 0);
            items.iter().for_each(|v| pt_seq_push(s, *v));
            s
        }
    }

    fn shown(s: *const PtSeq) -> String {
        let mut out = String::new();
        // SAFETY: `s` is live in every test.
        seq_repr(unsafe { seq(s as u64) }, &mut out);
        out
    }

    #[test]
    fn descriptors_locate_tuple_fields() {
        let desc: &'static [u8] = b"t\x03lisf";
        assert_eq!(slot(desc, 0), b"lisf");
        assert_eq!(slot(desc, 1), b"sf");
        assert_eq!(slot(desc, 2), b"f");
        assert_eq!(span(b"t\x02lit\x01fs"), 7);
    }

    #[test]
    fn reprs_match_python() {
        let ints = make(b"li", &[1, (-2i64) as u64]);
        assert_eq!(shown(ints), "[1, -2]");
        let one = make(b"t\x01f", &[2.5f64.to_bits()]);
        assert_eq!(shown(one), "(2.5,)");
        let nested = make(b"lli", &[ints as u64]);
        assert_eq!(shown(nested), "[[1, -2]]");
        let words = make(b"ls", &[]);
        assert_eq!(shown(words), "[]");
        let empty = make(b"t\x00", &[]);
        assert_eq!(shown(empty), "()");
        // SAFETY: each is live and dropped once.
        unsafe {
            pt_seq_dec(nested);
            pt_seq_dec(ints);
            pt_seq_dec(words);
            pt_seq_dec(one);
            pt_seq_dec(empty);
        }
    }

    #[test]
    fn equality_and_order_are_lexicographic() {
        let a = make(b"li", &[1, 2]);
        let b = make(b"li", &[1, 3]);
        let c = make(b"li", &[1]);
        // SAFETY: all live.
        unsafe {
            assert_eq!(pt_seq_eq(a, a), 1);
            assert_eq!(pt_seq_eq(a, b), 0);
            assert_eq!(pt_seq_cmp(a, b), -1);
            assert_eq!(pt_seq_cmp(c, a), -1);
            assert_eq!(pt_list_contains(b, 3), 1);
            [a, b, c].into_iter().for_each(|s| pt_seq_dec(s));
        }
        let nan = f64::NAN.to_bits();
        let x = make(b"lf", &[nan]);
        let y = make(b"lf", &[nan]);
        // SAFETY: both live.
        unsafe {
            assert_eq!(pt_seq_eq(x, y), 0);
            assert_eq!(pt_seq_cmp(x, y), 2);
            pt_seq_dec(x);
            pt_seq_dec(y);
        }
    }

    #[test]
    fn indexing_wraps_negatives_and_reports_range() {
        let s = make(b"li", &[10, 20, 30]);
        let mut status = 9;
        // SAFETY: `s` and `status` are live.
        unsafe {
            assert_eq!((pt_seq_get(s, -1, &mut status), status), (30, 0));
            pt_seq_get(s, 3, &mut status);
            assert_eq!(status, 1);
            assert_eq!(pt_list_set(s, -3, 11), 0);
            assert_eq!((pt_seq_get(s, 0, &mut status), status), (11, 0));
            assert_eq!(pt_list_set(s, -4, 0), 1);
            pt_seq_dec(s);
        }
    }

    #[test]
    fn dropping_releases_what_slots_own() {
        let inner = make(b"li", &[1]);
        let outer = make(b"lli", &[inner as u64, inner as u64]);
        // SAFETY: live until the final decs.
        unsafe {
            assert_eq!(seq(inner as u64).rc.get(), 3);
            pt_seq_dec(outer);
            assert_eq!(seq(inner as u64).rc.get(), 1);
            pt_seq_dec(inner);
        }
    }
}
