//! Dicts and sets: sequences ([`crate::seq`]) whose slots keep insertion
//! order, `key, value` pairs for a dict and keys for a set, with a hash
//! index over the keys. Descriptors: `d`, the key type and the value
//! type; `e` and the element type. A view (`v`, `k`/`v`/`i` for keys,
//! values or items, and the item type) is a list-like snapshot.
//!
//! Keys are ints, strings, bools, `None`, and tuples and optionals of
//! them (inference allows no other), so hashing never meets a float, a
//! list or a table. Nothing is ever removed, so the index only grows.

#![allow(unsafe_code)] // Slots are pointers; the C ABI below.

use std::hash::{DefaultHasher, Hash, Hasher};

use crate::seq::{self, kind, new_seq, slot, Kind, PtSeq};

/// The open-addressing index of a dict or set: entry numbers by hash,
/// linear probing, at most half full. [`EMPTY`] marks a free slot.
#[derive(Debug, Default)]
pub struct Index {
    slots: Vec<usize>,
}

const EMPTY: usize = usize::MAX;

/// Slots per entry: 2 for a dict, else 1.
pub(crate) fn stride(desc: &[u8]) -> usize {
    match kind(desc) {
        Kind::Dict => 2,
        _ => 1,
    }
}

/// Feeds key `v` of type `desc` to `h`, consistently with `seq::eq`.
fn hash(desc: &'static [u8], v: u64, h: &mut DefaultHasher) {
    match kind(desc) {
        // SAFETY: a string key holds a live string.
        Kind::Str => unsafe { seq::text(v) }.hash(h),
        Kind::Opt if v == 0 => 0u8.hash(h),
        Kind::Opt => hash(seq::opt(desc).0, seq::present(desc, v), h),
        Kind::Tuple => {
            // SAFETY: a tuple key holds a live tuple.
            let t = unsafe { seq::seq(v) };
            for (k, field) in t.items.borrow().iter().enumerate() {
                hash(slot(t.desc, k), *field, h);
            }
        }
        _ => v.hash(h),
    }
}

/// The key type of table `t`.
fn key_desc(t: &PtSeq) -> &'static [u8] {
    slot(t.desc, 0)
}

fn hash_of(desc: &'static [u8], v: u64) -> usize {
    let mut h = DefaultHasher::new();
    hash(desc, v, &mut h);
    h.finish() as usize
}

/// The entry number holding key `k`, if any.
fn find(t: &PtSeq, k: u64) -> Option<usize> {
    let desc = key_desc(t);
    let items = t.items.borrow();
    let index = t.index.borrow();
    let slots = &index.as_ref()?.slots;
    if slots.is_empty() {
        return None;
    }
    let mask = slots.len() - 1;
    let step = stride(t.desc);
    let mut at = hash_of(desc, k) & mask;
    loop {
        let entry = *slots.get(at)?;
        if entry == EMPTY {
            return None;
        }
        if seq::eq(desc, items.get(entry * step).copied()?, k) {
            return Some(entry);
        }
        at = (at + 1) & mask;
    }
}

/// Records entry `entry`, keyed by `k`, in `slots`.
fn place(slots: &mut [usize], hash: usize, entry: usize) {
    let mask = slots.len() - 1;
    let mut at = hash & mask;
    while slots.get(at).is_some_and(|s| *s != EMPTY) {
        at = (at + 1) & mask;
    }
    if let Some(s) = slots.get_mut(at) {
        *s = entry;
    }
}

/// Appends key `k` (and value `v` for a dict), adding references, and
/// indexes it. The key must be absent.
fn append(t: &PtSeq, k: u64, v: Option<u64>) {
    seq::mark_stored(t, key_desc(t), k);
    if let Some(v) = v {
        seq::mark_stored(t, slot(t.desc, 1), v);
    }
    let entries = {
        let mut items = t.items.borrow_mut();
        seq::inc(key_desc(t), k);
        items.push(k);
        if let Some(v) = v {
            seq::inc(slot(t.desc, 1), v);
            items.push(v);
        }
        items.len() / stride(t.desc)
    };
    let mut index = t.index.borrow_mut();
    let slots = &mut index.get_or_insert_with(Box::default).slots;
    if entries * 2 > slots.len() {
        grow(t, slots, entries);
        return;
    }
    place(slots, hash_of(key_desc(t), k), entries - 1);
}

/// Rebuilds `slots` with room for twice `entries`.
fn grow(t: &PtSeq, slots: &mut Vec<usize>, entries: usize) {
    *slots = vec![EMPTY; (entries * 4).next_power_of_two()];
    let items = t.items.borrow();
    let step = stride(t.desc);
    for (entry, k) in items.iter().step_by(step).enumerate() {
        place(slots, hash_of(key_desc(t), *k), entry);
    }
}

/// Whether tables `a` and `b` of one type are equal: the same keys,
/// and for a dict equal values, in any order.
pub(crate) fn same(a: &PtSeq, b: &PtSeq) -> bool {
    let step = stride(a.desc);
    let items = a.items.borrow();
    if items.len() != b.items.borrow().len() {
        return false;
    }
    items.chunks(step).all(|entry| {
        let Some(at) = entry.first().and_then(|k| find(b, *k)) else {
            return false;
        };
        match entry.get(1) {
            None => true,
            Some(v) => {
                let w = b.items.borrow().get(at * 2 + 1).copied();
                w.is_some_and(|w| seq::eq(slot(a.desc, 1), *v, w))
            }
        }
    })
}

/// Borrows a table argument.
///
/// # Safety
///
/// `t` is a live dict or set.
unsafe fn table<'a>(t: *const PtSeq) -> &'a PtSeq {
    // SAFETY: guaranteed by the caller.
    unsafe { seq::seq(t as u64) }
}

/// `d[k] = v`: replaces the value of an existing key, which keeps its
/// first key and its place, or appends the pair.
///
/// # Safety
///
/// `d` is a live dict, `k` and `v` live values of its key and value
/// types.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_dict_set(d: *const PtSeq, k: u64, v: u64) {
    // SAFETY: guaranteed by the caller.
    let d = unsafe { table(d) };
    let Some(entry) = find(d, k) else {
        return append(d, k, Some(v));
    };
    let desc = slot(d.desc, 1);
    seq::mark_stored(d, desc, v);
    seq::inc(desc, v);
    let old = d
        .items
        .borrow_mut()
        .get_mut(entry * 2 + 1)
        .map(|w| std::mem::replace(w, v));
    // Dropped after the borrow ends: freeing may reach other sequences.
    if let Some(old) = old {
        seq::dec(desc, old);
    }
}

/// `s.add(k)`: appends `k` unless present.
///
/// # Safety
///
/// `s` is a live set and `k` a live value of its element type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_set_add(s: *const PtSeq, k: u64) {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { table(s) };
    if find(s, k).is_none() {
        append(s, k, None);
    }
}

/// `k in t` for a dict or set.
///
/// # Safety
///
/// `t` is a live dict or set and `k` a live value of its key type.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_table_contains(t: *const PtSeq, k: u64) -> u8 {
    // SAFETY: guaranteed by the caller.
    u8::from(find(unsafe { table(t) }, k).is_some())
}

/// `d[k]`, a new reference; `*status` is 1, and the result 0, when `k`
/// is missing.
///
/// # Safety
///
/// `d` is a live dict, `k` a live value of its key type, and `status`
/// points to a writable byte.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_dict_get(d: *const PtSeq, k: u64, status: *mut u8) -> u64 {
    // SAFETY: guaranteed by the caller.
    let d = unsafe { table(d) };
    let v = find(d, k).and_then(|entry| d.items.borrow().get(entry * 2 + 1).copied());
    // SAFETY: guaranteed by the caller.
    unsafe { *status = u8::from(v.is_none()) };
    let Some(v) = v else {
        return 0;
    };
    seq::inc(slot(d.desc, 1), v);
    v
}

/// Part `part` (0 the key, 1 a dict's value) of entry `entry` of table
/// `t`, a new reference. Lowering keeps `entry` below the length.
///
/// # Safety
///
/// `t` is a live dict or set.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_table_entry(t: *const PtSeq, entry: i64, part: i64) -> u64 {
    // SAFETY: guaranteed by the caller.
    let t = unsafe { table(t) };
    let at = usize::try_from(entry).unwrap_or(usize::MAX);
    let k = at
        .saturating_mul(stride(t.desc))
        .saturating_add(usize::try_from(part).unwrap_or(0));
    let Some(v) = t.items.borrow().get(k).copied() else {
        crate::internal("a table entry out of range");
    };
    seq::inc(slot(t.desc, k), v);
    v
}

/// A new view (`desc[..len]`) of dict `d`: a snapshot of its keys,
/// values, or `(key, value)` tuples.
///
/// # Safety
///
/// `d` is a live dict and `desc` a view descriptor of its types, read-only
/// data that lives for the whole program.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_view_new(d: *const PtSeq, desc: *const u8, len: usize) -> *mut PtSeq {
    // SAFETY: guaranteed by the caller.
    let (d, desc) = unsafe { (table(d), std::slice::from_raw_parts(desc, len)) };
    let items = d.items.borrow();
    let (keys, values) = (slot(d.desc, 0), slot(d.desc, 1));
    let snapshot: Vec<u64> = match desc.get(1) {
        Some(b'k') => items.iter().step_by(2).copied().collect(),
        Some(b'v') => items.iter().skip(1).step_by(2).copied().collect(),
        _ => items
            .chunks(2)
            .map(|pair| {
                let t = new_seq(slot(desc, 0), Vec::with_capacity(2));
                // SAFETY: `t` is new and live; the pair matches its fields.
                unsafe { pair.iter().for_each(|v| seq::pt_seq_push(t, *v)) };
                t as u64
            })
            .collect(),
    };
    if matches!(desc.get(1), Some(b'k' | b'v')) {
        let item = if desc.get(1) == Some(&b'k') {
            keys
        } else {
            values
        };
        snapshot.iter().for_each(|v| seq::inc(item, *v));
    }
    new_seq(desc, snapshot)
}

/// `list(x)` of a dict (its keys), set or view: a new list of type
/// `desc[..len]`.
///
/// # Safety
///
/// `x` is a live dict, set or view and `desc` the descriptor of a list
/// of its items, read-only data that lives for the whole program.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_list_of(x: *const PtSeq, desc: *const u8, len: usize) -> *mut PtSeq {
    // SAFETY: guaranteed by the caller.
    let (x, desc) = unsafe { (table(x), std::slice::from_raw_parts(desc, len)) };
    let items: Vec<u64> = x
        .items
        .borrow()
        .iter()
        .step_by(stride(x.desc))
        .copied()
        .collect();
    let item = slot(desc, 0);
    items.iter().for_each(|v| seq::inc(item, *v));
    new_seq(desc, items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seq::{pt_seq_dec, pt_seq_len, pt_seq_new};

    fn empty(desc: &'static [u8]) -> *mut PtSeq {
        // SAFETY: `desc` is static.
        unsafe { pt_seq_new(desc.as_ptr(), desc.len(), 0) }
    }

    fn shown(s: *const PtSeq) -> String {
        // SAFETY: `s` is live in every test.
        unsafe { seq::shown(s) }.expect("shallow")
    }

    /// Keys and values stored into a marked dict or set are marked.
    #[test]
    fn entries_stored_into_marked_tables_are_marked() {
        let list = |v: u64| {
            let l = empty(b"li");
            // SAFETY: `l` is live.
            unsafe { seq::pt_seq_push(l, v) };
            l
        };
        let (old, new) = (list(1), list(2));
        let key = crate::str::into_ptr(crate::str::PtStr::new("k".into()));
        let d = empty(b"dili");
        let s = empty(b"es");
        // SAFETY: every table and list stays live until dropped below.
        unsafe {
            pt_dict_set(d, 0, old as u64);
            seq::mark(b"dili", d as u64);
            seq::mark(b"es", s as u64);
            pt_dict_set(d, 0, new as u64);
            pt_set_add(s, key as u64);
            let marked = |p: *mut PtSeq| seq::seq(p as u64).rc.is_marked();
            assert!(marked(old) && marked(new) && crate::str::borrow(key).rc.is_marked());
            crate::str::pt_str_dec(key);
            [old, new, d, s].into_iter().for_each(|p| pt_seq_dec(p));
        }
    }

    #[test]
    fn marked_dicts_survive_concurrent_readers() {
        let d = empty(b"dii");
        // SAFETY: `d` is live.
        unsafe { (0..64).for_each(|k| pt_dict_set(d, k, k * 10)) };
        seq::mark(b"dii", d as u64);
        let p = d as usize;
        let threads: Vec<_> = (0..4)
            .map(|_| {
                std::thread::spawn(move || {
                    let d = p as *const PtSeq;
                    let mut status = 0;
                    for k in 0..if cfg!(miri) { 40 } else { 10_000 } {
                        let k = k % 64;
                        // SAFETY: `d` stays live until every thread is joined.
                        unsafe {
                            assert_eq!((pt_dict_get(d, k, &mut status), status), (k * 10, 0));
                            assert_eq!(pt_table_contains(d, k + 64), 0);
                        }
                    }
                })
            })
            .collect();
        for t in threads {
            assert!(t.join().is_ok());
        }
        // SAFETY: all threads are done.
        unsafe { pt_seq_dec(d) };
    }

    #[test]
    fn dicts_keep_order_and_first_keys_and_replace_values() {
        let d = empty(b"dii");
        let mut status = 9;
        // SAFETY: `d` and `status` are live.
        unsafe {
            for (k, v) in [(3, 30), (1, 10), (3, 31), (2, 20)] {
                pt_dict_set(d, k, v);
            }
            assert_eq!(shown(d), "{3: 31, 1: 10, 2: 20}");
            assert_eq!(pt_seq_len(d), 3);
            assert_eq!((pt_dict_get(d, 1, &mut status), status), (10, 0));
            pt_dict_get(d, 4, &mut status);
            assert_eq!(status, 1);
            assert_eq!(pt_table_contains(d, 2), 1);
            assert_eq!(pt_table_entry(d, 2, 0), 2);
            pt_seq_dec(d);
        }
    }

    #[test]
    fn sets_dedupe_grow_and_compare_order_blind() {
        let a = empty(b"ei");
        let b = empty(b"ei");
        // SAFETY: both live until dropped.
        unsafe {
            (0..100).for_each(|k| pt_set_add(a, k));
            (0..100).rev().for_each(|k| pt_set_add(b, k));
            pt_set_add(a, 5);
            assert_eq!(pt_seq_len(a), 100);
            assert!(same(table(a), table(b)));
            assert_eq!(pt_table_contains(a, 99), 1);
            assert_eq!(pt_table_contains(a, 100), 0);
            pt_set_add(b, 100);
            assert!(!same(table(a), table(b)));
            pt_seq_dec(a);
            pt_seq_dec(b);
        }
        let none = empty(b"ei");
        assert_eq!(shown(none), "set()");
        // SAFETY: live.
        unsafe { pt_seq_dec(none) };
    }

    #[test]
    fn string_and_tuple_keys_hash_by_value() {
        let key = |text: &str| crate::str::into_ptr(crate::str::PtStr::new(text.into())) as u64;
        let d = empty(b"dsi");
        let (a, a2) = (key("a"), key("a"));
        // SAFETY: every object is live until dropped below.
        unsafe {
            pt_dict_set(d, a, 1);
            pt_dict_set(d, a2, 2);
            assert_eq!(shown(d), "{'a': 2}");
            crate::str::pt_str_dec(a as *mut _);
            crate::str::pt_str_dec(a2 as *mut _);
            pt_seq_dec(d);
        }
        let pairs = empty(b"et\x02is");
        let t = |n: u64, s: u64| {
            let t = empty(b"t\x02is");
            // SAFETY: `t` is new; the fields match.
            unsafe {
                seq::pt_seq_push(t, n);
                seq::pt_seq_push(t, s);
            }
            t
        };
        let (b, b2) = (key("b"), key("b"));
        let (x, y) = (t(1, b), t(1, b2));
        // SAFETY: as above.
        unsafe {
            pt_set_add(pairs, x as u64);
            pt_set_add(pairs, y as u64);
            assert_eq!(shown(pairs), "{(1, 'b')}");
            [x, y, pairs].into_iter().for_each(|s| pt_seq_dec(s));
            crate::str::pt_str_dec(b as *mut _);
            crate::str::pt_str_dec(b2 as *mut _);
        }
    }

    #[test]
    fn views_and_lists_snapshot_the_dict() {
        let d = empty(b"dib");
        let keys: &[u8] = b"vki";
        let items: &[u8] = b"vit\x02ib";
        let list: &[u8] = b"li";
        // SAFETY: every object is live until dropped below.
        unsafe {
            pt_dict_set(d, 1, 1);
            pt_dict_set(d, 2, 0);
            let k = pt_view_new(d, keys.as_ptr(), keys.len());
            let i = pt_view_new(d, items.as_ptr(), items.len());
            let l = pt_list_of(d, list.as_ptr(), list.len());
            pt_dict_set(d, 3, 1);
            assert_eq!(shown(k), "dict_keys([1, 2])");
            assert_eq!(shown(i), "dict_items([(1, True), (2, False)])");
            assert_eq!(shown(l), "[1, 2]");
            assert_eq!(shown(d), "{1: True, 2: False, 3: True}");
            [k, i, l, d].into_iter().for_each(|s| pt_seq_dec(s));
        }
    }
}
