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
//! tuple, `c` for a record ([`name_at`]), `r` for a range (see [`crate::range`]), `x` for an exception
//! (see [`crate::exc`]), `y` for bytes or a file (see [`crate::obj`]). Ownership follows strings: functions borrow
//! their arguments, and slots read out are new references.

#![allow(unsafe_code)] // Slots are pointers; the C ABI below.

use crate::sync::{Count, Ref, Slot};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering::Relaxed};
use std::sync::{PoisonError, RwLock};

use crate::obj::{self, PtObj};
use crate::range::Range;
use crate::str::{self as pstr, PtStr};
use crate::table::{self, Index};
use crate::task::{self, PtFut};

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
    /// Whether `desc` holds a [`CYCLIC`] back-reference: the sequence may
    /// sit on a reference cycle, so a drop that leaves it alive makes it
    /// a collector candidate (D152).
    pub(crate) cyc: bool,
    /// Whether the collector's candidate buffer holds this sequence. A
    /// buffered sequence whose count reaches zero is emptied but not
    /// freed: the collector frees it.
    pub(crate) buffered: AtomicBool,
    /// The collector's place for this sequence in the graph it walks
    /// (its index plus one), 0 outside a collection ([`crate::gc`]).
    pub(crate) place: AtomicU32,
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
    /// A frozen record (D129): a tuple of its fields, printed with its
    /// record's and fields' names.
    Record,
    /// A non-frozen dataclass's object (D146, D150): a record's layout, but
    /// mutable, so its slots may be unset until `__init__` fills them.
    Data,
    /// A plain class's object (D144, D147): the same, compared by identity
    /// and printed only by its own `__repr__`, which the walker cannot call.
    Plain,
    Range,
    Opt,
    Exc,
    Dict,
    Set,
    View,
    /// Bytes or a file ([`crate::obj`]).
    Obj,
    /// A task's future ([`crate::task`]): only a local list of the
    /// function that submitted the tasks holds one, so it is never
    /// marked, compared, hashed or printed.
    Fut,
}

pub(crate) fn kind(desc: &[u8]) -> Kind {
    match desc.first() {
        Some(b'f') => Kind::Float,
        Some(b'b') => Kind::Bool,
        Some(b'n') => Kind::None,
        Some(b's') => Kind::Str,
        Some(b'l') => Kind::List,
        Some(b't') => Kind::Tuple,
        Some(b'c') => Kind::Record,
        Some(b'm') => Kind::Data,
        Some(b'p') => Kind::Plain,
        Some(b'r') => Kind::Range,
        Some(b'o') => Kind::Opt,
        Some(b'x') => Kind::Exc,
        Some(b'd') => Kind::Dict,
        Some(b'e') => Kind::Set,
        Some(b'v') => Kind::View,
        Some(b'y') => Kind::Obj,
        Some(b'u') => Kind::Fut,
        _ => Kind::Int,
    }
}

/// A descriptor's back-reference to an enclosing record type (D174): `^`
/// and the record's number in two 7-bit bytes, low first. A recursive
/// record's descriptor is finite this way; [`resolve`] finds the full
/// descriptor again.
const BACKREF: u8 = b'^';

/// A back-reference to a type that can close a reference cycle (D175):
/// as [`BACKREF`], but its presence in a descriptor makes the sequence a
/// candidate for the cycle collector ([`crate::gc`]).
const CYCLIC: u8 = b'&';

/// The registered full descriptors of the records back-references name,
/// by number. Written once each, before any code runs.
static DESCS: RwLock<BTreeMap<u16, &'static [u8]>> = RwLock::new(BTreeMap::new());

/// Registers `desc[..len]` as the descriptor of record number `id`.
///
/// # Safety
/// `desc` points to `len` bytes of read-only data that stay valid for the
/// whole run.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_register_desc(id: u64, desc: *const u8, len: usize) {
    // SAFETY: guaranteed by the caller.
    let desc = unsafe { std::slice::from_raw_parts(desc, len) };
    let mut descs = DESCS.write().unwrap_or_else(PoisonError::into_inner);
    descs.insert(id as u16, desc);
}

/// `desc`, or the full descriptor it back-references.
#[inline]
pub(crate) fn resolve(desc: &[u8]) -> &[u8] {
    if !matches!(desc.first(), Some(&(BACKREF | CYCLIC))) {
        return desc;
    }
    let [_, lo, hi, ..] = *desc else {
        return desc;
    };
    let id = u16::from(lo) | u16::from(hi) << 7;
    let descs = DESCS.read().unwrap_or_else(PoisonError::into_inner);
    descs.get(&id).copied().unwrap_or(desc)
}

/// The length of the one type `desc` starts with.
fn span(desc: &[u8]) -> usize {
    if matches!(desc.first(), Some(&(BACKREF | CYCLIC))) {
        return 3.min(desc.len());
    }
    match kind(desc) {
        Kind::List | Kind::Opt | Kind::Set => 1 + span(desc.get(1..).unwrap_or_default()),
        Kind::Dict => {
            let key = 1 + span(desc.get(1..).unwrap_or_default());
            (key + span(desc.get(key..).unwrap_or_default())).min(desc.len())
        }
        Kind::View => 2 + span(desc.get(2..).unwrap_or_default()),
        Kind::Tuple => fields_end(desc),
        Kind::Record | Kind::Data => names_end(desc),
        // A plain class's descriptor ends with its class's two bytes.
        Kind::Plain => (names_end(desc) + 2).min(desc.len()),
        _ => 1,
    }
}

/// Where the names of a record descriptor end: its own, then its fields'.
fn names_end(desc: &[u8]) -> usize {
    let names = usize::from(desc.get(1).copied().unwrap_or(0)) + 1;
    (0..names).fold(fields_end(desc), |at, _| name_at(desc, at).1)
}

/// Where the field types of a tuple or record descriptor end: its
/// count byte, then that many types.
fn fields_end(desc: &[u8]) -> usize {
    let n = desc.get(1).copied().unwrap_or(0);
    let fields = (0..n).fold(2, |at, _| at + span(desc.get(at..).unwrap_or_default()));
    fields.min(desc.len())
}

/// The name at `at` of a record descriptor, and where it ends. A
/// record's descriptor is `c`, its field count byte and field types as a
/// tuple's, then its own name and its fields' names last first (the
/// order [`show_record`] pushes them in), each a length byte and its
/// UTF-8 bytes.
fn name_at(desc: &[u8], at: usize) -> (&str, usize) {
    let end = (at + 1 + usize::from(desc.get(at).copied().unwrap_or(0))).min(desc.len());
    let name = desc
        .get(at + 1..end)
        .and_then(|b| std::str::from_utf8(b).ok());
    (name.unwrap_or_default(), end)
}

/// The descriptor of slot `k` of a sequence described by `desc`. An
/// exception's own descriptor is `x`, its kind's code and the type of
/// its one slot, if it has one; an `OSError` the runtime raised has `E`
/// before it ([`errno_form`]). A program's own class (D143) has its
/// base's code and its two names ([`exc_names`]) before that type.
pub(crate) fn slot(desc: &'static [u8], k: usize) -> &'static [u8] {
    resolve(slot_at(desc, k))
}

/// [`slot`], before a back-reference is followed.
fn slot_at(desc: &'static [u8], k: usize) -> &'static [u8] {
    match kind(desc) {
        Kind::List | Kind::Set => return desc.get(1..).unwrap_or_default(),
        Kind::Range => return b"i",
        Kind::Exc => {
            let arg = desc.get(exc_arg_at(desc)..).unwrap_or_default();
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
    let inner = resolve(desc.get(1..).unwrap_or_default());
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
        // SAFETY: a future slot holds a handle to a live future.
        Kind::Fut => unsafe { task::pt_fut_inc(v as *const PtFut) },
        // SAFETY: a sequence slot holds a live sequence.
        Kind::List
        | Kind::Tuple
        | Kind::Record
        | Kind::Data
        | Kind::Plain
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
    // An object's slot is null until `__init__` stores it.
    if v == 0 {
        return;
    }
    match kind(desc) {
        Kind::Opt if v == 0 => {}
        Kind::Opt => dec_into(opt_owner(desc), v, todo),
        // SAFETY: a string slot owns a reference to a live string.
        Kind::Str => unsafe { pstr::pt_str_dec(v as *mut PtStr) },
        // SAFETY: an object slot owns a reference to a live object.
        Kind::Obj => unsafe { obj::pt_obj_dec(v as *mut PtObj) },
        // SAFETY: a future slot owns a handle to a live future.
        Kind::Fut => unsafe { task::pt_fut_dec(v as *const PtFut) },
        Kind::List
        | Kind::Tuple
        | Kind::Record
        | Kind::Data
        | Kind::Plain
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
pub(crate) fn free_all(mut todo: Vec<u64>) {
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
        if s.cyc {
            crate::gc::candidate(s, p);
        }
        return;
    }
    if s.buffered.load(Relaxed) {
        // The collector's buffer still names it: empty it, and leave
        // the freeing to the collector ([`crate::gc`]).
        let items = std::mem::take(&mut *s.items.borrow_mut());
        release(s.desc, items, todo);
        return;
    }
    // SAFETY: the last reference: `p` came from `Box::into_raw` and no
    // other reference remains.
    let owned = unsafe { Box::from_raw(p as *mut PtSeq) };
    crate::live_dec();
    release(owned.desc, owned.items.into_inner(), todo);
}

/// The types of the slots of a sequence described by `desc`, in order,
/// found by one walk of the descriptor: [`slot`] for each `k` walks it
/// from the start every time.
struct Slots {
    desc: &'static [u8],
    k: usize,
    /// The next field's type, for a tuple, record or object.
    rest: &'static [u8],
}

impl Slots {
    fn new(desc: &'static [u8]) -> Slots {
        Slots {
            desc,
            k: 0,
            rest: desc.get(2..).unwrap_or_default(),
        }
    }
}

impl Iterator for Slots {
    type Item = &'static [u8];

    fn next(&mut self) -> Option<&'static [u8]> {
        let (desc, k) = (self.desc, self.k);
        self.k += 1;
        let ty = match kind(desc) {
            Kind::List | Kind::Set => desc.get(1..).unwrap_or_default(),
            Kind::Dict => {
                let key = desc.get(1..).unwrap_or_default();
                if k % 2 == 0 {
                    key
                } else {
                    key.get(span(key)..).unwrap_or_default()
                }
            }
            Kind::Tuple | Kind::Record | Kind::Data | Kind::Plain => {
                let ty = self.rest;
                self.rest = ty.get(span(ty)..).unwrap_or_default();
                ty
            }
            _ => slot_at(desc, k),
        };
        Some(resolve(ty))
    }
}

/// Calls `f` with the index and target of each sequence reference
/// `items` hold, as slots of `s`: the edges the cycle collector follows
/// ([`crate::gc`]).
pub(crate) fn for_children(s: &PtSeq, items: &[u64], mut f: impl FnMut(usize, u64)) {
    for (k, (ty, &v)) in Slots::new(s.desc).zip(items).enumerate() {
        // Scalars, strings and bytes are the common slots.
        if matches!(ty.first(), Some(b'i' | b'f' | b'b' | b'n' | b's' | b'y')) {
            continue;
        }
        if let Some(c) = child(ty, v) {
            f(k, c);
        }
    }
}

/// The sequence slot `v` of type `desc` holds a reference to, if any: the
/// edge the cycle collector follows ([`crate::gc`]).
pub(crate) fn child(desc: &[u8], v: u64) -> Option<u64> {
    if v == 0 {
        return None;
    }
    match kind(desc) {
        Kind::Opt => child(opt_owner(desc), v),
        Kind::List
        | Kind::Tuple
        | Kind::Record
        | Kind::Data
        | Kind::Plain
        | Kind::Range
        | Kind::Exc
        | Kind::Dict
        | Kind::Set
        | Kind::View => Some(v),
        _ => None,
    }
}

/// Drops the references `items` (slots of a sequence described by `desc`)
/// own.
pub(crate) fn release(desc: &'static [u8], items: Vec<u64>, todo: &mut Vec<u64>) {
    for (ty, v) in Slots::new(desc).zip(items) {
        dec_into(ty, v, todo);
    }
}

/// [`cyclic`], remembered for the last few descriptors allocated: a
/// program allocates few types, many times.
fn cyclic_cached(desc: &'static [u8]) -> bool {
    thread_local! {
        static SEEN: std::cell::Cell<[(usize, bool); 8]> = const { std::cell::Cell::new([(0, false); 8]) };
    }
    let key = desc.as_ptr() as usize;
    let at = (key >> 3) % 8;
    SEEN.with(|seen| {
        let mut table = seen.get();
        if table[at].0 == key {
            return table[at].1;
        }
        let found = cyclic(desc);
        table[at] = (key, found);
        seen.set(table);
        found
    })
}

/// Whether a sequence of type `desc` may sit on a reference cycle: a
/// [`CYCLIC`] back-reference is among its types.
fn cyclic(desc: &[u8]) -> bool {
    let rest = |at: usize| desc.get(at..).unwrap_or_default();
    match kind(desc) {
        _ if desc.first() == Some(&CYCLIC) => true,
        Kind::List | Kind::Opt | Kind::Set => cyclic(rest(1)),
        Kind::View => cyclic(rest(2)),
        Kind::Dict => cyclic(rest(1)) || cyclic(rest(1 + span(rest(1)))),
        Kind::Tuple | Kind::Record | Kind::Data | Kind::Plain => {
            let mut at = 2;
            (0..desc.get(1).copied().unwrap_or(0)).any(|_| {
                let field = rest(at);
                at += span(field);
                cyclic(field)
            })
        }
        _ => false,
    }
}

/// Marks what slot `v` of type `desc` holds, and everything it reaches,
/// as reachable from another thread ([`crate::sync`]). A value already
/// marked is skipped with what it reaches, so a walk ends on a cycle and
/// never writes a mark twice. Iterative, as nesting can be deep.
pub(crate) fn mark(desc: &'static [u8], v: u64) {
    let mut todo = vec![(desc, v)];
    let mut held = Held::new();
    while let Some((desc, v)) = todo.pop() {
        mark_one(desc, v, &mut todo, &mut held);
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

/// Takes a container's new reference to `v`, marking `v` first when the
/// container is `marked`.
fn store(desc: &'static [u8], v: u64, marked: bool) {
    if marked {
        mark(desc, v);
    }
    inc(desc, v);
}

fn mark_one(desc: &'static [u8], v: u64, todo: &mut Vec<(&'static [u8], u64)>, held: &mut Held) {
    // An object's slot is null until `__init__` stores it.
    if v == 0 {
        return;
    }
    match kind(desc) {
        Kind::Opt if v == 0 => {}
        Kind::Opt => todo.push((opt_owner(desc), v)),
        // SAFETY: a string slot holds a live string.
        Kind::Str => _ = unsafe { pstr::borrow(v as *const PtStr) }.rc.mark(),
        // SAFETY: an object slot holds a live object.
        Kind::Obj => obj::mark(unsafe { obj::borrow(v as *const PtObj) }),
        Kind::List
        | Kind::Tuple
        | Kind::Record
        | Kind::Data
        | Kind::Plain
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
            held.push(items);
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
/// data; `v` is a live value of that type, or 0: a module value not yet
/// set, or a scalar, which has nothing to mark.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_mark(desc: *const u8, len: usize, v: u64) {
    if v == 0 || crate::sync::tsan::fault("unmarked") {
        return;
    }
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
        Kind::List | Kind::Tuple | Kind::Record | Kind::Data | Kind::View => {
            a == b || unsafe { seq_eq(seq(a), seq(b)) }
        }
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
        // Python cannot order `None`, ranges, exceptions or records; the
        // compiler never asks.
        Kind::Opt
        | Kind::Record
        | Kind::Data
        | Kind::Plain
        | Kind::Range
        | Kind::Exc
        | Kind::Dict
        | Kind::Set
        | Kind::View
        | Kind::Obj
        | Kind::Fut => None,
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

/// Read borrows of the containers a walk has queued children of, kept
/// until the walk ends: a queued child stays reachable from its borrowed
/// parent, so a writer rule 4 failed to keep out conflicts instead of
/// freeing the child before the walk uses it.
///
/// A walk borrows a child only after its parent, so releasing in
/// reverse order lets every guard go while its container is still kept
/// alive by the parent's guard: a parent released first could let a
/// writer free a child whose guard is still pending.
struct Held(Vec<Ref<'static, Vec<u64>>>);

impl Held {
    fn new() -> Self {
        Held(Vec::new())
    }

    fn push(&mut self, guard: Ref<'static, Vec<u64>>) {
        self.0.push(guard);
    }

    /// Releases the most recent borrow; false when none is left.
    fn release_last(&mut self) -> bool {
        self.0.pop().is_some()
    }
}

impl Drop for Held {
    fn drop(&mut self) {
        while self.release_last() {}
    }
}

/// What is left of a `repr` in progress.
enum Step {
    Show(&'static [u8], u64),
    Text(&'static str),
    /// The end of a list, tuple or exception; whether it is on the walk's
    /// `active` path of container that can close a cycle.
    Leave(bool),
}

/// Appends Python's `repr` of slot `v` of type `desc` to `out`. A loop
/// with its own stack, since exceptions nest to any depth.
pub(crate) fn repr(desc: &'static [u8], v: u64, out: &mut String) -> Result<(), TooDeep> {
    repr_holding(desc, v, out, &mut Held::new())
}

/// [`repr`], leaving the borrows of the containers it read in `held`.
fn repr_holding(
    desc: &'static [u8],
    v: u64,
    out: &mut String,
    held: &mut Held,
) -> Result<(), TooDeep> {
    let mut todo = vec![Step::Show(desc, v)];
    let mut depth = 0;
    // The containers being shown that can close a cycle, outermost first:
    // meeting one again shows `...` as CPython does.
    let mut active: Vec<u64> = Vec::new();
    while let Some(step) = todo.pop() {
        let (desc, v) = match step {
            Step::Show(desc, v) => (desc, v),
            Step::Text(t) => {
                out.push_str(t);
                continue;
            }
            Step::Leave(tracked) => {
                depth -= 1;
                if tracked {
                    active.pop();
                }
                continue;
            }
        };
        let nests = matches!(
            kind(desc),
            Kind::List
                | Kind::Tuple
                | Kind::Record
                | Kind::Data
                | Kind::Plain
                | Kind::Exc
                | Kind::Dict
                | Kind::Set
                | Kind::View
        );
        let tracked = nests && closes_cycles(desc, v);
        if tracked {
            if active.contains(&v) {
                out.push_str(cycle_mark(desc));
                continue;
            }
            active.push(v);
        }
        if nests {
            if depth >= MAX_NESTING {
                return Err(TooDeep);
            }
            depth += 1;
            todo.push(Step::Leave(tracked));
        }
        show(desc, v, out, &mut todo, held)?;
    }
    Ok(())
}

/// Whether slot `v` of type `desc` holds a list, tuple, record, dict or
/// set that can sit on a reference cycle.
fn closes_cycles(desc: &[u8], v: u64) -> bool {
    let kinds = [
        Kind::List,
        Kind::Tuple,
        Kind::Record,
        Kind::Data,
        Kind::Dict,
        Kind::Set,
    ];
    // SAFETY: a sequence slot holds a live sequence.
    kinds.contains(&kind(desc)) && v != 0 && unsafe { seq(v) }.cyc
}

/// What a container already being shown is shown as, as `run` does.
fn cycle_mark(desc: &[u8]) -> &'static str {
    match kind(desc) {
        Kind::List => "[...]",
        Kind::Tuple => "(...)",
        Kind::Dict | Kind::Set => "{...}",
        _ => "...",
    }
}

/// Writes what of slot `v` of type `desc` comes first, and pushes the
/// rest on `todo`, last first.
fn show(
    desc: &'static [u8],
    v: u64,
    out: &mut String,
    todo: &mut Vec<Step>,
    held: &mut Held,
) -> Result<(), TooDeep> {
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
        Kind::List | Kind::Tuple => show_seq(unsafe { seq(v) }, out, todo, held),
        // SAFETY: a record slot holds a live record.
        Kind::Record | Kind::Data => show_record(unsafe { seq(v) }, out, todo, held),
        Kind::Plain => out.push_str(&plain_repr(desc, v)?),
        // SAFETY: a range slot holds a live range.
        Kind::Range => out.push_str(&unsafe { range_fields(v as *const PtSeq) }.repr()),
        Kind::Opt if v == 0 => out.push_str("None"),
        Kind::Opt => todo.push(Step::Show(opt(desc).0, present(desc, v))),
        // SAFETY: an exception slot holds a live exception.
        Kind::Exc => show_exc(unsafe { seq(v) }, out, todo, held),
        // SAFETY: a table or view slot holds a live one.
        Kind::Dict | Kind::Set => show_table(unsafe { seq(v) }, out, todo, held),
        // SAFETY: as above.
        Kind::View => show_view(unsafe { seq(v) }, out, todo, held),
        // Inference never lets a future be printed.
        Kind::Fut => out.push_str("<Future>"),
    }
    Ok(())
}

/// A plain class's `__repr__`, as compiled: takes the thread's context and
/// a borrowed object, gives a new string; null when it raised (the
/// exception stays pending) or the call depth ran out.
pub type ReprFn = unsafe extern "C" fn(*const crate::exc::PtCtx, *const PtSeq) -> *mut PtStr;

/// The registered `__repr__`s by class (a type's number in the program).
/// Written once each, before any code runs.
static REPRS: RwLock<BTreeMap<u16, ReprFn>> = RwLock::new(BTreeMap::new());

/// Registers `f` as the `__repr__` of class `class`.
///
/// # Safety
/// `f` is a function of the [`ReprFn`] shape that stays valid for the
/// whole run.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_register_repr(class: u64, f: ReprFn) {
    let mut reprs = REPRS.write().unwrap_or_else(PoisonError::into_inner);
    reprs.insert(class as u16, f);
}

thread_local! {
    /// `__repr__` calls the walker has under way on this thread.
    static REPR_CALLS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

/// Whether a `__repr__` the walker called is running on this thread: a
/// borrow conflict then is that `__repr__` changing what is printed
/// (D172, D173).
pub(crate) fn in_repr() -> bool {
    REPR_CALLS.with(|n| n.get()) > 0
}

/// The class a plain object's descriptor names: its last two bytes.
fn class_of(desc: &[u8]) -> u16 {
    let end = span(desc);
    let (lo, hi) = (end.saturating_sub(2), end.saturating_sub(1));
    let byte = |k| u16::from(desc.get(k).copied().unwrap_or(0));
    byte(lo) | byte(hi) << 7
}

/// `repr` of plain object `v` of type `desc`: what its class's
/// `__repr__` returns. An object whose class registered none (the
/// compiler refuses those) shows as `<object>`. Failing leaves the
/// exception the call raised pending.
fn plain_repr(desc: &'static [u8], v: u64) -> Result<String, TooDeep> {
    let reprs = REPRS.read().unwrap_or_else(PoisonError::into_inner);
    let Some(f) = reprs.get(&class_of(desc)).copied() else {
        return Ok("<object>".to_string());
    };
    drop(reprs);
    REPR_CALLS.with(|n| n.set(n.get() + 1));
    // SAFETY: `f` was registered as a `__repr__`, `v` is a live object.
    let s = unsafe { f(crate::exc::pt_ctx(), v as *const PtSeq) };
    REPR_CALLS.with(|n| n.set(n.get() - 1));
    if s.is_null() {
        return Err(TooDeep);
    }
    // SAFETY: a non-null result is a live string we own.
    let text = unsafe { pstr::borrow(s) }.text().to_string();
    // SAFETY: as above; its reference is ours to give up.
    unsafe { pstr::pt_str_dec(s) };
    match crate::exc::is_pending() {
        true => Err(TooDeep),
        false => Ok(text),
    }
}

/// `[1, 2]` or `(1,)`.
fn show_seq(s: &'static PtSeq, out: &mut String, todo: &mut Vec<Step>, held: &mut Held) {
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
    held.push(items);
}

/// `P(x=1, s='a')` (D133).
fn show_record(r: &'static PtSeq, out: &mut String, todo: &mut Vec<Step>, held: &mut Held) {
    let (name, mut at) = name_at(r.desc, fields_end(r.desc));
    out.push_str(name);
    out.push('(');
    todo.push(Step::Text(")"));
    let items = r.items.borrow();
    for (k, v) in items.iter().enumerate().rev() {
        let (field, end) = name_at(r.desc, at);
        at = end;
        todo.push(Step::Show(slot(r.desc, k), *v));
        todo.push(Step::Text("="));
        todo.push(Step::Text(field));
        if k > 0 {
            todo.push(Step::Text(", "));
        }
    }
    held.push(items);
}

/// `{1: 'a'}`, `{1, 2}`, or `set()` for an empty set.
fn show_table(t: &'static PtSeq, out: &mut String, todo: &mut Vec<Step>, held: &mut Held) {
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
    held.push(items);
}

/// `dict_keys([1, 2])`, `dict_values([...])` or `dict_items([...])`.
fn show_view(v: &'static PtSeq, out: &mut String, todo: &mut Vec<Step>, held: &mut Held) {
    let name = match v.desc.get(1) {
        Some(b'k') => "dict_keys([",
        Some(b'v') => "dict_values([",
        _ => "dict_items([",
    };
    out.push_str(name);
    todo.push(Step::Text("])"));
    let items = v.items.borrow();
    for (k, item) in items.iter().enumerate().rev() {
        todo.push(Step::Show(slot(v.desc, k), *item));
        if k > 0 {
            todo.push(Step::Text(", "));
        }
    }
    held.push(items);
}

/// `ValueError('bad')`, or `ValueError()` without an argument.
fn show_exc(e: &'static PtSeq, out: &mut String, todo: &mut Vec<Step>, held: &mut Held) {
    out.push_str(exc_names(e).1);
    out.push('(');
    if let Some(os) = errno_form(e) {
        out.push_str(&format!("{}, {})", os.errno, pstr::repr(os.strerror)));
        return;
    }
    todo.push(Step::Text(")"));
    let items = e.items.borrow();
    if let Some(v) = items.first() {
        todo.push(Step::Show(slot(e.desc, 0), *v));
    }
    held.push(items);
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
    code_at(desc, 1)
}

/// The kind code at `at` of an exception descriptor.
fn code_at(desc: &[u8], at: usize) -> u8 {
    desc.get(at).map_or(u8::MAX, |c| c.wrapping_sub(b'0'))
}

/// Whether exception descriptor `desc` is of a program's own class,
/// whose code follows the builtins' (D143).
fn is_user_exc(desc: &[u8]) -> bool {
    let code = code_at(desc, 1);
    code != u8::MAX && usize::from(code) >= crate::exc::BUILTINS
}

/// Where an exception descriptor's argument type starts: after `x` and
/// the code, and for a program's own class after its base's code and
/// its two names.
fn exc_arg_at(desc: &[u8]) -> usize {
    if !is_user_exc(desc) {
        return 2;
    }
    let shown = name_at(desc, 3).1;
    name_at(desc, shown).1
}

/// The builtin class exception `e` is or derives from, by kind code.
fn exc_base(e: &PtSeq) -> u8 {
    match is_user_exc(e.desc) {
        true => code_at(e.desc, 2),
        false => code_at(e.desc, 1),
    }
}

/// Exception `e`'s class names: the one an uncaught exception shows,
/// module-qualified (`a.E`), and its own, which `repr` shows (D140,
/// D141). A builtin class has one name for both.
pub(crate) fn exc_names(e: &PtSeq) -> (&'static str, &'static str) {
    if !is_user_exc(e.desc) {
        let name = crate::exc::name(code_at(e.desc, 1));
        return (name, name);
    }
    let (shown, at) = name_at(e.desc, 3);
    (shown, name_at(e.desc, at).0)
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
        let key_error = exc_base(s) == crate::exc::KEY_ERROR;
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
    if crate::gc::due() {
        crate::gc::collect();
    }
    let index = matches!(kind(desc), Kind::Dict | Kind::Set).then(Box::default);
    crate::live_inc();
    Box::into_raw(Box::new(PtSeq {
        rc: Count::new(),
        desc,
        items: Slot::new(items),
        index: Slot::new(index),
        cyc: cyclic_cached(desc),
        buffered: AtomicBool::new(false),
        place: AtomicU32::new(0),
    }))
}

/// A new object described by `desc[..len]` (D150): `n` null slots, which
/// its `__init__` stores before anything reads or prints it (D145).
///
/// # Safety
///
/// `desc` points to `len` bytes of the compiler's read-only descriptor.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_obj_new(desc: *const u8, len: usize, n: i64) -> *mut PtSeq {
    // SAFETY: guaranteed by the caller.
    let desc: &'static [u8] = unsafe { std::slice::from_raw_parts(desc, len) };
    let n = usize::try_from(n).unwrap_or(0);
    new_seq(desc, vec![0; n])
}

/// `s.k = v` on an object: adds a reference to `v`'s target and drops the
/// old slot's, if it was stored.
///
/// # Safety
///
/// `s` is a live object whose field `k` has `v`'s type, and `v` is live.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_obj_set(s: *const PtSeq, k: i64, v: u64) {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { seq(s as u64) };
    let desc = slot(s.desc, usize::try_from(k).unwrap_or(0));
    let old = s.items.replace(k, v, |marked| store(desc, v, marked));
    // Dropped after the borrow ends: freeing may reach other sequences.
    if let Some(old) = old {
        dec(desc, old);
    }
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
    // On a busy `s`, `v` is marked and counted under the writer bit,
    // before it lands; the walk never borrows `s`, since it stops at
    // what is already marked.
    s.items
        .push(v, |k, marked| store(slot(s.desc, k), v, marked));
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
    let desc = slot(s.desc, 0);
    // Retained under the read borrow, so no writer can free one first.
    let items = s.items.borrow();
    items.iter().for_each(|v| inc(desc, *v));
    new_seq(s.desc, items.clone())
}

/// `s += t` on lists: appends `t`'s elements in order, each gaining a
/// reference. `t` may be `s`.
///
/// # Safety
///
/// `s` and `t` are live lists of one type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_list_extend(s: *const PtSeq, t: *const PtSeq) {
    // SAFETY: guaranteed by the caller.
    let (s, t) = unsafe { (seq(s as u64), seq(t as u64)) };
    // A copy first, so `xs += xs` adds each element once.
    let items = t.items.borrow().clone();
    push_all(s, &items);
}

/// `s *= n` on a list: `n` copies of its elements in place, each copy
/// gaining a reference; none for `n <= 0`. 1, changing nothing, when too
/// large to allocate; else 0.
///
/// # Safety
///
/// `s` is a live list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_list_scale(s: *const PtSeq, n: i64) -> u8 {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { seq(s as u64) };
    let n = usize::try_from(n).unwrap_or(0);
    if n == 0 {
        let old = std::mem::take(&mut *s.items.borrow_mut());
        // Dropped after the borrow ends: freeing may reach other sequences.
        let desc = slot(s.desc, 0);
        old.into_iter().for_each(|v| dec(desc, v));
        return 0;
    }
    // Repeated within the list itself: no snapshot to allocate, so only
    // the checked reservation can fail.
    let desc = slot(s.desc, 0);
    let mut items = s.items.borrow_mut();
    let len = items.len();
    if len == 0 || n == 1 {
        return 0;
    }
    let Some(more) = len.checked_mul(n - 1) else {
        return 1;
    };
    if items.try_reserve_exact(more).is_err() {
        return 1;
    }
    let marked = items.is_marked();
    for k in (0..len).cycle().take(more) {
        let v = items[k];
        store(desc, v, marked);
        items.push(v);
    }
    0
}

/// Appends `items` to list `s`, each gaining a reference.
fn push_all(s: &PtSeq, items: &[u64]) {
    let desc = slot(s.desc, 0);
    items.iter().for_each(|&v| {
        s.items.push(v, |_, marked| store(desc, v, marked));
    });
}

/// `sum(s, start)` of a list, tuple or view of floats: CPython's
/// compensated sum ([`crate::fsum`], D57).
///
/// # Safety
///
/// `s` is a live list, tuple or view whose items are floats.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_seq_fsum(s: *const PtSeq, start: f64) -> f64 {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { seq(s as u64) };
    let items = s.items.borrow();
    crate::fsum::fsum(start, items.iter().map(|&w| f64::from_bits(w)))
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

/// Makes the caller's reference `s` to a list a `for` loop's hold
/// (atomics cost A): it also holds a read borrow, so [`pt_seq_len_held`]
/// and [`pt_seq_get_held`] skip the borrow word. [`pt_seq_release_held`]
/// ends both. Ends the program if a writer holds `s`.
///
/// # Safety
///
/// `s` is a live list the caller holds a reference to.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_seq_hold(s: *const PtSeq) -> *const PtSeq {
    // SAFETY: guaranteed by the caller.
    unsafe { seq(s as u64) }.items.hold();
    s
}

/// Ends a hold [`pt_seq_hold`] made: drops its borrow, then its
/// reference. Null is ignored.
///
/// # Safety
///
/// `s` is null or a hold the caller gives up.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_seq_release_held(s: *mut PtSeq) {
    if s.is_null() {
        return;
    }
    // SAFETY: a live hold, by the caller's guarantee; the borrow is its.
    unsafe { seq(s as u64).items.unhold() };
    // SAFETY: the hold's reference, which the caller gives up.
    unsafe { pt_seq_dec(s) };
}

/// `len(s)` under a hold.
///
/// # Safety
///
/// `s` is a hold from [`pt_seq_hold`] that has not ended.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_seq_len_held(s: *const PtSeq) -> i64 {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { seq(s as u64) };
    // SAFETY: the caller's hold is a read borrow of `s.items`.
    let n = unsafe { s.items.held() }.len() / table::stride(s.desc);
    i64::try_from(n).unwrap_or(i64::MAX)
}

/// `s[i]` under a hold, as [`pt_seq_get`].
///
/// # Safety
///
/// `s` is a hold from [`pt_seq_hold`] that has not ended, and `status`
/// points to a writable byte.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_seq_get_held(s: *const PtSeq, i: i64, status: *mut u8) -> u64 {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { seq(s as u64) };
    // SAFETY: the caller's hold is a read borrow of `s.items`.
    let items = unsafe { s.items.held() };
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

/// `s[i]` under a hold, borrowed (atomics cost B): as
/// [`pt_seq_get_held`] without a reference of its own, so the element
/// is valid only while the hold lasts.
///
/// # Safety
///
/// `s` is a hold from [`pt_seq_hold`] that has not ended, and `status`
/// points to a writable byte.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_seq_item_held(s: *const PtSeq, i: i64, status: *mut u8) -> u64 {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { seq(s as u64) };
    // SAFETY: the caller's hold is a read borrow of `s.items`.
    let items = unsafe { s.items.held() };
    let found = position(i, items.len()).map(|k| items[k]);
    // SAFETY: guaranteed by the caller.
    unsafe { report(status, u8::from(found.is_none())) };
    found.unwrap_or(0)
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
    let Some(old) = s.items.replace(i, v, |marked| store(desc, v, marked)) else {
        return 1;
    };
    // Dropped after the borrow ends: freeing may reach other sequences.
    dec(desc, old);
    0
}

/// Ordinary integer-list read, selected by the compiler's monomorphic type.
/// Uses the same borrow and negative/bounds checks as [`pt_seq_get`].
/// Integers own no reference, so retaining a slot does no work.
///
/// # Safety
///
/// `s` is a live `list[int]` (descriptor exactly `li`) and `status` points
/// to a writable byte. This is an internal compiler/runtime contract.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_int_list_get(s: *const PtSeq, i: i64, status: *mut u8) -> u64 {
    // SAFETY: guaranteed by the caller, as for pt_seq_get.
    let s = unsafe { seq(s as u64) };
    let Some((_, v)) = s.items.get(i, |_, _| {}) else {
        // SAFETY: guaranteed by the caller.
        unsafe { report(status, 1) };
        return 0;
    };
    // SAFETY: guaranteed by the caller.
    unsafe { report(status, 0) };
    v
}

/// Ordinary integer-list store. A failed bounds check changes nothing.
/// The same `Slot::replace` borrow/synchronization path is retained;
/// integer slots need no retain, transitive marking or release.
///
/// # Safety
///
/// `s` is a live `list[int]` (descriptor exactly `li`).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_int_list_set(s: *const PtSeq, i: i64, v: u64) -> u8 {
    // SAFETY: guaranteed by the caller, as for pt_list_set.
    let s = unsafe { seq(s as u64) };
    u8::from(s.items.replace(i, v, |_| {}).is_none())
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
/// a merge sort that takes from the right only when it strictly beats
/// the left, so descending when `reverse` is set (D70), equal items
/// keeping their order either way.
///
/// # Safety
///
/// `s` is a live list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_list_sort(s: *const PtSeq, reverse: u8) {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { seq(s as u64) };
    let desc = slot(s.desc, 0);
    // One write borrow for the whole sort: a party rule 4 failed to keep
    // out conflicts rather than seeing the list empty mid-sort.
    let mut items = s.items.borrow_mut();
    let taken = std::mem::take(&mut *items);
    *items = merge_sort(desc, taken, reverse != 0);
}

/// Reverses list `s` in place: `map`'s results, made in order and taken
/// from the end ([`crate::task::pt_results_next`]).
///
/// # Safety
///
/// `s` is a live list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_list_reverse(s: *const PtSeq) {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { seq(s as u64) };
    s.items.borrow_mut().reverse();
}

/// Removes and gives `s`'s last slot, with the reference it held.
pub(crate) fn pop(s: &PtSeq) -> Option<u64> {
    s.items.borrow_mut().pop()
}

/// Removes every slot of `s`, giving them with their references.
pub(crate) fn take_all(s: &PtSeq) -> Vec<u64> {
    std::mem::take(&mut *s.items.borrow_mut())
}

fn merge_sort(desc: &[u8], mut items: Vec<u64>, reverse: bool) -> Vec<u64> {
    if items.len() < 2 {
        return items;
    }
    let right = items.split_off(items.len() / 2);
    let left = merge_sort(desc, items, reverse);
    let right = merge_sort(desc, right, reverse);
    let mut out = Vec::with_capacity(left.len() + right.len());
    let (mut left, mut right) = (left.into_iter().peekable(), right.into_iter().peekable());
    while let (Some(l), Some(r)) = (left.peek(), right.peek()) {
        let (a, b) = if reverse { (*l, *r) } else { (*r, *l) };
        let next = match cmp(desc, a, b) {
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

/// `s.insert(i, v)` on a list: `i` counts from the end when negative,
/// and one past either end clamps, as in CPython; `v` gains a reference.
///
/// # Safety
///
/// `s` is a live list and `v` a live value of its element type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_list_insert(s: *const PtSeq, i: i64, v: u64) {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { seq(s as u64) };
    let desc = slot(s.desc, 0);
    mark_stored(s, desc, v);
    inc(desc, v);
    let mut items = s.items.borrow_mut();
    let len = i64::try_from(items.len()).unwrap_or(i64::MAX);
    let at = if i < 0 { i.saturating_add(len) } else { i };
    let at = usize::try_from(at.clamp(0, len)).unwrap_or(0);
    items.insert(at, v);
}

/// Removes item `i` of list `s`, dropping its reference; lowering keeps
/// `i` in range (an `i` out of range changes nothing).
///
/// # Safety
///
/// `s` is a live list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_list_remove(s: *const PtSeq, i: i64) {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { seq(s as u64) };
    let old = {
        let mut items = s.items.borrow_mut();
        position(i, items.len()).map(|k| items.remove(k))
    };
    // Dropped after the borrow ends: freeing may reach other sequences.
    if let Some(old) = old {
        dec(slot(s.desc, 0), old);
    }
}

/// The first position of an item of list `s` `==` to `v`, or -1. With
/// `raise` set, `s.index(v)`: a missing `v` raises CPython's `ValueError`
/// naming it at the site instead (D71).
///
/// # Safety
///
/// `s` is a live list, `v` a live value of its element type, and the
/// site's words as [`crate::os::site`] takes them.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)] // The site's four words, as `pt_str_find` takes them.
pub unsafe extern "C" fn pt_list_index(
    s: *const PtSeq,
    v: u64,
    raise: u8,
    head: *const u8,
    head_len: usize,
    tail: *const u8,
    tail_len: usize,
) -> i64 {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { seq(s as u64) };
    let desc = slot(s.desc, 0);
    let found = s.items.borrow().iter().position(|x| eq(desc, *x, v));
    if let Some(at) = found {
        return i64::try_from(at).unwrap_or(i64::MAX);
    }
    if raise != 0 {
        // SAFETY: guaranteed by the caller.
        let site = unsafe { crate::os::site(head, head_len, tail, tail_len) };
        let mut shown = String::new();
        if repr(desc, v, &mut shown).is_err() {
            site.fail(crate::exc::TOO_DEEP);
        }
        site.raise_text(crate::exc::VALUE_ERROR, format!("{shown} is not in list"));
    }
    -1
}

/// `s.count(v)` on a list: how many items are `==` to `v`.
///
/// # Safety
///
/// `s` is a live list and `v` a live value of its element type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_list_count(s: *const PtSeq, v: u64) -> i64 {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { seq(s as u64) };
    let desc = slot(s.desc, 0);
    let n = s.items.borrow().iter().filter(|x| eq(desc, **x, v)).count();
    i64::try_from(n).unwrap_or(i64::MAX)
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

/// Fresh shallow list slice. Retain elements while the source is guarded.
///
/// # Safety
/// `s` is a live list; `step` is nonzero (checked at the source site).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_list_slice(
    s: *const PtSeq,
    start: i64,
    stop: i64,
    step: i64,
    present: i64,
) -> *mut PtSeq {
    // SAFETY: guaranteed by the caller, following pt_list_copy.
    let s = unsafe { seq(s as u64) };
    let items = s.items.borrow();
    let indices = crate::slice::indices(
        items.len(),
        (present & 1 != 0).then_some(start),
        (present & 2 != 0).then_some(stop),
        step,
    );
    let desc = slot(s.desc, 0);
    let selected = indices
        .into_iter()
        .flatten()
        .map(|k| {
            let v = items[k];
            inc(desc, v);
            v
        })
        .collect();
    new_seq(s.desc, selected)
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
    fn record_descriptors_skip_their_names() {
        // `P(x: int, at: Q(s: list[str]))`, `Q` named `Qé`.
        let q: &'static [u8] = b"c\x01ls\x03Q\xc3\xa9\x01s";
        assert_eq!(span(q), q.len());
        assert_eq!(name_at(q, 4), ("Qé", 8));
        assert_eq!(name_at(q, 8), ("s", 10));
        let p = b"c\x02ic\x01ls\x03Q\xc3\xa9\x01s\x01P\x02at\x01x";
        assert!(slot(p, 1).starts_with(q));
        assert_eq!(span(slot(p, 1)), q.len());
        assert_eq!(span(p), p.len());
        assert_eq!(span(b"c\x00\x01E"), 4);
    }

    #[test]
    fn records_print_with_their_names_and_compare_by_fields() {
        let inner = make(b"li", &[7]);
        let desc: &'static [u8] = b"c\x02ili\x01P\x02xs\x01x";
        let a = make(desc, &[1, inner as u64]);
        // SAFETY: `inner` is live; `b` takes a second reference.
        unsafe { pt_seq_inc(inner) };
        let b = make(desc, &[1, inner as u64]);
        assert_eq!(shown(a), "P(x=1, xs=[7])");
        assert!(eq(desc, a as u64, b as u64));
        let empty = make(b"c\x00\x01E", &[]);
        assert_eq!(shown(empty), "E()");
        // SAFETY: each is live and dropped once; the records own `inner`.
        unsafe {
            pt_seq_dec(a);
            pt_seq_dec(b);
            pt_seq_dec(empty);
        }
    }

    /// A new object of type `desc` with `n` null slots.
    fn object(desc: &'static [u8], n: i64) -> *mut PtSeq {
        // SAFETY: `desc` is static.
        unsafe { pt_obj_new(desc.as_ptr(), desc.len(), n) }
    }

    #[test]
    fn objects_store_fields_and_release_what_they_replace() {
        // `P(xs: list[int], n: int)` as a dataclass.
        let desc: &'static [u8] = b"m\x02lii\x01P\x01n\x02xs";
        assert_eq!(span(desc), desc.len());
        let (first, second) = (make(b"li", &[1]), make(b"li", &[2]));
        let p = object(desc, 2);
        // SAFETY: `p` and both lists are live; `p` takes its own references,
        // beside the one each list's maker holds.
        unsafe {
            pt_obj_set(p, 0, first as u64);
            pt_obj_set(p, 1, 7);
            assert_eq!(seq(first as u64).rc.get(), 2);
            pt_obj_set(p, 0, second as u64);
            assert_eq!(seq(first as u64).rc.get(), 1);
            assert_eq!(seq(second as u64).rc.get(), 2);
        }
        assert_eq!(shown(p), "P(xs=[2], n=7)");
        // SAFETY: each is live and dropped once.
        unsafe {
            [first, second, p].into_iter().for_each(|s| pt_seq_dec(s));
        }
    }

    #[test]
    fn an_object_dropped_before_its_init_ran_frees_safely() {
        // A string and a list slot, both still null: `__init__` raised.
        let desc: &'static [u8] = b"p\x02sli\x01P\x01b\x01a";
        let p = object(desc, 2);
        let keep = make(b"li", &[3]);
        // SAFETY: `p` is live and unmarked-shared; marking and dropping
        // must both skip the null slots.
        unsafe {
            pt_mark(desc.as_ptr(), desc.len(), p as u64);
            pt_seq_dec(p);
            pt_seq_dec(keep);
        }
    }

    #[test]
    fn dataclass_objects_compare_by_fields_and_plain_ones_by_identity() {
        let data: &'static [u8] = b"m\x01i\x01P\x01x";
        let plain: &'static [u8] = b"p\x01i\x01P\x01x";
        for (desc, same) in [(data, true), (plain, false)] {
            let (a, b) = (object(desc, 1), object(desc, 1));
            // SAFETY: both live and dropped once.
            unsafe {
                pt_obj_set(a, 0, 4);
                pt_obj_set(b, 0, 4);
                assert_eq!(pt_seq_eq(a, a), 1);
                assert_eq!(pt_seq_eq(a, b), u8::from(same));
                pt_seq_dec(a);
                pt_seq_dec(b);
            }
        }
    }

    unsafe extern "C" fn fake_repr(_: *const crate::exc::PtCtx, o: *const PtSeq) -> *mut PtStr {
        // SAFETY: the walker passes a live object.
        let n = unsafe { seq(o as u64) }.items.borrow()[0];
        pstr::into_ptr(PtStr::new(format!("P#{n}").into()))
    }

    unsafe extern "C" fn failing_repr(_: *const crate::exc::PtCtx, _: *const PtSeq) -> *mut PtStr {
        std::ptr::null_mut()
    }

    /// A plain object in a container shows as what its registered
    /// `__repr__` returns; one whose call fails stops the walk.
    #[test]
    fn plain_objects_show_through_their_registered_repr() {
        const OK: &[u8] = b"p\x01i\x01P\x01x\x05\x00";
        const BAD: &[u8] = b"p\x01i\x01P\x01x\x06\x00";
        // SAFETY: the functions match `ReprFn`.
        unsafe {
            pt_register_repr(5, fake_repr);
            pt_register_repr(6, failing_repr);
        }
        assert_eq!((class_of(OK), class_of(BAD)), (5, 6));
        let (ok, bad) = (object(OK, 1), object(BAD, 1));
        // SAFETY: both live; slot 0 is an int.
        unsafe {
            pt_obj_set(ok, 0, 7);
            pt_obj_set(bad, 0, 7);
        }
        let mut list_desc = b"l".to_vec();
        list_desc.extend_from_slice(OK);
        let list = make(
            Box::leak(list_desc.into_boxed_slice()),
            &[ok as u64, ok as u64],
        );
        assert_eq!(shown(list), "[P#7, P#7]");
        let mut out = String::new();
        assert_eq!(repr(BAD, bad as u64, &mut out), Err(TooDeep));
        // SAFETY: each is live and dropped once.
        unsafe {
            pt_seq_dec(list);
            pt_seq_dec(ok);
            pt_seq_dec(bad);
        }
    }

    /// A record that holds its own type through an optional: its
    /// descriptor names itself with a back-reference, which walkers follow.
    #[test]
    fn back_references_resolve_to_the_registered_descriptor() {
        const LINK: &[u8] = b"c\x02io^\x07\x00\x04Link\x03nxt\x05value";
        // SAFETY: the descriptor is static.
        unsafe { pt_register_desc(7, LINK.as_ptr(), LINK.len()) };
        assert_eq!(span(LINK), LINK.len());
        assert_eq!(span(b"^\x07\x00"), 3);
        assert_eq!(opt(slot(LINK, 1)).0, LINK);
        assert_eq!(resolve(b"^\x07\x00"), LINK);
        assert_eq!(resolve(b"i"), b"i");
        let tail = make(LINK, &[2, 0]);
        let head = make(LINK, &[1, tail as u64]);
        assert_eq!(shown(head), "Link(value=1, nxt=Link(value=2, nxt=None))");
        // SAFETY: each is live and dropped once; `head` released its field.
        unsafe { pt_seq_dec(head) };
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
    fn extend_appends_a_copy_even_of_itself() {
        let inner = make(b"li", &[1]);
        let xs = make(b"lli", &[inner as u64]);
        let ys = make(b"lli", &[inner as u64]);
        // SAFETY: all live until dropped below, each once.
        unsafe {
            pt_list_extend(xs, ys);
            assert_eq!(shown(xs), "[[1], [1]]");
            pt_list_extend(xs, xs);
            assert_eq!(shown(xs), "[[1], [1], [1], [1]]");
            assert_eq!(seq(inner as u64).rc.get(), 6);
            pt_seq_dec(ys);
            pt_seq_dec(xs);
            assert_eq!(seq(inner as u64).rc.get(), 1);
            pt_seq_dec(inner);
        }
    }

    #[test]
    fn insert_clamps_and_remove_drops_the_reference() {
        let inner = make(b"li", &[1]);
        let xs = make(b"lli", &[]);
        // SAFETY: all live until dropped below, each once.
        unsafe {
            pt_list_insert(xs, 0, inner as u64);
            pt_list_insert(xs, -100, inner as u64);
            pt_list_insert(xs, 100, inner as u64);
            assert_eq!(shown(xs), "[[1], [1], [1]]");
            assert_eq!(seq(inner as u64).rc.get(), 4);
            pt_list_remove(xs, 1);
            pt_list_remove(xs, 7);
            assert_eq!(shown(xs), "[[1], [1]]");
            assert_eq!(seq(inner as u64).rc.get(), 3);
            pt_seq_dec(xs);
            assert_eq!(seq(inner as u64).rc.get(), 1);
            pt_seq_dec(inner);
        }
        let ys = make(b"li", &[1, 2]);
        // SAFETY: live until dropped, once.
        unsafe {
            pt_list_insert(ys, -1, 9);
            assert_eq!(shown(ys), "[1, 9, 2]");
            pt_seq_dec(ys);
        }
    }

    #[test]
    fn index_and_count_compare_by_value() {
        let xs = make(
            b"lf",
            &[0.0f64.to_bits(), 2.0f64.to_bits(), (-0.0f64).to_bits()],
        );
        let nan = make(b"lf", &[f64::NAN.to_bits()]);
        let none = std::ptr::null();
        // SAFETY: live until dropped; no site is read without `raise`.
        unsafe {
            assert_eq!(
                pt_list_index(xs, (-0.0f64).to_bits(), 0, none, 0, none, 0),
                0
            );
            assert_eq!(pt_list_index(xs, 2.0f64.to_bits(), 0, none, 0, none, 0), 1);
            assert_eq!(pt_list_index(xs, 5.0f64.to_bits(), 0, none, 0, none, 0), -1);
            assert_eq!(pt_list_count(xs, 0.0f64.to_bits()), 2);
            assert_eq!(pt_list_count(nan, f64::NAN.to_bits()), 0);
            pt_seq_dec(xs);
            pt_seq_dec(nan);
        }
    }

    #[test]
    fn sort_reversed_keeps_equal_items_in_order() {
        let items = [0.0f64, -0.0, 1.0, 0.0].map(f64::to_bits);
        let xs = make(b"lf", &items);
        // SAFETY: live until dropped, once.
        unsafe {
            pt_list_sort(xs, 1);
            assert_eq!(shown(xs), "[1.0, 0.0, -0.0, 0.0]");
            pt_list_sort(xs, 0);
            assert_eq!(shown(xs), "[0.0, -0.0, 0.0, 1.0]");
            pt_seq_dec(xs);
        }
    }

    #[test]
    fn scale_repeats_in_place_or_empties() {
        let inner = make(b"li", &[1]);
        let xs = make(b"lli", &[inner as u64]);
        // SAFETY: all live until dropped below, each once.
        unsafe {
            assert_eq!(pt_list_scale(xs, 1), 0);
            assert_eq!(shown(xs), "[[1]]");
            assert_eq!(pt_list_scale(xs, 3), 0);
            assert_eq!(shown(xs), "[[1], [1], [1]]");
            assert_eq!(seq(inner as u64).rc.get(), 4);
            assert_eq!(pt_list_scale(xs, i64::MAX), 1, "too large");
            assert_eq!(shown(xs), "[[1], [1], [1]]", "unchanged");
            assert_eq!(pt_list_scale(xs, -1), 0);
            assert_eq!(shown(xs), "[]");
            assert_eq!(seq(inner as u64).rc.get(), 1);
            assert_eq!(pt_list_scale(xs, 5), 0);
            assert_eq!(shown(xs), "[]");
            pt_seq_dec(xs);
            pt_seq_dec(inner);
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

    #[test]
    fn a_marked_list_marks_what_it_gains() {
        let inner = make(b"li", &[1]);
        let outer = make(b"lli", &[inner as u64]);
        let fresh = make(b"li", &[2]);
        let ys = make(b"lli", &[fresh as u64]);
        // SAFETY: all live until dropped below, each once.
        unsafe {
            mark(b"lli", outer as u64);
            assert!(!seq(fresh as u64).rc.is_marked());
            pt_list_extend(outer, ys);
            assert!(seq(fresh as u64).rc.is_marked(), "extend marks");
            assert_eq!(pt_list_scale(outer, 2), 0);
            assert_eq!(shown(outer), "[[1], [2], [1], [2]]");
            assert_eq!(seq(inner as u64).rc.get(), 3);
            assert_eq!(seq(fresh as u64).rc.get(), 4);
            pt_seq_dec(outer);
            pt_seq_dec(ys);
            assert_eq!(seq(inner as u64).rc.get(), 1);
            assert_eq!(seq(fresh as u64).rc.get(), 1);
            pt_seq_dec(inner);
            pt_seq_dec(fresh);
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
    fn a_hold_reads_without_borrowing_and_ends_free() {
        let inner = make(b"li", &[4]);
        let list = make(b"lli", &[inner as u64, inner as u64]);
        let mut status = 9;
        // SAFETY: `list` lives until the last dec; the hold ends before.
        unsafe {
            // The loop's own reference, which the hold keeps.
            seq(list as u64).rc.inc();
            let h = pt_seq_hold(list) as *mut PtSeq;
            assert_eq!(h, list);
            assert_eq!(seq(list as u64).rc.get(), 2);
            assert!(!seq(list as u64).items.is_free());
            assert_eq!(pt_seq_len_held(h), 2);
            let got = pt_seq_get_held(h, -1, &mut status);
            assert_eq!((got, status), (inner as u64, 0));
            assert_eq!(seq(inner as u64).rc.get(), 4);
            pt_seq_dec(got as *mut PtSeq);
            assert_eq!(pt_seq_get_held(h, 2, &mut status), 0);
            assert_eq!(status, 1);
            // An ordinary read nests inside the hold.
            assert_eq!(pt_seq_len(list), 2);
            pt_seq_release_held(h);
            pt_seq_release_held(std::ptr::null_mut());
            assert!(seq(list as u64).items.is_free());
            assert_eq!(seq(list as u64).rc.get(), 1);
            pt_seq_dec(list);
            pt_seq_dec(inner);
        }
    }

    #[test]
    fn a_held_item_is_borrowed_and_counts_nothing() {
        let inner = make(b"li", &[1]);
        let list = make(b"lli", &[inner as u64, inner as u64]);
        let mut status = 9;
        // SAFETY: both live until their last dec; the hold ends before.
        unsafe {
            seq(list as u64).rc.inc();
            let h = pt_seq_hold(list) as *mut PtSeq;
            let before = seq(inner as u64).rc.get();
            assert_eq!(pt_seq_item_held(h, 1, &mut status), inner as u64);
            assert_eq!(status, 0);
            assert_eq!(pt_seq_item_held(h, -2, &mut status), inner as u64);
            assert_eq!(seq(inner as u64).rc.get(), before);
            assert_eq!(pt_seq_item_held(h, 2, &mut status), 0);
            assert_eq!(status, 1);
            assert_eq!(pt_seq_item_held(h, -3, &mut status), 0);
            assert_eq!(status, 1);
            pt_seq_release_held(h);
            pt_seq_dec(list);
            pt_seq_dec(inner);
        }
    }

    #[test]
    fn marked_holds_survive_concurrent_readers() {
        let inner = make(b"li", &[7, 8, 9]);
        let outer = make(b"lli", &[inner as u64; 8]);
        mark(b"lli", outer as u64);
        let p = outer as usize;
        let threads: Vec<_> = (0..4)
            .map(|_| {
                std::thread::spawn(move || {
                    let outer = p as *const PtSeq;
                    let mut status = 0;
                    // SAFETY: `outer` stays live until every thread is
                    // joined; each thread's hold ends before it does.
                    unsafe {
                        seq(outer as u64).rc.inc();
                        let h = pt_seq_hold(outer) as *mut PtSeq;
                        for k in 0..if cfg!(miri) { 40 } else { 10_000 } {
                            let got = pt_seq_get_held(h, k % pt_seq_len_held(h), &mut status);
                            assert_eq!(status, 0);
                            pt_seq_dec(got as *mut PtSeq);
                        }
                        pt_seq_release_held(h);
                    }
                })
            })
            .collect();
        for t in threads {
            assert!(t.join().is_ok());
        }
        // SAFETY: all threads are done; both still live.
        unsafe {
            assert!(seq(outer as u64).items.is_free());
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

    /// Ordinary accesses keep Python bounds and overwrite stale status on
    /// success/failure. Invalid stores leave the list untouched, even marked.
    #[test]
    fn integer_access_preserves_bounds_status_and_marked_stores() {
        integer_bounds(pt_seq_get, pt_list_set);
        integer_bounds(pt_int_list_get, pt_int_list_set);
    }

    fn integer_bounds(
        get: unsafe extern "C" fn(*const PtSeq, i64, *mut u8) -> u64,
        set: unsafe extern "C" fn(*const PtSeq, i64, u64) -> u8,
    ) {
        for marked in [false, true] {
            for values in [&[][..], &[10, 20, 30][..]] {
                let list = make(b"li", values);
                if marked {
                    mark(b"li", list as u64);
                }
                let mut status = 9;
                // SAFETY: list and status stay live; integer slots own no refs.
                unsafe {
                    for i in [i64::MIN, -4, 3, i64::MAX] {
                        assert_eq!(get(list, i, &mut status), 0);
                        assert_eq!(status, 1);
                        assert_eq!(set(list, i, 99), 1);
                    }
                    assert_eq!(&*seq(list as u64).items.borrow(), values);
                    if values.is_empty() {
                        for i in [-1, 0, 1] {
                            assert_eq!(get(list, i, &mut status), 0);
                            assert_eq!(status, 1);
                            assert_eq!(set(list, i, 99), 1);
                        }
                    } else {
                        assert_eq!(set(list, -3, u64::MAX), 0);
                        assert_eq!(set(list, -1, 7), 0);
                        assert_eq!(get(list, 0, &mut status), u64::MAX);
                        assert_eq!(status, 0);
                        seq(list as u64).rc.inc();
                        let held = pt_seq_hold(list) as *mut PtSeq;
                        // Ordinary reads still borrow under an existing hold.
                        assert_eq!(get(list, -1, &mut status), 7);
                        assert_eq!(status, 0);
                        assert_eq!(pt_seq_get_held(held, -1, &mut status), 7);
                        assert_eq!(pt_seq_item_held(held, -1, &mut status), 7);
                        pt_seq_release_held(held);
                    }
                    assert!(seq(list as u64).items.is_free());
                    pt_seq_dec(list);
                }
            }
        }
    }

    #[test]
    fn integer_access_leaves_owned_element_fallbacks_counted() {
        for marked in [false, true] {
            let a = make(b"li", &[1]);
            let b = make(b"li", &[2]);
            let outer = make(b"lli", &[a as u64]);
            if marked {
                mark(b"lli", outer as u64);
            }
            let mut status = 9;
            // SAFETY: all sequences live; each read gives its own reference.
            unsafe {
                assert_eq!(pt_seq_get(outer, -1, &mut status), a as u64);
                assert_eq!(status, 0);
                assert_eq!(seq(a as u64).rc.get(), 3);
                assert_eq!(pt_list_set(outer, 1, b as u64), 1);
                assert_eq!(seq(b as u64).rc.get(), 1);
                assert_eq!(pt_list_set(outer, 0, b as u64), 0);
                assert_eq!(seq(a as u64).rc.get(), 2);
                assert_eq!(seq(b as u64).rc.get(), 2);
                assert_eq!(seq(b as u64).rc.is_marked(), marked);
                pt_seq_dec(a); // read result
                pt_seq_dec(outer);
                assert_eq!(seq(b as u64).rc.get(), 1);
                pt_seq_dec(a);
                pt_seq_dec(b);
            }
        }
    }

    #[test]
    fn integer_access_keeps_shared_reads_synchronized() {
        integer_shared_reads(pt_seq_get);
        integer_shared_reads(pt_int_list_get);
    }

    fn integer_shared_reads(get: unsafe extern "C" fn(*const PtSeq, i64, *mut u8) -> u64) {
        let list = make(b"li", &[1, 2, 3]);
        mark(b"li", list as u64);
        let address = list as usize;
        let threads: Vec<_> = (0..4)
            .map(|_| {
                std::thread::spawn(move || {
                    let list = address as *const PtSeq;
                    let mut status = 9;
                    for i in 0..if cfg!(miri) { 8 } else { 10_000 } {
                        // SAFETY: owner keeps list live until every reader joins.
                        unsafe {
                            assert_eq!(get(list, i % 3, &mut status), (i % 3 + 1) as u64);
                            assert_eq!(status, 0);
                        }
                    }
                })
            })
            .collect();
        for thread in threads {
            assert!(thread.join().is_ok());
        }
        // SAFETY: all readers joined; owner now gives up its sole reference.
        unsafe {
            assert!(seq(list as u64).items.is_free());
            assert_eq!(seq(list as u64).rc.get(), 1);
            pt_seq_dec(list);
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
    fn sliced_heap_elements_survive_source_destruction() {
        let inner = make(b"li", &[7]);
        let source = make(b"lli", &[inner as u64, inner as u64]);
        // SAFETY: source and inner are live; slicing borrows source and
        // adds owned references to each selected element before returning.
        unsafe {
            let selected = pt_list_slice(source, 0, 0, -1, 0);
            pt_seq_dec(source);
            pt_seq_dec(inner);
            assert_eq!(shown(selected), "[[7], [7]]");
            let nested = seq(selected as u64).items.borrow()[0];
            pt_seq_push(nested as *const PtSeq, 9);
            assert_eq!(shown(selected), "[[7, 9], [7, 9]]");
            pt_seq_dec(selected);
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

    /// A `repr` keeps every container it queued children of borrowed
    /// until it is done, so none of those children can be replaced and
    /// freed before it is shown; a marking walk releases what it held.
    #[test]
    fn walks_hold_their_containers_until_done() {
        let inner = make(b"ls", &[]);
        let word = crate::str::into_ptr(PtStr::new("w".into()));
        // SAFETY: both live; the list takes its own reference.
        unsafe {
            pt_seq_push(inner, word as u64);
            pstr::pt_str_dec(word);
        }
        let outer = make(b"lls", &[inner as u64]);
        // SAFETY: `outer` keeps `inner` alive; the walks below see it
        // owned by its parent alone.
        unsafe { pt_seq_dec(inner) };
        let free = |p: *mut PtSeq| {
            // SAFETY: both lists are live until the end of the test.
            unsafe { seq(p as u64) }.items.is_free()
        };
        let mut held = Held::new();
        let mut out = String::new();
        assert_eq!(
            repr_holding(b"lls", outer as u64, &mut out, &mut held),
            Ok(())
        );
        assert_eq!(
            (out.as_str(), free(outer), free(inner)),
            ("[['w']]", false, false)
        );
        // Children go first: `inner` is free while `outer`, which keeps
        // it alive, is still held.
        assert!(held.release_last());
        assert_eq!((free(outer), free(inner)), (false, true));
        assert!(held.release_last() && !held.release_last());
        assert!(free(outer));
        mark(b"lls", outer as u64);
        assert!(free(outer) && free(inner));
        // SAFETY: the test's one reference, which frees both.
        unsafe { pt_seq_dec(outer) };
    }

    /// A walk that stops early still releases deepest first: each link
    /// of the chain is owned by its parent alone.
    #[test]
    #[cfg_attr(miri, ignore = "a 10,000-link chain is too slow under Miri")]
    fn a_walk_cut_short_releases_children_before_parents() {
        let mut links = vec![make(b"x0", &[])];
        for _ in 0..MAX_NESTING {
            let next = make(b"x0x", &[links[links.len() - 1] as u64]);
            // SAFETY: `next` holds its own reference.
            unsafe { pt_seq_dec(links[links.len() - 1]) };
            links.push(next);
        }
        links.reverse();
        let free = |p: *mut PtSeq| {
            // SAFETY: every link is live while the head is.
            unsafe { seq(p as u64) }.items.is_free()
        };
        let mut held = Held::new();
        let mut out = String::new();
        assert_eq!(
            repr_holding(b"x0x", links[0] as u64, &mut out, &mut held),
            Err(TooDeep)
        );
        let mut busy: Vec<usize> = (0..links.len()).filter(|&i| !free(links[i])).collect();
        assert!(busy.len() > 1, "the walk held a chain");
        while let Some(deepest) = busy.pop() {
            assert!(held.release_last());
            assert!(free(links[deepest]));
            assert!(busy.iter().all(|&i| !free(links[i])), "parents still held");
        }
        assert!(!held.release_last());
        // SAFETY: the head's one reference, which frees the chain.
        unsafe { pt_seq_dec(links[0]) };
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
                assert!(seq(xs as u64).items.is_free(), "{n}: released");
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
