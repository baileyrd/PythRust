# M6c: concurrency

**Status: decided 2026-11-01.** Nano approved the scope proposal's six recommendations (Decided 1 to 6 below); the six points marked **For Nano** further down are still open. The `pythrust-expert` review of this note's first draft found that rule 4, as first worded, missed races between tasks, through nested containers and module variables, and in the runtime's own read paths; the rules below are tightened to close them, and the points that are new language rules are marked **For Nano**. Not yet implemented; [roadmap.md](../roadmap.md#phase-5-m6-plan) orders the work. The syntax for pausing and locking was settled earlier in [concurrency-syntax.md](concurrency-syntax.md); this note settles how tasks start, how values cross threads, what `run` does, and narrows what a pause point is.

## Objective

[design.md](../design.md#concurrency) promises uncolored concurrency with three sharing tiers: move or lend read-only, `shared[T]`, and channels. M6c delivers it in the smallest form a real program needs first: fclone-lite hashing each size group's candidates in parallel. `shared[T]`, `pause` and channels follow the driver, so a measured program sets their priority.

## Decided

1. **Tasks start through `concurrent.futures`.** A subset of CPython's `ThreadPoolExecutor`: `with concurrent.futures.ThreadPoolExecutor() as ex:`, `ex.submit(f, *args)` returning a future, `fut.result()`, and `ex.map(f, xs)`. Leaving the `with` block waits for every task it started, so a task's lifetime is lexical, as with `std::thread::scope`. CPython runs these programs unchanged, so it stays the oracle. Rejected: `threading.Thread` (no result value, and tasks outlive any scope, so lending cannot be proven) and a new `spawn` builtin (not Python).
2. **One OS thread per running task, in a pool.** `max_workers` defaults to CPython 3.13's `min(32, (os.process_cpu_count() or 1) + 4)`. A blocking call blocks its worker. M:N parking of lightweight tasks (design.md) can replace the pool later without changing any program's meaning.
3. **Reference counts are atomic only for values that cross.** A value a task can reach is marked thread-shared, as in Koka's Perceus runtime; a count operation tests the mark and uses an atomic operation only on a marked value. Single-threaded programs keep the non-atomic path. Gate: no benchmark in [benchmarks.md](../benchmarks.md) slows by more than 3% when no task runs.
4. **Exclusivity over a task's lifetime, in loose code too.** From `submit` to the end of the executor's `with` block, no two parties (the submitter and each task the block started) may touch one container where either changes or keeps it. A race on a list is memory unsafety natively, so this holds in loose code as well: a program CPython runs may be rejected, never run differently. Each rejection is a listed deviation. [Rule 4 in full](#rule-4-in-full) spells out what counts.
5. **`run` runs each task to completion at `submit`,** in program order, with its own call depth starting from zero. That is a schedule a real pool may also produce; [Run at submit](#run-at-submit) lists what it does not reproduce.
6. **Pause points are waits on other tasks:** `result()`, each step of iterating `map`'s results, leaving an executor's `with` block, a queue's `get` and `put`, and acquiring a `shared` lock (each when it lands). File I/O and `print` are not pause points in this cut, which narrows [concurrency-syntax.md](concurrency-syntax.md#pause-marker); counting them would put `pause` on every `print` in tempered code and promise no interleaving guarantee under OS threads. Revisit with M:N parking.

## The surface

```python
import concurrent.futures
import hashlib


def digest(path: str) -> str:
    f = open(path, "rb")
    data = f.read()
    f.close()
    return hashlib.sha256(data).hexdigest()


def digests(paths: list[str]) -> list[str]:
    out: list[str] = []
    with concurrent.futures.ThreadPoolExecutor() as ex:
        for h in ex.map(digest, paths):
            out.append(h)
    return out
```

- `import concurrent.futures` is the one dotted import, and `ThreadPoolExecutor` is reached through it; the other import forms stay in [Deliberate omissions](../grammar.md#deliberate-omissions-phase-2).
- `with` leaves the reserved words. In this cut `with` takes only an executor; `shared` values follow in step 6 of the plan.
- A future's type is `Future[T]` where `T` is the function's result; `result()` returns `T` and re-raises what escaped the task, so the task's `raises` set joins the caller's at `result()`. A catchable fault in a task also re-raises there, as in CPython. A fatal fault (overflow, recursion depth, out of memory) in a task stops the program with `run`'s diagnostic, as D7 makes it fatal anywhere.
- `ex.submit(f, *args)` and `ex.map(f, xs)` take a function name, checked like a call of `f` with those arguments; `own`'s summaries also count them as calls of `f`, so a task that itself submits work, and a nested executor, are covered. `map` reads `xs` once, when called, and submits every task then, as CPython does. Keyword arguments, `max_workers` and `shutdown` wait for a program that needs them.
- **For Nano (scope), proposed:** an executor, its futures and `map`'s result stay local to the function that made them: they cannot be returned, passed to a call, stored in a global or held by an exception; only a local list may hold futures, and that list cannot be passed either; `map`'s result is used only inside the block. An executor at module level is refused. Since Pythrust has no `global` statement and `def` is top-level only, only module-level code rebinds a module variable, so this also means no module variable is rebound while a task runs. Rule 4's handling of module variables depends on this; the alternative (module-level executors) needs a rule against rebinding a module variable any task reads before the block ends.
- **For Nano (tempered code), proposed:** tempered functions may not use executors until the `pause` check lands (step 6), so step 6 cannot break a tempered program step 3 accepted. The alternative is to land the `pause` check with step 3.

## Rule 4 in full

Each party is the submitter (the function that owns the `with` block, with everything it does and calls from its first `submit` or `map` to the block's end) or one task (the submitted function, with everything it calls). For each party, `own`'s summaries (A3, A8) give the containers it reads, changes and keeps, including module containers its callees reach and the default values of parameters (a default is one container shared by every call that binds it).

- **Reach.** Touching a value touches every container reachable from it: nested lists, dicts and sets, and lists inside tuples, optionals and exceptions. A6's rule (1), that a list inside another is its own list, does not apply across parties: `r = rows[0]` then `r.append(1)` while a task reads `rows` is rejected.
- **Pairs.** Two parties conflict when one changes or keeps a container the other touches. The submitter is checked against every task, and every task against every other task of the block, including tasks of a nested executor.
- **Files, for Nano, proposed:** no file reaches a task in this cut. A task's argument types (bound defaults included), and the types of the module variables and defaults in its `own` read set, may not hold a file anywhere inside them (a list, dict, set, tuple or optional of one); `dyn` in those types is refused too until a runtime check can see inside it (the check is on types, so it does not depend on `own` following a file through copies, which it does only syntactically, per grammar.md's parameter-modes note). A task may open and use its own files, as `digest` above does. The alternative is to let one task own a file and forbid the submitter any `read` or `close` on a file that existed before the block, from its first `submit` to the block's end.
- **Results.** `result()` and each value `map` yields carry the submitted function's return-alias summary (D3 lets a loose function return its parameter), so a result that may share a container with an argument counts as that container.
- **For Nano (map), proposed:** `ex.map(f, xs)` where `f` changes or keeps its parameter is refused in this cut. The alternative is an A5-style proof that `xs`'s elements are distinct; without one of the two, rule 4's pair check does not cover `map`.
- Strings are immutable and their counts are covered by rule 3; nothing else about them is shared.

| Example | Verdict |
| --- | --- |
| a task reads `xs`; the submitter reads `xs` | accepted |
| a task reads `xs`; the submitter appends to `xs` | rejected: "`xs` is read by a task until the `with` block ends" |
| a task changes `xs`; the submitter uses `xs` after the block | accepted |
| `ex.map(add, [1, 2])` where `add` appends to a module list | rejected: two tasks change one list |
| a task reads `rows`; the submitter appends to `rows[0]` | rejected (reach) |
| `ex.submit(rd, f)` twice, `rd` reading the file | rejected (files) |
| `ex.submit(ident, xs).result().append(1)` while another task reads `xs` | rejected (results) |

## Runtime changes

- **Reads must not race.** Today every read of a list, tuple, exception, dict or set takes a `RefCell` borrow (`seq.rs`, `table.rs`), which writes a non-atomic borrow counter, so two concurrent readers race. **For Nano, proposed:** step 2 first tries safe cells with an atomic borrow counter that never blocks and panics on a conflict (as `atomic_refcell`'s `AtomicRefCell`, written in the runtime rather than added as a dependency), so a compiler bug in rule 4 is a panic rather than undefined behaviour; one thread may hold several read borrows of one cell, as `seq_cmp` does for `xs < xs`. Not `std::sync::RwLock`: a reentrant read may deadlock and a conflicting write blocks. Step 2 measures them against the 3% gate. Only if they miss it does step 2 propose a runtime-internal cell whose reads write nothing, relying on rule 4 alone; that is new `unsafe` outside the C ABI surface CLAUDE.md allows, so it would come back to Nano first.
- **Marking.** At each `submit` (and each task `map` submits), the runtime marks the arguments (including defaults the submit leaves unbound, which the submitter loads) and the values of the module variables and defaults in the new task's `own` read set, each with everything it reaches. Marking skips a value already marked, which also ends the walk on a cycle, and the mark is read atomically and never written twice, so a walk that reaches a value an earlier task is reading does not race with it. Rule 4 guarantees no running task changes those, so marking never walks a container another thread is changing; marking all module values instead would race with a task appending to a module list it alone uses. Results are not marked again: a result is fresh or reachable from marked values, and a fresh result a task stored in a module container can be reached during the block only by parties rule 4 already rejects. The future's completion synchronises the threads (release on completion, acquire in `result()`), so the worker's last count operation happens before the submitter's first. Marking costs time in proportion to what is reachable; the 3% gate measures programs where no task runs.
- **Per thread:** the pending exception and its flag (codegen reaches the flag only through `FnGen::pending_flag`, [faults.md](faults.md)) and the call-depth counter `pythrust_depth`, so each task counts depth from zero as CPython's threads do. Step 2 checks that Cranelift's thread-local support works on all four CI targets.
- **Output.** Buffered stdout keeps its lock, now contended; step 2 checks that one `print` call writes its whole line under one acquisition. `LIVE`, the leak counter, is already atomic.
- The interpreter keeps `Rc`, which rule 5 allows.

## Run at submit

Running each task at `submit` matches a native build and CPython for programs rule 4 accepts, except where scheduling shows. Step 3 records each difference below in Deviations:

- **Print order.** Output printed by a task and by the submitter or another task running at the same time may interleave differently.
- **Unobserved exceptions.** A catchable exception escaping a task whose `result()` is never called is dropped, as in CPython. Under run-at-submit the task still runs to completion first.
- **Breaking out of `map`.** CPython cancels tasks that have not started when a `for` over `map`'s results is left early; Pythrust runs every task `map` submitted. **For Nano, proposed:** keep this as a deviation (recommended), or refuse `break`/`return` out of a `for` over `map`.

## Oracle

CPython 3.13 runs every accepted program. The conformance corpus keeps only programs whose output does not depend on scheduling: results gathered through `result()` or `map`, no `print` in a task, and no early exit from a `for` over `map`. Programs that rule 4 rejects carry `# expect-error:` markers and are listed in Deviations.

## Driver

fclone-lite hashes each size group's candidates through an executor. Its groups must equal `fclone_lite.py`'s as sets. Benchmark rows compare it with the serial native build and with `rusty_fclone`, which hashes with rayon. No speedup is claimed before those rows.

## Not in this cut

M:N parking, `async`/`await`, process pools, `threading` and its locks, cancellation and timeouts, `Future.done()`/`add_done_callback`/`as_completed`, and handing Pythrust values to Rust threads ([rust-interop.md](rust-interop.md#threads) keeps that closed). A task may call an M6b binding; the binding's "calling thread" is then the worker.

## Risks

- **Channels under rule 5.** A task waiting on a queue that a later task feeds would hang under run-at-submit. When channels land, `run` either refuses that shape by name or gains a small cooperative scheduler; the choice goes to Nano with that step.
- **The gate is below wall-time noise.** benchmarks.md's shared-container timings vary by more than 3%, so the gate is measured in instruction counts (`valgrind --tool=cachegrind`, `Ir`), with wall time reported beside it. If marking misses the gate, the fallback is atomic counts only in programs that import `concurrent.futures`, chosen per build.
- **Memcheck does not see races.** The threaded corpus also runs under a race detector (ThreadSanitizer, or Helgrind/DRD where TSan is unavailable), and the runtime's stress tests run under Miri where they can.
- **Rule reach.** Rule 4 may reject more CPython programs than expected; the front-end step's corpus measures this before native work starts.
- **Platforms.** Thread creation, stack size and thread-local storage differ across Linux, macOS and Windows; every task needs the same recursion limit and message as `run`.
