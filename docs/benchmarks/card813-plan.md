# Current three-way comparison: preparation for #813

Status: design and authority review only, 2026-11-08. No three-way benchmark
has run. Rust source counterparts and the three-way measurement harness still
need implementation and validation. Commands below that name planned files are
explicitly marked; they are not a claim that those files exist or passed.
No software installation, production change, PR publication or queue transition
is authorized by this preparation step.

## Source checkpoint and authority review

Prepared in the existing clean `codex/card813-current-comparison` worktree.
Fresh fetch and GitHub branch read agreed on main
`8b8b48537cc0c60fa266a26f61089213ee6a79fc`, tree
`99e5b0d5b4223300f8eb23b2d323d1c3c58a8a17` (after PR133).
Re-fetch before the eventual run; record the actual baseline if main moves.
The plan is an uncommitted documentation addition, not a measured source head.

Authority routes reviewed: `CLAUDE.md`, its README/design/grammar/roadmap/handoff
sequence and `docs/handoff/{SPEC,DECISIONS,QA,OPEN_QUESTIONS}.md`; relevant
native/runtime-resolution and codegen optimization settings; benchmark docs,
`bench/run.py`, `bench/profile_workloads.py`, and reports #808 through #811.
No AGENTS.md or .agents skills exist in this checkout. RemindMe was searched.

Relevant constraints: grammar is canonical; D2 checked i64, D9 Cranelift,
D22 dependency-free portable SHA, D27 unsafe boundary, D29 C parked,
D30 CPython oracle coverage, D34 Rust >=1.97, D63 private scalar pair returns,
and D64 bounded string fusion all stand. Do not edit language decisions or
production code. The initial handoff checkpoint and reports' pending-review
text are historical; fetched main and board evidence supersede their status
claims. #810's candidate is rejected and production access remains unchanged;
#809 and #811 are already on main. Do not rerun those pilots as new work.
#396/PR119, deferred #563 and D29 C remain untouched.

Existing harness limitations for this request: `run.py` calls PythRust output
"native", has no handwritten Rust, batches trials by implementation, and
retains no raw evidence file. `profile_workloads.py` provides useful strict
output/resource/statistics helpers but also batches trials inside `measure`;
it is not itself a balanced three-way harness. Preserve these historical tools;
a focused new harness must interleave individual executions.

## Matched workload contract

Use original .pyr bytes unchanged for both CPython and PythRust. No loop bounds,
algorithms or result aggregation may be silently replaced. Handwritten Rust
uses i64 for values and accumulators, checked safe indexing and standard Vec,
String and scalar pairs; no dependencies, unsafe, library sort, memoization,
closed-form checksum, parallelism or alternate hashing.

| Case | Exact work and inputs | Required stdout, including final newline |
|---|---|---|
| numeric | `steps` on each integer in `range(1,1000000)` (999,999 starts). While n != 1: even n // 2, otherwise 3*n+1; count every transition; sum counts and track longest. | `131434272 524\n` |
| sorting | Generate 300,000 values with seed 12345, update `(seed*1103515245+12345)%2147483648`, append seed%1000000. Middle-pivot in-place Hoare-style partition; recurse smaller side and iterate larger side exactly as source. Full adjacent sortedness check; print first, middle and last. | `True 6 499909 999999\n` |
| strings | i in 0..999,999; form left-associated `"k" + str(i) + ":" + str(i%97)`; add character length. Every 50th iteration append key[0] to line. Include all construction and line growth in execution. | `9785790 20000\n` |
| tuples | i in 0..999,999; while n > 0 call div_mod(n,10), immediately unpack quotient/remainder, add digit count/sum, return and unpack that pair, accumulate both fields. Zero has zero digits in this program. | `5888889 27000000\n` |
| startup | PythRust/CPython source `pass\n`; Rust `fn main() {}`. | empty bytes |

Source SHA-256 identities at checkpoint:

| File | SHA-256 |
|---|---|
| bench/numeric.pyr | 0e0c00db73adeb4076c9af8a4be855ebb68f50d717a74e45c99b053730f0be97 |
| bench/sorting.pyr | 59002a9199a1e2a098d65340a84ba3e7216d05e5525fe262791fb8fac39e5969 |
| bench/strings.pyr | a0b923620761d138f66acc92ff4e344827cac2b8c94183131fa5320fa5e38381 |
| bench/tuples.pyr | 2fbca60f4f894848d9bbe74a515d85442e54370e5eebf21a4443e81f5a6640d1 |

Rust translation details: signed lo/hi/i/j preserve sorting's possible -1
sentinel; convert to usize only at proven nonnegative access sites using checked
conversion, without unsafe. Keep explicit temp/read/write swaps. Rust prints
Python's capitalized True/False. Decimal strings use to_string and left-to-right
concatenation; no format! substitution, precomputed lengths, constant 'k' in
place of key[0], or one reserved output buffer. Rust String ownership may reuse
buffers while Python/PythRust strings are immutable; that is an explicit runtime
representation difference, not grounds to force an identical allocation count.
All timed strings are ASCII, so byte length/index zero agree with code points.
Boundary tests cover Unicode separately with chars(), without claiming the
ASCII timing measures Unicode access. Rust returns ordinary (i64,i64), allowing
inlining/scalar returns. Do not box pairs or prohibit inlining to simulate an
unoptimized runtime. PythRust's D63 already optimizes this call shape.

Arithmetic domain gate: separately verify every intermediate/accumulator in the
full datasets fits signed i64 (including Collatz peaks and LCG multiplication).
For LCG, 0 <= seed < 2^31 and seed*1103515245+12345 < 2^63 is an analytic bound.
Timed Rust default is optimized ordinary integer operations with overflow checks
off; a separately labelled optimized overflow-checks-on control runs the same
source. PythRust always checks overflow; CPython integers are arbitrary precision.
Equal successful outputs in the bounded domain do not make overflow/error policy
identical. Do not test Collatz(0), which never terminates under this algorithm.

## Optimizer and equal-work gate

Primary ordinary optimized Rust has no per-operation black_box, no inline(never)
and no volatile or allocator instrumentation. Normal optimizer transformations,
constant division strength reduction, pair scalarization and allocation reuse
are legitimate implementation effects. Inspect generated optimized code before
attributing any ratio to equivalent work. Preserve assembly and source hashes.

A final printed length alone does not prove all string construction survived.
If ordinary Rust elides material work, report that row as a same-program/result
comparison with the omitted work stated; exclude it from the equal-work table.
A separate diagnostic Rust variant may apply black_box to an already constructed
key and final line so content must be materialized, retaining all source-level
operations. Identify each barrier location and retain its source/assembly. Never
mix its time with the ordinary Rust row or claim it has zero cost; a barrier
cannot prove allocation/copy parity. If evidence still cannot prove matching
work, leave that case out of the equal-work ranking. Do not cripple the idiomatic
baseline to manufacture a desired comparison. No universal language ranking.

## Correctness before timing

Validate exact stdout, empty stderr and exit 0 for every warmup and sample;
first validate PythRust interpreter on reduced boundary sources and native on
full cases against CPython 3.13. Validate Rust full outputs against the same
bytes. Store actual bytes plus hashes; do not normalize away mismatches.

Separate diagnostic correctness programs compare full sorted vectors (not just
four printed fields), input LCG vectors, operation/partition counts and scalar
step/digit results. Golden-vector digest is SHA-256 of an explicitly specified
canonical decimal-line encoding, outside timing. Independent reference sort is
allowed only in the correctness oracle, never as the timed algorithm. Cases:
empty/one/two/all-equal/reverse/sorted/negative sorting vectors; Collatz 1,2,3,27;
digits 0,1,9,10,99,100,999999; decimal 0/sign/digit boundaries and i64 extrema;
small string limits around 49/50/51 and 96/97/98. Keep test entry points separate
from timed main. Unsupported negative digit inputs are not silently generalized.

Harness tests must reject missing/duplicate/out-of-order trial IDs, incomplete
conditions, mismatched output, nonzero exit, timeout, nonempty stderr, missing
executables, stale hashes, invalid affinity, overwrite of an existing result
bundle, wrong median/MAD, NaN/infinite/nonpositive wall samples, malformed CPU/
context-switch fields and truncated bundles. Boundary counts zero/one and invalid
negative counts must be tested. A failed run retains partial evidence and an
explicit incomplete status; verifier must refuse it for a report.

## Timing and evidence protocol

One host, no simultaneous task-owned build/test/profiling during samples. Record
UTC start/end, OS/kernel/libc, CPU model/features, permitted and chosen affinity,
load snapshots, cgroup quota/cpuset (read /proc/self/cgroup and mountinfo to locate
limits; unavailable is unknown, not unlimited), filesystem type and shared-host
limits. Pin parent and all child processes to the same permitted single CPU via
os.sched_setaffinity before launching, avoiding a taskset process in each sample.
Fail if pinning fails; no claim of exclusive hardware, governor or frequency
control. No cache eviction, host changes or tracing/security retries.

Two warmups per condition; 18 measured rounds per case for the primary three-way
suite. Use all six runtime permutations three times, shuffle permutation blocks
with seed 813, and store the actual full order (not just the seed). Each runtime
appears equally in each position; a round executes all three serially. Checked
Rust and materialization diagnostics are separate balanced passes. Retain every
sample including outliers, user/system CPU and context switches. Whole-process
perf_counter_ns wall time spans launch through output capture and wait/exit.
Report median, MAD and min/max; MAD is descriptive, not a confidence interval.
Near-noise differences need a separate preregistered repeat with new order seed,
retaining the first pass, not deleting slow trials.

Separate boundaries: compiler/runtime bootstrap build once with logged wall time;
per-workload compile/link (one warmup, seven warm rebuilds); ordinary whole-process
execution; empty startup. Never subtract startup, read controls, build or link
samples to invent steady-state/kernel time. No interpreter execution timing is
needed. No instrumentation in ordinary timing binaries. No datasets on disk for
the core suite; source/binary caches are warm after parity/warmups. If a larger
Rust repeat is needed because startup dominates, label it as a new workload with
all three implementations enlarged identically and preserve original results.

Raw JSON schema must include baseline HEAD/tree and dirty patch, measurement
harness HEAD/tree, Cargo.lock hash, exact source bytes/hashes, compiler executable
and runtime archive hashes, every produced binary hash, executable paths, tool
versions (rustc -Vv, cargo -V, linker/cc, Python -VV/build/config/GIL), all commands,
flags/environment controls, trial orders, outputs, resources and raw samples.
Reject unexplained RUSTFLAGS/CARGO_ENCODED_RUSTFLAGS/PYTHRUST_RT_SRC/LD_PRELOAD.
Explicitly select matching release archive using PYTHRUST_RT. Hash sources and
artifacts again after timing. Freeze compiler/runtime source across preparations;
if code changes, rebuild and start a new evidence bundle. Later publication must
match the tested tree, with any evidence-only commit identity distinguished.

## Reproduction commands and implementation handoff

These commands use files already present; they are future build/validation
instructions and have NOT been executed in this preparation (toolchain absent).
Run from the existing isolated worktree, with Rust >=1.97 and CPython 3.13:

```sh
git fetch origin main
git rev-parse origin/main origin/main^{tree}
# If main moved, preserve this work and choose/record the new baseline explicitly.
cargo build --locked --release -p pythrust-compiler -p pythrust-rt
export PYTHRUST_RT="$PWD/target/release/libpythrust_rt.a"
export PYTHRUST_BIN="$PWD/target/release/pythrust"
export PYTHON=/usr/bin/python3.13
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo test --locked --test conformance corpus_matches_native
cargo test --locked --test expression_limits --test scalar_pair --test string_chain
python3.13 -B -m unittest discover -s bench -p 'test_*.py' -v
git diff --check
```

Direct existing-source smoke command, using a new task-owned directory outside
the repo, after build (not a balanced benchmark):

```sh
BENCH_WORK=$(mktemp -d /tmp/pythrust-813-smoke.XXXXXXXX)
cp bench/numeric.pyr bench/sorting.pyr bench/strings.pyr bench/tuples.pyr "$BENCH_WORK/"
for name in numeric sorting strings tuples; do
  "$PYTHRUST_BIN" build "$BENCH_WORK/$name.pyr"
  /usr/bin/python3.13 -B "$BENCH_WORK/$name.pyr"
  "$BENCH_WORK/$name"
done
# Preserve this directory on failure; compare exact bytes before any timing.
```

Planned implementation files (NOT present yet): `bench/three_way.py`,
`bench/test_three_way.py`, `bench/verify_three_way.py` and
`bench/rust/{numeric,sorting,strings,tuples,startup}.rs`. Expected CLI contract:

```sh
# Planned commands; implement and test these interfaces before use.
/usr/bin/python3.13 -B bench/three_way.py --prepare --work /absolute/new-work \
  --compiler "$PYTHRUST_BIN" --rustc /absolute/rustc --cpu 0
/usr/bin/python3.13 -B bench/three_way.py --work /absolute/new-work \
  --runs 18 --seed 813 --output /absolute/new-work/results.json
/usr/bin/python3.13 -B bench/verify_three_way.py /absolute/new-work/results.json
```

Select an actually permitted --cpu, not blindly CPU 0. Preparation must refuse
an existing work path. Rust commands emitted by that harness, per source:
`rustc --edition=2021 -C opt-level=3 -C overflow-checks=off -C codegen-units=1
-C target-cpu=generic SOURCE -o BINARY`; checked control changes only
`overflow-checks=on`. Record default panic/LTO/linker and target triple; no
native-target-only flag for just one implementation. If PythRust's observed ISA
selection differs, document the difference and use a separate Rust target-cpu
control rather than silently relabeling builds. Keep assembly generation outside
timing using the same flags and `--emit=asm`.

Tasks/files/hash cases are excluded from the initial three-way suite: matching
scheduler/pool work or full file discovery/read/SHA requires additional proof.
The old partial/xxh3 Rust driver cannot establish equivalence. Broader existing
PythRust/CPython timings may be supplemental only and require a fresh run; no
historic value may populate a current table.

## What is ready and blocked

Ready: isolated checkout and exact checkpoint, authority/scope review, four
unchanged sources and identity/output contract, measurement/test design and
commands. CPython 3.13.5 is available at /usr/bin/python3.13; the default Python
on PATH is 3.12.14. Untimed preparation runs are correctness evidence only.

Blocked: no discoverable cargo/rustc; no installation permission. Rust source,
harness implementation, equal-work code inspection/domain proof, native/interpreter
parity, release builds, full tests and measured results remain outstanding.
No PR, commit, remote publication or Done transition has occurred. Resume in this
same branch once the coordinator resolves toolchain access; do not discard work.
