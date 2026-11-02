//! Tasks (M6c, `docs/research/concurrency.md`): an executor's pool of
//! worker threads, and the futures its tasks complete.
//!
//! An executor starts workers as tasks arrive, one OS thread per task
//! running at once, up to the machine's parallelism; leaving the `with`
//! block ([`pt_exec_end`]) waits for every task and joins them. A task
//! is a compiled thunk that unpacks its arguments, calls the task's
//! function on the worker with the worker's own context, and returns
//! the result's slot. The worker keeps what escaped (a pending
//! exception) instead, and publishes the task's unfinished output line,
//! so every task's output reaches stdout when it ends.
//!
//! What crosses threads, and why no count is shared unmarked:
//! - The arguments the thunk reads are marked by the submitter first
//!   (`pt_mark`), so the worker's count operations on them are atomic.
//! - A result, or an escaped exception, the worker made is reached by
//!   the worker alone until it completes the future, and by the
//!   submitter alone after: the future's lock orders the worker's last
//!   step before the submitter's first, so plain counts suffice.
//! - The future is the one value both threads hold at once; it is an
//!   `Arc`, so its count is atomic, and generated code never touches
//!   its fields.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread::{self, JoinHandle};

use crate::exc::{self, PtCtx};
use crate::seq;

/// A task's compiled entry: called on a worker with the worker's
/// context and the arguments its submitter packed, it returns the
/// result's slot (0 when an exception escaped), and drops the
/// references it was given.
pub type Thunk = unsafe extern "C" fn(ctx: *const PtCtx, args: *const u64) -> u64;

/// A worker's stack: the main thread's usual 8 MiB, so a task reaches
/// the same call depth as the module's code before its
/// `RecursionError`.
const STACK: usize = 8 << 20;

/// How a task ended.
enum Outcome {
    /// Its result's slot, holding a reference.
    Value(u64),
    /// What escaped it, raised again by each `result()`.
    Raised(exc::Raised),
}

/// A task's result, shared by the worker that completes it and the
/// submitter that takes it.
pub struct PtFut {
    /// The result's type descriptor, for its references.
    desc: &'static [u8],
    done: Mutex<Option<Outcome>>,
    ready: Condvar,
}

impl PtFut {
    fn new(desc: &'static [u8]) -> PtFut {
        PtFut {
            desc,
            done: Mutex::new(None),
            ready: Condvar::new(),
        }
    }

    fn complete(&self, outcome: Outcome) {
        *lock(&self.done) = Some(outcome);
        self.ready.notify_all();
    }

    /// Waits for the task; runs `take` on how it ended.
    fn wait<R>(&self, take: impl FnOnce(&Outcome) -> R) -> R {
        let mut done = lock(&self.done);
        loop {
            if let Some(outcome) = done.as_ref() {
                return take(outcome);
            }
            done = self
                .ready
                .wait(done)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }
}

impl Drop for PtFut {
    fn drop(&mut self) {
        let outcome = self.done.get_mut().unwrap_or_else(PoisonError::into_inner);
        match outcome.take() {
            Some(Outcome::Value(v)) => seq::dec(self.desc, v),
            Some(Outcome::Raised(r)) => r.release(),
            None => {}
        }
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

/// The tasks an executor has not started.
#[derive(Default)]
struct Queue {
    state: Mutex<(VecDeque<Job>, bool)>,
    ready: Condvar,
}

impl Queue {
    fn push(&self, job: Job) {
        lock(&self.state).0.push_back(job);
        self.ready.notify_one();
    }

    /// The next task, or `None` once the queue is closed and empty.
    fn next(&self) -> Option<Job> {
        let mut state = lock(&self.state);
        loop {
            if let Some(job) = state.0.pop_front() {
                return Some(job);
            }
            if state.1 {
                return None;
            }
            state = self
                .ready
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    fn close(&self) {
        lock(&self.state).1 = true;
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

    /// Queues a task, starting a worker for it while under the limit.
    fn submit(&mut self, thunk: Thunk, args: Vec<u64>, desc: &'static [u8]) -> Arc<PtFut> {
        let fut = Arc::new(PtFut::new(desc));
        let job = Job {
            thunk,
            args,
            fut: Arc::clone(&fut),
        };
        self.queue.push(job);
        if self.workers.len() < self.limit {
            self.workers.push(spawn(Arc::clone(&self.queue)));
        }
        fut
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
fn spawn(queue: Arc<Queue>) -> JoinHandle<()> {
    let work = move || {
        while let Some(job) = queue.next() {
            job.run();
        }
    };
    match thread::Builder::new().stack_size(STACK).spawn(work) {
        Ok(handle) => handle,
        Err(err) => crate::fail_with(&format!("cannot start a task's thread: {err}")),
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// `ThreadPoolExecutor()`: a new executor, as many workers at most as
/// the machine runs threads at once.
#[allow(unsafe_code)] // SAFETY: no arguments; the result is owned by the caller.
#[unsafe(no_mangle)]
pub extern "C" fn pt_exec_new() -> *mut PtExec {
    let limit = thread::available_parallelism().map_or(1, usize::from);
    Box::into_raw(Box::new(PtExec::new(limit)))
}

/// `ex.submit(...)`, or one task of `ex.map(...)`: queues `thunk` with
/// a copy of `args[..n]`, giving a new future whose result has type
/// `desc[..len]`.
///
/// # Safety
///
/// `exec` is a live executor this thread made; `args` points to `n`
/// slots, each holding a reference the task takes, of values marked
/// (`pt_mark`) with everything they reach; `thunk` reads exactly
/// those slots; `desc` points to `len` bytes of the compiler's
/// read-only descriptor data.
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
    // SAFETY: guaranteed by the caller, see above.
    let (exec, args, desc) = unsafe {
        (
            &mut *exec,
            std::slice::from_raw_parts(args, n).to_vec(),
            std::slice::from_raw_parts(desc, len),
        )
    };
    Arc::into_raw(exec.submit(thunk, args, desc))
}

/// `fut.result()`: waits for the task, then gives its result with a new
/// reference, or raises again what escaped it and gives 0.
///
/// # Safety
///
/// `fut` is a live future.
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

/// Adds a reference to a future.
///
/// # Safety
///
/// `fut` is a live future.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_fut_inc(fut: *const PtFut) {
    // SAFETY: guaranteed by the caller; `fut` came from `Arc::into_raw`.
    unsafe { Arc::increment_strong_count(fut) };
}

/// Drops a reference to a future; the last one drops its result.
///
/// # Safety
///
/// `fut` is a live future, and the caller's reference is not used again.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_fut_dec(fut: *const PtFut) {
    // SAFETY: guaranteed by the caller; `fut` came from `Arc::into_raw`.
    unsafe { Arc::decrement_strong_count(fut) };
}

/// Leaving the executor's `with` block: waits for every task it
/// started, then frees it.
///
/// # Safety
///
/// `exec` came from [`pt_exec_new`] on this thread and is not used again.
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn pt_exec_end(exec: *mut PtExec) {
    // SAFETY: guaranteed by the caller.
    let exec = unsafe { Box::from_raw(exec) };
    exec.end();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    const INT: &[u8] = b"i";

    /// Adds its two integer arguments.
    #[allow(unsafe_code)]
    unsafe extern "C" fn add(_ctx: *const PtCtx, args: *const u64) -> u64 {
        // SAFETY: submitted with two slots.
        let args = unsafe { std::slice::from_raw_parts(args, 2) };
        args[0] + args[1]
    }

    /// Raises a `ValueError` with no message.
    #[allow(unsafe_code)]
    unsafe extern "C" fn fails(_ctx: *const PtCtx, _args: *const u64) -> u64 {
        exc::raise_value_error();
        0
    }

    static SEEN: AtomicU64 = AtomicU64::new(0);

    /// Records its argument.
    #[allow(unsafe_code)]
    unsafe extern "C" fn note(_ctx: *const PtCtx, args: *const u64) -> u64 {
        // SAFETY: submitted with one slot.
        let n = unsafe { *args };
        SEEN.fetch_add(n, Ordering::Relaxed);
        0
    }

    #[allow(unsafe_code)]
    fn submit(exec: &mut PtExec, thunk: Thunk, args: &[u64]) -> Arc<PtFut> {
        exec.submit(thunk, args.to_vec(), INT)
    }

    #[allow(unsafe_code)]
    fn result(fut: &Arc<PtFut>) -> u64 {
        // SAFETY: the test holds the future.
        unsafe { pt_result(Arc::as_ptr(fut)) }
    }

    #[test]
    fn tasks_give_their_results() {
        let mut exec = PtExec::new(2);
        let futs: Vec<Arc<PtFut>> = (0..8).map(|n| submit(&mut exec, add, &[n, 10])).collect();
        let got: Vec<u64> = futs.iter().map(result).collect();
        assert_eq!(got, (10..18).collect::<Vec<u64>>());
        assert!(exec.workers.len() <= 2);
        exec.end();
    }

    #[test]
    fn leaving_the_block_waits_for_every_task() {
        let mut exec = PtExec::new(3);
        let before = SEEN.load(Ordering::Relaxed);
        for _ in 0..5 {
            drop(submit(&mut exec, note, &[1]));
        }
        exec.end();
        assert_eq!(SEEN.load(Ordering::Relaxed) - before, 5);
    }

    #[test]
    fn what_escapes_a_task_is_raised_by_each_result() {
        let mut exec = PtExec::new(1);
        let fut = submit(&mut exec, fails, &[]);
        for _ in 0..2 {
            assert_eq!(result(&fut), 0);
            assert_eq!(exc::pt_pending_is(1 << exc::VALUE_ERROR), 1);
            let e = exc::pt_catch();
            assert!(!e.is_null());
            #[allow(unsafe_code)]
            // SAFETY: `pt_catch` gave this thread the reference.
            unsafe {
                seq::pt_seq_dec(e)
            };
        }
        exec.end();
    }

    /// Under Miri, a leak of the exception fails the test.
    #[test]
    fn a_dropped_futures_exception_is_released() {
        let mut exec = PtExec::new(1);
        drop(submit(&mut exec, fails, &[]));
        exec.end();
    }
}
