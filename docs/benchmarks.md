# Native benchmarks (M4)

Four programs, each valid Python, in `bench/`. Each is compiled with `pythrust build` and timed against CPython running the same file. Run them with:

```
cargo build --release && cargo build --release -p pythrust-rt
python3 bench/run.py --runs 7
```

`run.py` fails if a native program's output differs from CPython's, and reports the median wall time, including process start.

## Results

Linux x86-64, 4-core Intel Xeon at 2.80 GHz in a shared cloud container, CPython 3.13.17, release build, 7 runs each. Times are the median with the fastest and slowest run; the ranges are wide (the container is shared), so read the speedups to about one significant figure.

| Benchmark | What it does | Native s | CPython s | Speedup |
|---|---|---:|---:|---:|
| `numeric` | Collatz step counts for 1..10⁶: int arithmetic, branches, calls | 0.637 (0.596–0.692) | 12.989 (12.957–14.516) | 20x |
| `sorting` | in-place quicksort of 300,000 ints, plus a sortedness check | 0.121 (0.112–0.190) | 0.996 (0.785–1.149) | 8x |
| `strings` | 10⁶ small strings built with `+` and `str()`, plus one growing line | 0.452 (0.405–0.496) | 0.586 (0.503–0.703) | 1.3x |
| `tuples` | digit counts and sums via `div_mod` returning a tuple, unpacked: about 6.9×10⁶ tuples | 0.617 (0.554–0.692) | 1.113 (0.962–1.225) | 1.8x |
| `tuples_baseline` | control for `tuples`, below; not one of the four | 0.091 (0.082–0.101) | 1.080 (0.974–1.280) | 12x |

## Reading them

- **Scalar code is fast.** `numeric` is about 20x faster than CPython: checked `i64` arithmetic in registers, against boxed ints and bytecode dispatch.
- **Lists of ints are fast enough.** `sorting` is about 8x: each `xs[i]` is a call into the runtime (bounds check, slot load), not an inline load.
- **String building is at parity.** `strings` spends its time allocating: every `+` and `str()` makes a new string object (malloc, a reference count, a UTF-8 copy), as CPython does, and CPython's allocator is well tuned for exactly this. The growing line is not the cost: without it, one measurement gave 0.42 s native and 0.45 s CPython.
- **Tuples are where native code loses most of its edge.** `tuples_baseline` does the same digit scan as `tuples`, with the same parameter divisor and a call per digit, but no tuple: `div_mod` becomes two calls (`quot`, `rem`), and `digits` packs its count and total into one int that the caller splits. The tuple version takes about 6.8x as long (0.617 s against 0.091 s). Most of that gap is plausibly the boxed tuple (allocate, push two slots, read two fields through the runtime, drop), but the programs still differ: the control makes one more call per digit, which favours the tuple version, and packs and unpacks an int per number, which favours neither much. So the gap is about 76 ns per tuple amortized over the whole program, not an isolated cost of the layout. A micro-benchmark of tuple construction alone would pin it down.

Where values stay in registers, native code is an order of magnitude faster; where it allocates, each allocation is an opaque runtime call and most of the advantage goes. Those runtime calls, and the boxed tuple layout, are what to look at when performance becomes the goal.

## Refcount elision (A7)

Before `rc::elide`, after it (#50), and after its follow-ups (cross-block copies, last-use moves, `pt_seq_dec` without an allocation), same machine (4-core Xeon, shared container), release build, 5 runs each, median seconds:

| Benchmark | Before | #50 | Follow-ups |
|---|---:|---:|---:|
| `numeric` | 0.483 | 0.487 | 0.484 |
| `sorting` | 0.085 | 0.081 | 0.080 |
| `strings` | 0.243 | 0.240 | 0.244 |
| `tuples` | 0.410 | 0.407 | 0.355 |
| `tuples_baseline` | 0.081 | 0.081 | 0.081 |

`sorting` gains about 5% from #50: `quicksort` borrows its list, so its recursive calls touch no count. `tuples` gains about 13% from the follow-ups, nearly all of it the runtime change: `pt_seq_dec` used to allocate a work list on every call, even when the count stayed above zero, and now allocates only when a freed sequence holds another. The rest is allocation (every `+`, `str()` and tuple is a runtime object) and runtime calls, which elision does not reach.

## Shared values (M6c)

A value a task can reach is *marked*, and its count and borrow word then change by atomic read-modify-writes; every other value keeps plain loads and stores (`sync.rs`). Same machine (4-core Xeon, shared container), release build.

Per operation, one thread, uncontended (`cargo test --release -p pythrust-rt -- --ignored --nocapture atomics_cost`, three runs):

| Operation | Unmarked ns | Marked ns |
|---|---:|---:|
| count `inc` then `dec` | 1.0 to 1.9 | 19.9 to 24.8 |
| read borrow taken and dropped | 0.6 to 1.6 | 18.6 to 19.6 |

On a program, 7 runs, median seconds (`bench/run.py`): `shared` gives one task a list of 200,000 strings to walk 40 times; `shared_baseline` calls the same function directly, so nothing is marked.

| Benchmark | Native s | CPython s |
|---|---:|---:|
| `shared` | 0.430 (0.385 to 0.485) | 0.287 (0.253 to 0.425) |
| `shared_baseline` | 0.122 (0.101 to 0.171) | 0.261 (0.213 to 0.433) |

Here the task is slower than the serial run: the task version takes 3.5x as long as the direct call, about 38 ns more per element read: one borrow of the list and one count `inc`/`dec` of the string, all atomic, which the per-operation rows account for. That puts it behind CPython. The cost falls only on marked values: a value no task can reach takes the unmarked path, one load and a branch more than before marking existed (the 3% gate of step 2). Borrowing a marked list once per loop rather than per read is done ([below](#holding-a-list-per-loop-atomics-cost-a)); skipping the count of an element only read is next. Step 4c measured CPU-bound tasks against the serial build and CPython threads ([below](#cpu-bound-tasks-m6c-step-4c)).

### Holding a list per loop (atomics cost A)

A `for` loop whose body cannot change its list holds one read borrow of it for the whole loop, not one per `len` and read ([native.md](native.md#strings-lists-and-tuples)). `shared_2` and `shared_4` split `shared`'s 40 passes over its list between two and four tasks on the same list. Same machine, release build, by the cores the process may use (`taskset -c 0`, `0-1`, `0-3`), 7 runs, median seconds; before is `main` at 83b2fce, built and run in the same session as after.

| Benchmark | Before, 1 core | Before, 2 cores | Before, 4 cores | After, 1 core | After, 2 cores | After, 4 cores |
|---|---:|---:|---:|---:|---:|---:|
| `shared` (1 task) | 0.451 (0.436 to 0.482) | 0.482 (0.344 to 0.517) | 0.466 (0.452 to 0.501) | 0.175 (0.169 to 0.218) | 0.203 (0.163 to 0.215) | 0.204 (0.180 to 0.235) |
| `shared_2` | 0.445 (0.366 to 0.483) | 1.896 (1.410 to 2.087) | 2.015 (1.675 to 2.495) | 0.196 (0.186 to 0.226) | 0.130 (0.118 to 0.155) | 0.127 (0.115 to 0.152) |
| `shared_4` | 0.444 (0.345 to 0.494) | 1.880 (1.499 to 2.393) | 2.773 (2.254 to 3.134) | 0.157 (0.138 to 0.201) | 0.123 (0.114 to 0.158) | 0.098 (0.080 to 0.104) |
| `shared_baseline` | 0.123 (0.114 to 0.149) | 0.133 (0.120 to 0.163) | 0.125 (0.099 to 0.135) | 0.110 (0.100 to 0.122) | 0.112 (0.110 to 0.142) | 0.120 (0.108 to 0.141) |

Before, tasks on one list on more than one core were 4x to 6x slower than on one core: every read changed the list's one borrow word, so its cache line moved between the cores on every read. After, a loop changes that word twice. One task still takes about 1.6x as long as the direct call: each string's count `inc`/`dec` stays atomic (option B, below).

### Reading elements without a count (atomics cost B)

A held loop whose variable only reads each element while the hold lasts reads it without a count of its own ([native.md](native.md#strings-lists-and-tuples)): no `inc`/`dec` per element, marked or not. A and B built and run in the same session, same method as above, median seconds:

| Benchmark | A, 1 core | A, 2 cores | A, 4 cores | B, 1 core | B, 2 cores | B, 4 cores |
|---|---:|---:|---:|---:|---:|---:|
| `shared` (1 task) | 0.179 (0.146 to 0.189) | 0.176 (0.150 to 0.190) | 0.179 (0.144 to 0.208) | 0.091 (0.062 to 0.109) | 0.076 (0.067 to 0.097) | 0.103 (0.092 to 0.116) |
| `shared_2` | 0.165 (0.138 to 0.181) | 0.117 (0.114 to 0.130) | 0.116 (0.098 to 0.136) | 0.084 (0.078 to 0.098) | 0.076 (0.059 to 0.087) | 0.062 (0.059 to 0.073) |
| `shared_4` | 0.181 (0.177 to 0.185) | 0.115 (0.099 to 0.124) | 0.085 (0.073 to 0.090) | 0.079 (0.071 to 0.091) | 0.060 (0.048 to 0.068) | 0.058 (0.053 to 0.063) |
| `shared_baseline` | 0.116 (0.109 to 0.128) | 0.115 (0.093 to 0.145) | 0.112 (0.094 to 0.130) | 0.080 (0.074 to 0.090) | 0.091 (0.084 to 0.102) | 0.085 (0.082 to 0.115) |

The one-task row is now within the noise of the direct call, which gained too: its counts were plain, but still two runtime calls per element. Each program also builds its list of 200,000 strings, which no row here changes.

## fclone-lite on tasks (M6c driver)

`apps/fclone-lite/fclone_tasks.py` is fclone-lite with every candidate (a file sharing its size with another) hashed through one executor's `map`; the serial `fclone_lite.py` hashes them one by one. Both are valid Python, and they print the same groups (test `fclone_tasks_matches_fclone_lite`, and identical output on all three trees here, with rusty_fclone finding the same group counts). Same 4-core VM, warm cache, release `pythrust` (2026-11-02), one warm-up then 5 runs, median seconds. Trees shaped like those of M6b's parallel-hashing rows (`roadmap.md`, M6b follow-up), regenerated, sizes approximate: 900 MB of 30 files of 30 MB with 10 duplicate pairs; 105 MB of 4,400 files of 5 sizes, 400 duplicates; 153 MB of 4,400 files over 479 sizes, 400 duplicates. Core counts by `taskset`.

| Program | 900 MB, 30 files | 105 MB, 5 sizes | 153 MB, 479 sizes |
|---|---:|---:|---:|
| native `fclone_lite` | 5.719 (5.470 to 5.955) | 0.761 (0.717 to 0.811) | 0.970 (0.935 to 1.308) |
| native `fclone_tasks`, 1 core | 5.937 (5.579 to 6.195) | 0.784 (0.701 to 0.838) | 1.001 (0.950 to 1.023) |
| native `fclone_tasks`, 2 cores | 2.903 (2.872 to 3.050) | 0.378 (0.369 to 0.407) | 0.533 (0.496 to 0.569) |
| native `fclone_tasks`, 4 cores | 2.102 (2.081 to 2.549) | 0.246 (0.211 to 0.268) | 0.270 (0.265 to 0.295) |
| CPython `fclone_lite.py` | 2.917 (2.855 to 3.007) | 0.435 (0.415 to 0.466) | 0.558 (0.526 to 0.569) |
| CPython `fclone_tasks.py` | 0.991 (0.896 to 1.384) | 1.368 (1.271 to 1.601) | 1.555 (1.504 to 1.677) |
| rusty_fclone | 0.081 (0.063 to 0.089) | 0.223 (0.189 to 0.242) | 0.261 (0.231 to 0.331) |

On one core the task build's ranges overlap the serial build's. On the large-file tree it trails CPython's own threads, where both are hash-bound; CPython's lead there comes in part from hashing with OpenSSL ([M6a](roadmap.md#phase-5-m6-plan): 1.2x to 1.5x our SHA-256 on this CPU), on threads that release the GIL; on the small-file trees CPython's threads are slower than its serial run. rusty_fclone is not like-for-like (partial hashing, xxh3), as in M6b. These are the author's timings on one VM.

## CPU-bound tasks (M6c step 4c)

Two programs, each with a serial control that runs the same work by direct calls, all four valid Python, in `bench/`. Same machine (4 cores, so 8 workers by default), release build, CPython 3.13 (with the GIL), 7 runs, median seconds (`bench/run.py`). CPython runs each file as is, so the task rows are CPython's own `ThreadPoolExecutor`.

- `tasks_cpu`: Collatz step counts for 1..800,000 in eight chunks, one task each through `ex.map`. Only ints cross to the workers, so nothing is marked.
- `tasks_alloc`: eight tasks, each making and dropping 400,000 strings with `str()`. Each task makes its own values, so nothing is marked; every string is a malloc and a free.

| Benchmark | Native s | CPython s |
|---|---:|---:|
| `tasks_cpu` | 0.141 (0.128 to 0.155) | 7.380 (7.042 to 7.714) |
| `tasks_cpu_serial` | 0.457 (0.435 to 0.513) | 6.454 (6.401 to 6.752) |
| `tasks_alloc` | 0.038 (0.034 to 0.049) | 0.343 (0.319 to 0.411) |
| `tasks_alloc_serial` | 0.133 (0.106 to 0.158) | 0.324 (0.275 to 0.371) |
| `shared` | 0.432 (0.390 to 0.512) | 0.255 (0.228 to 0.298) |
| `shared_baseline` | 0.113 (0.089 to 0.168) | 0.254 (0.189 to 0.275) |

On these four cores, tasks run CPU-bound work 3.2x (`tasks_cpu`) and 3.5x (`tasks_alloc`) faster than the serial native build; on one core they cost 14% and 44% more than it ([scaling](#cpu-bound-tasks-m6c-step-4c), below). CPython's threads are no faster than its serial run (the GIL), so the native task rows are 52x and 9x CPython's. These are the only speedups claimed: work that tasks split evenly, with nothing marked. The `shared` rows (from [Shared values](#shared-values-m6c), same run) are where tasks lose: one task walking a marked list takes 3.8x as long as the direct call, and is slower than CPython (before loops held their list; [after](#holding-a-list-per-loop-atomics-cost-a), CPython not re-measured).

Scaling, by the cores the process may use (`taskset -c 0`, `0-1`, `0-3`), 7 runs, median seconds. The executor has no `max_workers` yet, so the pool follows the cores: `min(32, cpus + 4)`, 5, 6 and 8 workers.

| Benchmark | 1 core | 2 cores | 4 cores |
|---|---:|---:|---:|
| `tasks_cpu` | 0.528 (0.505 to 0.576) | 0.291 (0.250 to 0.364) | 0.140 (0.134 to 0.157) |
| `tasks_cpu_serial` | 0.462 (0.365 to 0.523) | 0.466 (0.393 to 0.501) | 0.482 (0.381 to 0.520) |
| `tasks_alloc` | 0.156 (0.136 to 0.233) | 0.093 (0.073 to 0.099) | 0.055 (0.039 to 0.061) |
| `tasks_alloc_serial` | 0.108 (0.107 to 0.109) | 0.144 (0.130 to 0.232) | 0.148 (0.130 to 0.209) |
| `shared` | 0.444 (0.349 to 0.455) | 0.458 (0.397 to 0.571) | 0.454 (0.358 to 0.469) |
| `shared_baseline` | 0.114 (0.098 to 0.118) | 0.133 (0.101 to 0.159) | 0.133 (0.098 to 0.178) |

`tasks_cpu` gains 1.8x from one core to two and 2.1x from two to four; on one core it costs 14% more than serial (eight chunks time-sliced over five threads). `tasks_alloc` gains 1.7x per doubling, and on one core costs 44% more than serial. `shared` is one task, so cores do not help it: its cost is the atomics, not the threads.

`tasks_alloc` was first 4.4x *slower* than its serial run: in the first run 0.768 s (0.643 to 0.959) against 0.176 s (0.154 to 0.197), pinned to one core 0.28 s, so most of the cost was threads on different cores waiting on each other. Two causes, both fixed in this step:

- **A shared leak counter.** Every string, sequence and object allocation and free bumped one atomic counter (`LIVE`), which only a debug runtime reads, to fail leaking programs. Bumped from four cores at once, its cache line moved between them on every allocation. A release runtime no longer counts.
- **`realloc` in `str(int)`.** `str(n)` formatted into a `String` and shrank it to a `Box<str>` by `realloc`. glibc's `malloc` and `free` mostly use a per-thread cache, but `realloc` takes the arena's lock, and `strace -f -k` showed the workers' futex waits with `realloc` on the stack, and `getrusage` without `strace` gave 1.3 s of system time per run. `str(n)` now allocates once at its exact length. A standalone Rust program doing `n.to_string().into()` on 8 threads shows the same: 4x to 6x slower than serial (in two sessions), and faster than serial once the string is allocated at its exact length.

Each fix alone leaves the task run 3x to 4x slower than serial; it takes both (7 runs, median seconds):

| Runtime | `tasks_alloc` | `tasks_alloc_serial` |
|---|---:|---:|
| neither fix | 0.702 (0.613 to 0.773) | 0.175 (0.139 to 0.197) |
| counter only | 0.515 (0.485 to 0.602) | 0.128 (0.106 to 0.191) |
| `str(int)` only | 0.548 (0.475 to 0.760) | 0.156 (0.127 to 0.183) |
| both | 0.038 (0.034 to 0.049) | 0.133 (0.106 to 0.158) |

Other runtime functions that build a string through a `String` and may shrink it still `realloc`: `str` of a float (`num::repr`), `repr` of a string (`pt_str_repr`) and the negative-exponent hint (`pow_hint`). They are not on these benchmarks' path. Single-threaded, writing an int into an exact-capacity `String` is 10 to 15% slower than `to_string` in a microbenchmark (std has a fast path); the serial row still improved, as it also lost the counter.


## Internal two-int return/unpack (#806)

Same-host comparison against freshly built main
`e69297ed9406eaf445bf198f9e3a1b995e49a20d` (PR #126), 2026-11-08.
Release compiler and runtime, x86-64 Linux 6.18.57, glibc 2.41,
AMD EPYC 9V74, Rust 1.99.1/LLVM 23.1.1, GCC 14.2, CPython 3.13.5.
Pinned to CPU 0, nine process samples per version after a warm-up, warm files;
the shared host still varies. Sources and call shapes are identical between
versions. CPython output is checked for every case. Raw samples, source hashes,
binary hashes and environment: [card806.json](benchmarks/card806.json).

| Workload | Before median seconds (min–max) | Scalar clone median seconds (min–max) | Pair tuples before → after | Heap allocation calls before → after |
| --- | --- | --- | --- | --- |
| `tuple_unpack` (4M direct calls/unpacks) | 0.307225 (0.289608–0.390100) | 0.028025 (0.025715–0.065565) | 4,000,000 → 0 | 8,000,004 → 4 |
| existing `tuples` | 0.510902 (0.492569–0.668932) | 0.050138 (0.047321–0.052298) | 6,888,889 → 0 | 13,777,783 → 5 |
| empty process startup control | 0.000897 (0.000716–0.001107) | 0.000849 (0.000718–0.001230) | 0 → 0 | 1 → 1 |

Checksums (stdout): `tuple_unpack`: `800016000000`; existing `tuples`:
`5888889 27000000`. Execution medians improve about 11.0x and 10.2x on this
host. These are process wall times (startup included), not compile times or
portable speed guarantees. Startup was measured separately and was not
subtracted; the tiny 100-call cases are dominated by startup and support no
speed claim. Compile/link medians are essentially neutral on this sample:
`tuple_unpack` 0.074418 → 0.067935 s, existing `tuples` 0.069554 → 0.060780 s;
complete ranges are in the raw file. Retaining an original function plus a
private clone can increase compiler work/object code; this prototype does not
claim a general compilation or size improvement.

The separate GNU-linker probe wraps `pt_seq_new`, `malloc`, `calloc` and
`realloc`; it is never linked into the timed executables. It counts two-int
tuple objects and successful linked heap allocation calls, not all allocations
inside libc/shared libraries. Each eliminated tuple removes its object and slot
vector allocations. Reallocation counts stay 1/1 (`tuple_unpack`) and 2/2
(`tuples`). A 100-call fast-path regression has tuple counts 100 → 0 and heap
calls 204 → 4; its boxed fallback stays 100 tuples/204 heap calls; mixed direct
unpack plus named boxed sites go from 200 → 100 tuples, 404 → 204 heap calls.
A multi-return/caught-exception/boundary case also goes from 100 → 0 pair tuples, with non-tuple heap calls retained (537 → 337). The Linux `scalar_pair` test asserts elimination and fallback using this probe.
Instrumentation is absent from production code and has zero disabled-path cost.

Reproduce (absolute paths; preserve base compiler with its runtime alongside):

```sh
# In an isolated worktree at e69297e, and again in the changed worktree:
cargo build --locked --release -p pythrust-compiler -p pythrust-rt
# Copy the base target/release/{pythrust,libpythrust_rt.a} to /abs/before/.
# Avoid sharing a target directory across source worktrees, or force a rebuild.
taskset -c 0 /usr/bin/python3.13 bench/tuple_returns.py \
  --before /abs/before/pythrust --after /abs/after/target/release/pythrust \
  --runs 9 --output /abs/results.json
# Small allocation/fallback assertions only:
PYTHON=/usr/bin/python3.13 cargo test --locked --test scalar_pair
```

The published `tuples_baseline` is motivation only: it changes the call shape
and packs a result into an integer. It is not used to attribute these gains.
No string fusion, general escape analysis, interpreter/runtime representation
change, external ABI change or #807/#808 completion is claimed.


## Adjacent native string chains (#807)

Compared freshly built main `c633a4fcc9c7dfc41ed66e9346a0797f1793502a`
(PR #127/#806) with implementation `8614f13b58ea69685b6f9a15a2ea18e6edf921f7`,
2026-11-08. Same host: x86-64 Linux 6.18.57, AMD EPYC 9V74,
glibc 2.41, Rust 1.99.1/LLVM 23.1.1, GCC 14.2, CPython 3.13.5
(`/usr/bin/python3.13`). Pinned to CPU 0; nine samples after one warm-up per
executable; identical source and call shape before/after. These are process wall
times including startup, excluding compilation and instrumentation. Shared-host
noise remains. All stdout matches CPython. Raw sources, checksums, binary and
runtime hashes, every sample, compile/link times, bounded regressions and
injected allocation-failure results: [card807.json](benchmarks/card807.json).

| Workload | Before median s (min–max) | After median s (min–max) | Concat result objects before → after | Concat buffer allocations before → after | Text bytes copied before → after |
| --- | --- | --- | --- | --- | --- |
| `string_chain` (2M identical direct triple calls) | 0.130884 (0.127143–0.138675) | 0.065087 (0.062926–0.080676) | 4,000,000 → 2,000,000 | 4,000,000 → 2,000,000 | 120,000,000 → 70,000,000 |
| existing `strings` | 0.229039 (0.218971–0.261097) | 0.234212 (0.218773–0.254133) | 3,020,000 → 3,020,000 | 3,020,000 → 3,020,000 | 224,573,570 → 224,573,570 |
| empty-process startup control | 0.000742 (0.000696–0.000890) | 0.000750 (0.000707–0.000938) | 0 → 0 | 0 → 0 | 0 → 0 |

The isolated case is about 2.0x faster on this host. The existing workload is
2.3% slower by median with overlapping ranges and identical counts: it has no
eligible site, so no improvement is claimed. This is a bounded prototype,
not broad string acceleration. Output checksums: isolated `58000000`; existing
`9785790 20000`. Compile/link medians are 0.051268 → 0.053904 s (isolated)
and 0.053221 → 0.053494 s (existing); no compilation speedup is claimed.
Startup is measured separately and is not subtracted.

Allocation instrumentation is linked only into separate probe executables.
GNU `--wrap` counts successful linked `malloc`/`calloc` and `realloc` calls,
including non-string overhead. Isolated total heap allocations are
8,000,010 → 4,000,010, plus 1 realloc each; existing `strings` stays
14,080,006 allocations plus 1 realloc. These are **linked allocator calls**,
not all allocator activity hidden inside shared libc. Concat helper returns
count result objects; scoped allocator calls count their object and text-buffer
allocations without inspecting `PtStr` layout. Buffer counts exclude the returned
object; an all-empty triple has no buffer allocation. Scoped wrapped
`memcpy`/`memmove` counts actual bytes written into allocated text buffers,
excluding object-construction and stack traffic. Probe capacity overflow is an
explicit failure; concat realloc counts are zero in every reported case, so no
unobserved allocator-internal resize copies are attributed as savings.
No counters or fault injection exist in ordinary production/runtime code.

Bounded assertions include empty/mixed-empty, Unicode triples, later-call and
right-associated fallbacks. For 100 Unicode triples, result objects 200 → 100,
text buffers 200 → 100, and actual text bytes copied 1,400 → 900. The later-call
fallback keeps 200 objects/200 buffers/1,400 bytes; right-associated keeps
200/200/1,600. Forced first concat allocation refusal remains fatal (not caught
by `except Exception`) and a later operand's instrumented construction marker
never occurs. Synthetic length tests cover prefix and final overflow without
large allocations. Low-memory success thresholds and allocator request-size
stderr necessarily differ when allocations are removed; no new exception/OOM
policy is introduced.

Reproduce with independent compiler/runtime artifacts (absolute paths):

```sh
# In clean worktrees at c633a4f and the implementation revision:
cargo build --locked --release -p pythrust-compiler -p pythrust-rt
# Preserve the base target/release/{pythrust,libpythrust_rt.a} together.
taskset -c 0 /usr/bin/python3.13 bench/string_chains.py \
  --before /abs/base/bin/pythrust --after /abs/current/target/release/pythrust \
  --runs 9 --output /abs/card807.json
PYTHON=/usr/bin/python3.13 PYTHRUST_BIN=/abs/current/target/release/pythrust \
  cargo test --locked --test string_chain --test scalar_pair
```

Runtime pointer/UTF-8/alias/size tests pass under Miri. Shared-string worker
inputs pass ThreadSanitizer with five pinned and five unpinned schedules.
Hosted required CI and independent review of the unchanged published head
remain mandatory; this document does not assert either has completed.


## Current workload profile (#808)

The [current baseline, raw evidence and ranked experiments](benchmarks/card808.md)
measure main `f542d98`, including merged #806/PR127 and #807/PR128.
This measurement-only report separates build/startup controls and fclone
discovery/read/repeated-SHA controls, documents tmpfs and profiling limits,
and proposes at most three new experiments. It does not implement them.
