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
//! tuple, `r` for a range (see [`crate::range`]), `x` for an exception
//! (see [`crate::exc`]). Ownership follows strings: functions borrow
//! their arguments, and slots read out are new references.

#![allow(unsafe_code)] // Slots are pointers; the C ABI below.

use std::cell::{Cell, RefCell};
use std::cmp::Ordering;
use std::sync::atomic::Ordering::Relaxed;

use crate::range::Range;
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
    Range,
    Opt,
    Exc,
}

fn kind(desc: &[u8]) -> Kind {
    match desc.first() {
        Some(b'f') => Kind::Float,
        Some(b'b') => Kind::Bool,
        Some(b'n') => Kind::None,
        Some(b's') => Kind::Str,
        Some(b'l') => Kind::List,
        Some(b't') => Kind::Tuple,
        Some(b'r') => Kind::Range,
        Some(b'o') => Kind::Opt,
        Some(b'x') => Kind::Exc,
        _ => Kind::Int,
    }
}

/// The length of the one type `desc` starts with.
fn span(desc: &[u8]) -> usize {
    match kind(desc) {
        Kind::List | Kind::Opt => 1 + span(desc.get(1..).unwrap_or_default()),
        Kind::Tuple => {
            let n = desc.get(1).copied().unwrap_or(0);
            let fields = (0..n).fold(2, |at, _| at + span(desc.get(at..).unwrap_or_default()));
            fields.min(desc.len())
        }
        _ => 1,
    }
}

/// The descriptor of slot `k` of a sequence described by `desc`. An
/// exception's own descriptor is `x`, its kind's code and the type of
/// its one slot, if it has one.
fn slot(desc: &'static [u8], k: usize) -> &'static [u8] {
    match kind(desc) {
        Kind::List => return desc.get(1..).unwrap_or_default(),
        Kind::Range => return b"i",
        Kind::Exc => return desc.get(2..).unwrap_or_default(),
        _ => {}
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

/// For an optional slot type `desc` (`o` and the inner type): the inner
/// type and whether a present value is boxed, which an `int`, `float`
/// or `bool` is, in a one-field tuple. A string, list or tuple is held
/// directly; `None` is null.
fn opt(desc: &[u8]) -> (&[u8], bool) {
    let inner = desc.get(1..).unwrap_or_default();
    let boxed = matches!(kind(inner), Kind::Int | Kind::Float | Kind::Bool);
    (inner, boxed)
}

/// The inner value of a present optional slot `v` of type `desc`.
fn present(desc: &[u8], v: u64) -> u64 {
    let (_, boxed) = opt(desc);
    if !boxed {
        return v;
    }
    // SAFETY: a present boxed optional holds a live one-field tuple.
    let cell = unsafe { seq(v) };
    cell.items.borrow().first().copied().unwrap_or(0)
}

/// The type whose count an optional slot `v` of type `desc` holds a
/// reference by: its box (a tuple) or its inner type.
fn opt_owner(desc: &[u8]) -> &[u8] {
    match opt(desc) {
        (_, true) => b"t",
        (inner, false) => inner,
    }
}

/// Adds a reference to what slot `v` of type `desc` holds, if anything.
fn inc(desc: &[u8], v: u64) {
    match kind(desc) {
        Kind::Opt if v == 0 => {}
        Kind::Opt => inc(opt_owner(desc), v),
        // SAFETY: a string slot holds a live string.
        Kind::Str => unsafe { pstr::pt_str_inc(v as *const PtStr) },
        // SAFETY: a sequence slot holds a live sequence.
        Kind::List | Kind::Tuple | Kind::Range | Kind::Exc => unsafe {
            pt_seq_inc(v as *const PtSeq)
        },
        _ => {}
    }
}

/// Drops the reference slot `v` of type `desc` owns, if any.
fn dec(desc: &[u8], v: u64) {
    let mut freed = Vec::new();
    dec_into(desc, v, &mut freed);
    free_all(freed);
}

/// Drops the reference slot `v` of type `desc` owns, if any: a string's
/// now, a sequence's by pushing it on `todo` for [`free_all`].
fn dec_into(desc: &[u8], v: u64, todo: &mut Vec<u64>) {
    match kind(desc) {
        Kind::Opt if v == 0 => {}
        Kind::Opt => dec_into(opt_owner(desc), v, todo),
        // SAFETY: a string slot owns a reference to a live string.
        Kind::Str => unsafe { pstr::pt_str_dec(v as *mut PtStr) },
        Kind::List | Kind::Tuple | Kind::Range | Kind::Exc => todo.push(v),
        _ => {}
    }
}

/// Drops one reference to each sequence in `todo`, freeing those at
/// zero and dropping what their slots own, in a loop: an exception
/// chain (`e = ValueError(e)`) can be any length, so freeing must not
/// recurse.
fn free_all(mut todo: Vec<u64>) {
    while let Some(p) = todo.pop() {
        // SAFETY: each entry is a live sequence whose reference is given up.
        let s = unsafe { seq(p) };
        let count = s.rc.get() - 1;
        if count > 0 {
            s.rc.set(count);
            continue;
        }
        // SAFETY: the last reference: `p` came from `Box::into_raw` and no
        // other reference remains.
        let owned = unsafe { Box::from_raw(p as *mut PtSeq) };
        LIVE.fetch_sub(1, Relaxed);
        for (k, v) in owned.items.into_inner().into_iter().enumerate() {
            dec_into(slot(owned.desc, k), v, &mut todo);
        }
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
        // SAFETY: range slots hold live ranges.
        Kind::Range => unsafe {
            range_fields(a as *const PtSeq).same(&range_fields(b as *const PtSeq))
        },
        Kind::Opt if a == 0 || b == 0 => a == b,
        Kind::Opt => eq(opt(desc).0, present(desc, a), present(desc, b)),
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
        // Python cannot order `None`, ranges or exceptions; the compiler
        // never asks.
        Kind::Opt | Kind::Range | Kind::Exc => None,
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

/// How deep lists, tuples and exceptions may nest inside a `repr`
/// before it is a `RecursionError`, as `run` counts them.
const MAX_NESTING: usize = 10_000;

/// A `repr` nested deeper than [`MAX_NESTING`].
#[derive(Debug, PartialEq)]
pub(crate) struct TooDeep;

/// What is left of a `repr` in progress.
enum Step {
    Show(&'static [u8], u64),
    Text(&'static str),
    /// The end of a list, tuple or exception.
    Leave,
}

/// Appends Python's `repr` of slot `v` of type `desc` to `out`. A loop
/// with its own stack, since exceptions nest to any depth.
fn repr(desc: &'static [u8], v: u64, out: &mut String) -> Result<(), TooDeep> {
    let mut todo = vec![Step::Show(desc, v)];
    let mut depth = 0;
    while let Some(step) = todo.pop() {
        let (desc, v) = match step {
            Step::Show(desc, v) => (desc, v),
            Step::Text(t) => {
                out.push_str(t);
                continue;
            }
            Step::Leave => {
                depth -= 1;
                continue;
            }
        };
        if matches!(kind(desc), Kind::List | Kind::Tuple | Kind::Exc) {
            if depth >= MAX_NESTING {
                return Err(TooDeep);
            }
            depth += 1;
            todo.push(Step::Leave);
        }
        show(desc, v, out, &mut todo);
    }
    Ok(())
}

/// Writes what of slot `v` of type `desc` comes first, and pushes the
/// rest on `todo`, last first.
fn show(desc: &'static [u8], v: u64, out: &mut String, todo: &mut Vec<Step>) {
    match kind(desc) {
        Kind::Int => out.push_str(&(v as i64).to_string()),
        Kind::Float => out.push_str(&crate::num::repr(f64::from_bits(v))),
        Kind::Bool => out.push_str(if v != 0 { "True" } else { "False" }),
        Kind::None => out.push_str("None"),
        // SAFETY: a string slot holds a live string.
        Kind::Str => out.push_str(&pstr::repr(unsafe { text(v) })),
        // SAFETY: a sequence slot holds a live sequence.
        Kind::List | Kind::Tuple => show_seq(unsafe { seq(v) }, out, todo),
        // SAFETY: a range slot holds a live range.
        Kind::Range => out.push_str(&unsafe { range_fields(v as *const PtSeq) }.repr()),
        Kind::Opt if v == 0 => out.push_str("None"),
        Kind::Opt => todo.push(Step::Show(opt(desc).0, present(desc, v))),
        // SAFETY: an exception slot holds a live exception.
        Kind::Exc => show_exc(unsafe { seq(v) }, out, todo),
    }
}

/// `[1, 2]` or `(1,)`.
fn show_seq(s: &PtSeq, out: &mut String, todo: &mut Vec<Step>) {
    let items = s.items.borrow();
    let tuple = kind(s.desc) == Kind::Tuple;
    out.push(if tuple { '(' } else { '[' });
    todo.push(Step::Text(match (tuple, items.len()) {
        (true, 1) => ",)",
        (true, _) => ")",
        (false, _) => "]",
    }));
    for (k, v) in items.iter().enumerate().rev() {
        todo.push(Step::Show(slot(s.desc, k), *v));
        if k > 0 {
            todo.push(Step::Text(", "));
        }
    }
}

/// `ValueError('bad')`, or `ValueError()` without an argument.
fn show_exc(e: &PtSeq, out: &mut String, todo: &mut Vec<Step>) {
    out.push_str(crate::exc::name(exc_kind(e as *const PtSeq as u64)));
    out.push('(');
    todo.push(Step::Text(")"));
    if let Some(v) = e.items.borrow().first() {
        todo.push(Step::Show(slot(e.desc, 0), *v));
    }
}

/// The kind code of exception `e`, a live exception.
pub(crate) fn exc_kind(e: u64) -> u8 {
    // SAFETY: guaranteed by the caller.
    let desc = unsafe { seq(e) }.desc;
    desc.get(1).map_or(u8::MAX, |c| c.wrapping_sub(b'0'))
}

/// `str(e)` of a live exception `e`: its argument as a string, or empty
/// without one. An exception argument, optional or not, gives its own
/// message, followed in a loop, as `run` does, so chains of any length
/// cannot exhaust the stack.
pub(crate) fn exc_message(mut e: u64) -> Result<String, TooDeep> {
    loop {
        // SAFETY: guaranteed by the caller, and exception slots hold live
        // exceptions.
        let s = unsafe { seq(e) };
        let Some(mut v) = s.items.borrow().first().copied() else {
            return Ok(String::new());
        };
        let mut desc = slot(s.desc, 0);
        if kind(desc) == Kind::Opt && v != 0 {
            (desc, v) = (opt(desc).0, present(desc, v));
        }
        match kind(desc) {
            Kind::Exc => e = v,
            // SAFETY: a string slot holds a live string.
            Kind::Str => return Ok(unsafe { text(v) }.to_string()),
            _ => {
                let mut out = String::new();
                repr(desc, v, &mut out)?;
                return Ok(out);
            }
        }
    }
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

/// `list(s)`: a new list with `s`'s type and elements, each gaining a
/// reference (a shallow copy).
///
/// # Safety
///
/// `s` is a live list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_list_copy(s: *const PtSeq) -> *mut PtSeq {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { seq(s as u64) };
    let items = s.items.borrow().clone();
    let desc = slot(s.desc, 0);
    items.iter().for_each(|v| inc(desc, *v));
    LIVE.fetch_add(1, Relaxed);
    Box::into_raw(Box::new(PtSeq {
        rc: Cell::new(1),
        desc: s.desc,
        items: RefCell::new(items),
    }))
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
    free_all(vec![s as u64]);
}

/// `a == b`.
///
/// # Safety
///
/// `a` and `b` are live sequences (or ranges) of one type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_seq_eq(a: *const PtSeq, b: *const PtSeq) -> u8 {
    // SAFETY: guaranteed by the caller.
    let desc = unsafe { seq(a as u64) }.desc;
    u8::from(eq(desc, a as u64, b as u64))
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

/// Python's `repr` of sequence (or range) `s`.
///
/// # Safety
///
/// `s` is a live sequence.
unsafe fn shown(s: *const PtSeq) -> Result<String, TooDeep> {
    let mut out = String::new();
    // SAFETY: guaranteed by the caller.
    repr(unsafe { seq(s as u64) }.desc, s as u64, &mut out)?;
    Ok(out)
}

/// The bounds of range `r`.
///
/// # Safety
///
/// `r` is a live range.
pub(crate) unsafe fn range_fields(r: *const PtSeq) -> Range {
    // SAFETY: guaranteed by the caller.
    let items = unsafe { seq(r as u64) }.items.borrow();
    let at = |k: usize| items.get(k).map_or(0, |v| *v as i64);
    Range {
        start: at(0),
        stop: at(1),
        step: at(2),
    }
}

/// Writes Python's `repr` of `s`, as `print` does. The compiler renders
/// a value that may hold an exception with [`pt_str_from_seq`] first,
/// so `s` never nests too deep.
///
/// # Safety
///
/// `s` is a live sequence holding no exception.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_write_seq(s: *const PtSeq) {
    // SAFETY: guaranteed by the caller.
    match unsafe { shown(s) } {
        Ok(text) => crate::write(text.as_bytes()),
        Err(TooDeep) => crate::internal("an exception printed without being rendered first"),
    }
}

/// `str(s)`, a new string; null when it nests deeper than `run` allows.
///
/// # Safety
///
/// `s` is a live sequence.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_str_from_seq(s: *const PtSeq) -> *mut PtStr {
    // SAFETY: guaranteed by the caller.
    match unsafe { shown(s) } {
        Ok(text) => pstr::into_ptr(PtStr::new(text.into())),
        Err(TooDeep) => std::ptr::null_mut(),
    }
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
        // SAFETY: `s` is live in every test.
        unsafe { super::shown(s) }.expect("shallow")
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
    fn ranges_show_and_compare_as_python_inside_lists() {
        let empty = make(b"r", &[0, 0, 1]);
        let also_empty = make(b"r", &[5, 2, 3]);
        let odd = make(b"r", &[1, 9, 2]);
        let xs = make(b"lr", &[empty as u64, odd as u64]);
        let ys = make(b"lr", &[also_empty as u64, odd as u64]);
        // SAFETY: every sequence here is live until dropped below.
        unsafe {
            assert_eq!(shown(xs), "[range(0, 0), range(1, 9, 2)]");
            assert_eq!(pt_seq_eq(empty, also_empty), 1);
            assert_eq!(pt_seq_eq(xs, ys), 1);
            assert_eq!(pt_seq_eq(empty, odd), 0);
            [xs, ys, empty, also_empty, odd]
                .into_iter()
                .for_each(|s| pt_seq_dec(s));
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

    #[test]
    fn copies_are_shallow_and_independent() {
        let inner = make(b"li", &[1]);
        let outer = make(b"lli", &[inner as u64]);
        // SAFETY: live until the final decs.
        unsafe {
            let copy = pt_list_copy(outer);
            assert_eq!(seq(inner as u64).rc.get(), 3);
            assert_eq!(pt_seq_eq(copy, outer), 1);
            pt_seq_push(copy, inner as u64);
            assert_eq!((pt_seq_len(copy), pt_seq_len(outer)), (2, 1));
            pt_seq_dec(copy);
            assert_eq!(seq(inner as u64).rc.get(), 2);
            pt_seq_dec(outer);
            pt_seq_dec(inner);
        }
    }

    #[test]
    fn exceptions_show_their_argument_and_compare_by_identity() {
        let word = pstr::into_ptr(PtStr::new("bad".into()));
        let bad = make(b"x0s", &[word as u64]);
        let bare = make(b"x1", &[]);
        let wrapped = make(b"x2x", &[bad as u64]);
        let maybe = make(b"x3ob", &[0]);
        let pair = make(
            b"x0t\x02if",
            &[make(b"t\x02if", &[7, 0.5f64.to_bits()]) as u64],
        );
        let all = make(b"lx", &[bad as u64, bare as u64, wrapped as u64]);
        assert_eq!(
            shown(all),
            "[ValueError('bad'), IndexError(), ZeroDivisionError(ValueError('bad'))]"
        );
        let messages =
            [bad, bare, wrapped, maybe, pair].map(|e| exc_message(e as u64).expect("shallow"));
        assert_eq!(messages, ["bad", "", "bad", "None", "(7, 0.5)"]);
        assert_eq!(exc_kind(wrapped as u64), 2);
        assert!(eq(b"x", bad as u64, bad as u64));
        assert!(!eq(b"x", bad as u64, bare as u64));
        // SAFETY: each is live and dropped once; the slots keep the
        // string and the inner exception alive until their owners go.
        unsafe {
            let inner = seq(pair as u64).items.borrow()[0];
            pt_seq_dec(inner as *mut PtSeq);
            pstr::pt_str_dec(word);
            [all, bad, bare, wrapped, maybe, pair]
                .into_iter()
                .for_each(|s| pt_seq_dec(s));
        }
    }

    #[test]
    fn deep_exception_chains_render_to_the_limit_and_free_in_a_loop() {
        // `e = ValueError(e)` repeatedly, as a list element: the list
        // counts toward the nesting too, as in `run`.
        let chain = |n: usize| {
            let mut e = make(b"x0", &[]);
            for _ in 1..n {
                let next = make(b"x0x", &[e as u64]);
                // SAFETY: `next` holds its own reference to `e`.
                unsafe { pt_seq_dec(e) };
                e = next;
            }
            e
        };
        for (n, fits) in [(MAX_NESTING - 1, true), (MAX_NESTING, false)] {
            let e = chain(n);
            let xs = make(b"lx", &[e as u64]);
            // SAFETY: both live until dropped here.
            unsafe {
                assert_eq!(super::shown(xs).is_ok(), fits, "{n}");
                assert_eq!(exc_message(e as u64), Ok(String::new()));
                pt_seq_dec(e);
                pt_seq_dec(xs);
            }
        }
        // A chain far deeper than any native stack frees without recursing.
        let long = chain(1_000_000);
        // SAFETY: the last reference.
        unsafe { pt_seq_dec(long) };
    }

    #[test]
    fn optional_slots_box_scalars_and_hold_references_directly() {
        let five = make(b"t\x01i", &[5]);
        let six = make(b"t\x01i", &[6]);
        let xs = make(b"loi", &[0, five as u64]);
        let ys = make(b"loi", &[0, six as u64]);
        // SAFETY: every sequence here is live until dropped below.
        unsafe {
            assert_eq!(shown(xs), "[None, 5]");
            assert_eq!(pt_seq_eq(xs, ys), 0);
            assert_eq!(pt_seq_eq(xs, xs), 1);
            assert_eq!(pt_list_contains(xs, 0), 1);
            // The list's references keep the boxes alive.
            pt_seq_dec(five);
            pt_seq_dec(six);
            assert_eq!(shown(ys), "[None, 6]");
            pt_seq_dec(xs);
            pt_seq_dec(ys);
        }
        let inner = make(b"li", &[1]);
        let nested = make(b"loli", &[inner as u64, 0]);
        assert_eq!(shown(nested), "[[1], None]");
        // SAFETY: as above.
        unsafe {
            pt_seq_dec(inner);
            pt_seq_dec(nested);
        }
    }
}
