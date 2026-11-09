# Signed half and remainder campaign #814

Retain the bounded compiler change for independent exact-head review and required
CI. Two same-host balanced comparisons reduce numeric execution about 49–50%
and sorting about 4–5%. This PR implements only signed-i64 division/remainder by
positive literal `2`; it adds no general inliner, list or string optimization.
The small string-control differences and the negative-control investigation are
reported below, rather than suppressed. #815–817 remain coordinator-owned next
work; #396/PR119, D29 C and deferred #563 remain untouched.

## Frozen sources and protocol

Fresh fetch at admission confirmed main `f3f93a31aad142d2498a5ea23da075a75ac59ee9`,
tree `ada605b5799a28268ad36717e74f755265ce9473`. Existing #813 sources, measurements
and targets were inspected read-only. One isolated checkout and branch
`codex/card814-signed-half` use separate task-owned baseline/candidate targets.
The already installed official Rust/Cargo 1.99.1 and CPython 3.13.5 were reused.
No installation, dependency/lockfile, ABI, unsafe, scheduler or ownership change.

[Baseline](card814-baseline.json), [paired pass](card814-pair.json),
[42-round repeat](card814-repeat.json), [controls](card814-controls.json),
[identical-path negative control](card814-path-control.json),
[baseline profiles](card814-profiles-base.json),
[candidate profiles](card814-profiles-candidate.json),
[probe/linker provenance](card814-probe-provenance.json),
[source/build checkpoint](card814-state.json), [code sizes](card814-code.json)
and [selected generated code](card814-code/) retain evidence and identities.
The measured candidate is the recorded base plus its exact tracked diff and
build-input hashes; later verifier/document additions do not alter those compiler
or runtime bytes. Full staged/unstaged state and new source identities are retained
at the evidence checkpoint; the final PR tree supplies all added test/harness bytes.

The four `.pyr` files and handwritten Rust are unchanged from #813. Same Collatz,
LCG/quicksort, string construction/growth and digit-scan algorithms, bounds,
checksums and stdout. Each full execution must exit exactly 0 with empty stderr;
reduced cases match the interpreter; Rust boundary tests and the independent
input/domain oracle pass. All timed arithmetic fits i64. Checked Rust and string
materialization remain separate controls. Rust is optimized with `-C opt-level=3
-C overflow-checks=off -C codegen-units=1 -C target-cpu=generic`, edition 2021,
ordinary safe Vec/String/scalar tuples, no dependency, no forced noinline/boxing,
no requested LTO. The checked control changes only overflow checks. PythRust
compiler/runtime are built together with the locked graph, release defaults,
Cranelift speed/PIC/generic ISA, and explicit matching `PYTHRUST_RT`.

Ordinary timing includes launch, output capture and wait/exit; compilation is
excluded. Empty startup is separately observed and never subtracted. Two full
warmups precede each condition. Primary three-way runs use all six permutations
three times in seeded balanced blocks (18 rounds); native comparisons use balanced
orders in 24 then 42 rounds. List/inline diagnostics use 24 rounds; identical-path
negative controls use 42. All samples, outliers, order, warmup results, user/system
CPU, context switches and exact statuses are retained. No task-owned build/test/
profile overlapped ordinary timing. Seven `-pg -no-pie` PC-profile runs per workload
and linked atomic count probes are separate. Baseline profiling overlapped a
focused debug compilation, so its probe wall times are especially unsuitable as
ordinary timing. Profile/count executables are now separately retained: all eight
original profile hashes were reproduced exactly before preserving their copies.

Linux x86-64 Debian 13.6 / AMD EPYC 9V74, glibc 2.41, nine permitted CPUs 0–8;
parent and children pinned to CPU 0. Cgroup quota files are unavailable. Task-owned
artifacts are overlayfs, warm after preparation; no cold-cache/disk claim. Shared
host/frequency/tenants are uncontrolled and process namespace observations do not
prove exclusivity. Compiler, runtime, Python, Rust sources/binaries, tool versions,
source/build state, affinity, environment and resource records are in raw JSON.
No perf/Valgrind are installed; no new denied tracing/security action was attempted.
Earlier #808 ptrace denial is not new tracing evidence.

## Fresh main baseline

Median ± MAD seconds (ranges and raw trial order in JSON).

| Workload | PythRust | CPython | Handwritten Rust |
|---|---:|---:|---:|
| numeric | 0.469907 ± 0.018034 | 8.984353 ± 0.144087 | 0.129395 ± 0.004980 |
| sorting | 0.124064 ± 0.003125 | 0.652826 ± 0.035972 | 0.024559 ± 0.000820 |
| strings | 0.218665 ± 0.006220 | 0.359134 ± 0.014757 | 0.073270 ± 0.002900 |
| tuples | 0.047416 ± 0.001762 | 0.744388 ± 0.022219 | 0.007220 ± 0.000328 |
| startup | 0.000880 ± 0.000065 | 0.009586 ± 0.000327 | 0.000899 ± 0.000066 |

#813's numeric/sort/string/tuple native medians were .465/.128/.215/.046 s.
These fresh values reproduce their broad ranking; the independent passes and
shared host do not establish a source speedup between historical baselines.
Startup is sub-millisecond native on this pass and remains included.

## Production experiment and paired results

The typed IR must name positive `Operand::Int(2)` and an exactly `Ty::Int`
numerator. Only then `//` emits an arithmetic right shift by one and `%` masks the
low bit. For every signed i64, including MIN and negative odd operands, these are
Python floor quotient and nonnegative remainder. Lowering already evaluated the
numerator; no operand work, pending-fault branch or order is moved. All dynamic,
other-constant, zero, negative-divisor and float paths retain the original code.

Pre-optimization CLIF tests require the eligible shift/mask and preserve general
`sdiv`/`srem` for dynamic, 0, ±1, -2, 3 and 10 controls. The deterministic CPython
corpus checks extrema, odd/even signs, zero, wider seeded i64 values, floats/bools,
single side-effecting numerators, failing calls and indexes. Separate native/
interpreter tests assert exact exit 1/stdout/stderr for numerator overflow and
MIN // -1. CPython's arbitrary-precision result is not adopted: checked-i64
failure remains the documented deviation. One old diagnostic-count assertion
expected an impossible overflow site at `//2`; its initial failure and the corrected
passing run are retained in validation records.

| Workload | Pass 24: baseline → candidate s | Change | Repeat 42: baseline → candidate s | Change |
|---|---:|---:|---:|---:|
| numeric | 0.461824 → 0.229070 | -50.40% | 0.458234 → 0.231804 | -49.41% |
| sorting | 0.121024 → 0.115200 | -4.81% | 0.120232 → 0.115002 | -4.35% |
| strings | 0.214592 → 0.218894 | +2.00% | 0.212702 → 0.215734 | +1.43% |
| tuples | 0.045058 → 0.045496 | +0.97% | 0.045110 → 0.045201 | +0.20% |
| startup | 0.000803 → 0.000797 | -0.82% | 0.000818 → 0.000816 | -0.20% |

Numeric wins 66/66 paired rounds; sorting wins 58/66. The repeat's numeric MADs
are .012112/.003695 s; sorting .003651/.003181 s. Generated `steps` shrinks
264→179 bytes, and quicksort 2287→2239 bytes; sorting also contains eligible
midpoint `//2`. No overflow check on unrelated add/multiply is removed. Ordinary
ELF `size` text totals fall 464 bytes numeric and 176 bytes sorting (text totals
include read-only data/unwind sections, not just instructions).

Strings show +2.00% then +1.43% median shifts, with paired median shifts +2.43%
and +2.11%; this direction is reported, not called a speedup. Recompiling each
version with the *same original source path* yields **byte-for-byte identical**
string and tuple executables. Their 42-round identical-byte negative control still
shows strings .222337→.225323 s (+1.34%, MAD .007810/.009908) and tuples
.046693→.046695 s. Thus this experiment cannot assign the small string shift to
the compiler change; percent-level launch/layout/shared-host variation remains a
limit. Original differing source paths embed different diagnostic text/data sizes.
No material code-induced slowdown is established on these controls. This is not a
universal sub-percent no-regression guarantee.

### List and helper controls

| Diagnostic | Baseline median ± MAD s | Candidate median ± MAD s |
|---|---:|---:|
| int_access | 0.062605 ± 0.000987 | 0.063395 ± 0.001254 |
| float_access | 0.067432 ± 0.003350 | 0.066005 ± 0.002306 |
| generic_access | 0.066201 ± 0.001655 | 0.065988 ± 0.001878 |
| source_inline_diagnostic | 0.045509 ± 0.000430 | 0.021752 ± 0.000436 |

The list controls perform unchanged four-million load/store loops over typed int,
float and inferred generic int lists. CPython output is independently checked.
Differences are within their observed spread; the old #810 shared-accessor
candidate remains rejected. Mixed int/float lists and `list[dyn]` are named compile
errors under current homogeneous-list rules: exact source/status/diagnostic is
retained, and no executable mixed/dyn measurement is invented. Existing list
alias/index/error/borrow suites pass; no list implementation changed.

The **source-inline diagnostic** alone replaces immediate `div_mod(n,10)` unpacking
with ordered scalar quotient/remainder assignments inside digits, using the
baseline compiler. It preserves this successful workload's arithmetic and output,
but removes the helper/call-depth path and exposes literal 10. It is **not** a
production inliner or a substitute primary workload. Time .045509→.021752 s
(-52.20%) demonstrates a remaining experiment, not a safe general transformation.
Its hot scalar digit function is 224 vs 272 bytes and the separate scalar division
helper disappears; ELF text totals 326598 vs 377206 also reflect removed cold
error/runtime-link closure and different source diagnostic strings, not just
inlining instructions. The shipped candidate leaves tuple helper code unchanged.
#816 therefore has distinct residual work; positive-literal-2 lowering does not
solve the parameter-divisor-10 call. No speculative broader compiler inliner or
constant-10 production rule was introduced.

## Fresh profiles versus earlier evidence

Software histograms count in-image PCs at roughly 100 Hz, excluding shared-library
allocator PCs; no hardware counters, stack/call-graph attribution, branch counts or
causal wall-time fractions. Tuples accumulate only ~25 captured samples baseline,
so percentages have coarse resolution. Count probes observe linked allocator/
copy/runtime-call edges, missing inlined/same-object/internal libc activity. Atomic
probe counters perturb execution; their wall times are never timing evidence.

- **Reproduced — numeric loop:** #808 had 99.34% captured PCs in steps; fresh main
  has 99.33% (2.97 sampled image seconds/7 runs). Candidate has 98.19%, 1.63 s.
  Ordinary matched timings and the generated floor-adjustment removal establish
  the gain; profile seconds alone do not. Cranelift already reduced constant 2 to
  shifts on main, but retained signed floor corrections; the candidate supplies
  the exact Python signed identities earlier generated code lacked.
- **Reproduced — repeated list checks/dispatch:** fresh main records 10,852,716
  ordinary get calls, eight heap allocations, and set/slot/get plus scalar
  inc/dec descriptor dispatch occupy most captured image time. Earlier #808 raw
  profiles already contained inc/dec: these are not newly discovered. Source
  inspection confirms bounds/negative-index/status and borrow checks remain per
  access. Int elements use raw slots; `inc`/`dec` dispatch on their descriptor
  does not imply a per-int atomic refcount or heap box. Check-specific cycle or
  branch-miss cost is unavailable. Candidate counts are identical.
- **Changed — formatting share; reproduced — residual string work:** formatter-
  inclusive main has no old generic decimal-formatting hot symbols; integer
  conversion is 6.67% and general character count 4.00% of main's captured PCs,
  versus #808's 9.28% each plus generic fmt work. This is consistent with shipped
  #811, not a measured isolated gain from changing profile percentages. Concats/
  join-copy remain hot: 54.66% together on this baseline. The 14,080,006 linked
  allocations, 459,660,492 requested bytes, 234,379,386 linked copy bytes and
  7,040,002 string dec calls exactly reproduce #808/#811; str inc remains zero.
  The formatter changed conversion work, not exact-sized ownership allocation.
  Output allocations and growing-line copies are included; no scoped count here
  assigns all bytes to avoidable temporaries. #807's strict borrowed-tail fusion
  still does not cover the arbitrary interleaved temporaries of this workload.
- **Newly visible measurement, reproduced mechanism — tuple helper:** #808 retained
  counts but no tuple PC histogram; #813 inspected helper calls/generated code but
  supplied no new CPU profile. This campaign adds the targeted seven-run histogram:
  84% (0.21 s/7) in scalar div_mod, 12% in scalar digits on main. The pair path
  allocates only five startup/output objects, zero tuple element/refcount reads;
  D63 scalar returns are working. Generated helper retains three general `idiv`
  sites on its ordinary path; the source-inline diagnostic now measures the
  constant-exposure opportunity. Candidate profile buckets shift to 73.81%/23.81%
  with coarse samples; no tuple improvement from //2 is claimed.

No new wrapping/unwrapping bottleneck is established in these scalar workloads.
Repeated descriptor selection is measured as runtime activity; bounds/borrow and
error checks remain required. Optional/dynamic boxing outside these four programs,
allocator internals, hardware events and task contention are not measured. No
attempt is made to reopen D29 C or infer unseen lock stacks.

## Ranked next experiments (proposals, not dispatched)

This ordering uses captured in-image self seconds per completed *baseline*
workload as a limited cost proxy, corroborated by counts/source, not causal
end-to-end savings. Missing shared-library PCs and coarse buckets prevent a
complete cross-workload cost ranking.

1. **#815 compiler-selected integer access:** set/slot/get/descriptor inc/dec
   account for ~0.56 sampled image seconds across seven sorting runs (~.080/run).
   Try one proven `list[int]` call path bypassing scalar descriptor dispatch while
   retaining the existing bounds/status/borrow/alias contract. Keep float and
   generic fallbacks literally untouched. Use these controls and #810 safety
   coverage; do not restore its rejected 4.9–8.2% float-regressing shared accessor.
2. **#817 residual concat/copy:** concat plus join-copy account for .41 sampled
   seconds/7 (~.059/run), with 14.08M allocations and 234.38M linked requested copy
   bytes. First scope counts to individual concat/conversion/line operations.
   A bounded runtime binary-concat copy implementation can test removal of generic
   join dispatch while keeping exact-sized owned output and char counts; target
   temporary prefixes only with existing D64 proof. Preserve Unicode, operand/
   failure order and allocation-failure/serial/task controls. No broad fusion or
   altered allocation behavior is authorized by these counts.
3. **#816 residual small helper:** scalar div_mod is .21 sampled seconds/7
   (~.030/run), independently supported by the -52% source-inline diagnostic.
   Test one compiler-proven arithmetic helper/call-site clone preserving evaluation,
   spans, call-depth/fault/pending state and all uncertain generic paths; compare
   isolated and combined variants/code size. This campaign covers the initial
   inspection and diagnostic only, so the coordinator must not repeat it or count
   diagnostic source substitution as a shipped compiler optimization.

## Final candidate three-way rerun

[Fresh candidate raw samples](card814-after.json) use 18 balanced rounds with
seed 8141, unchanged algorithms and fresh CPython/Rust references. The candidate
compiler/runtime identities exactly match the paired experiment.

| Workload | PythRust median ± MAD s | CPython median ± MAD s | Rust median ± MAD s |
|---|---:|---:|---:|
| numeric | 0.231950 ± 0.006199 | 8.664487 ± 0.129761 | 0.124974 ± 0.004315 |
| sorting | 0.120385 ± 0.004540 | 0.666460 ± 0.033458 | 0.024251 ± 0.000478 |
| strings | 0.213647 ± 0.004391 | 0.343903 ± 0.007135 | 0.068701 ± 0.001029 |
| tuples | 0.047314 ± 0.001788 | 0.733934 ± 0.011723 | 0.007189 ± 0.000328 |
| startup | 0.001030 ± 0.000174 | 0.011655 ± 0.000796 | 0.001050 ± 0.000180 |

Within each fresh primary pass, numeric PythRust/Rust falls from 3.63× main to
1.86× candidate; CPython/candidate is 37.4× on the final pass. Those ratios describe
this host and these inputs, not universal language performance. Candidate sorting
still takes about 5× Rust, strings 3.1× and tuples 6.6×; the ranked follow-ups remain
relevant. The 24/42-round direct native pairs, rather than differences between
separate three-way phases, establish the production gain. Checked Rust numeric
medians are .239865 s on the baseline control pass and .222197 s on the final
control pass, versus ordinary Rust .127501/.120578 in their respective controls;
checked arithmetic has a material workload-specific cost. No startup subtraction
or claim of identical overflow/panic policies is made.

All 1920 ordinary measured executions and 164 warmups in the two three-way,
two native pairs, list/inline and identical-path passes are retained and successful.
The separate 56 PC-profile executions and eight count probes are excluded from
ordinary samples. Preparation parity/build/test executions are also excluded.

## Verification and reproduction

`verify_three_way.py` accepts immutable historical schema 1; new schema 2 adds
full source state, tool/environment records, domain oracle, retained warmups and
post-timing hashes. The campaign verifier additionally pins main's build inputs
to independently merged #813 evidence, compares Rust/PythRust workload bytes,
rejects mismatched timing/probe pairs, validates exact status/resources/seeded
orders/statistics and distinguishes the byte-identical negative control. Tests
corrupt identities, omitted fields, input/source bytes, CPU IDs, environment,
seeds/warmups/resources, exit/abort statuses and counts. Historical raw evidence
was never rewritten. Original gprof binaries were re-created hash-identically;
wrapper sources, tool identities and separate count objects are retained.

The verifier corrections bind each timing invocation to its executable and source
(including CPython identity and `-B`), compare both sets of production build
inputs, and bind schema-2 ordinary builds to compiler/source/output and the exact
edition/optimization/codegen/CPU/overflow flags. Mutation tests exercise command
substitution, input additions/deletions, and build argument corruption.

Historical schema-1 records retain their original, narrower provenance checks.
Historical schema-2 records did not store checked/materialized control build
commands: their flags cannot be independently validated from an absent command.
They remain readable with that limitation. New preparations record those commands
and mark `control_build_protocol=1`, which requires their presence and validates
checked overflow-on and ordinary/materialized overflow-off optimized builds.
No old raw JSON, compiler/runtime byte, workload, or measurement was changed.
The validation aggregate is reconstructed from retained logs, including the initial
failure; it is not the lost original aggregate or a new test run.

Use new absolute work/target directories, not these retained ones. First build
baseline compiler/runtime together with `cargo build --locked --release -p
pythrust-compiler -p pythrust-rt`, prepare/run `bench/three_way.py` as #813 with
seed 814 and 18 rounds. Freeze compiler/runtime paths. Apply the focused compiler
change, build a separate target and use:

```sh
python3.13 -B bench/division_campaign.py prepare --baseline /abs/base \
  --work /abs/candidate --compiler /abs/candidate-target/release/pythrust
python3.13 -B bench/division_campaign.py profiles --baseline /abs/base \
  --work /abs/base-profiles --output /abs/base-profiles/results.json
python3.13 -B bench/division_campaign.py profiles --baseline /abs/candidate \
  --work /abs/candidate-profiles --output /abs/candidate-profiles/results.json
python3.13 -B bench/division_campaign.py pair --baseline /abs/base \
  --work /abs/candidate --runs 24 --seed 814 --output /abs/pair.json
python3.13 -B bench/division_campaign.py pair --baseline /abs/base \
  --work /abs/candidate --runs 42 --seed 81442 --output /abs/repeat.json
python3.13 -B bench/division_controls.py prepare /abs/base /abs/candidate /abs/controls
python3.13 -B bench/division_controls.py measure /abs/controls /abs/controls/results.json
python3.13 -B bench/experiments/card814_path_control.py /abs/base /abs/candidate /abs/path-control
python3.13 -B -m unittest discover -s bench -p 'test_*.py' -v
python3.13 -B bench/verify_division_campaign.py docs/benchmarks/card814-baseline.json \
  docs/benchmarks/card814-pair.json docs/benchmarks/card814-profiles-base.json \
  docs/benchmarks/card814-profiles-candidate.json
```

Run profiling and timings sequentially; do not compile/run tests alongside timings.
The final candidate also receives a fresh 18-round three-way pass.

[Validation](card814-validation.json): locked fmt/clippy/full CPython3.13 Cargo
suite (1042 passed, five pre-existing ignored manual probes), locked release
compiler/runtime, release native corpus, expression-limit/scalar-pair/string-chain/
signed-half suites, benchmark verifier/corruption tests, CI structure and whitespace.
No local Miri/TSan/Valgrind run is claimed: audited runtime/synchronization code is
unchanged. Hosted required CI and independent review remain coordinator gates;
the author does not self-approve, merge or mark Done. The historical board-claim actor-case error was later resolved; it is not a
current publication or validation result. The earlier execution IDs below identify
the preserved measurement campaign.
Execution `01a1bc24-5c46-74ac-a2c7-1f879ca30038`, initial turn
`01a1bc24-6563-7198-80ed-14f6981d000a`.


## Preserved-work continuation validation

The three verifier corrections and CPython invocation-alias regression checks
were validated without rerunning the benchmark campaign. The local continuation
used CPython 3.13 and Rust 1.97.0: locked formatting, Clippy with warnings denied,
full Cargo tests (1,042 passed, zero failed, five pre-existing ignored), release
compiler/runtime, release native corpus and the expression-limit/scalar-pair/
string-chain/signed-half suites passed. The benchmark/verifier suite now has
45 passing tests; all 20 CI logic tests pass. New build outputs are separate
from the retained campaign bytes. Raw disassembly padding remains byte-exact;
its scoped Git whitespace attribute avoids treating preserved objdump output as
editable source formatting.

The reconstructed historical validation aggregate and its retained command/log
references describe the private preserved handoff, not files newly generated by
this continuation. Local continuation logs are retained separately. No new
performance claim is derived from the continuation's checks. The independent
prepublication review identified a CPython symlink-invocation regression, which
was corrected with acceptance and corruption tests. Required hosted CI and final
exact-commit review remain gates; no merge or Done transition is claimed here.
