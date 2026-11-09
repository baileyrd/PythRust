# Compiler-selected integer-list access #815

This experiment compares fresh current-main and candidate binaries on one host.
The production gate requires repeatable integer/sorting gains, semantic parity,
and no material float/generic/control slowdown. Final measured results and
validation are recorded below; independent exact-head review and hosted CI are
required before merge/Done. #810's shared-accessor candidate remains rejected.

## Scope and proof

D66 permits two internal compiler/runtime entrypoints, `pt_int_list_get` and
`pt_int_list_set`, selected only when lowered monomorphic IR proves the receiver
is `Ty::List(Compound::List(Ty::Int))`. An inferred generic integer instantiation
has this proof too; an inferred float instantiation does not. Result type alone
is insufficient: tuple fields, optional and nested lists stay generic. Held-loop
reads retain their existing interfaces. This is an internal compiler/runtime
interface addition; it changes no external binding/task ABI or representation.

Both helpers use the original `Slot::get`/`replace`, including idle, held, busy,
marked and synchronized checks and the same Python negative/bounds checks.
Integers own/reach no heap reference; only descriptor traversal and scalar
retain/store/release dispatch are omitted. There is no unchecked indexing, new
lock scheme, per-integer atomic refcount elimination or boxing claim. Status
bytes, `IndexError`, pending-fault branches, evaluation order and cleanup remain
on the original codegen path. General `pt_seq_get` and `pt_list_set` bodies are
unchanged. #646/#647 holds and element elision, D29 C, D63 pairs, D64 strings,
#814 signed-half lowering, #396/PR119 and deferred #563 remain untouched.

## Protocol and provenance

Admission main is `626a3487223bc48569b627541da5920a653508aa`, tree
`77fa65f721d8b2b053bd43f66263a5ae3bbc75ab`. One branch,
`codex/card815-int-list`, uses `/workspace/card815`. The configured stale clean
checkout was preserved; no protected #814 author/recovery workspace was used.
The initial executor startup failed before access; one authorized normal retry
restored the same executor. No security/permission setting was changed.

Baseline was built and copied before production edits. Candidate was rebuilt
and separately prepared after correcting a test-only held-read expectation.
The [preliminary prepared candidate](card815-preliminary-preparation.json) and failed test log remain preserved, with
no ordinary timing samples taken from it. Final production build-input hashes,
complete tracked source diffs, compiler/runtime and each binary/source hash,
exact build/execution commands, tool versions, environment, CPU affinity, cgroup
and host snapshots are embedded in the raw timing records. The only changed
tracked compiler/runtime inputs are codegen/mod.rs, codegen/seq.rs and seq.rs.
The final validation build is checked against the measured artifacts.

Locked compiler/runtime builds use default Cargo release optimization and the
existing Cranelift speed/PIC/generic ISA configuration. Local toolchain is Rust
1.97.0/LLVM 22.1.6 and CPython 3.13.5. This fresh host is Intel Xeon Platinum 8573C,
Linux x86-64/glibc2.41, CPUs 0–4 permitted and a four-CPU cgroup quota. Every timed
parent/child is pinned to CPU 0. Shared tenants, frequency/cache state and host
scheduling remain uncontrolled; visible process checks cannot prove exclusivity.
Absolute timings are not comparable to earlier EPYC/Rust1.99 campaigns.

Unchanged numeric, sorting, string and tuple sources come from the existing
benchmark files. Integer and float load/store controls use #810's matched sources;
the inferred variants remove only annotations. Task controls perform two read-only
workers over the same marked list, on the same pinned CPU, with identical work
and output for each native condition. They do not test four-core scaling or D29 C.
Every full native input matched CPython output, and reduced copies matched the
interpreter, during preparation. Mixed float/int lists and `list[dyn]` remain
named unsupported compile errors under current language rules; both exact source,
command and diagnostic records are retained. No mixed executable is invented.

Ordinary measurements include process launch, capture, execution and shutdown;
no startup is subtracted. Compilation, instrumentation and profiling are excluded.
Each batch has two full warmups per condition, then seeded balanced/interleaved
orders: 24 rounds (seed 815), followed by 42 rounds (seed 1815). Every warmup/sample,
wall and child CPU time, context switches, output/status and trial order is kept,
including outliers. No task-owned build, test or probe overlaps either timing
batch. Host records delimit each batch. Sources/build settings/output checks and
preserved binaries are identical across the two batches.

The new harness reuses #814's order/resource/hash helpers. It addresses #810 reuse
gaps with command-to-binary/source/build linkage, exact build settings and source
input sets, warmup/resource/output checks and recomputed summaries. Corruption
tests exercise substituted commands, changed flags, missing trials/warmups,
added production inputs, changed output and missing unsupported evidence. Historical
#810/#814 files and verifiers are not rewritten or relabelled.

## Results

### 24-round comparison

| Workload | Baseline median ± MAD, s | Candidate median ± MAD, s | Median change |
|---|---:|---:|---:|
| sorting | 0.223817 ± 0.024942 | 0.115282 ± 0.014883 | -48.49% |
| numeric | 0.554808 ± 0.016211 | 0.573014 ± 0.031972 | +3.28% |
| strings | 0.338832 ± 0.003658 | 0.342746 ± 0.005322 | +1.16% |
| tuples | 0.086146 ± 0.001508 | 0.088051 ± 0.001973 | +2.21% |
| int_access | 0.100888 ± 0.002774 | 0.028856 ± 0.001872 | -71.40% |
| float_access | 0.104026 ± 0.007003 | 0.101835 ± 0.005160 | -2.11% |
| generic_int | 0.101675 ± 0.005297 | 0.030212 ± 0.002101 | -70.29% |
| generic_float | 0.098693 ± 0.002229 | 0.098740 ± 0.002244 | +0.05% |
| task_int | 0.238625 ± 0.005706 | 0.231127 ± 0.003134 | -3.14% |
| task_float | 0.243240 ± 0.006306 | 0.241865 ± 0.004124 | -0.57% |

### 42-round comparison

| Workload | Baseline median ± MAD, s | Candidate median ± MAD, s | Median change |
|---|---:|---:|---:|
| sorting | 0.177495 ± 0.004005 | 0.093843 ± 0.002061 | -47.13% |
| numeric | 0.556942 ± 0.023721 | 0.556548 ± 0.021198 | -0.07% |
| strings | 0.342817 ± 0.004572 | 0.346817 ± 0.008924 | +1.17% |
| tuples | 0.089545 ± 0.002507 | 0.089058 ± 0.001894 | -0.54% |
| int_access | 0.099522 ± 0.001821 | 0.028881 ± 0.001490 | -70.98% |
| float_access | 0.100667 ± 0.003158 | 0.099381 ± 0.002020 | -1.28% |
| generic_int | 0.102334 ± 0.003328 | 0.029128 ± 0.001583 | -71.54% |
| generic_float | 0.100001 ± 0.001519 | 0.100152 ± 0.001684 | +0.15% |
| task_int | 0.257710 ± 0.013921 | 0.242904 ± 0.011293 | -5.75% |
| task_float | 0.239488 ± 0.003598 | 0.239246 ± 0.003611 | -0.10% |

Retain this bounded candidate for independent review. Sorting improves 47–48%
and ordinary/inferred integer access about 70–72%, each winning all 66 paired
rounds across the two batches. Float and inferred-float fallback shifts are
within their spread. Numeric and tuple first-batch upward shifts do not repeat.
The task integer control has smaller 3–6% median reductions amid wider spread;
this is not a claim about task scaling, contention or the deferred D29 C work.

The primary string medians are +1.16%/+1.17%. This was investigated, not hidden.
Same-source-path builds of six unaffected controls are **not byte-identical**;
the first diagnostic wrongly assumed they would be and failed before ordinary
timing. Its log and artifacts remain preserved. A fresh 42-round same-path
string comparison is +0.68% (.335955→.338241s, MAD .003863/.005469s). A separate
42-round negative control, two copies of the candidate string binary with identical
SHA-256, is +0.78% (.338656→.341312s, MAD .007913/.009985s). Thus no material
string slowdown is established beyond the measured noise here, but these
diagnostics do not prove a zero code-induced difference. The baseline/candidate
linked images differ; the same-path comparison is not labelled a self-control.

All 1,320 primary ordinary samples and 80 primary warmups are retained in
[24-round raw evidence](card815-pair24.json) and [42-round raw evidence](card815-pair42.json).
The [path/control diagnostic](card815-path-control.json) adds 168 ordinary samples
and eight warmups. All trial order, ranges and child resources remain in JSON.
No sample or outlier was discarded. Historical absolute timings are not used
as this experiment's baseline.

## Mechanism and separate profiling

[Separate probe evidence](card815-probes.json) retains eight linked count/profile
cases: baseline/candidate sorting, integer access, float access and inferred float.
Each count run is outside ordinary timing; seven `-pg -no-pie` link-only PC runs
per case are separate again. Executables, probe objects, linker wrapper contents,
commands, tool hashes, all 56 profile execution results, gmon hashes and gprof
reports are retained. Actual gmon files, complete disassembly and binaries remain
under `/workspace/card815-revised/probes`; no binary/cache is committed.
[Selected generated code](card815-code/) and its [command/hash manifest](card815-code.json)
are published. The new probe verifier tests malformed counters, substituted build
or execution commands, changed probe flags/wrappers and missing profile trials.

**Reproduced:** sorting still executes 10,852,716 ordinary reads and 2,714,780
writes with no holds. Each isolated float/int/inferred-float control still executes
4,000,000 reads and 4,000,000 writes. The new probe does not remeasure #814's eight
allocations; allocation counts from that earlier campaign are not relabelled new.
On this host, sorting baseline captured PC fractions are slot 21.57%, get 21.08%,
set 17.65%, scalar descriptor inc 10.29% and dec 4.41%. This reproduces the lead's
mechanism/ranking, not its absolute cost or a causal wall-time decomposition.

**Changed/new:** candidate sorting and integer control route every ordinary
read/store through the typed helpers; their generic read/store counters become
zero and typed totals exactly match baseline totals. Float/inferred-float totals
remain exclusively generic. Candidate sorting captured PCs shift to typed get
43.88%, quicksort 42.86%, typed set 5.10%; slot dispatch is 2.04%. The generated caller
and typed helper code show the narrower selection and absence of descriptor
retain/store/release traversal on those slots. The old general-accessor source
bodies remain unchanged. This tests the compiler-selected interface that #810 did
not test; it does not revive #810's guard in the shared float path.

PC fractions are statistical in-image samples at about 100 Hz, excluding shared
library PCs; Cranelift/runtime code was linked with `-pg`, not rebuilt with call
instrumentation. No call-graph counts or causal per-instruction latency are
claimed. Instrumented durations include probe overhead and are never combined
with ordinary results. Relative fractions naturally move when work is removed;
float profile-fraction shifts are not evidence of float wall-time regression.
Host/toolchain differences prevent absolute profile comparisons with #814.

## Correctness and validation

[Validation manifest](card815-validation.json) links the complete
[retained logs](card815-logs/), exit statuses and SHA-256 values. Against final
changed Rust source:

- `cargo fmt --all --check` and `cargo clippy --locked --all-targets -- -D warnings`: passed.
- `PYTHON=/usr/bin/python3.13 cargo test --locked`: **1,043 passed, zero failed, five existing ignored**; all 51 conformance checks passed, including interpreter/CPython/native corpora.
- `cargo build --locked --release -p pythrust-compiler -p pythrust-rt`: passed; final compiler/runtime SHA-256 match the measured candidate.
- Release `PYTHRUST_BIN` native corpus plus CLI, expression limits, scalar pairs, string chains and signed-half suites: passed.
- All 51 benchmark/verifier tests, including saved #815 bundles and corruption cases, plus 20 CI Python logic tests and CI structure check: passed.
- Affected Slot/task/hold/integer runtime tests under Miri seeds0–7: passed with the existing permissive-provenance setting. New typed accessors execute in the generalized bounds/held-read/shared-reader tests.
- Three affected runtime tests under TSan with instrumented std/runtime: passed. The native marked-list task corpus fixture passed five pinned and five unpinned TSan/parity executions.
- Final whitespace, saved-artifact integrity and production-input consistency: passed.

Existing Python-oracled `int_list_access` and `tasks_int_list_access` cover
negative/extreme/boundary indices, invalid stores, caught faults and later success,
effectful receiver/index/value order, aliases, nested/Optional/tuple/string/float
fallbacks, holds and marked accesses after join. Focused codegen tests require
selection for typed/inferred integers and fallback for typed/inferred floats,
bools, strings, nested/Optional lists, tuples and holds. Existing checked-i64
fatal overflow deviations remain covered; no language rule was relaxed.

Three unsuccessful preliminary checks are retained: a too-specific held-codegen
test expected item-held while this valid shape used get-held (corrected/rebuilt);
Miri could not create its default cache outside writable roots (normal task-owned
`XDG_CACHE_HOME` succeeded); and a byte-identical baseline/candidate string
hypothesis failed before timing (fresh comparison and a separate true self-control
followed). These are not hidden passing runs or negative optimization samples.
No automated approval denial occurred in this #815 work. No Valgrind or other
platform execution was run locally; hosted memcheck/race/platform gates and
coordinator-owned independent exact-head review remain required. This author
has not approved, merged or marked the card Done.

## Reproduction

The commands below describe the original collection protocol at measured author
head `83d06f3905e4ce296e924658481d4777ea096aa7`, using a fresh baseline worktree at the admission commit.
Activate the repository environment toolchain and build both packages together:

```sh
cargo build --locked --release -p pythrust-compiler -p pythrust-rt
python3.13 bench/int_list_campaign.py prepare --work /absolute/work --label baseline --build-log /absolute/baseline-build.log
# Apply only the final production delta, build the same locked release command,
# then prepare candidate with its build log. Existing folders are never overwritten.
python3.13 bench/int_list_campaign.py prepare --work /absolute/work --label candidate --build-log /absolute/candidate-build.log
# Stop builds/tests/probes before ordinary timing:
python3.13 bench/int_list_campaign.py measure --work /absolute/work --output /absolute/pair24.json --runs 24 --seed 815
python3.13 bench/int_list_campaign.py measure --work /absolute/work --output /absolute/pair42.json --runs 42 --seed 1815
# Separate probes, with GNU cc/linker/gprof/objdump:
python3.13 bench/int_list_probes.py collect /absolute/work /absolute/probes.json
python3.13 -m unittest discover -s bench -p test_int_list_campaign.py -v
```

For the historical replay, preparation needs the final Python harness copied
into the baseline checkout; the production source manifest must remain clean.
No timings from these instructions are assumed until actually run. All retained
binary/probe artifacts stay task-local and are excluded from publication.

## Verifier-only independent-review follow-up

The [consolidated review](https://github.com/baileyrd/PythRust/pull/136#issuecomment-6749365396)
approved production safety at measured author head
`83d06f3905e4ce296e924658481d4777ea096aa7`, but held merge for two P2 verifier
false acceptances. The independent audit found the actual saved source identities
and measurements consistent. This follow-up changes only evidence validation,
mutation tests and documentation; it preserves production Rust, workload inputs,
raw samples/profiles, preliminary diagnostics and original validation records.
No timing or profiling campaign was restarted, and the existing tables and
qualified control/shared-host/profile/unsupported-list claims remain unchanged.

The [immutable source export](card815-source-manifest.json) inventories all 126
production files for each condition, with SHA-256 and Git blob IDs derived from
Git objects at admitted baseline `626a3487223bc48569b627541da5920a653508aa` and
reviewed candidate `83d06f3905e4ce296e924658481d4777ea096aa7`. Its complete-byte
SHA-256 is pinned in the verifier. The export also freezes the measured tools,
compiler/runtime/harness identities and workload text/oracles. Missing/added inputs,
valid-looking wrong source digests and incomplete tool inventories now reject.
Historical validation reads this campaign export and archived evidence, rather
than whichever production/workload files a future checkout contains.

Supporting probes require complete object/binary/profile/gmon/wrapper/tool identity
records and the exact counting/profiling commands, including every required wrap
and `-pg -no-pie`. The supplied wrapper source must match the historical command
template; recomputing a hash after removing flags no longer suffices. The new
`int_list_evidence.py` checks both primary preparations agree and cross-binds
probe/path preparations, generated-code binaries/commands and final-build hashes
to that canonical measured preparation. It also checks the original published
logs and selected disassembly bytes. Existing CI's `test_int_list_campaign.py`
step exercises the new mutations and cross-bundle checks.

Ten false acceptances were first reproduced in red mutation tests. Expanded
regressions cover symmetric input removal/addition, missing/malformed tools, wrong
source/compiler/runtime identities, absent/empty probe digests, recomputed wrappers
without required flags/wraps, supporting-bundle substitutions and final-build/code
linkage. A test verifies the complete historical bundle with an empty future
production root and rejects a modified export. The current verifier is deliberately
a validator of this frozen campaign; fresh measurements would need their own
manifest and must not overwrite these records. Validate the saved bundle with:

```sh
python3.13 bench/int_list_evidence.py
```

[Follow-up validation](card815-verifier-followup.json) and its separate logs retain
the red/green outcomes. Formatting, warnings-denied locked Clippy, the full locked
CPython 3.13 Cargo suite (1043 passed, zero failed, five existing ignored), all 54
benchmark/verifier tests, 20 CI logic tests and CI structure checks passed again.
Release builds/corpus, Miri, TSan, long ordinary timings and profile collection were
not rerun for this verifier-only correction; their original results and artifacts
are preserved. The exact new published head/tree and fresh hosted-CI status are
recorded in the PR correction comment and Mill Board #815. Independent re-review
and all required checks on that new head remain mandatory; no author approval,
merge or Done transition is implied.
