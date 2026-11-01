//! Reference counts and borrow cells that tasks (M6c) may share.
//!
//! A runtime object is touched by one thread until it is *marked*
//! (docs/research/concurrency.md, rule 3): marking happens before the
//! object is handed to a task, and is never undone. An unmarked object's
//! count and borrow state change by plain loads and stores, as `Cell`
//! and `RefCell` did; a marked one's by atomic read-modify-writes. The
//! mark is the top bit of the same word, so one load decides the path.
//!
//! A conflicting borrow ends the program as a runtime bug, as `RefCell`
//! panics on one: rule 4 rejects programs that could conflict, so a
//! conflict means the compiler let one through, and failing loudly keeps
//! that from becoming silent memory corruption.
//!
//! Safe code cannot share a [`Slot`] between threads: it is not `Sync`,
//! since an unmarked one takes no atomic steps. Runtime objects reach
//! another thread only as pointers through the C ABI, whose callers
//! guarantee the object was marked first (`lib.rs`); that `unsafe`
//! contract, not a type, carries the invariant across.
//!
//! This module holds the runtime's only `unsafe` outside its C ABI
//! (Nano, 2026-11-01): [`Slot`] hands out references into an
//! `UnsafeCell` guarded by its borrow word.

#![allow(unsafe_code)]

use std::cell::UnsafeCell;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{fence, AtomicUsize, Ordering};

use crate::seq::position;

/// The mark bit, shared by [`Count`] and [`Slot`].
const MARK: usize = 1 << (usize::BITS - 1);
/// A [`Slot`]'s writer bit; the bits below it count readers.
const WRITER: usize = 1 << (usize::BITS - 2);

/// A reference count, starting at one.
#[derive(Debug)]
pub(crate) struct Count(AtomicUsize);

impl Count {
    pub(crate) const fn new() -> Self {
        Count(AtomicUsize::new(1))
    }

    /// Whether the object may be reached by another thread.
    #[cfg(test)]
    pub(crate) fn is_marked(&self) -> bool {
        self.0.load(Ordering::Relaxed) & MARK != 0
    }

    /// Marks the count; returns whether it was already marked.
    pub(crate) fn mark(&self) -> bool {
        self.0.fetch_or(MARK, Ordering::Relaxed) & MARK != 0
    }

    /// The number of references.
    #[cfg(test)]
    pub(crate) fn get(&self) -> usize {
        self.0.load(Ordering::Relaxed) & !MARK
    }

    pub(crate) fn inc(&self) {
        let v = self.0.load(Ordering::Relaxed);
        if v & MARK == 0 {
            self.0.store(v + 1, Ordering::Relaxed);
        } else {
            self.0.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Drops a reference; true when it was the last one, after which
    /// every other thread's use of the object happens before the caller
    /// frees it.
    pub(crate) fn dec(&self) -> bool {
        let v = self.0.load(Ordering::Relaxed);
        if v & MARK == 0 {
            self.0.store(v - 1, Ordering::Relaxed);
            return v == 1;
        }
        let last = self.0.fetch_sub(1, Ordering::Release) & !MARK == 1;
        if last {
            fence(Ordering::Acquire);
        }
        last
    }
}

/// A borrow cell with `RefCell`'s API, safe to share once marked.
pub(crate) struct Slot<T> {
    /// [`MARK`], [`WRITER`], and the number of readers.
    state: AtomicUsize,
    value: UnsafeCell<T>,
}

impl<T: std::fmt::Debug> std::fmt::Debug for Slot<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Slot")
    }
}

impl<T> Slot<T> {
    pub(crate) const fn new(value: T) -> Self {
        Slot {
            state: AtomicUsize::new(0),
            value: UnsafeCell::new(value),
        }
    }

    /// Whether the slot may be reached by another thread.
    pub(crate) fn is_marked(&self) -> bool {
        self.state.load(Ordering::Relaxed) & MARK != 0
    }

    /// Marks the slot.
    pub(crate) fn mark(&self) {
        self.state.fetch_or(MARK, Ordering::Relaxed);
    }

    pub(crate) fn into_inner(self) -> T {
        self.value.into_inner()
    }

    /// Reads the value; ends the program if a writer holds it.
    pub(crate) fn borrow(&self) -> Ref<'_, T> {
        let s = self.state.load(Ordering::Relaxed);
        if s & WRITER != 0 {
            conflict();
        }
        if s & MARK == 0 {
            self.state.store(s + 1, Ordering::Relaxed);
        } else {
            let prev = self.state.fetch_add(1, Ordering::Acquire);
            if prev & WRITER != 0 {
                conflict();
            }
        }
        Ref { slot: self }
    }

    /// Changes the value; ends the program if anyone else holds it.
    pub(crate) fn borrow_mut(&self) -> RefMut<'_, T> {
        let s = self.state.load(Ordering::Relaxed);
        if s & !MARK != 0 {
            conflict();
        }
        if s & MARK == 0 {
            self.state.store(WRITER, Ordering::Relaxed);
        } else if self
            .state
            .compare_exchange(MARK, MARK | WRITER, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            conflict();
        }
        RefMut {
            slot: self,
            marked: s & MARK != 0,
        }
    }

    fn release_read(&self) {
        let s = self.state.load(Ordering::Relaxed);
        if s & MARK == 0 {
            self.state.store(s - 1, Ordering::Relaxed);
        } else {
            self.state.fetch_sub(1, Ordering::Release);
        }
    }

    fn release_write(&self) {
        let s = self.state.load(Ordering::Relaxed);
        if s & MARK == 0 {
            self.state.store(0, Ordering::Relaxed);
        } else {
            self.state.fetch_and(!WRITER, Ordering::Release);
        }
    }
}

/// Copying reads and swaps on a vector of plain values, the runtime's
/// hottest paths. An unmarked, unborrowed slot skips the borrow word's
/// round trip: no other thread can reach it, and nothing else runs
/// while the reference lives.
impl<T: Copy> Slot<Vec<T>> {
    /// Whether this thread alone can reach the slot and nobody holds it.
    fn idle(&self) -> bool {
        self.state.load(Ordering::Relaxed) == 0
    }

    /// Copies out the item at Python index `i` (negative counts from
    /// the end) with its position, after `retain(k, item)` has run;
    /// `None` out of range. On a busy slot `retain` runs under the read
    /// borrow, so a writer rule 4 failed to keep out conflicts instead
    /// of freeing the item first.
    #[inline]
    pub(crate) fn get(&self, i: i64, retain: impl FnOnce(usize, T)) -> Option<(usize, T)> {
        if !self.idle() {
            return self.get_borrowed(i, retain);
        }
        // SAFETY: see `idle`; the reference ends in this statement, and
        // `position` runs no other code.
        let (k, v) = {
            let items = unsafe { &*self.value.get() };
            position(i, items.len()).map(|k| (k, items[k]))?
        };
        retain(k, v);
        Some((k, v))
    }

    #[cold]
    fn get_borrowed(&self, i: i64, retain: impl FnOnce(usize, T)) -> Option<(usize, T)> {
        let items = self.borrow();
        let (k, v) = position(i, items.len()).map(|k| (k, items[k]))?;
        retain(k, v);
        Some((k, v))
    }

    /// Appends `v`, giving its position, with `retain(k, marked)` run
    /// once for it. On a busy slot `retain` runs under the writer bit
    /// before `v` lands, so the slot's reference to `v` exists before any
    /// party can see `v`, even one rule 4 should have kept out; on an
    /// idle one (this thread's alone) it runs after.
    #[inline]
    pub(crate) fn push(&self, v: T, retain: impl FnOnce(usize, bool)) -> usize {
        if !self.idle() {
            return self.push_borrowed(v, retain);
        }
        let k = {
            // SAFETY: see `idle`; the reference ends in this block.
            let items = unsafe { &mut *self.value.get() };
            items.push(v);
            items.len() - 1
        };
        retain(k, false);
        k
    }

    #[cold]
    fn push_borrowed(&self, v: T, retain: impl FnOnce(usize, bool)) -> usize {
        let mut items = self.borrow_mut();
        let k = items.len();
        retain(k, items.is_marked());
        items.push(v);
        k
    }

    /// Puts `v` at Python index `i`, giving back the old item, with
    /// `retain(marked)` run once for `v`, ordered as in [`Slot::push`];
    /// `None`, changing nothing and running nothing, out of range.
    #[inline]
    pub(crate) fn replace(&self, i: i64, v: T, retain: impl FnOnce(bool)) -> Option<T> {
        if !self.idle() {
            return self.replace_borrowed(i, v, retain);
        }
        let old = {
            // SAFETY: see `idle`; the reference ends in this block, and
            // `position` runs no other code.
            let items = unsafe { &mut *self.value.get() };
            let k = position(i, items.len())?;
            std::mem::replace(&mut items[k], v)
        };
        retain(false);
        Some(old)
    }

    #[cold]
    fn replace_borrowed(&self, i: i64, v: T, retain: impl FnOnce(bool)) -> Option<T> {
        let mut items = self.borrow_mut();
        let k = position(i, items.len())?;
        retain(items.is_marked());
        Some(std::mem::replace(&mut items[k], v))
    }
}

fn conflict() -> ! {
    crate::internal("two tasks used one value at once")
}

/// A read borrow of a [`Slot`].
pub(crate) struct Ref<'a, T> {
    slot: &'a Slot<T>,
}

impl<T> Deref for Ref<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: this read borrow is counted in the state word, which
        // admits no writer until it is dropped.
        unsafe { &*self.slot.value.get() }
    }
}

impl<T> Drop for Ref<'_, T> {
    fn drop(&mut self) {
        self.slot.release_read();
    }
}

/// A write borrow of a [`Slot`].
pub(crate) struct RefMut<'a, T> {
    slot: &'a Slot<T>,
    /// Whether the slot was marked when borrowed; a writer excludes
    /// every other party, so no other thread marks it meanwhile.
    marked: bool,
}

impl<T> RefMut<'_, T> {
    /// Whether the slot may be reached by another thread, without
    /// reading its state word again.
    pub(crate) fn is_marked(&self) -> bool {
        self.marked
    }
}

impl<T> Deref for RefMut<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        // SAFETY: the writer bit admits no other borrow until drop.
        unsafe { &*self.slot.value.get() }
    }
}

impl<T> DerefMut for RefMut<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: the writer bit admits no other borrow until drop.
        unsafe { &mut *self.slot.value.get() }
    }
}

impl<T> Drop for RefMut<'_, T> {
    fn drop(&mut self) {
        self.slot.release_write();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    /// A slot the test marked before sharing it, as the ABI contract
    /// requires of every caller.
    struct Marked<T>(Slot<T>);

    // SAFETY: built only by `marked`, after the slot is marked, so every
    // step on it is atomic: one writer (`T: Send`) or many readers
    // sharing `&T` (`T: Sync`), as a reader-writer lock.
    unsafe impl<T: Send + Sync> Sync for Marked<T> {}

    fn marked<T>(value: T) -> Arc<Marked<T>> {
        let s = Slot::new(value);
        s.mark();
        Arc::new(Marked(s))
    }

    /// `Slot` must not be `Sync`: the line below would be ambiguous,
    /// and fail to compile, if it were.
    trait AmbiguousIfSync<A> {
        fn check() {}
    }
    impl<T: ?Sized> AmbiguousIfSync<()> for T {}
    impl<T: ?Sized + Sync> AmbiguousIfSync<u8> for T {}
    const _: fn() = <Slot<Vec<u64>> as AmbiguousIfSync<_>>::check;

    /// Iterations per thread; Miri runs far fewer.
    const ROUNDS: usize = if cfg!(miri) { 40 } else { 10_000 };

    #[test]
    fn unmarked_counts_count() {
        let c = Count::new();
        c.inc();
        assert_eq!((c.get(), c.dec(), c.dec()), (2, false, true));
    }

    #[test]
    fn marking_keeps_the_count_and_is_idempotent() {
        let c = Count::new();
        c.inc();
        assert!(!c.mark());
        assert!(c.mark());
        assert_eq!((c.get(), c.is_marked()), (2, true));
    }

    #[test]
    fn a_slot_admits_many_readers_on_one_thread() {
        let s = Slot::new(vec![1, 2]);
        let (a, b) = (s.borrow(), s.borrow());
        assert_eq!(a.len() + b.len(), 4);
        drop((a, b));
        s.borrow_mut().push(3);
        assert_eq!(s.into_inner(), vec![1, 2, 3]);
    }

    #[test]
    fn copying_reads_and_swaps_take_python_indexes() {
        let s = Slot::new(vec![10u64, 20, 30]);
        assert_eq!(
            (
                s.get(0, |_, _| {}),
                s.get(-1, |_, _| {}),
                s.get(3, |_, _| {}),
                s.get(-4, |_, _| {})
            ),
            (Some((0, 10)), Some((2, 30)), None, None)
        );
        assert_eq!(
            (s.replace(-3, 11, |_| {}), s.replace(3, 0, |_| {})),
            (Some(10), None)
        );
        s.mark();
        assert_eq!(
            (
                s.get(0, |_, _| {}),
                s.replace(1, 21, |_| {}),
                s.get(-2, |_, _| {})
            ),
            (Some((0, 11)), Some(20), Some((1, 21)))
        );
        assert_eq!(s.into_inner(), vec![11, 21, 30]);
    }

    /// On a marked slot the caller retains the item while the read
    /// borrow still holds off writers.
    #[test]
    fn a_marked_read_retains_under_its_borrow() {
        let s = Slot::new(vec![4u64, 5]);
        s.mark();
        let readers = |s: &Slot<Vec<u64>>| s.state.load(Ordering::Relaxed) & !MARK;
        let mut seen = None;
        assert_eq!(
            s.get(-1, |k, v| seen = Some((k, v, readers(&s)))),
            Some((1, 5))
        );
        assert_eq!((seen, readers(&s)), (Some((1, 5, 1)), 0));
    }

    /// On a marked slot the writer retains the stored item while it
    /// holds the writer bit, before the item is in place.
    #[test]
    fn a_marked_store_retains_before_it_lands() {
        let s = Slot::new(vec![1u64]);
        s.mark();
        let writing = |s: &Slot<Vec<u64>>| s.state.load(Ordering::Relaxed) & WRITER != 0;
        let mut pushed = None;
        let k = s.push(2, |k, marked| pushed = Some((k, marked, writing(&s))));
        assert_eq!((k, pushed), (1, Some((1, true, true))));
        let mut replaced = None;
        let old = s.replace(-1, 3, |marked| replaced = Some((marked, writing(&s))));
        assert_eq!((old, replaced), (Some(2), Some((true, true))));
        let mut ran = false;
        assert_eq!(s.replace(5, 9, |_| ran = true), None);
        assert!(!ran && !writing(&s));
        assert_eq!(s.into_inner(), vec![1, 3]);
    }

    /// A held read borrow sends copying reads down the counted path,
    /// which still admits them.
    #[test]
    fn copying_reads_respect_a_held_borrow() {
        let s = Slot::new(vec![1u64, 2]);
        let held = s.borrow();
        assert_eq!(s.get(1, |_, _| {}), Some((1, 2)));
        drop(held);
        assert_eq!(s.replace(0, 5, |_| {}), Some(1));
    }

    #[test]
    fn marked_counts_balance_across_threads() {
        let c = Arc::new(Count::new());
        c.mark();
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let c = Arc::clone(&c);
                std::thread::spawn(move || {
                    for _ in 0..ROUNDS {
                        c.inc();
                        assert!(!c.dec());
                    }
                })
            })
            .collect();
        for t in threads {
            assert!(t.join().is_ok());
        }
        assert_eq!(c.get(), 1);
    }

    #[test]
    fn marked_slots_admit_concurrent_readers() {
        let s = marked(vec![1u64; 64]);
        let threads: Vec<_> = (0..4)
            .map(|_| {
                let s = Arc::clone(&s);
                std::thread::spawn(move || {
                    (0..ROUNDS)
                        .map(|_| s.0.borrow().iter().sum::<u64>())
                        .sum::<u64>()
                })
            })
            .collect();
        for t in threads {
            assert!(matches!(t.join(), Ok(t) if t == 64 * ROUNDS as u64));
        }
        s.0.borrow_mut().push(1);
        assert_eq!(s.0.borrow().len(), 65);
    }
}
