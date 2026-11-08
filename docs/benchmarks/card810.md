# Integer-list access dispatch experiment (#810)

The measured exact-descriptor dispatch variant is **rejected**. Sorting takes
about 41% less wall time and an isolated integer load/store control about 56%
less, but the matched float-list fallback takes 8.2% more in the first batch and
4.9% more in the larger repeat. This fails the card's no-material-regressions
acceptance gate. The final PR retains evidence, a reproducible candidate patch
and safety regressions; production sequence access is unchanged from main.
This result does not reject typed integer access in general.

## Bounded variant and mechanism

The sole candidate is [this patch](../../bench/experiments/card810.patch).
`pt_seq_get` checks for the exact immutable compiler descriptor `li` and invokes
`Slot::get` with an empty retain callback. `pt_list_set` selects `Slot::replace`
with an empty callback and returns its ordinary bounds status. Integer slots own
and reach no heap value, so descriptor traversal and element retain/mark/release
are no-ops in that case. Every other descriptor retains the generic path.
D65 records the project expert's permission for this reversible dispatch pilot.

No codegen, IR, sequence layout, runtime/binding/task ABI, dependency, unsafe
surface, Slot implementation, lock or scheduler changes were made. In particular,
the ordinary idle/marked/busy borrow checks and negative/bounds checks still run.
Read status and codegen's `IndexError`/pending handling are unchanged. Evaluation
and fault order, aliases, #646/#647 holds/element-count elision, D63 pairs, D64
strings, #809 SHA and #812 diagnostics are preserved. D29 C stays parked.

[Separate linked access counts](card810-counts.json) are identical before/after:
sorting performs 10,852,716 ordinary reads and 2,714,780 writes, with no held
accesses; each isolated control performs 4,000,000 reads and 4,000,000 writes.
The matching controls differ only in integer/float type and literals. Their
checksums are `28000000` and `28000000.0`. The unchanged sorting algorithm,
300,000 pseudo-random inputs and seed produce `True 6 499909 999999` in CPython
3.13, interpreter and both native variants. Lower integer times with identical
access counts are consistent with removing dispatch/ownership overhead, not
removing safeguards or doing fewer accesses. The experiment does not isolate the
nanosecond contribution of each instruction, or prove contention.

## Measurements

[Primary raw evidence](card810.json): fifteen ordinary release samples per
baseline/candidate/CPython condition after two warmups; order randomized each
round with seed 810. No instrumented binary is timed. Each process must exit
successfully, print its exact oracle output and have empty stderr. All durations
include startup and shutdown; none are subtracted. CPython medians are:
sorting 0.639356 s, int_access 0.216585 s, float_access 0.212255 s, numeric 8.653677 s, strings 0.348043 s, tuples 0.771822 s.

| Workload | Baseline median (MAD; min–max), s | Candidate median (MAD; min–max), s | Change |
|---|---:|---:|---:|
| sorting | 0.119333 (0.003413; 0.113136–0.171559) | 0.070431 (0.002981; 0.066552–0.076276) | -40.98% |
| int_access | 0.066930 (0.004721; 0.060107–0.084053) | 0.029327 (0.001344; 0.027062–0.048122) | -56.18% |
| float_access | 0.065409 (0.001632; 0.060666–0.086496) | 0.070781 (0.002582; 0.065215–0.084045) | +8.21% |
| numeric | 0.460457 (0.014612; 0.436935–0.532448) | 0.463910 (0.010272; 0.440479–0.512902) | +0.75% |
| strings | 0.224577 (0.004361; 0.220217–0.279826) | 0.226171 (0.005002; 0.218083–0.266740) | +0.71% |
| tuples | 0.048954 (0.002203; 0.045804–0.056457) | 0.049648 (0.003521; 0.045433–0.057028) | +1.42% |

[Additional raw trials](card810-repeat.json): thirty-one randomized native-only
samples after two warmups per condition, seed 1810, with the same preserved
binaries, input/output and affinity. These resolve the initially near-noise
controls and repeat the candidate's float fallback regression. Child CPU times
and voluntary/involuntary context switches are retained alongside wall times.

| Workload | Baseline median (MAD; min–max), s | Candidate median (MAD; min–max), s | Change |
|---|---:|---:|---:|
| sorting | 0.119648 (0.002930; 0.114067–0.210731) | 0.070129 (0.001409; 0.066300–0.079858) | -41.39% |
| int_access | 0.065572 (0.004225; 0.059232–0.104622) | 0.028542 (0.001272; 0.026599–0.085019) | -56.47% |
| float_access | 0.062145 (0.001161; 0.060708–0.118693) | 0.065191 (0.000667; 0.063780–0.077163) | +4.90% |
| numeric | 0.482889 (0.015432; 0.429792–0.646307) | 0.484831 (0.015440; 0.432065–0.536953) | +0.40% |
| strings | 0.240004 (0.006229; 0.217478–0.313631) | 0.238543 (0.005315; 0.220722–0.271133) | -0.61% |
| tuples | 0.054818 (0.002880; 0.045895–0.065824) | 0.054315 (0.003644; 0.046530–0.067934) | -0.92% |

The unaffected numeric/string/tuple controls move by under 1% in the larger
repeat, within their spread. The 4.9% float regression exceeds either condition's
MAD and repeats the first batch's direction. That is a material access fallback
cost under this card's gate, even though the primary sorting gain is larger.
No claim is made that small control changes are real speedups. MAD and ranges
are descriptive, not confidence intervals.

## Identities, environment and reproduction

[Source identities](card810-identities.json): fresh main baseline
`4da6b73d115d4182582731287f6d9da7737573a7`, tree
`0e0fed75b64672cbb20599f661fecc8674eccbf6`; measured production-code snapshot
`49e603cd8b4c6fd5567aed55a02fcfa276e3b2d1`, tree
`a0649ee16e1ceef155cfe3dbe6cbdf4741de9107`. The snapshot is main plus only the
candidate runtime patch. Shell push had no GitHub credentials; connected API
publication recreates this exact tree as commit
`e53cb22e61816980238fcb2adc0afd1c8f56640b` (commit metadata differs). That remote
snapshot is an ancestor of the final evidence commit, where the patch is removed. The timed build also had new conformance registrations in
the working tree (recorded in its raw source diff); these are not compiler or
runtime build inputs. Final tests and docs are additional to the measured code.
Compiler/runtime archive and every workload binary/input SHA-256 are in the
primary record. Cargo.lock is unchanged, with its hash in source identities.

One isolated `pythrust-card810` worktree and one new task branch were used.
Locked baseline and candidate release compiler/runtime builds used the same
host, paths, toolchain and build settings. Baseline artifacts were copied before
applying the candidate; each native condition explicitly selected its matching
runtime archive. The preserved binaries were rechecked by SHA-256 before repeats.

Linux x86-64, AMD EPYC 9V74, Rust 1.99.1 / LLVM 23.1.1, CPython 3.13.5 at
`/usr/bin/python3.13`; CPU 0 affinity for every timed process. Exact versions,
CPU features, original affinity, UTC timestamp and load snapshots are retained.
No compiler/test/other benchmark was launched alongside timings, and the harness
refused competing visible named processes. The execution environment's PID
namespace limits visibility of other workers; it cannot certify exclusive host
use. One-minute host loads at phase boundaries were 0.42–0.99. Median child
CPU/wall ratios for repeated sorting were 0.988/0.979 and float access
0.976/0.977 (baseline/candidate). Shared host scheduling, frequency and cache
state remain limitations; the raw ranges include outliers. No worker was killed,
no cache was flushed, and historical #808 host timings are not a speedup baseline.

To reproduce with this final checkout's harness, create one fresh isolated
checkout at the exact baseline; set `PYTHRUST_BENCH_ROOT` to that checkout and
use an absolute task-owned `work` path. Run the harness from its final checkout
so `profile_workloads.py` remains importable:

```sh
cargo build --locked --release -p pythrust-compiler -p pythrust-rt
# In the baseline checkout, with the final harness path below:
PYTHRUST_BENCH_ROOT="$PWD" python3.13 /path/to/final/bench/int_list_access.py --work /task/work --prepare before
git apply /path/to/final/bench/experiments/card810.patch
cargo build --locked --release -p pythrust-compiler -p pythrust-rt
PYTHRUST_BENCH_ROOT="$PWD" python3.13 /path/to/final/bench/int_list_access.py --work /task/work --prepare after
# Stop builds/tests and inspect visible load before either timing phase.
PYTHRUST_BENCH_ROOT="$PWD" python3.13 /path/to/final/bench/int_list_access.py --work /task/work --output /task/work/timings.json --runs 15
PYTHRUST_BENCH_ROOT="$PWD" python3.13 /path/to/final/bench/int_list_access.py --work /task/work --repeat-native --output /task/work/repeat.json --runs 31
# Separate instrumentation, never part of ordinary timings:
PYTHRUST_BENCH_ROOT="$PWD" python3.13 /path/to/final/bench/int_list_access.py --work /task/work --counts
# Verify the checked-in complete bundle from the final checkout:
python3.13 bench/int_list_access.py --verify-bundle docs/benchmarks
```

`prepare` refuses an existing condition directory. The count phase requires GNU
linker wrapping and `bench/int_list_counts.c` from the final checkout; copy that
probe to the isolated checkout before the phase (it is not a production input).
The harness checks raw statistics, complete trial orders, exact unchanged
sources, source/binary identities and access-count/output consistency. Its tests
reject missing samples, wrong statistics, altered sources, malformed counters
and incomplete repeat-resource evidence. Existing benchmark harness tests remain.

## Correctness and validation

New CPython-oracled corpus fixtures `int_list_access` and
`tasks_int_list_access` cover negative/extreme and out-of-range reads/writes,
empty lists, caught faults and successful accesses after catch, effectful
receiver/index/value and early exception order, aliases, generic instantiations,
float/string/Optional/nested/tuple fallbacks, held-loop reads, shared task reads
and marked writes after join. Runtime tests cover stale status overwrite,
unchanged invalid stores, marked/unmarked integer bounds, reads nested in holds,
fallback reference counts/transitive marking, and concurrent marked readers.
Existing ownership/hold and D63/D64/SHA/diagnostic suites are retained.

The measured candidate passed the complete debug and release native corpora,
focused runtime tests, six marked/held/shared tests across eight Miri schedules,
and three dispatch runtime tests under local TSan (instrumented std/runtime).
These are local checks, not claims about hosted CI or independent approval.
Final-tree validation passed locally:

- `cargo fmt --all --check` and `cargo clippy --locked --all-targets -- -D warnings`.
- `PYTHON=/usr/bin/python3.13 cargo test --locked`: 1,039 passed, zero failed,
  five pre-existing ignored manual timing probes; all 51 conformance checks pass.
- Locked release compiler/runtime build, full release-native corpus and affected
  CLI, scalar-pair, string-chain and expression-limit suites with absolute release
  `PYTHRUST_BIN`.
- All benchmark harness tests, complete saved-result verification, CI-structure
  check and `git diff --check`.
- Final six integer/held/shared tests under eight Miri schedules and three final
  integer access tests with instrumented std/runtime under local TSan.

These checks do not substitute for the final exact-head hosted gates.
Production `seq.rs` before `cfg(test)` is byte-for-byte equal to baseline.
Independent review and all required hosted checks remain the coordinator's gate.
#396/PR119 and #648/D29 C remain held, #811 remains queued, and issue73 stays open.
