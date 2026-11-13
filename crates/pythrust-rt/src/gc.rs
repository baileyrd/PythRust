//! The cycle collector (M6 step 5-5, D151, D152, D175): synchronous trial
//! deletion over the sequences that may sit on a reference cycle.
//!
//! A sequence whose descriptor holds a cyclic back-reference
//! ([`seq::PtSeq::cyc`]) is a *candidate* when a drop leaves its count
//! above zero: it goes in a buffer, once. When the buffer holds
//! [`THRESHOLD`] candidates, the next allocation collects, and the end of
//! the program collects what remains. A collection:
//!
//! 1. finds the candidates' graph, following only candidate-capable
//!    sequences (a sequence that cannot reach a cycle is on none);
//! 2. subtracts each reference inside the graph from its target's count,
//!    on paper, in a side table: a sequence left with a count has a
//!    reference from outside, a root;
//! 3. keeps every sequence reachable from a root, and frees the rest,
//!    dropping what their slots hold outside the garbage as a drop does.
//!
//! It never runs while an executor is alive (workers hold and count
//! values at any moment; candidates they buffer wait for the submitting
//! thread), and gives up, keeping its candidates, when a sequence it
//! must read is write-borrowed.

#![allow(unsafe_code)] // Frees sequences by pointer, as `seq::drop_one` does.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::sync::{Mutex, MutexGuard, PoisonError};

use crate::seq::{self, PtSeq};

/// Buffered candidates that make the next allocation collect.
const THRESHOLD: usize = 1024;

/// The buffered candidates, as sequence pointers.
static ROOTS: Mutex<Vec<u64>> = Mutex::new(Vec::new());
/// `ROOTS.len()`, readable without the lock.
static BUFFERED: AtomicUsize = AtomicUsize::new(0);
/// Executors started and not ended.
static EXECUTORS: AtomicUsize = AtomicUsize::new(0);

fn roots() -> MutexGuard<'static, Vec<u64>> {
    ROOTS.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(crate) fn executor_started() {
    EXECUTORS.fetch_add(1, Relaxed);
}

pub(crate) fn executor_ended() {
    EXECUTORS.fetch_sub(1, Relaxed);
}

/// Buffers `s` (at `p`), a candidate, unless it already is.
pub(crate) fn candidate(s: &PtSeq, p: u64) {
    if s.buffered.swap(true, Relaxed) {
        return;
    }
    let mut roots = roots();
    roots.push(p);
    BUFFERED.store(roots.len(), Relaxed);
}

/// Whether enough candidates wait, and nothing else runs, for a collection.
pub(crate) fn due() -> bool {
    due_at(BUFFERED.load(Relaxed), EXECUTORS.load(Relaxed))
}

fn due_at(buffered: usize, executors: usize) -> bool {
    buffered >= THRESHOLD && executors == 0
}

/// Collects until the candidates are gone or stop yielding: the end of the
/// program.
pub(crate) fn collect_all() {
    while BUFFERED.load(Relaxed) > 0 && collect() {}
}

/// A candidate's place in the graph: its reference count less the
/// references inside the graph, and the sequences it holds references to.
struct Node {
    external: isize,
    kids: Vec<u64>,
}

/// Collects the buffered candidates; whether it could (no write borrow in
/// the way) and freed anything or emptied the buffer.
pub(crate) fn collect() -> bool {
    let taken = std::mem::take(&mut *roots());
    BUFFERED.store(0, Relaxed);
    // SAFETY: a buffered sequence is live: a zero count leaves it
    // allocated for this function to free.
    let seqs = |p: &u64| unsafe { seq::seq(*p) };
    taken
        .iter()
        .for_each(|p| seqs(p).buffered.store(false, Relaxed));
    let (dead, live): (Vec<u64>, Vec<u64>) = taken.iter().partition(|p| seqs(p).rc.get() == 0);
    let Some(graph) = graph(&live) else {
        for &p in &taken {
            candidate(seqs(&p), p);
        }
        return false;
    };
    let garbage = garbage(&graph);
    free(&garbage);
    dead.iter().for_each(|&p| drop_box(p));
    true
}

/// The graph of sequences `from` reach, following candidate-capable ones;
/// `None` when one cannot be read.
fn graph(from: &[u64]) -> Option<HashMap<u64, Node>> {
    let mut graph: HashMap<u64, Node> = HashMap::new();
    let mut todo = from.to_vec();
    while let Some(p) = todo.pop() {
        if graph.contains_key(&p) {
            continue;
        }
        // SAFETY: reached from a live candidate by a counted reference.
        let s = unsafe { seq::seq(p) };
        let items = s.items.try_borrow()?;
        let kids: Vec<u64> = items
            .iter()
            .enumerate()
            .filter_map(|(k, &v)| seq::child(seq::slot(s.desc, k), v))
            // SAFETY: as above.
            .filter(|&c| unsafe { seq::seq(c) }.cyc)
            .collect();
        drop(items);
        todo.extend(&kids);
        let external = isize::try_from(s.rc.get()).unwrap_or(isize::MAX);
        graph.insert(p, Node { external, kids });
    }
    let edges: Vec<u64> = graph.values().flat_map(|n| n.kids.clone()).collect();
    for kid in edges {
        if let Some(n) = graph.get_mut(&kid) {
            n.external -= 1;
        }
    }
    Some(graph)
}

/// The sequences of `graph` no reference from outside it reaches.
fn garbage(graph: &HashMap<u64, Node>) -> Vec<u64> {
    let mut todo: Vec<u64> = graph
        .iter()
        .filter(|(_, n)| n.external > 0)
        .map(|(&p, _)| p)
        .collect();
    let mut live = std::collections::HashSet::new();
    while let Some(p) = todo.pop() {
        if live.insert(p) {
            todo.extend(&graph[&p].kids);
        }
    }
    graph
        .keys()
        .copied()
        .filter(|p| !live.contains(p))
        .collect()
}

/// Frees `garbage`: drops what each holds outside it, then the sequences.
fn free(garbage: &[u64]) {
    let set: std::collections::HashSet<u64> = garbage.iter().copied().collect();
    let mut todo = Vec::new();
    for &p in garbage {
        // SAFETY: garbage, referenced only from garbage.
        let s = unsafe { seq::seq(p) };
        let items = std::mem::take(&mut *s.items.borrow_mut());
        let outside = items.into_iter().enumerate().map(|(k, v)| {
            let held = seq::child(seq::slot(s.desc, k), v).is_some_and(|c| set.contains(&c));
            (k, if held { 0 } else { v })
        });
        for (k, v) in outside {
            seq::dec_slot(s.desc, k, v, &mut todo);
        }
    }
    seq::free_all(todo);
    garbage.iter().for_each(|&p| drop_box(p));
}

/// Frees the allocation of sequence `p` once its slots hold nothing.
fn drop_box(p: u64) {
    // SAFETY: `p` came from `Box::into_raw`, nothing refers to it, and
    // its items were taken.
    drop(unsafe { Box::from_raw(p as *mut PtSeq) });
    crate::live_dec();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::str::{self as pstr, PtStr};
    use std::sync::Mutex;

    /// One test at a time: they share the candidate buffer.
    static ONE: Mutex<()> = Mutex::new(());

    /// A class `N` with a link to another `N` or `None`, and a tag. The
    /// link's back-reference names type 900, registered below.
    const NODE: &[u8] = b"m\x02o&\x04\x07s\x01N\x03tag\x04next";

    fn setup() -> MutexGuard<'static, ()> {
        let guard = ONE.lock().unwrap_or_else(PoisonError::into_inner);
        // SAFETY: `NODE` is read-only data for the whole run.
        unsafe { seq::pt_register_desc(900, NODE.as_ptr(), NODE.len()) };
        guard
    }

    /// A new node tagged with `tag`; the tag is also returned, with a
    /// reference of the test's own, so its count says whether the node
    /// is alive.
    fn node() -> (*mut PtSeq, *mut PtStr) {
        let tag = pstr::into_ptr(PtStr::new("t".into()));
        // SAFETY: `NODE` is a descriptor; slot 1 is a string.
        unsafe {
            let n = seq::pt_obj_new(NODE.as_ptr(), NODE.len(), 2);
            seq::pt_obj_set(n, 1, tag as u64);
            (n, tag)
        }
    }

    fn link(from: *mut PtSeq, to: *mut PtSeq) {
        // SAFETY: both live nodes; the store takes its own reference.
        unsafe { seq::pt_obj_set(from, 0, to as u64) };
    }

    fn drop_ref(n: *mut PtSeq) {
        // SAFETY: gives up a reference the test holds.
        unsafe { seq::pt_seq_dec(n) };
    }

    /// The count of `tag`: 2 while its node lives (the node's and the
    /// test's), 1 after.
    fn count(tag: *mut PtStr) -> usize {
        // SAFETY: the test holds a reference.
        unsafe { pstr::borrow(tag) }.rc.get()
    }

    #[test]
    fn a_dropped_cycle_is_freed_by_a_collection_not_by_counts() {
        let _g = setup();
        let ((a, ta), (b, tb)) = (node(), node());
        link(a, b);
        link(b, a);
        drop_ref(a);
        drop_ref(b);
        assert_eq!((count(ta), count(tb)), (2, 2), "counts alone leak a cycle");
        assert!(collect());
        assert_eq!((count(ta), count(tb)), (1, 1));
        drop_ref_str(ta);
        drop_ref_str(tb);
    }

    fn drop_ref_str(s: *mut PtStr) {
        // SAFETY: gives up the test's reference.
        unsafe { pstr::pt_str_dec(s) };
    }

    #[test]
    fn a_cycle_something_still_holds_survives() {
        let _g = setup();
        let ((a, ta), (b, tb)) = (node(), node());
        link(a, b);
        link(b, a);
        drop_ref(b);
        assert!(collect());
        assert_eq!((count(ta), count(tb)), (2, 2));
        drop_ref(a);
        assert!(collect());
        assert_eq!((count(ta), count(tb)), (1, 1));
        drop_ref_str(ta);
        drop_ref_str(tb);
    }

    #[test]
    fn a_node_reachable_from_a_live_one_is_kept_with_the_dead_cycle_around_it() {
        let _g = setup();
        let ((a, ta), (b, tb), (c, tc)) = (node(), node(), node());
        link(a, b);
        link(b, a);
        link(b, c); // replaced below: `c` is held by `b`
        drop_ref(a);
        drop_ref(b);
        assert!(collect());
        // `c` was held by garbage and by the test: it lives, uncounted
        // by the dead cycle now.
        assert_eq!((count(ta), count(tb), count(tc)), (1, 1, 2));
        drop_ref(c);
        drop_ref_str(ta);
        drop_ref_str(tb);
        drop_ref_str(tc);
    }

    #[test]
    fn a_buffered_node_freed_by_counts_is_left_for_the_collector() {
        let _g = setup();
        let (a, ta) = node();
        // SAFETY: `a` is live; this reference is dropped below.
        unsafe { seq::pt_seq_inc(a) };
        drop_ref(a); // buffered, count 1
        drop_ref(a); // zero: emptied, not freed
        assert_eq!(count(ta), 1);
        assert!(collect());
        drop_ref_str(ta);
    }

    #[test]
    fn a_write_borrowed_node_defers_the_collection() {
        let _g = setup();
        let ((a, ta), (b, tb)) = (node(), node());
        link(a, b);
        link(b, a);
        drop_ref(a);
        drop_ref(b);
        // SAFETY: `a` still allocated: the cycle holds it.
        let held = unsafe { seq::seq(a as u64) }.items.borrow_mut();
        assert!(!collect());
        drop(held);
        assert!(collect());
        assert_eq!((count(ta), count(tb)), (1, 1));
        drop_ref_str(ta);
        drop_ref_str(tb);
    }

    #[test]
    fn a_collection_is_due_at_the_threshold_with_no_executor_alive() {
        assert!(!due_at(THRESHOLD - 1, 0));
        assert!(due_at(THRESHOLD, 0));
        assert!(!due_at(THRESHOLD, 1));
    }

    #[test]
    fn candidates_are_buffered_once_and_cleared_by_a_collection() {
        let _g = setup();
        let (a, ta) = node();
        // SAFETY: `a` is live; each drop gives up a reference taken here.
        unsafe { seq::pt_seq_inc(a) };
        unsafe { seq::pt_seq_inc(a) };
        drop_ref(a);
        drop_ref(a);
        assert_eq!(roots().iter().filter(|&&p| p == a as u64).count(), 1);
        assert!(collect());
        assert!(!roots().contains(&(a as u64)));
        drop_ref(a);
        drop_ref_str(ta);
    }

    #[test]
    fn a_cyclic_value_shows_as_dots_where_it_comes_back() {
        let _g = setup();
        let (a, ta) = node();
        link(a, a);
        let mut out = String::new();
        seq::repr(NODE, a as u64, &mut out).expect("shallow");
        assert_eq!(out, "N(next=..., tag='t')");
        // Break the cycle by hand: the count alone then frees it.
        // SAFETY: slot 0 holds the node's reference to itself.
        unsafe { seq::pt_obj_set(a, 0, 0) };
        drop_ref(a);
        drop_ref_str(ta);
    }
}
