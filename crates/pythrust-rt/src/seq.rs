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
//! (see [`crate::exc`]), `y` for bytes or a file (see [`crate::obj`]). Ownership follows strings: functions borrow
//! their arguments, and slots read out are new references.

#![allow(unsafe_code)] // Slots are pointers; the C ABI below.

use crate::sync::{Count, Slot};
use std::cmp::Ordering;
use std::sync::atomic::Ordering::Relaxed;

use crate::obj::{self, PtObj};
use crate::range::Range;
use crate::str::{self as pstr, PtStr};
use crate::table::{self, Index};
use crate::LIVE;

/// A list or tuple object.
#[derive(Debug)]
pub struct PtSeq {
    pub(crate) rc: Count,
    /// The descriptor of this sequence's own type (`l…` or `t…`).
    pub(crate) desc: &'static [u8],
    pub(crate) items: Slot<Vec<u64>>,
    /// A dict's or set's hash index ([`crate::table`]); `None` for
    /// every other sequence.
    pub(crate) index: Slot<Option<Box<Index>>>,
}

/// What a descriptor describes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Kind {
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
    Dict,
    Set,
    View,
    /// Bytes or a file ([`crate::obj`]).
    Obj,
}

pub(crate) fn kind(desc: &[u8]) -> Kind {
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
        Some(b'd') => Kind::Dict,
        Some(b'e') => Kind::Set,
        Some(b'v') => Kind::View,
        Some(b'y') => Kind::Obj,
        _ => Kind::Int,
    }
}

/// The length of the one type `desc` starts with.
fn span(desc: &[u8]) -> usize {
    match kind(desc) {
        Kind::List | Kind::Opt | Kind::Set => 1 + span(desc.get(1..).unwrap_or_default()),
        Kind::Dict => {
            let key = 1 + span(desc.get(1..).unwrap_or_default());
            (key + span(desc.get(key..).unwrap_or_default())).min(desc.len())
        }
        Kind::View => 2 + span(desc.get(2..).unwrap_or_default()),
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
/// its one slot, if it has one; an `OSError` the runtime raised has `E`
/// before it ([`errno_form`]).
pub(crate) fn slot(desc: &'static [u8], k: usize) -> &'static [u8] {
    match kind(desc) {
        Kind::List | Kind::Set => return desc.get(1..).unwrap_or_default(),
        Kind::Range => return b"i",
        Kind::Exc => {
            let arg = desc.get(2..).unwrap_or_default();
            return arg.strip_prefix(b"E").unwrap_or(arg);
        }
        Kind::View => return desc.get(2..).unwrap_or_default(),
        Kind::Dict if k.is_multiple_of(2) => return desc.get(1..).unwrap_or_default(),
        Kind::Dict => {
            let key = desc.get(1..).unwrap_or_default();
            return key.get(span(key)..).unwrap_or_default();
        }
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
pub(crate) unsafe fn seq<'a>(p: u64) -> &'a PtSeq {
    // SAFETY: guaranteed by the caller.
    unsafe { &*(p as *const PtSeq) }
}

/// Borrows the string a slot points to.
///
/// # Safety
///
/// `p` is a live string for the borrow.
pub(crate) unsafe fn text<'a>(p: u64) -> &'a str {
    // SAFETY: guaranteed by the caller.
    unsafe { pstr::borrow(p as *const PtStr) }.text()
}

/// For an optional slot type `desc` (`o` and the inner type): the inner
/// type and whether a present value is boxed, which an `int`, `float`
/// or `bool` is, in a one-field tuple. A string, list or tuple is held
/// directly; `None` is null.
pub(crate) fn opt(desc: &[u8]) -> (&[u8], bool) {
    let inner = desc.get(1..).unwrap_or_default();
    let boxed = matches!(kind(inner), Kind::Int | Kind::Float | Kind::Bool);
    (inner, boxed)
}

/// The inner value of a present optional slot `v` of type `desc`.
pub(crate) fn present(desc: &[u8], v: u64) -> u64 {
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
pub(crate) fn inc(desc: &[u8], v: u64) {
    match kind(desc) {
        Kind::Opt if v == 0 => {}
        Kind::Opt => inc(opt_owner(desc), v),
        // SAFETY: a string slot holds a live string.
        Kind::Str => unsafe { pstr::pt_str_inc(v as *const PtStr) },
        // SAFETY: an object slot holds a live object.
        Kind::Obj => unsafe { obj::pt_obj_inc(v as *const PtObj) },
        // SAFETY: a sequence slot holds a live sequence.
        Kind::List
        | Kind::Tuple
        | Kind::Range
        | Kind::Exc
        | Kind::Dict
        | Kind::Set
        | Kind::View => unsafe { pt_seq_inc(v as *const PtSeq) },
        _ => {}
    }
}

/// Drops the reference slot `v` of type `desc` owns, if any.
pub(crate) fn dec(desc: &[u8], v: u64) {
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
        // SAFETY: an object slot owns a reference to a live object.
        Kind::Obj => unsafe { obj::pt_obj_dec(v as *mut PtObj) },
        Kind::List
        | Kind::Tuple
        | Kind::Range
        | Kind::Exc
        | Kind::Dict
        | Kind::Set
        | Kind::View => todo.push(v),
        _ => {}
    }
}

/// Drops one reference to each sequence in `todo`, freeing those at
/// zero and dropping what their slots own, in a loop: an exception
/// chain (`e = ValueError(e)`) can be any length, so freeing must not
/// recurse.
fn free_all(mut todo: Vec<u64>) {
    while let Some(p) = todo.pop() {
        drop_one(p, &mut todo);
    }
}

/// Drops one reference to sequence `p`; at zero frees it, pushing the
/// sequences its slots own on `todo`. `todo` only allocates when a slot
/// holds a sequence, so dropping a count above zero, or freeing a
/// sequence of scalars and strings, allocates nothing.
fn drop_one(p: u64, todo: &mut Vec<u64>) {
    // SAFETY: `p` is a live sequence whose reference is given up.
    let s = unsafe { seq(p) };
    if !s.rc.dec() {
        return;
    }
    // SAFETY: the last reference: `p` came from `Box::into_raw` and no
    // other reference remains.
    let owned = unsafe { Box::from_raw(p as *mut PtSeq) };
    LIVE.fetch_sub(1, Relaxed);
    for (k, v) in owned.items.into_inner().into_iter().enumerate() {
        dec_into(slot(owned.desc, k), v, todo);
    }
}

/// Marks what slot `v` of type `desc` holds, and everything it reaches,
/// as reachable from another thread ([`crate::sync`]). A value already
/// marked is skipped with what it reaches, so a walk ends on a cycle and
/// never writes a mark twice. Iterative, as nesting can be deep.
pub(crate) fn mark(desc: &'static [u8], v: u64) {
    let mut todo = vec![(desc, v)];
    while let Some((desc, v)) = todo.pop() {
        mark_one(desc, v, &mut todo);
    }
}

/// Marks `v` (of slot type `desc`) before it is stored into `into` if
/// `into` is marked, keeping marking transitive: everything a marked
/// value reaches is marked, even what it gains after its own marking.
/// An unmarked `v` is reachable from this thread alone, so the walk does
/// not race.
pub(crate) fn mark_stored(into: &PtSeq, desc: &'static [u8], v: u64) {
    if into.items.is_marked() {
        mark(desc, v);
    }
}

fn mark_one(desc: &'static [u8], v: u64, todo: &mut Vec<(&'static [u8], u64)>) {
    match kind(desc) {
        Kind::Opt if v == 0 => {}
        Kind::Opt => todo.push((opt_owner(desc), v)),
        // SAFETY: a string slot holds a live string.
        Kind::Str => _ = unsafe { pstr::borrow(v as *const PtStr) }.rc.mark(),
        // SAFETY: an object slot holds a live object.
        Kind::Obj => obj::mark(unsafe { obj::borrow(v as *const PtObj) }),
        Kind::List
        | Kind::Tuple
        | Kind::Range
        | Kind::Exc
        | Kind::Dict
        | Kind::Set
        | Kind::View => {
            // SAFETY: a sequence slot holds a live sequence.
            let s = unsafe { seq(v) };
            if s.rc.mark() {
                return;
            }
            s.items.mark();
            s.index.mark();
            let items = s.items.borrow();
            todo.extend(
                items
                    .iter()
                    .enumerate()
                    .map(|(k, &item)| (slot(s.desc, k), item)),
            );
        }
        _ => {}
    }
}

/// Marks the value `v` of type `desc[..len]` and everything it reaches
/// as reachable from another thread, before a task can see it (M6c).
///
/// # Safety
///
/// `desc` points to `len` bytes of the compiler's read-only descriptor
/// data; `v` is a live value of that type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_mark(desc: *const u8, len: usize, v: u64) {
    // SAFETY: guaranteed by the caller.
    let desc: &'static [u8] = unsafe { std::slice::from_raw_parts(desc, len) };
    mark(desc, v);
}

/// `a == b` for two slots of type `desc`.
pub(crate) fn eq(desc: &[u8], a: u64, b: u64) -> bool {
    match kind(desc) {
        Kind::Float => f64::from_bits(a) == f64::from_bits(b),
        // SAFETY: string slots hold live strings.
        Kind::Str => unsafe { text(a) == text(b) },
        // SAFETY: object slots hold live objects.
        Kind::Obj => unsafe { obj::borrow(a as *const PtObj).same(obj::borrow(b as *const PtObj)) },
        // SAFETY: sequence slots hold live sequences.
        Kind::List | Kind::Tuple | Kind::View => a == b || unsafe { seq_eq(seq(a), seq(b)) },
        // SAFETY: table slots hold live tables.
        Kind::Dict | Kind::Set => a == b || unsafe { table::same(seq(a), seq(b)) },
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
pub(crate) fn cmp(desc: &[u8], a: u64, b: u64) -> Option<Ordering> {
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
        Kind::Opt | Kind::Range | Kind::Exc | Kind::Dict | Kind::Set | Kind::View | Kind::Obj => {
            None
        }
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
        let nests = matches!(
            kind(desc),
            Kind::List | Kind::Tuple | Kind::Exc | Kind::Dict | Kind::Set | Kind::View
        );
        if nests {
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
        // SAFETY: an object slot holds a live object.
        Kind::Obj => out.push_str(&unsafe { obj::borrow(v as *const PtObj) }.repr()),
        // SAFETY: a sequence slot holds a live sequence.
        Kind::List | Kind::Tuple => show_seq(unsafe { seq(v) }, out, todo),
        // SAFETY: a range slot holds a live range.
        Kind::Range => out.push_str(&unsafe { range_fields(v as *const PtSeq) }.repr()),
        Kind::Opt if v == 0 => out.push_str("None"),
        Kind::Opt => todo.push(Step::Show(opt(desc).0, present(desc, v))),
        // SAFETY: an exception slot holds a live exception.
        Kind::Exc => show_exc(unsafe { seq(v) }, out, todo),
        // SAFETY: a table or view slot holds a live one.
        Kind::Dict | Kind::Set => show_table(unsafe { seq(v) }, out, todo),
        // SAFETY: as above.
        Kind::View => show_view(unsafe { seq(v) }, out, todo),
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

/// `{1: 'a'}`, `{1, 2}`, or `set()` for an empty set.
fn show_table(t: &PtSeq, out: &mut String, todo: &mut Vec<Step>) {
    let items = t.items.borrow();
    let dict = kind(t.desc) == Kind::Dict;
    if !dict && items.is_empty() {
        out.push_str("set()");
        return;
    }
    out.push('{');
    todo.push(Step::Text("}"));
    let stride = if dict { 2 } else { 1 };
    for (k, v) in items.iter().enumerate().rev() {
        todo.push(Step::Show(slot(t.desc, k), *v));
        match (dict, k % 2) {
            (true, 1) => todo.push(Step::Text(": ")),
            _ if k >= stride => todo.push(Step::Text(", ")),
            _ => {}
        }
    }
}

/// `dict_keys([1, 2])`, `dict_values([...])` or `dict_items([...])`.
fn show_view(v: &PtSeq, out: &mut String, todo: &mut Vec<Step>) {
    let name = match v.desc.get(1) {
        Some(b'k') => "dict_keys([",
        Some(b'v') => "dict_values([",
        _ => "dict_items([",
    };
    out.push_str(name);
    todo.push(Step::Text("])"));
    for (k, item) in v.items.borrow().iter().enumerate().rev() {
        todo.push(Step::Show(slot(v.desc, k), *item));
        if k > 0 {
            todo.push(Step::Text(", "));
        }
    }
}

/// `ValueError('bad')`, or `ValueError()` without an argument.
fn show_exc(e: &PtSeq, out: &mut String, todo: &mut Vec<Step>) {
    out.push_str(crate::exc::name(exc_kind(e as *const PtSeq as u64)));
    out.push('(');
    if let Some(os) = errno_form(e) {
        out.push_str(&format!("{}, {})", os.errno, pstr::repr(os.strerror)));
        return;
    }
    todo.push(Step::Text(")"));
    if let Some(v) = e.items.borrow().first() {
        todo.push(Step::Show(slot(e.desc, 0), *v));
    }
}

/// The arguments of an `OSError` the runtime raised: CPython's
/// `errno`, `strerror` and `filename`.
pub(crate) struct Errno<'a> {
    pub errno: i64,
    pub strerror: &'a str,
    pub filename: Option<&'a str>,
}

impl Errno<'_> {
    /// `str(e)`: `[Errno 2] No such file or directory: 'p'`.
    fn message(&self) -> String {
        let path = self.filename.map(|p| format!(": {}", pstr::repr(p)));
        let path = path.unwrap_or_default();
        format!("[Errno {}] {}{path}", self.errno, self.strerror)
    }
}

/// `e`'s [`Errno`] arguments, if the runtime raised it from an OS error:
/// its descriptor has `E` before its slot, a tuple `(int, str, str |
/// None)`, which no program can build.
pub(crate) fn errno_form(e: &PtSeq) -> Option<Errno<'_>> {
    if e.desc.get(2) != Some(&b'E') {
        return None;
    }
    let p = e.items.borrow().first().copied()?;
    // SAFETY: an errno exception's slot holds a live `(int, str, str |
    // None)` tuple, which it keeps alive while `e` is borrowed.
    let fields = unsafe { seq(p) }.items.borrow().clone();
    let [errno, strerror, filename] = fields.as_slice() else {
        return None;
    };
    // SAFETY: as above: a live string, and a live string or null.
    let (strerror, filename) = unsafe {
        let filename = (*filename != 0).then(|| text(*filename));
        (text(*strerror), filename)
    };
    Some(Errno {
        errno: *errno as i64,
        strerror,
        filename,
    })
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
/// cannot exhaust the stack. A `KeyError`'s message is its argument's
/// `repr`, as in Python.
pub(crate) fn exc_message(mut e: u64) -> Result<String, TooDeep> {
    loop {
        // SAFETY: guaranteed by the caller, and exception slots hold live
        // exceptions.
        let s = unsafe { seq(e) };
        if let Some(os) = errno_form(s) {
            return Ok(os.message());
        }
        let Some(mut v) = s.items.borrow().first().copied() else {
            return Ok(String::new());
        };
        let mut desc = slot(s.desc, 0);
        if kind(desc) == Kind::Opt && v != 0 {
            (desc, v) = (opt(desc).0, present(desc, v));
        }
        let key_error = exc_kind(e) == crate::exc::KEY_ERROR;
        match kind(desc) {
            Kind::Exc if !key_error => e = v,
            // SAFETY: a string slot holds a live string.
            Kind::Str if !key_error => return Ok(unsafe { text(v) }.to_string()),
            _ => {
                let mut out = String::new();
                repr(desc, v, &mut out)?;
                return Ok(out);
            }
        }
    }
}

/// Resolves a Python index (negative counts from the end) into `0..len`.
pub(crate) fn position(i: i64, len: usize) -> Option<usize> {
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
    new_seq(desc, Vec::with_capacity(cap))
}

/// A new sequence of type `desc` holding `items`, whose references it
/// takes; a dict or set gets an empty index, so `items` must be empty.
pub(crate) fn new_seq(desc: &'static [u8], items: Vec<u64>) -> *mut PtSeq {
    let index = matches!(kind(desc), Kind::Dict | Kind::Set).then(Box::default);
    LIVE.fetch_add(1, Relaxed);
    Box::into_raw(Box::new(PtSeq {
        rc: Count::new(),
        desc,
        items: Slot::new(items),
        index: Slot::new(index),
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
    // A marked `s` is borrowed while `v` is marked; the walk never
    // borrows `s`, since it stops at what is already marked. Counting `v`
    // after it lands is safe: rule 4 gives this thread `s` alone.
    let k = s.items.push(v, |k| mark(slot(s.desc, k), v));
    inc(slot(s.desc, k), v);
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
    new_seq(s.desc, items)
}

/// `len(s)`.
///
/// # Safety
///
/// `s` is a live sequence.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_seq_len(s: *const PtSeq) -> i64 {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { seq(s as u64) };
    let n = s.items.borrow().len() / table::stride(s.desc);
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
    let Some((_, v)) = s.items.get(i, |k, v| inc(slot(s.desc, k), v)) else {
        // SAFETY: guaranteed by the caller.
        unsafe { report(status, 1) };
        return 0;
    };
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
    mark_stored(s, desc, v);
    let Some(old) = s.items.replace(i, v) else {
        return 1;
    };
    inc(desc, v);
    // Dropped after the swap: freeing may reach other sequences.
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
    s.rc.inc();
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
    let mut todo = Vec::new();
    drop_one(s as u64, &mut todo);
    free_all(todo);
}

/// Sorts list `s` in place by `<`, stably, as `sorted` does in `run`:
/// a merge sort that takes from the right only when strictly less.
///
/// # Safety
///
/// `s` is a live list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_list_sort(s: *const PtSeq) {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { seq(s as u64) };
    let desc = slot(s.desc, 0);
    let items = std::mem::take(&mut *s.items.borrow_mut());
    *s.items.borrow_mut() = merge_sort(desc, items);
}

fn merge_sort(desc: &[u8], mut items: Vec<u64>) -> Vec<u64> {
    if items.len() < 2 {
        return items;
    }
    let right = items.split_off(items.len() / 2);
    let (left, right) = (merge_sort(desc, items), merge_sort(desc, right));
    let mut out = Vec::with_capacity(left.len() + right.len());
    let (mut left, mut right) = (left.into_iter().peekable(), right.into_iter().peekable());
    while let (Some(l), Some(r)) = (left.peek(), right.peek()) {
        let next = match cmp(desc, *r, *l) {
            Some(Ordering::Less) => right.next(),
            _ => left.next(),
        };
        out.extend(next);
    }
    out.extend(left);
    out.extend(right);
    out
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
pub(crate) unsafe fn shown(s: *const PtSeq) -> Result<String, TooDeep> {
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
    fn marking_reaches_nested_values_once() {
        let inner = make(b"li", &[1, 2]);
        let outer = make(b"lli", &[inner as u64, inner as u64]);
        // SAFETY: both live until dropped below.
        unsafe {
            assert!(!seq(inner as u64).rc.is_marked());
            mark(b"lli", outer as u64);
            assert!(seq(outer as u64).rc.is_marked());
            assert!(seq(inner as u64).rc.is_marked());
            assert_eq!(seq(inner as u64).rc.get(), 3);
            pt_seq_dec(outer);
            pt_seq_dec(inner);
        }
    }

    /// Four threads read one marked list of lists at once, as tasks
    /// reading a shared argument do: element reads take and drop
    /// references, and comparing the list with itself takes two read
    /// borrows of it on one thread. Counts must balance.
    #[test]
    fn marked_lists_survive_concurrent_readers() {
        let inner = make(b"li", &[7, 8, 9]);
        let outer = make(b"lli", &[inner as u64; 8]);
        mark(b"lli", outer as u64);
        let p = outer as usize;
        let threads: Vec<_> = (0..4)
            .map(|_| {
                std::thread::spawn(move || {
                    let outer = p as *const PtSeq;
                    let mut status = 0;
                    for k in 0..if cfg!(miri) { 40 } else { 10_000 } {
                        // SAFETY: `outer` stays live until every thread is
                        // joined; a read slot is a new reference.
                        unsafe {
                            let got = pt_seq_get(outer, k % 8, &mut status);
                            assert_eq!((pt_seq_cmp(outer, outer), status), (0, 0));
                            pt_seq_dec(got as *mut PtSeq);
                        }
                    }
                })
            })
            .collect();
        for t in threads {
            assert!(t.join().is_ok());
        }
        // SAFETY: all threads are done; both still live.
        unsafe {
            assert_eq!(seq(inner as u64).rc.get(), 9);
            assert_eq!(seq(outer as u64).rc.get(), 1);
            pt_seq_dec(outer);
            pt_seq_dec(inner);
        }
    }

    #[test]
    fn marked_tuples_survive_concurrent_readers() {
        let inner = make(b"li", &[1, 2]);
        let pair = make(b"t\x02li", &[inner as u64, 5]) as usize;
        mark(b"t\x02li", pair as u64);
        let threads: Vec<_> = (0..4)
            .map(|_| {
                std::thread::spawn(move || {
                    let pair = pair as *const PtSeq;
                    let mut status = 0;
                    for _ in 0..if cfg!(miri) { 40 } else { 10_000 } {
                        // SAFETY: `pair` stays live until every thread is
                        // joined; a read field is a new reference.
                        unsafe {
                            let list = pt_seq_get(pair, 0, &mut status);
                            assert_eq!((pt_seq_get(pair, -1, &mut status), status), (5, 0));
                            pt_seq_dec(list as *mut PtSeq);
                        }
                    }
                })
            })
            .collect();
        for t in threads {
            assert!(t.join().is_ok());
        }
        // SAFETY: all threads are done; both still live.
        unsafe {
            assert_eq!(seq(inner as u64).rc.get(), 2);
            pt_seq_dec(pair as *mut PtSeq);
            pt_seq_dec(inner);
        }
    }

    /// A list marked, then given a fresh element, then marked again (two
    /// executor blocks) shares that element safely; and marking a new
    /// container that reaches the list while other threads read it (a
    /// submit during a running task) neither races nor loses counts.
    #[test]
    fn remarking_after_a_store_and_marking_during_reads_keep_counts() {
        let list = make(b"lli", &[]);
        mark(b"lli", list as u64);
        let fresh = make(b"li", &[1]);
        // SAFETY: both live; the push adds the list's own reference.
        unsafe { pt_seq_push(list, fresh as u64) };
        mark(b"lli", list as u64);
        let p = list as usize;
        let readers: Vec<_> = (0..3)
            .map(|_| {
                std::thread::spawn(move || {
                    let list = p as *const PtSeq;
                    let mut status = 0;
                    for _ in 0..if cfg!(miri) { 40 } else { 10_000 } {
                        // SAFETY: `list` stays live until every thread is
                        // joined; a read element is a new reference.
                        unsafe {
                            let got = pt_seq_get(list, -1, &mut status);
                            pt_seq_dec(got as *mut PtSeq);
                        }
                    }
                })
            })
            .collect();
        let outer = make(b"llli", &[list as u64]);
        mark(b"llli", outer as u64);
        for t in readers {
            assert!(t.join().is_ok());
        }
        // SAFETY: all threads are done; every list is live.
        unsafe {
            assert_eq!(seq(fresh as u64).rc.get(), 2);
            pt_seq_dec(outer);
            pt_seq_dec(list);
            assert_eq!(seq(fresh as u64).rc.get(), 1);
            pt_seq_dec(fresh);
        }
    }

    /// A value stored into a marked container is marked with what it
    /// reaches; one stored into an unmarked container is not.
    #[test]
    fn values_stored_into_marked_containers_are_marked() {
        let (a, b, c) = (make(b"li", &[1]), make(b"li", &[2]), make(b"li", &[3]));
        let outer = make(b"lli", &[a as u64]);
        let plain = make(b"lli", &[]);
        mark(b"lli", outer as u64);
        let marked = |p: *mut PtSeq| {
            // SAFETY: every list here is live until the end of the test.
            unsafe { seq(p as u64) }.rc.is_marked()
        };
        // SAFETY: all lists live; each store adds its own reference.
        unsafe {
            pt_seq_push(plain, b as u64);
            assert!(!marked(b));
            pt_seq_push(outer, b as u64);
            assert_eq!(pt_list_set(outer, 0, c as u64), 0);
            assert!(marked(b) && marked(c));
            [a, b, c, outer, plain]
                .into_iter()
                .for_each(|p| pt_seq_dec(p));
        }
    }

    /// A marked list goes to another thread, which changes it, and comes
    /// back: the join orders every count operation, so none is lost.
    #[test]
    fn a_marked_list_handed_between_threads_keeps_its_counts() {
        let inner = make(b"li", &[7]);
        let outer = make(b"lli", &[inner as u64, inner as u64]) as usize;
        mark(b"lli", outer as u64);
        let item = inner as u64;
        let worker = std::thread::spawn(move || {
            let outer = outer as *const PtSeq;
            let fresh = make(b"li", &[8]);
            // SAFETY: `outer` is live and this thread alone uses it until
            // the join; `fresh`'s reference moves into the list.
            unsafe {
                assert_eq!(pt_list_set(outer, 0, fresh as u64), 0);
                pt_seq_dec(fresh);
                pt_seq_push(outer, item);
            }
        });
        assert!(worker.join().is_ok());
        // SAFETY: the worker is done; every value is live.
        unsafe {
            assert_eq!(shown(outer as *const PtSeq), "[[8], [7], [7]]");
            assert_eq!(seq(inner as u64).rc.get(), 3);
            pt_seq_dec(outer as *mut PtSeq);
            assert_eq!(seq(inner as u64).rc.get(), 1);
            pt_seq_dec(inner);
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
    fn dropping_allocates_only_for_sequences_in_slots() {
        let inner = make(b"li", &[1]);
        let flat = make(b"ls", &[]);
        let outer = make(b"lli", &[inner as u64]);
        // SAFETY: live until each reference is dropped.
        unsafe {
            pt_seq_inc(inner);
            let mut todo = Vec::new();
            drop_one(inner as u64, &mut todo);
            drop_one(flat as u64, &mut todo);
            assert_eq!(todo.capacity(), 0);
            drop_one(outer as u64, &mut todo);
            assert_eq!(todo, vec![inner as u64]);
            free_all(todo);
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
