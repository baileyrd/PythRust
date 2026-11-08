# Current performance baseline and next experiments (#808)

Target: main `f542d982b7a86af0a889d252d927533b92fd6b04` (merged PR #128/#807, including PR #127/#806). Measurement only: no compiler/runtime, language, ABI, dependency or optimizer changes. #767’s mapping, held #396/PR #119 and open issue #73 are unchanged.

## Method and reproducibility

- Linux 6.18.57 x86-64, AMD EPYC 9V74, glibc 2.41, Rust 1.99.1 / LLVM 23.1.1, GCC 14.2 / GNU ld and gprof 2.44. Nine CPUs permitted; execution affinity is CPU 0 or CPUs 0–3. For both runtimes the pool cap is 5 workers on CPU 0 and 8 on CPUs 0–3; pools start workers on demand. Compilation inherits the parent’s nine-CPU affinity.
- CPython **3.13.5**, `/usr/bin/python3.13`, GIL enabled, SHA-256 through `_hashlib` / OpenSSL 3.5.7. CPU flags advertise `sha_ni`; OpenSSL’s selected instruction path was not profiled. Identical SHA-256 inputs and digests are compared, but the implementations differ: these hash timings are not an isolated language-speed comparison.
- Release compiler and adjacent release runtime built together with the locked Cargo graph and default release optimization. `PYTHRUST_RT` is explicitly set to that archive. Compiler/runtime, source, input-manifest and timed-binary SHA-256 identities are in [raw results](card808.json); additional observed host/tool facts and the failed trace are in [host evidence](card808-host.json). Production source is unchanged from the target commit.
- Seven raw samples after one warmup per runtime/case/affinity, with native/Python order reversed on alternate programs. Every run must succeed, have empty stderr and equal the oracle stdout. Fclone also matches an independent SHA-256 manifest/group oracle. No missing or mismatched sample is accepted. This is sequential case ordering, not randomized or confidence-interval estimation.
- Execution is **whole-process wall time**, including launch, initialization, output capture and shutdown. Empty-process startup is a separate control and is never subtracted. Compiler-process times include emission and linking; a separate set of seven linker-wrapper diagnostics measures the nested cc invocation. Those samples are not paired, and are not subtracted to invent frontend time. Each compiler timing follows an initial sanity build plus one timing warmup; the separate link diagnostics follow that series with no extra warmup. Warm build-cache effects are retained.
- All fixtures are deterministic synthetic data in a new task-owned temporary directory. `/tmp` is **tmpfs** on this host. Files are warm after generation, parity runs and warmups; there is no cold-cache or disk-throughput claim, no cache eviction, security change or private-file scan. This limits extrapolation to disk-backed and cold workloads.

Run from the repository root (GNU/Linux; no new Python/Rust packages):

```sh
cargo build --locked --release -p pythrust-compiler -p pythrust-rt
/usr/bin/python3.13 bench/profile_workloads.py \
  --compiler "$PWD/target/release/pythrust" --runs 7 --profiles \
  --output /tmp/card808-rerun.json
/usr/bin/python3.13 bench/verify_profile_results.py /tmp/card808-rerun.json
/usr/bin/python3.13 -m unittest discover -s bench -p test_profile_workloads.py -v
```

The harness performs generation, ordinary timing, separate profile linking and cleanup; it retains compact text/JSON, not datasets or binary profiles. Intermediate checkpoints are intentionally incomplete and must pass the verifier before use. Increase repeats for decisions near noise. All generated diagnostic sources are included verbatim in the raw JSON.

## Repository suite

Seconds, four permitted CPUs; median ± median absolute deviation (MAD), **not** a confidence interval. Min/max, all seven values, user/system CPU and context-switch samples are in JSON. Historical [M4 results](https://github.com/baileyrd/PythRust/blob/53f793c8296b872441d9be5f8a4ee8d44083d229/docs/benchmarks.md) used a different host/state; do not compare them as a measured speedup.

| Program | Native s ± MAD | CPython s ± MAD |
| --- | ---: | ---: |
| `numeric` | 0.461736 ± 0.011031 | 8.713447 ± 0.148810 |
| `shared` | 0.077621 ± 0.005706 | 0.256388 ± 0.006153 |
| `shared_2` | 0.059853 ± 0.001609 | 0.284512 ± 0.007375 |
| `shared_4` | 0.049474 ± 0.006100 | 0.274270 ± 0.004034 |
| `shared_baseline` | 0.080432 ± 0.001550 | 0.262025 ± 0.010610 |
| `sorting` | 0.129422 ± 0.003333 | 0.806270 ± 0.078938 |
| `string_chain` | 0.073577 ± 0.004175 | 0.432045 ± 0.012874 |
| `strings` | 0.222488 ± 0.003865 | 0.387741 ± 0.025183 |
| `tasks_alloc` | 0.067599 ± 0.004565 | 0.389633 ± 0.018310 |
| `tasks_alloc_serial` | 0.147583 ± 0.001847 | 0.360400 ± 0.022457 |
| `tasks_cpu` | 0.110642 ± 0.010966 | 6.886432 ± 0.065073 |
| `tasks_cpu_serial` | 0.370725 ± 0.016529 | 6.727920 ± 0.110716 |
| `tuple_unpack` | 0.029373 ± 0.000818 | 0.803087 ± 0.048800 |
| `tuples` | 0.048682 ± 0.001520 | 0.808383 ± 0.025239 |
| `tuples_baseline` | 0.060065 ± 0.000905 | 0.870126 ± 0.019460 |
| `startup` | 0.001869 ± 0.000091 | 0.011732 ± 0.000539 |

Ordinary compiler-process medians across the 27 source cases span 0.057–0.108 s; separate nested linker medians span 0.052–0.096 s. They vary enough that no build-speed claim is warranted. Startup alone is ~1.9 ms native and ~11.7 ms CPython here; millisecond rows are particularly sensitive to launch and imports.

## Fclone and file-access/hash controls

Small trees: 256 files around 4 KiB, four sizes, eight subdirectories. Large trees: eight files around 4 MiB (32 MiB total), four sizes, flat directory. Duplicate trees pair identical payloads; distinct trees keep repeated sizes but all contents differ. The unique-size tree has 64 distinct lengths (1 KiB upward) in eight subdirectories, so size pruning hashes no candidates. File-root is a real fixture file passed as the root: both existing drivers return no groups because their collector expects a directory; it is a no-op control, not single-file hashing.

Seconds, four permitted CPUs. Both drivers hash **all candidate bytes with SHA-256**. CPython validates identical printed groups (including the empty outputs).

| Dataset | Native serial | Native tasks | CPython serial | CPython tasks |
| --- | ---: | ---: | ---: | ---: |
| `small_dup` | 0.008161 | 0.005765 | 0.026448 | 0.053898 |
| `small_unique` | 0.009482 | 0.005679 | 0.020906 | 0.055392 |
| `large_dup` | 0.169640 | 0.067442 | 0.051743 | 0.058951 |
| `large_unique` | 0.170269 | 0.060029 | 0.049032 | 0.047846 |
| `unique_sizes` | 0.002040 | 0.002438 | 0.018335 | 0.027479 |
| `file_root` | 0.002139 | 0.002041 | 0.017969 | 0.029332 |

On `large_dup`, native task median is 0.154625 s on one CPU and 0.067442 s on four; serial is 0.153187 s and 0.169640 s respectively. That supports useful parallel hashing here, not a universal scheduling speedup. Small-file CPython threads are slower than its serial driver on this tmpfs tree. `fclone_fast.pyr`/`fastfs.group_xxh3` was inspected but excluded from like-for-like timing: it uses streamed **xxh3-128**, whereas these drivers read whole files and use SHA-256. Matching duplicate groups alone would not make those algorithms comparable.

The diagnostic sources use the same collector and all eight large files. Discovery totals names/sizes; read adds full reads and closes; loaded-SHA preloads all buffers once, then hashes each eight times (256 MiB total hashed) and prints a checksum/digest. These are independent controls with different work, not additive phase durations. The loaded-SHA row **still includes** discovery, a 32 MiB preload, process startup and teardown; no read/startup time is subtracted.

| Diagnostic | Native s ± MAD | CPython s ± MAD |
| --- | ---: | ---: |
| `phase_discover` | 0.002149 ± 0.000085 | 0.016956 ± 0.000934 |
| `phase_read` | 0.017786 ± 0.000960 | 0.031986 ± 0.000693 |
| `phase_loaded_sha256` | 1.210297 ± 0.028937 | 0.223385 ± 0.026279 |

## Profile evidence and concurrency controls

- **CPU:** separately linked `-pg -no-pie` executables, seven runs on CPU 0, GNU software PC histograms at about 100 Hz. The raw flat profiles retain accumulated sampled seconds: numeric 3.04 s (99.34% in generated `steps`); sorting 0.66 s (25.76% `pt_list_set`, 25.00% `seq::slot`, 23.49% `pt_seq_get`); strings 0.97 s (19.59% join/copy helper, 17.53% concat, 9.28% character counting, 9.28% `pt_str_from_int`, plus generic formatting); loaded-SHA 7.34 s (99.80% compressor). Percentages are **of captured in-image PCs**, not all process CPU/wall time: shared-library allocator/OpenSSL PCs are absent, symbol bucketing and low counts limit precision, and no call-graph instrumentation is claimed.
- **Allocation/copy/refcount:** a separate GNU `--wrap` probe counts successful linked allocator calls, requested allocation bytes (not peak/live memory), linked memcpy/memmove request bytes (not just text), selected runtime reference/read/hold calls and submissions. It does not see inlined/same-object or libc-internal activity. Null reference calls count as calls. Counters are atomic for correctness and can themselves introduce contention; their wall times are never used to infer scheduling or contention. `strings` has 14,080,006 allocations, one realloc, 459,660,492 requested bytes, 234,379,386 linked copy-request bytes, 7,040,002 string-decrement calls and zero string-increment calls. `sorting` has 10,852,716 ordinary reads but only eight allocations. `tasks_alloc_serial` has 6,400,004 allocations for 3.2M conversions.
- **Probe perturbation:** string CPU-profile median is 0.254918 s and count-probe wall is 0.369416 s, against ordinary 0.222488 s (the probes are pinned to one CPU; ordinary single-thread code may run on CPUs 0–3). These are diagnostics with changed linkage/affinity and noise, not calibrated overhead corrections. Concurrent count probes are confined to one CPU and not used as latency evidence.
- **Concurrency:** all comparisons below use ordinary release binaries. User/system CPU and voluntary/involuntary switches are collected from `getrusage(RUSAGE_CHILDREN)` per process; they do not identify lock stacks. `perf`/Valgrind are not installed. `strace` was attempted once and child tracing was denied (`PTRACE_TRACEME: Operation not permitted`); it was stopped, with no privilege/security change. No syscall-trace evidence or causal shared-cache-line diagnosis is claimed.

| Workload | Native 1 CPU s | Native 4 CPUs s |
| --- | ---: | ---: |
| `tasks_cpu` | 0.373467 | 0.110642 |
| `tasks_cpu_serial` | 0.369016 | 0.370725 |
| `tasks_alloc` | 0.160793 | 0.067599 |
| `tasks_alloc_serial` | 0.155427 | 0.147583 |
| `shared_4` | 0.068245 | 0.049474 |
| `shared_baseline` | 0.069521 | 0.080432 |
| `granularity_8_tasks` | 0.009814 | 0.005290 |
| `granularity_8_serial` | 0.008539 | 0.009845 |
| `granularity_128_tasks` | 0.009068 | 0.005205 |
| `granularity_128_serial` | 0.008786 | 0.009261 |
| `granularity_8192_tasks` | 0.012796 | 0.009160 |
| `granularity_8192_serial` | 0.009228 | 0.008432 |

The coarse CPU control supports ~3.35x native parallel/serial improvement on four CPUs; allocations ~2.18x. The 8/128/8192-task diagnostics each perform 3,276,800 modulo/add iterations, and each task form is checked against its exact serial and CPython counterpart. The per-chunk modulo sequence/checksum changes with chunk size, so compare each **pair**, not algorithms across chunk counts. At 8192 tasks the four-CPU difference from serial is within its broad task MAD (2.39 ms); on one CPU tasks cost ~3.57 ms more than serial. Its count probe confirms 8192 submissions. On four CPUs its uninstrumented median user/system CPU is 14.52/5.64 ms and voluntary switches 186, versus serial 8.31/0 ms and one switch. Those observations locate overhead in task machinery broadly; they do not identify a lock or shared-data contention. This is a small synthetic overhead lead, not evidence for a scheduler rewrite.

`shared_4` and its direct control each have 8M borrowed held-element reads and 40 holds; neither adds per-element string-reference calls. The existing hold/count elision is working. Initialization and object marking remain included, so these data do not isolate sharing cost or justify reopening parked read-only task regions (D29).

## Ranked follow-up experiments (proposals only)

1. **Portable SHA-256 compression pilot.** Minimal reproducer: generated `phase_loaded_sha256` source plus the `large_dup` manifest; the full rerun command above reproduces both. `sha256.rs::compress` accounts for 99.80% of captured PCs; 256 MiB of repeated hashing plus preload takes 1.210 s native versus 0.223 s CPython/OpenSSL. Full large-file fclone also trails CPython serial. Scope one safe-Rust compressor/schedule variant and its microbenchmark, keeping SHA-256, whole-file behavior, runtime/binding ABI and dependencies fixed. Likely impact is **unknown**; a portable pilot might recover only a small fraction of the OpenSSL gap, and this sample does not prove why OpenSSL is faster. Compare identical byte vectors, lengths 0/1/55/56/63/64/65 and larger inputs, CPython/FIPS digests, padding/overflow behavior, duplicate groups and 1/4-CPU regressions. D22 remains in force: this is not approval for dependencies, SIMD unsafe code or another hash.

2. **Monomorphic int-list access pilot.** Minimal reproducer: `bench/sorting.pyr` copied into task-owned scratch, unchanged algorithm/checksum, with its ordinary source as control. The profile concentrates ~74% of captured PCs in set/slot/get, and the probe sees 10.85M reads with almost no allocation. Scope narrowly to typed int load/store helpers or dispatch specialization, preserving the ordinary list representation, borrow/synchronization checks, bounds/status propagation, aliasing and every uncertain fallback. Gain is uncertain: these profile percentages are an opportunity ceiling on captured CPU, not an end-to-end prediction; retain only an improvement beyond repeated-run noise. Check negative/out-of-range indexes, fault and evaluation order, aliases, held loops, shared task reads, native/CPython sorting parity and the full native corpus. Do not reopen D29’s parked shared-region feature on a synthetic result.

3. **Integer-to-string formatting pilot.** Minimal reproducers: unchanged `bench/tasks_alloc_serial.pyr` and `bench/strings.pyr` (plus their task control). Scope a safe private decimal-formatting/known-ASCII-length path in `str.rs`, avoiding generic formatting/character rescans while retaining ordinary owned string layout and exact-size allocation (no return of the prior parallel `realloc` regression). `strings` samples include int conversion/formatting and character-count work; the serial allocation control makes 3.2M conversions / 6.4M allocations. Allocation counts alone cannot assign their latency, and the fraction of character-count samples from integer conversions is unknown: expect at most a workload-dependent fraction, with **no proven speedup** yet. Check zero, sign, all digit boundaries and i64 extrema against CPython, count/code-point lengths, allocation/copy/realloc counts, 1/4-CPU allocation controls, #807 string-chain/parity/fault regressions and the native corpus. Reject a change that merely moves cost into contention.

**Already tracked once:** [#806/PR127](https://github.com/baileyrd/PythRust/pull/127) and [card806.json](card806.json) cover the internal two-int return/unpack clone (D63); this baseline includes it, and proposes no repeat tuple experiment. [#807/PR128](https://github.com/baileyrd/PythRust/pull/128) and [card807.json](card807.json) cover adjacent prepared-operand triple fusion (D64): it benefits the isolated chain but leaves existing `strings` at 3,020,000 concat objects/buffers and 224,573,570 scoped text-copy bytes. Current total linked allocations agree with that prior evidence. Experiment 3 is a new formatting lead, not a repetition or silent widening of D64. None of these recommendations is a new language decision or implementation card.

## Confidence, limits and validation

Large CPU/hash contrasts and deterministic counts are reproducible evidence on this shared host. CPU frequency/governor and other tenants are uncontrolled; no host settings were changed. Small percentage differences, build/link variation and millisecond controls need more randomized repeats before an implementation decision. No cold-storage, peak-memory, allocator-internal, lock-stack, hardware-counter, cross-host or cross-architecture conclusion is supported. Profile time is not ordinary timing, counts are not causal time attribution, and startup/cache effects were not subtracted. No optimization is implemented by this card.

Local validation and exact published-tree identities are recorded in the draft PR. Required checks: harness tests/statistics verification; `cargo fmt --all --check`; `cargo clippy --locked --all-targets -- -D warnings`; `PYTHON=/usr/bin/python3.13 cargo test --locked`; locked release compiler/runtime build; CPython 3.13 + absolute release `PYTHRUST_BIN` native corpus, expression limits, scalar-pair and string-chain tests; `git diff --check`. Hosted CI and independent review of the unchanged head remain coordinator gates. No unrelated local sanitizer suite is claimed.

For a small individual reproducer, extract the recorded source and regenerate its named deterministic tree without rerunning the full suite:

```sh
export PYTHRUST_BIN="$PWD/target/release/pythrust"
/usr/bin/python3.13 - <<'PY'
import json, os, pathlib, subprocess, sys, tempfile
sys.path.insert(0, "bench")
import profile_workloads as p
r = json.loads(pathlib.Path("docs/benchmarks/card808.json").read_text())
with tempfile.TemporaryDirectory(prefix="pythrust-808-repro-") as t:
    root = pathlib.Path(t)
    p.generate_tree(root / "data", count=8, size=4*1024*1024, duplicates=True)
    src = root / "hash.pyr"
    src.write_text(r["cases"]["phase_loaded_sha256"]["source"])
    compiler = pathlib.Path(os.environ["PYTHRUST_BIN"]).resolve()
    env = dict(os.environ, PYTHRUST_RT=str(compiler.parent / "libpythrust_rt.a"))
    p.run([compiler, "build", src], env=env)
    oracle = p.run([sys.executable, src, root / "data"]).stdout
    print(p.measure(["taskset", "-c", "0", src.with_suffix(""), root / "data"], 7, oracle))
PY
```

For sorting/strings/conversion, use the corresponding recorded source, omit the tree argument, and retain the same build/parity/measure calls. `profile()` and `write_linker()` in the harness show the exact separate `-pg -no-pie` and GNU `--wrap` link commands; `--profiles` executes them and verifies profile output. No binary profile needs to be committed.
