# Current PythRust, CPython and handwritten Rust comparison (#813)

Fresh same-host results for main `8b8b48537cc0c60fa266a26f61089213ee6a79fc`
(tree `99e5b0d5b4223300f8eb23b2d323d1c3c58a8a17`, after PR133),
2026-11-08. PythRust takes less time than CPython on all four cases;
handwritten optimized Rust takes less time than PythRust on all four.
These are bounded implementation/workload comparisons, not a universal ranking.
No compiler/runtime optimization or dependency change is part of this report.

## Measured comparison

Seconds, whole-process median ± MAD (MAD is descriptive, not a confidence interval).
All 18 samples per primary condition, min/max, per-child resources, balanced
orders, stdout bytes, sources, commands and artifact hashes: [raw evidence](card813.json).

| Workload | PythRust | CPython 3.13.5 | Handwritten Rust | CPython / PythRust | PythRust / Rust |
|---|---:|---:|---:|---:|---:|
| numeric | 0.465233 ± 0.013073 | 8.633038 ± 0.070409 | 0.125029 ± 0.004236 | 18.56x | 3.72x |
| sorting | 0.127557 ± 0.004333 | 0.709692 ± 0.049536 | 0.025896 ± 0.001413 | 5.56x | 4.93x |
| strings | 0.214682 ± 0.004605 | 0.356451 ± 0.015072 | 0.070582 ± 0.002319 | 1.66x | 3.04x |
| tuples | 0.046418 ± 0.000943 | 0.738995 ± 0.011295 | 0.007176 ± 0.000201 | 15.92x | 6.47x |

The ratios compare elapsed medians, not a claim of saved CPU instructions.
Empty startup medians are 1.025 ms PythRust, 10.563 ms CPython and 1.082 ms Rust;
individual ranges overlap for the two native programs. Startup is included in
workload times and never subtracted. The ~7 ms Rust tuple workload is particularly
sensitive to process launch; no steady-state kernel time is inferred.

## What was matched

Original `bench/{numeric,sorting,strings,tuples}.pyr` bytes run unchanged under
PythRust and CPython. [Handwritten safe Rust](../../bench/rust/) preserves the
source algorithms, loop bounds, arithmetic domain and outputs:

- Numeric: Collatz transitions for starts 1..999,999, total/longest `131434272 524`.
- Sorting: 300,000 values from the same LCG seed 12345/constants, same middle-pivot
  quicksort partition and smaller-side recursion, full sortedness check;
  `True 6 499909 999999`. No library sort is timed. Rust correctness tests compare
  the full 300,000-element result with a separate standard-sort reference.
- Strings: 1M left-associated k+decimal+colon+decimal keys; sum lengths and append
  key[0] every 50th iteration, `9785790 20000`. Rust concatenation may reuse owned
  String buffers and grow the line geometrically. Immutable strings, allocators
  and library conversion implementations differ; allocation/copy parity is not
  claimed. All timed text is ASCII, so Rust byte length/index agrees with code points.
- Tuples: original div_mod/digits calls and immediate unpacking for 0..999,999,
  `5888889 27000000`. Rust uses ordinary scalar tuples; PythRust includes merged
  D63 pair-return optimization. No boxing or noinline barrier is forced on Rust.

Every full warmup and sample exits 0 with exactly those bytes and empty stderr.
Reduced sources additionally match the PythRust interpreter. Four optimized
Rust test binaries each pass their boundary test; sort includes empty, singleton,
equal, reverse, negative and full deterministic vectors. Collatz/digit/string
boundaries include zero where meaningful, and signed decimal extrema. An
[independent untimed oracle](card813-domain.json), reproducible with
`python3.13 -B bench/check_three_way_domain.py`, verifies Collatz peak
56,991,483,520 and LCG maximum multiplication 2,369,763,582,350,864,175 fit i64;
it records canonical input/sorted vector SHA-256. Memoization and reference
library sorting occur only in this correctness oracle, never in timed programs.

## Compiler and semantic controls

PythRust integers check overflow; CPython integers are arbitrary precision.
Ordinary optimized Rust has overflow checks off. All successful timed arithmetic
stays within signed i64; positive divisions/remainders have the same meaning.
A separate 18-round balanced pass changes only Rust overflow-checks=on:

| Workload | Ordinary Rust median s | Checked Rust median s |
|---|---:|---:|
| numeric | 0.123411 | 0.188146 |
| sorting | 0.023733 | 0.023138 |
| strings | 0.071227 | 0.071715 |
| tuples | 0.007328 | 0.006906 |

Checked Rust is still faster than PythRust on these inputs. Numeric checking has
an observable cost here; small sorting/string/tuple control differences are not
proof that enabling checks improves performance. Controls are independent passes,
so use the paired ordinary row in that pass, not a historical or primary median.
Overflow panic/error policies and native bounds/runtime checks are not identical.

[Optimized assembly](card813-assembly/) retains numeric and digit-scan loops.
LLVM inlines tuple calls and uses reciprocal multiplication for constant division;
these are valid compiler effects. Ordinary string assembly retains both decimal
conversions, allocations and copies/growth in the million-iteration loop. No
per-operation black_box, noinline, volatile or allocator instrumentation exists
in the primary Rust binaries. A separate string materialization diagnostic applies
black_box to each constructed key and final line: paired ordinary median
0.068095 s versus materialized 0.075651 s. This barrier is not cost-free and does
not establish equal allocation counts; its time is not the ordinary Rust baseline.
Assembly observation supports retained work, not exact instruction/copy parity.

## Host, build and timing boundaries

Linux 6.18.57 x86-64, Debian 13.6, AMD EPYC 9V74, glibc 2.41;
CPython 3.13.5 at /usr/bin/python3.13, GIL enabled; Rust/Cargo 1.99.1,
LLVM 23.1.1, GCC 14.2 and GNU ld 2.44. Full observed details and official
installation/bootstrap identities: [host record](card813-host.json) and raw JSON.
The user-approved official rustup installer used --no-modify-path; no dependency,
security, spending or host setting change was made. Cargo fetched the existing
locked graph, not new dependency versions.

Original permitted affinity is CPUs 0–8; parent and all children are pinned to
CPU 0. The host is shared, frequency/governor/other tenants uncontrolled, and
namespace visibility does not establish exclusivity. Cgroup quota files are not
exposed; raw /proc/self/cgroup and mountinfo are retained, quota is unknown.
Task-owned artifacts are on overlayfs, warm after preparation/parity/warmups; no
filesystem data workload, cache eviction or cold-disk claim is made. There are
no tasks, hashing or file cases in this three-way suite; the old partial/xxh3
Rust driver is excluded.

PythRust release compiler and runtime were built together with the locked graph;
PYTHRUST_RT selects the matching archive explicitly. Cranelift opt_level=speed,
PIC and generic host ISA builder remain unchanged. Handwritten Rust uses
`--edition=2021 -C opt-level=3 -C overflow-checks=off -C codegen-units=1
-C target-cpu=generic`, default unwind panic and no requested LTO; the checked
control changes only overflow checks and materialization adds --cfg materialize.
The compiler source is frozen at main; harness/source additions were untracked
at measurement but their bytes/hashes are retained. Final evidence/CI additions
do not change any compiler/runtime build input, checked again after timing.

Bootstrap compilation (66 s on this run, cold dependency/build cache) is recorded
separately. Per-workload compile/link has one initial build then seven warm build
samples pinned to CPU 0: PythRust medians span 0.048–0.063 s; Rust 0.044–0.086 s.
No build-speed ranking is warranted from these warm small-source results.
Execution starts immediately before subprocess launch and ends after captured
output and wait/exit. No subtraction of build/link/startup or other controls.

Each condition has two warmups. Primary order uses all six runtime permutations
three times, shuffled in blocks with seed 813 plus case ordinal; every runtime
appears equally in each position. Paired controls use balanced two-runtime orders.
All 450 measured executions are retained, including outliers; there were no
failed/missing samples. No task-owned build/test/profiling ran alongside timing.
The complete bundle verifier checks cases/orders/resources/statistics, original
source identity, build samples, source bytes and preparation correctness records;
current artifact hashes were also rechecked independently.

## Reproduction and validation

Use an isolated checkout of the source checkpoint with the focused benchmark
files from this branch; preserve the original [preparation plan](card813-plan.md)
as a historical checkpoint. Set PATH to an available official Rust toolchain and
use absolute paths. Do not overwrite an existing work directory or result file.

```sh
cargo build --locked --release -p pythrust-compiler -p pythrust-rt
python3.13 -B -m unittest discover -s bench -p 'test_*.py' -v
python3.13 -B bench/three_way.py --prepare --work /absolute/new-work \
  --compiler "$PWD/target/release/pythrust" --rustc /absolute/toolchain/bin/rustc --cpu 0
python3.13 -B bench/three_way.py --work /absolute/new-work \
  --runs 18 --seed 813 --output /absolute/new-work/results.json
python3.13 -B bench/verify_three_way.py /absolute/new-work/results.json
python3.13 -B bench/verify_three_way.py docs/benchmarks/card813.json
python3.13 -B bench/check_three_way_domain.py
```

Select a permitted CPU; preparation fails if pinning fails. No installed package
beyond the approved Rust toolchain/components and existing Python is needed.
[Validation commands, summaries and full local-log hashes](card813-validation.json):
fmt, locked clippy and full Cargo tests passed (1,040 passed, zero failed, five
pre-existing ignored manual timing probes); release native corpus and release
expression-limit/scalar-pair/string-chain tests passed; all 34 benchmark harness
tests passed. CI structure and all 20 CI Python tests passed separately after
adding one harness-test command. Standalone Rust formatting and four boundary
test executables passed. Complete 10-case/450-sample evidence and preserved
artifact identities passed verification. No production source or lockfile changed.
No local Miri/TSan/Valgrind run is claimed: no unsafe/synchronization code changed.
Required exact-head hosted CI and independent source/evidence review remain
coordinator gates. Author will not self-approve, merge or mark #813 Done.
