//! Tasks (M6c, `docs/research/concurrency.md`): an executor's pool of
//! worker threads, and the futures its tasks complete.
//!
//! An executor starts a worker when a task arrives and none is idle, up
//! to CPython's default `min(32, cpus + 4)` (Decided 2); leaving the
//! `with` block ([`pt_exec_end`]) waits for every task and joins them.
//! A task is a compiled thunk that unpacks its arguments, calls the
//! task's function on the worker with the worker's own context, and
//! returns the result's slot. The worker keeps what escaped (a pending
//! exception) instead, and publishes the task's unfinished output line,
//! so every task's output reaches stdout when it ends.
//!
//! What crosses threads, and why no count is shared unmarked:
//! - The arguments the thunk reads, and the module values it loads, are
//!   marked by the submitter first (`pt_mark`), so every thread's count
//!   operations on them are atomic.
//! - A task's outcome (its result, or the exception that escaped it),
//!   when the worker made it, is unmarked: the worker reaches it alone
//!   until it completes the future, and the submitter alone after. The
//!   future's lock orders the worker's last step before the
//!   submitter's first. The submitter also drops it: the worker stores
//!   it under the lock and never touches it again, except when the
//!   submitter dropped every handle first and so never saw it.
//! - The future itself is held by both threads at once. Its memory is
//!   an `Arc`, so that count is atomic; its handles, which generated
//!   code counts, are the submitter's alone.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};

use crate::exc::{self, PtCtx};
use crate::seq::{self, PtSeq};

/// A task's compiled entry: called on a worker with the worker's
/// context and the arguments its submitter packed, it returns the
/// result's slot (0 when an exception escaped), and drops the
/// references it was given.
pub type Thunk = unsafe extern "C" fn(ctx: *const PtCtx, args: *const u64) -> u64;

/// A worker's stack: the main thread's usual 8 MiB, so a task reaches
/// the same call depth as the module's code before its
/// `RecursionError`.
const STACK: usize = 8 << 20;

/// CPython's `ThreadPoolExecutor()` default: `min(32, cpus + 4)`.
fn default_workers(cpus: usize) -> usize {
    (cpus + 4).min(32)
}

/// How a task ended. Its values cross threads by hand (the module's
/// doc): `Pending` keeps the exception as an address, so the compiler
/// checks nothing about sending it.
enum Outcome {
    /// Its result's slot, holding a reference.
    Value(u64),
    /// What escaped it, raised again by each `result()`.
    Raised(exc::Raised),
}

impl Outcome {
    fn release(self, desc: &[u8]) {
        match self {
            Outcome::Value(v) => seq::dec(desc, v),
            Outcome::Raised(r) => r.release(),
        }
    }
}

/// A future's state, under its lock.
#[derive(Default)]
struct Done {
    outcome: Option<Outcome>,
    /// The submitter dropped every handle; the worker releases the
    /// outcome itself.
    abandoned: bool,
}

/// A task's result, shared by the worker that completes it and the
/// submitter that takes it.
pub struct PtFut {
    /// The result's type descriptor, for its references.
    desc: &'static [u8],
    /// Handles generated code holds, all on the submitter's thread.
    handles: AtomicUsize,
    done: Mutex<Done>,
    ready: Condvar,
}

impl PtFut {
    fn new(desc: &'static [u8]) -> PtFut {
        PtFut {
            desc,
            handles: AtomicUsize::new(1),
            done: Mutex::default(),
            ready: Condvar::new(),
        }
    }

    /// Stores how the task ended; releases it here if nobody will take
    /// it.
    fn complete(&self, outcome: Outcome) {
        let mut done = lock(&self.done);
        if done.abandoned {
            drop(done);
            outcome.release(self.desc);
            return;
        }
        let value = match &outcome {
            Outcome::Value(v) => Some(*v),
            Outcome::Raised(_) => None,
        };
        let touched = value.filter(|_| crate::sync::tsan::fault("outcome"));
        let freed = value.filter(|_| crate::sync::tsan::fault("late-free"));
        if let Some(v) = touched {
            seq::inc(self.desc, v);
        }
        done.outcome = Some(outcome);
        self.ready.notify_all();
        drop(done);
        if let Some(v) = touched.or(freed) {
            seq::dec(self.desc, v);
        }
    }

    /// Waits for the task; runs `take` on how it ended.
    fn wait<R>(&self, take: impl FnOnce(&Outcome) -> R) -> R {
        let mut done = lock(&self.done);
        loop {
            if let Some(outcome) = done.outcome.as_ref() {
                return take(outcome);
            }
            done = self
                .ready
                .wait(done)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// Drops one handle; the last releases the outcome on this, the
    /// submitter's, thread, or leaves it to the worker if not yet
    /// there.
    fn drop_handle(&self) -> bool {
        if self.handles.fetch_sub(1, Ordering::Relaxed) != 1 {
            return false;
        }
        let mut done = lock(&self.done);
        let outcome = done.outcome.take();
        done.abandoned = outcome.is_none();
        drop(done);
        if let Some(outcome) = outcome {
            outcome.release(self.desc);
        }
        true
    }
}

impl Drop for PtFut {
    /// Every outcome was released by the last handle or the worker.
    fn drop(&mut self) {
        let done = self.done.get_mut().unwrap_or_else(PoisonError::into_inner);
        debug_assert!(
            done.outcome.is_none(),
            "a task's outcome outlived its future"
        );
    }
}

/// A task waiting for a worker.
struct Job {
    thunk: Thunk,
    args: Vec<u64>,
    fut: Arc<PtFut>,
}

impl Job {
    /// Runs the task on this worker and completes its future. Calling
    /// the compiled thunk is part of the C ABI surface.
    #[allow(unsafe_code)]
    fn run(self) {
        let ctx = exc::start_task();
        // SAFETY: `pt_submit`'s caller guarantees `thunk` reads
        // `args.len()` slots of the types it was compiled for, which the
        // submitter marked.
        let v = unsafe { (self.thunk)(ctx, self.args.as_ptr()) };
        let outcome = match exc::take_raised() {
            Some(r) => Outcome::Raised(r),
            None => Outcome::Value(v),
        };
        crate::publish_line();
        self.fut.complete(outcome);
    }
}

/// The tasks an executor has not started, and its idle workers.
#[derive(Default)]
struct Queue {
    state: Mutex<State>,
    ready: Condvar,
}

#[derive(Default)]
struct State {
    jobs: VecDeque<Job>,
    idle: usize,
    closed: bool,
}

impl Queue {
    /// Queues `job`; whether a worker is idle to take it.
    fn push(&self, job: Job) -> bool {
        let mut state = lock(&self.state);
        state.jobs.push_back(job);
        let idle = state.idle >= state.jobs.len();
        drop(state);
        self.ready.notify_one();
        idle
    }

    /// The next task, or `None` once the queue is closed and empty.
    fn next(&self) -> Option<Job> {
        let mut state = lock(&self.state);
        loop {
            if let Some(job) = state.jobs.pop_front() {
                return Some(job);
            }
            if state.closed {
                return None;
            }
            state.idle += 1;
            state = self
                .ready
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
            state.idle -= 1;
        }
    }

    fn close(&self) {
        lock(&self.state).closed = true;
        self.ready.notify_all();
    }
}

/// An executor: `with ThreadPoolExecutor() as ex`.
pub struct PtExec {
    queue: Arc<Queue>,
    workers: Vec<JoinHandle<()>>,
    limit: usize,
}

impl PtExec {
    fn new(limit: usize) -> PtExec {
        PtExec {
            queue: Arc::default(),
            workers: Vec::new(),
            limit: limit.max(1),
        }
    }

    /// Queues a task, starting a worker for it when none is idle and
    /// the pool is under its limit.
    fn submit(&mut self, thunk: Thunk, args: Vec<u64>, desc: &'static [u8]) -> Arc<PtFut> {
        let fut = Arc::new(PtFut::new(desc));
        let job = Job {
            thunk,
            args,
            fut: Arc::clone(&fut),
        };
        let idle = self.queue.push(job);
        if !idle && self.workers.len() < self.limit {
            self.grow();
        }
        fut
    }

    /// Starts a worker. Should the system refuse a thread, the workers
    /// already running take the task; with none, the program ends.
    fn grow(&mut self) {
        match spawn(Arc::clone(&self.queue)) {
            Ok(w) => self.workers.push(w),
            Err(_) if !self.workers.is_empty() => {}
            Err(err) => crate::fail_with(&format!("cannot start a task's thread: {err}")),
        }
    }

    /// Waits for every task and joins the workers.
    fn end(self) {
        self.queue.close();
        for w in self.workers {
            if w.join().is_err() {
                crate::internal("a task's worker panicked");
            }
        }
    }
}

/// Starts a worker on `queue`.
fn spawn(queue: Arc<Queue>) -> std::io::Result<JoinHandle<()>> {
    let work = move || {
        while let Some(job) = queue.next() {
            job.run();
        }
    };
    thread::Builder::new().stack_size(STACK).spawn(work)
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `ThreadPoolExecutor()`: a new executor.
#[allow(unsafe_code)] // SAFETY: no arguments; the result is owned by the caller.
#[unsafe(no_mangle)]
pub extern "C" fn pt_exec_new() -> *mut PtExec {
    let cpus = thread::available_parallelism().map_or(1, usize::from);
    Box::into_raw(Box::new(PtExec::new(default_workers(cpus))))
}

/// `ex.submit(...)`, or one task of `ex.map(...)`: queues `thunk` with
/// a copy of `args[..n]`, giving a new future (one handle) whose result
/// has type `desc[..len]`.
///
/// # Safety
///
/// `exec` is a live executor this thread made. `args` points to `n`
/// slots (it may dangle when `n` is 0), each holding a reference the
/// task takes. `thunk` reads exactly those slots and the module values
/// and defaults in its function's read set, and every one of these
/// values was marked (`pt_mark`) with everything it reaches. `desc`
/// points to `len` bytes of the compiler's read-only descriptor data.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_submit(
    exec: *mut PtExec,
    thunk: Thunk,
    args: *const u64,
    n: usize,
    desc: *const u8,
    len: usize,
) -> *const PtFut {
    let args = match n {
        0 => Vec::new(),
        // SAFETY: guaranteed by the caller, see above.
        _ => unsafe { std::slice::from_raw_parts(args, n) }.to_vec(),
    };
    // SAFETY: guaranteed by the caller, see above.
    let (exec, desc) = unsafe { (&mut *exec, std::slice::from_raw_parts(desc, len)) };
    Arc::into_raw(exec.submit(thunk, args, desc))
}

/// `fut.result()`: waits for the task, then gives its result with a new
/// reference, or raises again what escaped it and gives 0.
///
/// # Safety
///
/// `fut` is a live future this thread holds a handle to, and this is
/// the thread that submitted its task: handles, and the outcome they
/// release, are that thread's alone.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_result(fut: *const PtFut) -> u64 {
    // SAFETY: guaranteed by the caller.
    let fut = unsafe { &*fut };
    fut.wait(|outcome| match outcome {
        Outcome::Value(v) => {
            seq::inc(fut.desc, *v);
            *v
        }
        Outcome::Raised(r) => {
            r.raise_again();
            0
        }
    })
}

/// One step over `map`'s results (M6c step 4b-2), a list of futures
/// (`lu`) in reverse order ([`seq::pt_list_reverse`]): takes the next
/// future, waits for its task and gives its result with a new
/// reference. When the task raised, raises it again, gives 0, and drops
/// every future not yet taken, as CPython's `map` ends at an exception.
/// An empty list gives 0 and raises nothing; generated code checks the
/// length first.
///
/// # Safety
///
/// `s` is a live list of futures, and this is the thread that submitted
/// their tasks (as for [`pt_result`]).
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_results_next(s: *const PtSeq) -> u64 {
    // SAFETY: guaranteed by the caller.
    let s = unsafe { seq::seq(s as u64) };
    let Some(fut) = seq::pop(s) else {
        return 0;
    };
    // SAFETY: the list held this handle, which is now ours.
    let f = unsafe { &*(fut as *const PtFut) };
    let got = f.wait(|outcome| match outcome {
        Outcome::Value(v) => {
            seq::inc(f.desc, *v);
            Some(*v)
        }
        Outcome::Raised(r) => {
            r.raise_again();
            None
        }
    });
    let rest = match got {
        Some(_) => Vec::new(),
        None => seq::take_all(s),
    };
    for h in rest.into_iter().chain([fut]) {
        // SAFETY: each a handle the list held, now ours, not used again.
        unsafe { pt_fut_dec(h as *const PtFut) };
    }
    got.unwrap_or(0)
}

/// Adds a handle to a future.
///
/// # Safety
///
/// `fut` is a live future this thread holds a handle to, and this is
/// the thread that submitted its task: handles, and the outcome they
/// release, are that thread's alone.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_fut_inc(fut: *const PtFut) {
    // SAFETY: guaranteed by the caller.
    let fut = unsafe { &*fut };
    fut.handles.fetch_add(1, Ordering::Relaxed);
}

/// Drops a handle to a future; the last one releases its outcome here.
/// Null (a local never bound) is nothing.
///
/// # Safety
///
/// `fut` is null or a live future this thread holds a handle to, which
/// is not used again, and this is the thread that submitted its task.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_fut_dec(fut: *const PtFut) {
    if fut.is_null() {
        return;
    }
    // SAFETY: guaranteed by the caller.
    let last = unsafe { &*fut }.drop_handle();
    if last {
        // SAFETY: the handles together held the one `Arc` reference
        // `pt_submit` gave out with `Arc::into_raw`.
        unsafe { Arc::decrement_strong_count(fut) };
    }
}

/// Leaving the executor's `with` block, however it is left: waits for
/// every task it started, then frees it. Null (a block left before
/// its executor was made) is nothing.
///
/// # Safety
///
/// `exec` is null or came from [`pt_exec_new`] on this thread, and is
/// not used again.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_exec_end(exec: *mut PtExec) {
    if exec.is_null() {
        return;
    }
    // SAFETY: guaranteed by the caller.
    let exec = unsafe { Box::from_raw(exec) };
    exec.end();
}

#[cfg(test)]
#[allow(unsafe_code)] // The tests drive the C ABI as generated code does.
mod tests {
    use super::*;
    use crate::str::{self as pstr, PtStr};
    use std::sync::atomic::AtomicU64;

    const INT: &[u8] = b"i";
    const STR: &[u8] = b"s";
    const LIST: &[u8] = b"li";

    /// Adds its two integer arguments.
    unsafe extern "C" fn add(_ctx: *const PtCtx, args: *const u64) -> u64 {
        // SAFETY: submitted with two slots.
        let args = unsafe { std::slice::from_raw_parts(args, 2) };
        args[0] + args[1]
    }

    /// Raises a `ValueError` with no message.
    unsafe extern "C" fn fails(_ctx: *const PtCtx, _args: *const u64) -> u64 {
        exc::raise_value_error();
        0
    }

    /// A new string.
    unsafe extern "C" fn text(_ctx: *const PtCtx, _args: *const u64) -> u64 {
        pstr::into_ptr(PtStr::new("ab".into())) as u64
    }

    /// Reads its list argument, then drops the reference it was given.
    unsafe extern "C" fn read_list(_ctx: *const PtCtx, args: *const u64) -> u64 {
        // SAFETY: submitted with one live list, whose reference it owns.
        unsafe {
            let xs = *args as *mut PtSeq;
            let n = seq::pt_seq_len(xs);
            seq::pt_seq_dec(xs);
            n as u64
        }
    }

    /// Returns its list argument: the result is the marked argument.
    unsafe extern "C" fn same(_ctx: *const PtCtx, args: *const u64) -> u64 {
        // SAFETY: submitted with one live list, whose reference it owns.
        unsafe { *args }
    }

    /// A new list holding its list argument: a fresh, unmarked result
    /// that holds a marked child.
    unsafe extern "C" fn wrap(_ctx: *const PtCtx, args: *const u64) -> u64 {
        // SAFETY: submitted with one live list, whose reference it owns.
        seq::new_seq(NESTED, vec![unsafe { *args }]) as u64
    }

    const NESTED: &[u8] = b"lli";

    static SEEN: AtomicU64 = AtomicU64::new(0);

    /// Records its argument.
    unsafe extern "C" fn note(_ctx: *const PtCtx, args: *const u64) -> u64 {
        // SAFETY: submitted with one slot.
        SEEN.fetch_add(unsafe { *args }, Ordering::Relaxed);
        0
    }

    fn submit(exec: *mut PtExec, thunk: Thunk, args: &[u64], desc: &'static [u8]) -> *const PtFut {
        // SAFETY: the test's executor, slots and descriptor are live.
        unsafe {
            pt_submit(
                exec,
                thunk,
                args.as_ptr(),
                args.len(),
                desc.as_ptr(),
                desc.len(),
            )
        }
    }

    fn result(fut: *const PtFut) -> u64 {
        // SAFETY: the test holds a handle.
        unsafe { pt_result(fut) }
    }

    fn drop_fut(fut: *const PtFut) {
        // SAFETY: the test's handle, not used again.
        unsafe { pt_fut_dec(fut) }
    }

    fn end(exec: *mut PtExec) {
        // SAFETY: the test's executor, not used again.
        unsafe { pt_exec_end(exec) }
    }

    /// Takes the exception `result()` raised again, checking it is a
    /// `ValueError`.
    fn caught() -> *mut PtSeq {
        assert_eq!(exc::pt_pending_is(1 << exc::VALUE_ERROR), 1);
        let e = exc::pt_catch();
        assert!(!e.is_null());
        e
    }

    #[test]
    fn the_default_pool_is_cpythons() {
        assert_eq!(
            (default_workers(1), default_workers(8), default_workers(64)),
            (5, 12, 32)
        );
    }

    #[test]
    fn tasks_give_their_results() {
        let exec = pt_exec_new();
        let futs: Vec<*const PtFut> = (0..8).map(|n| submit(exec, add, &[n, 10], INT)).collect();
        let got: Vec<u64> = futs.iter().map(|f| result(*f)).collect();
        assert_eq!(got, (10..18).collect::<Vec<u64>>());
        futs.into_iter().for_each(drop_fut);
        end(exec);
    }

    #[test]
    fn leaving_the_block_waits_for_every_task() {
        let exec = Box::into_raw(Box::new(PtExec::new(3)));
        let before = SEEN.load(Ordering::Relaxed);
        for _ in 0..5 {
            drop_fut(submit(exec, note, &[1], INT));
        }
        end(exec);
        assert_eq!(SEEN.load(Ordering::Relaxed) - before, 5);
    }

    #[test]
    fn what_escapes_a_task_is_raised_by_each_result() {
        let exec = pt_exec_new();
        let fut = submit(exec, fails, &[], INT);
        for _ in 0..2 {
            assert_eq!(result(fut), 0);
            // SAFETY: `pt_catch` gave this thread the reference.
            unsafe { seq::pt_seq_dec(caught()) };
        }
        drop_fut(fut);
        end(exec);
    }

    /// The worker drops nothing a submitter may still count: here the
    /// exception, raised again and then used after the future's last
    /// handle went. Under Miri a race or a leak fails it.
    #[test]
    fn an_outcome_taken_is_released_on_the_submitters_thread() {
        let exec = pt_exec_new();
        for _ in 0..50 {
            let fut = submit(exec, fails, &[], INT);
            assert_eq!(result(fut), 0);
            let e = caught();
            let copy = submit(exec, text, &[], STR);
            let s = result(copy) as *mut PtStr;
            drop_fut(fut);
            drop_fut(copy);
            // SAFETY: this thread holds a reference to each.
            unsafe {
                assert_eq!((*e).rc.get(), 1);
                assert_eq!(pstr::borrow(s).text(), "ab");
                seq::pt_seq_inc(e);
                seq::pt_seq_dec(e);
                seq::pt_seq_dec(e);
                pstr::pt_str_dec(s);
            }
        }
        end(exec);
    }

    /// A future dropped before its task ends leaves the outcome to the
    /// worker. Under Miri a leak fails it.
    #[test]
    fn an_abandoned_outcome_is_released_by_the_worker() {
        let exec = pt_exec_new();
        drop_fut(submit(exec, fails, &[], INT));
        drop_fut(submit(exec, text, &[], STR));
        end(exec);
    }

    #[test]
    fn handles_count_until_the_last() {
        let exec = pt_exec_new();
        let fut = submit(exec, add, &[1, 2], INT);
        // SAFETY: the test holds a handle.
        unsafe { pt_fut_inc(fut) };
        drop_fut(fut);
        assert_eq!(result(fut), 3);
        drop_fut(fut);
        end(exec);
    }

    /// A list of futures (`lu`) counts its handles: copying one in adds
    /// a handle, and freeing the list drops each. Under Miri a leak or a
    /// double free fails it.
    #[test]
    fn a_list_of_futures_holds_their_handles() {
        const FUTS: &[u8] = b"lu";
        let exec = pt_exec_new();
        let fut = submit(exec, add, &[2, 3], INT);
        let futs = seq::new_seq(FUTS, Vec::new());
        // SAFETY: a live list of futures, and the test's live handle.
        unsafe { seq::pt_seq_push(futs, fut as u64) };
        drop_fut(fut);
        let mut status = 0u8;
        // SAFETY: a live list of futures, and a writable status byte.
        let got = unsafe { seq::pt_seq_get(futs, 0, &mut status) } as *const PtFut;
        assert_eq!((status, result(got)), (0, 5));
        drop_fut(got);
        // SAFETY: the test's reference to the list, the last.
        unsafe { seq::pt_seq_dec(futs) };
        end(exec);
    }

    /// `map`'s results: a reversed list of futures, each taken once, in
    /// order; an exception drops the futures not yet taken, and an
    /// empty list gives 0. Under Miri a leak or a double free fails it.
    #[test]
    fn results_are_taken_once_and_end_at_an_exception() {
        const FUTS: &[u8] = b"lu";
        let exec = pt_exec_new();
        let futs = seq::new_seq(FUTS, Vec::new());
        let tasks: [(Thunk, &[u64]); 4] =
            [(add, &[1, 2]), (add, &[3, 4]), (fails, &[]), (add, &[5, 6])];
        for (thunk, args) in tasks {
            let fut = submit(exec, thunk, args, INT);
            // SAFETY: a live list of futures, and the test's live handle.
            unsafe { seq::pt_seq_push(futs, fut as u64) };
            drop_fut(fut);
        }
        // SAFETY: the test's live list, on the submitting thread.
        let next = || unsafe { pt_results_next(futs) };
        // SAFETY: as above.
        unsafe { seq::pt_list_reverse(futs) };
        assert_eq!((next(), next()), (3, 7));
        assert_eq!(next(), 0);
        // SAFETY: `pt_catch` gave this thread the reference.
        unsafe { seq::pt_seq_dec(caught()) };
        // SAFETY: as above.
        assert_eq!(unsafe { seq::pt_seq_len(futs) }, 0);
        assert_eq!(next(), 0);
        assert!(exc::take_raised().is_none());
        // SAFETY: the test's reference to the list, the last.
        unsafe { seq::pt_seq_dec(futs) };
        end(exec);
    }

    /// Leaving a block before its executor was made, or dropping a
    /// future local never bound, is nothing.
    #[test]
    fn null_executors_and_futures_are_nothing() {
        end(std::ptr::null_mut());
        drop_fut(std::ptr::null());
    }

    /// Outcomes that alias a marked argument: the argument itself, and a
    /// fresh list holding it, each taken or abandoned. The submitter
    /// and the workers count the argument at once. Under Miri a race, a
    /// leak or a double free fails it.
    #[test]
    fn outcomes_holding_a_marked_argument_are_released_once() {
        let exec = Box::into_raw(Box::new(PtExec::new(4)));
        let xs = seq::new_seq(LIST, vec![1, 2]);
        seq::mark(LIST, xs as u64);
        let mut futs = Vec::new();
        for (thunk, desc) in [(same as Thunk, LIST), (wrap, NESTED)].repeat(4) {
            // SAFETY: `xs` is live; the task takes this reference.
            unsafe { seq::pt_seq_inc(xs) };
            futs.push((submit(exec, thunk, &[xs as u64], desc), desc));
        }
        for (k, (fut, desc)) in futs.into_iter().enumerate() {
            if k % 4 < 2 {
                let got = result(fut);
                assert!(got != 0);
                seq::dec(desc, got);
            }
            drop_fut(fut);
        }
        end(exec);
        // SAFETY: the test's own reference, the last.
        unsafe {
            assert_eq!((*xs).rc.get(), 1);
            seq::pt_seq_dec(xs);
        }
    }

    /// Tasks on several workers count a marked argument at once.
    #[test]
    fn a_marked_argument_is_counted_by_every_worker() {
        let exec = Box::into_raw(Box::new(PtExec::new(4)));
        let xs = seq::new_seq(LIST, vec![1, 2]);
        seq::mark(LIST, xs as u64);
        let futs: Vec<*const PtFut> = (0..8)
            .map(|_| {
                // SAFETY: `xs` is live; the task takes this reference.
                unsafe { seq::pt_seq_inc(xs) };
                submit(exec, read_list, &[xs as u64], INT)
            })
            .collect();
        assert!(futs.iter().all(|f| result(*f) == 2));
        futs.into_iter().for_each(drop_fut);
        end(exec);
        // SAFETY: the test's own reference, the last.
        unsafe {
            assert_eq!((*xs).rc.get(), 1);
            seq::pt_seq_dec(xs);
        }
    }

    /// One worker runs a failing task, then one that succeeds: nothing
    /// pending carries over.
    #[test]
    fn a_worker_starts_each_task_clean() {
        let exec = Box::into_raw(Box::new(PtExec::new(1)));
        let bad = submit(exec, fails, &[], INT);
        let good = submit(exec, add, &[2, 3], INT);
        assert_eq!(result(good), 5);
        assert_eq!(exc::pt_pending_is(u64::MAX), 0);
        drop_fut(bad);
        drop_fut(good);
        end(exec);
    }

    /// A task's unfinished line is published when the task ends.
    #[test]
    fn a_task_publishes_its_unfinished_line() {
        crate::write(b"partial");
        let fut = Arc::new(PtFut::new(INT));
        let job = Job {
            thunk: note,
            args: vec![0],
            fut: Arc::clone(&fut),
        };
        job.run();
        assert!(crate::LINE.with_borrow(Vec::is_empty));
        drop_fut(Arc::into_raw(fut));
    }
}
