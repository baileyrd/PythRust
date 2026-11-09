# Remaining string allocations and copying (#817)

Evidence-only result: **no production candidate attempted or retained**. This
profiles verified main `02c7816698f2d424ec0f2a487dce9f41c75bebe0`, tree
`40b65a9e91eb33dddd6946680d8780033277de1c`, after shipped #807, #811, #814
and #815. Production Rust, Cargo inputs, workload algorithms and inputs are
unchanged. This does not reject the optional private binary-concat dispatch
hypothesis, widen D64, change resource policy, or start #816.

## Scoped allocations and copies

The unchanged `bench/strings.pyr` executes one million iterations and prints
exactly `9785790 20000\n`. A new **serial-only**, separate GNU link-wrap probe
tracks opaque live object identities, retires them at actual deallocation and
classifies the returned allocation separately from its text destination. It
never reads `PtStr` layout. Successful allocator requests and linked
`memcpy`/`memmove` destinations are observed; known input/result byte lengths
cross-check every scoped buffer total. Unknown objects, nesting and table
capacity problems increment errors and invalidate the measurement. Index-result
byte lengths come from observed non-object allocations, not a layout assumption.
The probe is deliberately unsuitable for task instrumentation; task controls
below use ordinary binaries.

[Raw evidence](card817-evidence.json) retains counts, full raw counter text,
return PCs, profile samples, all ordinary samples and resources.
[Selected disassembly](card817-code/) maps the four concat return PCs in the
count binary to generated operations. Full disassembly, objects, link wrappers,
count/profile/release binaries and all 14 gmon files remain task-local under
`/workspace/card817`; none is published as a binary or cache.

| Operation/result | Calls | Successful allocations | Requested bytes | Linked copies into text buffers |
|---|---:|---:|---:|---:|
| Literal strings, including initial empty line | 2,000,001 | 4,000,001 | 66,000,032 | 2,000,000 |
| Integer conversion strings | 2,000,000 | 4,000,000 | 71,785,790 | 7,785,790 |
| `key[0]` | 20,000 | 40,000 | 660,000 | 20,000 |
| `"k" + str(i)`; return PC `0x407918` | 1,000,000 | 2,000,000 | 38,888,890 | 6,888,890 |
| Prefix + `":"`; return PC `0x407944` | 1,000,000 | 2,000,000 | 39,888,890 | 7,888,890 |
| Final key + `str(i % 97)`; PC `0x4079b3` | 1,000,000 | 2,000,000 | 41,785,790 | 9,785,790 |
| Growing line + indexed character; PC `0x407ac1` | 20,000 | 40,000 | 200,650,000 | 200,010,000 |
| Outside scopes, including startup/output support | — | 5 plus 1 realloc | 1,100 | 26 linked copies, destination unclassified |
| Total | — | 14,080,006 plus 1 realloc | 459,660,492 | 234,379,386 linked copies |

Each scoped result object requests 32 bytes on this build; that is inferred from
the returned allocation's recorded request, not hard-coded into the probe.
The 7,040,001 returned objects request 225,280,032 bytes. Their text buffers
request 234,379,360 bytes. Scoped reallocations, unknown/nesting/capacity errors
and non-buffer scoped linked copies are all zero. The empty line has an object
and no text allocation. Requests are cumulative traffic, not peak/live memory.

The growing-line operation accounts for 200,010,000 text-copy bytes: 199,990,000
left-operand prefix bytes and 20,000 newly appended bytes. Only the final 20,000
bytes remain in the last line value. The first two key-concat stages allocate
14,777,780 intermediate text bytes that are consumed by later concat stages;
the final key stage produces 9,785,790 bytes, mainly observed only via `len`.
Literal/conversion/index strings are also expression inputs, not printed text.
“Result allocation” therefore means the owned result of one operation, **not**
proof that it must be materialized under every semantics-preserving compiler.
The actual printed output is 14 bytes; outside-scope linked copies are not
relabeled as printed output or unavoidable text traffic.

Within observed linked edges each concat copies its complete result **once**
into an exact-sized buffer: 224,573,570 bytes across all four sites, with no
extra intra-operation copy. Repeated growing-prefix copies dominate byte
traffic, but bytes do not measure execution cost. Python semantics alone do not
require this allocation strategy. Removing key intermediates, literal
materialization or repeated line results needs additional repository-authorized
analysis of evaluation/failure order, ownership and resource policy; the current
D64 proof does not cover these shapes. No broader fusion, builder, mutable
string, shared buffer or allocation elimination was attempted. A private binary
copy/dispatch experiment remains an **untested optional hypothesis**: these
counts neither establish nor rule out instruction/dispatch savings.

## Reconciliation and execution-cost limits

| Earlier evidence | Fresh finding | Classification and limit |
|---|---|---|
| #808/#814: 14,080,006 allocations, 459,660,492 requested bytes, 234,379,386 linked copies | Exactly reproduced after #815 | Reproduced counts; not reused as fresh timings |
| #807: 3,020,000 binary concat results and 224,573,570 concat text copies on unchanged strings | Same totals; caller still has four binary concat sites | Reproduced; shipped prepared-parameter triple fusion does not apply here |
| #808/#811: conversion formatting and character scans; #811 shipped private decimal formatting | Current disassembly uses the private formatter; fresh conversion captures 12.18% of in-image PCs | Formatter mechanism reproduced; generic decimal formatting has not returned |
| #814: concat + join-copy 54.66% of captured in-image PCs, conversion 6.67%, character counting 4.00% | Fresh concat 20.51% + join-copy 18.91% = 39.42%; conversion 12.18%; character-count symbols 3.84% | Changed statistical measurement; Rust/toolchain/session differ, no causal speedup or wall-share conclusion |
| #815: list selection changes, small string shift qualified by same-path and identical-byte controls | String allocation/copy totals remain exact; fresh release self-controls below | Reproduced mechanism limits; #815 did not establish zero code-induced string difference |
| Historical linked-copy sum did not distinguish temporaries from result/output or growing line | Per-operation text/object buckets, four return sites, left/right byte totals | Newly visible from scoped destination tracking plus generated-code inspection |

The fresh PC report accumulates about 1.56 sampled seconds over 14 completed
runs (about 100 Hz). It is linked with `-pg -no-pie`; production compiler/runtime
are not rebuilt with call instrumentation. Shared-library allocator/memcpy PCs
are absent, and histogram/symbol bucketing can attribute boundary PCs to adjacent
or inactive symbols. There are no call-graph counts. No per-operation causal
wall time, allocator latency, peak memory, hardware counters, lock stacks or
unseen inlined/same-object/libc-internal copy bytes were measured. Sampling
fractions and requested-byte proportions cannot be converted to end-to-end
speedup predictions. Separate profile durations are never mixed with release
timings. No tracing/privilege/security change or denied historical retry was made.

## Fresh release measurements and self-controls

Same-host baseline is current main, not a historical absolute time. The
`baseline` and `replay` columns are **byte-identical copies of each ordinary
release executable**, not baseline/candidate production implementations.
The two 24/42-round passes use balanced seeded interleaving, two retained
warmups per condition and all output/status/user/system/context-switch samples.
All 1,584 ordinary samples and 96 warmups are retained, with complete order;
none is discarded. Builds, tests and probes stopped before ordinary timing.
Whole-process wall boundaries include launch/output capture/wait/shutdown and
exclude preparation/compilation. No startup subtraction. Full unchanged sources
match CPython 3.13 output; reduced controls match the interpreter. Existing mixed
and `dyn` list refusals are retained as named diagnostics, not relaxed.

Seconds, median ± MAD (not confidence intervals); ranges and raw samples in JSON.

| Workload | 24 baseline | 24 replay | 42 baseline | 42 replay |
|---|---:|---:|---:|---:|
| sorting | 0.060584 ± 0.001454 | 0.060868 ± 0.002284 | 0.059269 ± 0.002244 | 0.059339 ± 0.002501 |
| numeric | 0.287062 ± 0.008300 | 0.292274 ± 0.007062 | 0.289463 ± 0.007360 | 0.290174 ± 0.009777 |
| strings | 0.237863 ± 0.008900 | 0.236806 ± 0.013781 | 0.225091 ± 0.007682 | 0.225981 ± 0.007457 |
| tuples | 0.049984 ± 0.001305 | 0.051512 ± 0.001851 | 0.047705 ± 0.001487 | 0.048098 ± 0.002294 |
| int_access | 0.022910 ± 0.000777 | 0.023562 ± 0.000942 | 0.021832 ± 0.001266 | 0.022370 ± 0.001207 |
| float_access | 0.074889 ± 0.002602 | 0.075424 ± 0.002234 | 0.070209 ± 0.002949 | 0.071382 ± 0.002799 |
| generic_int | 0.021719 ± 0.000846 | 0.021747 ± 0.000805 | 0.021452 ± 0.001219 | 0.021571 ± 0.001675 |
| generic_float | 0.072622 ± 0.002486 | 0.072854 ± 0.003203 | 0.071942 ± 0.006177 | 0.069913 ± 0.004036 |
| task_int | 0.046468 ± 0.000895 | 0.046384 ± 0.001110 | 0.051991 ± 0.004354 | 0.054351 ± 0.004152 |
| task_float | 0.072653 ± 0.002100 | 0.072960 ± 0.001478 | 0.077245 ± 0.003547 | 0.077457 ± 0.003507 |
| tasks_alloc | 0.112527 ± 0.004782 | 0.111087 ± 0.002036 | 0.113572 ± 0.004297 | 0.111798 ± 0.004394 |
| tasks_alloc_serial | 0.106031 ± 0.003283 | 0.104732 ± 0.002757 | 0.107665 ± 0.003476 | 0.108725 ± 0.003815 |

The 42-round `task_int` replay median is about 4.5% above its baseline label,
with MADs about 4.2–4.4 ms and identical executable bytes; the 24-round pair is
nearly equal. Other small shifts also move across passes, and the string
absolute median changes between passes. These matched identical-byte controls
show host/order/launch variation; they are not candidate improvements or
regressions. There is no changed production binary for a code-induced-regression
claim. Task controls here are pinned to one permitted CPU, as were historical
campaign primary controls; no multicore throughput conclusion is made.

Linux x86-64 / AMD EPYC 9V74, glibc 2.41, CPython 3.13.5, Rust **1.97.0 /
LLVM 22.1.6**, GCC 14.2, GNU binutils 2.44; CPU 0 of the nine permitted CPUs.
Historical #808/#814/#815 used Rust 1.99.1 / LLVM 23.1.1 and other sessions.
Host identity, affinity, cgroup observations, tool hashes/versions and build
environment are retained. Shared host/frequency/other tenants are uncontrolled;
visible processes do not prove host exclusivity. No environment installation or
configuration changes were made. Release compiler/runtime use locked default
Cargo release settings; native code uses the existing speed/PIC/generic target.
`PYTHRUST_RT` selects the matching archive. No RUSTFLAGS, preload, unsafe,
dependency, ABI, scheduler, ownership or language change.

## Evidence integrity and reproduction

[Immutable manifest](card817-manifest.json) binds all 126 tracked production
inputs to Git blob IDs, byte counts, SHA-256 and the admitted main/tree, with an
empty candidate delta. It freezes the workload text/oracles, complete expected
tool inventory, compiler/runtime/build-log identities, preparation, measured
harness/probe source snapshots and supporting-record digests. This is a
campaign-specific integrity export with a digest pinned by the verifier; it is
not cryptographic attestation or independent evidence that a tool was honest.
Historical validation reads this export and published evidence, never arbitrary
future production/workload content. Two matching editable dictionaries alone
would not establish provenance.

The measured preparation harness/probe precede the startup-realloc correction;
the successful collection versions are also frozen separately. Later metadata,
verifier, documentation and WORK-path-option edits change no measured production
input/artifact, timing, counter or profile. No expensive measurement was repeated
solely for those later changes. Initial red import failure and the preliminary
count-validator failure remain in hashed validation logs; the latter's original
binary/object stays under `/workspace/card817/probe`.

The verifier checks complete profile objects/binary/wrapper/gmon identity shapes,
required wraps and `-pg -no-pie`, exact commands/flags, unique execution IDs and
profile-prefix linkage, count text, output/status/resources, timing order and
warmups, and generated-code linkage to measured binaries. Immutable exports
cross-bind preparations/tools/inputs/supporting records. Focused mutations reject
omitted inputs/tools, wrong source/compiler/artifact identities, malformed or
missing profile hashes, changed/rehashed flags/wrappers, duplicate executions and
cross-bundle substitutions. Separate semantic-guard tests bypass the outer
record digest to exercise command/profile rejection. A small independently
linked fake runtime calibrates object-vs-buffer classification, empty/UTF-8 byte
inputs, indexing and allocator-address reuse without consulting future Rust.

From a fresh checkout of the admitted main plus this profiling harness (GNU/Linux,
existing CPython 3.13 and repository Rust toolchain), use a new absolute work
folder and retain the successful release-build log there. Never overwrite this
historical campaign export or its artifacts:

```sh
source /workspace/.pythrust-tools/activate.sh
export PYTHRUST_817_WORK=/workspace/card817-new
mkdir "$PYTHRUST_817_WORK"
cargo build --locked --release -p pythrust-compiler -p pythrust-rt > "$PYTHRUST_817_WORK/release-build.log" 2>&1
python3.13 bench/string_copy_campaign.py prepare
python3.13 bench/string_copy_campaign.py probe
# Stop builds/tests/probes before each ordinary pass:
python3.13 bench/string_copy_campaign.py measure 24 817
python3.13 bench/string_copy_campaign.py measure 42 1817
# Historical bundle integrity (does not execute measured programs):
python3.13 bench/string_copy_campaign.py verify
python3.13 -m unittest discover -s bench -p test_string_copy_campaign.py -v
```

Fresh measurements require their own reviewed immutable export/pin, not accepting
new data against #817's historical fingerprints. This report makes no production
before/after claim and leaves independent exact-head review, fresh hosted CI,
approval/merge and Mill Board Done to the coordinator. #396/PR119 remains held;
D29 C, deferred #563 and residual #816 remain untouched.

## Validation and remaining gates

[Validation manifest](card817-validation.json) retains actual commands, statuses,
complete [hashed logs](card817-logs/), all final production-input hashes and final
release artifact identities. Both final compiler/runtime SHA-256 equal the
measured preparation; all 126 final production files match the admitted export.

| Stage | Local result |
|---|---|
| Formatting; locked all-target Clippy, warnings denied | Passed |
| Full locked Cargo suite, CPython 3.13 | 1,043 passed, zero failed, five existing ignored |
| Benchmark/verifier suite | 59 passed initially; expanded final 60 passed |
| CI Python logic and workflow structure | 20 tests and structure check passed |
| Locked optimized compiler/runtime build | Passed; measured hashes unchanged |
| Absolute release `PYTHRUST_BIN` native corpus plus CLI, expression limits, scalar pair, string chain, signed half | 79 passed, zero failed |
| Frozen measurement/final-build/log/code integrity; whitespace | Passed |
| Local Miri, TSan, Valgrind and other platforms | Never run for this evidence-only change; no production concurrency/ownership surface changed |
| Hosted CI and independent exact-head review | Required coordinator gates after publication |

A final export check caught a filename collision between initial and final build
logs; both original logs were restored under distinct names, and its failure log
is retained. This was publication metadata, not a compiler failure or a repeated
performance measurement. No automatic approval denial occurred in this #817
execution. No author approval, merge, Done transition or next-card work occurred.
