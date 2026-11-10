# Integer decimal formatting experiment (#811)

Retain the bounded private formatter for independent review and required CI.
A safe 20-byte stack buffer emits decimal digits and a sign, validates its local
ASCII slice, and makes one exact-sized owned string buffer. `pt_str_from_int`
uses that proven decimal byte length as its code-point count. `PtStr` layout,
reference counting, runtime/binding ABI, general UTF-8 constructors, concat scope,
scheduler and failure policy are unchanged. No dependency or production unsafe code is added.

This preserves #644/PR91 (commit `443375c`): no geometric growth or shrink/realloc
is introduced. D64/#807 remains unchanged. `unsigned_abs` handles `i64::MIN`, and
zero emits a digit. Twenty bytes suffice for the sign and nineteen decimal digits;
the ASCII invariant is local to this formatter, with checked UTF-8 conversion.

## Source and binary identities

Baseline: `bfaa9fed9a0899ddcbfd7f68af693c6e449a6be9`, tree
`a7346701613c3c11071092326278ff37d6f76abd` (fresh main after PR132).
Measured local candidate snapshot: `2ce557923665d868a4c791f745f0591fc6377f8c`, tree
`833fa528d67253c4c8ca4abad79272b588006474`. The shell Git client had no credentials;
connected GitHub API publication preserves that exact tree as commit
[`ea211bb4bd0999677e0111ef66342e2845a0499e`](https://github.com/baileyrd/PythRust/commit/ea211bb4bd0999677e0111ef66342e2845a0499e).
Only commit metadata differs. Raw records retain the actual local identities at
measurement time; the published equivalent tree is the reproducible source snapshot. Subsequent changes add evidence,
verifier coverage and continuity only; measured runtime/compiler source is frozen.
Both locked release pairs were built in this one isolated checkout, in separate
`target/card811-before` and `target/card811-after` directories. Inputs are fixed,
with hashes, actual outputs, source commits/trees, compiler/archive and every
workload binary digest in the [primary raw record](card811.json). No historical
#808 timing is a baseline for the speedup claimed here.

| SHA-256 | Before | After |
|---|---|---|
| compiler | `35908d40b46bae7794155b2cc6fb7b17699180d63ed16fbf282d768df0a4bbb7` | `dda98c547c550c55630913db6ec0fae2a4bf4c0f6af99aed925a94138bc4ddea` |
| archive | `ba0e5e18e179c4b751f3ba809da4f8cf0cbf54f5bfe00e5d64beadf18aa4ccc0` | `b9ca7723d54e9be36d92de67bab3c8f6acade2124cff44e3a56b1a15e5f9adfb` |

## Ordinary release measurements

Same Linux x86-64 host, Rust 1.99.1, CPython 3.13. Two warmups per condition;
21 interleaved randomized process executions (seed 811), then 41 new executions
(seed 1811) on identical preserved binaries. One CPU is `{0}`, four are `{0,1,2,3}`;
original allowed affinity is 0–8. No startup subtraction. Unchanged
`tasks_alloc_serial.pyr`, `tasks_alloc.pyr`, `strings.pyr` and a focused mixed-width,
signed conversion control (`int_format.pyr`, 3.4 million conversions) are used.
CPython/interpreter/native output parity was checked for all four before timing;
every warmup and timed run checks stdout, exit and empty stderr.

All values below are seconds, median ± MAD [minimum, maximum]. Reduction compares
medians; overlapping ranges reflect individual-run noise. Raw records include
trial order, finite nonnegative per-child user/system seconds and nonnegative
integral voluntary/involuntary switch counts.

### Primary, 21 samples

| Workload / CPUs | Before | After | Time reduction |
|---|---|---|---:|
| int_format/1 | 0.164493 ± 0.004447 [0.155677, 0.249408] | 0.142226 ± 0.006169 [0.131343, 0.182572] | 13.54% |
| int_format/4 | 0.165384 ± 0.005757 [0.155641, 0.212650] | 0.140579 ± 0.002668 [0.135405, 0.181259] | 15.00% |
| strings/1 | 0.228850 ± 0.006838 [0.219944, 0.363741] | 0.212639 ± 0.004816 [0.202637, 0.308516] | 7.08% |
| strings/4 | 0.224658 ± 0.008401 [0.216165, 0.317620] | 0.213824 ± 0.006205 [0.205121, 0.256222] | 4.82% |
| tasks_alloc/1 | 0.137612 ± 0.002030 [0.132380, 0.149846] | 0.105962 ± 0.001861 [0.102910, 0.114449] | 23.00% |
| tasks_alloc/4 | 0.058303 ± 0.005776 [0.039323, 0.070371] | 0.045344 ± 0.005243 [0.033048, 0.064347] | 22.23% |
| tasks_alloc_serial/1 | 0.136627 ± 0.002172 [0.128075, 0.169973] | 0.105814 ± 0.001470 [0.102218, 0.112803] | 22.55% |
| tasks_alloc_serial/4 | 0.136134 ± 0.002614 [0.130081, 0.160657] | 0.108807 ± 0.003464 [0.103070, 0.153544] | 20.07% |

### Independent order repeat, 41 samples

[Raw repeat](card811-repeat.json), same source and binary identities.

| Workload / CPUs | Before | After | Time reduction | Faster paired trials |
|---|---|---|---:|---:|
| int_format/1 | 0.163784 ± 0.004315 [0.158490, 0.233484] | 0.140630 ± 0.004595 [0.133638, 0.204812] | 14.14% | 40/41 |
| int_format/4 | 0.167746 ± 0.007237 [0.157186, 0.209586] | 0.143348 ± 0.007327 [0.131306, 0.187193] | 14.54% | 39/41 |
| strings/1 | 0.227398 ± 0.006573 [0.213209, 0.352257] | 0.219988 ± 0.007836 [0.202400, 0.340843] | 3.26% | 37/41 |
| strings/4 | 0.231052 ± 0.008985 [0.215695, 0.264894] | 0.216412 ± 0.007655 [0.202048, 0.259928] | 6.34% | 34/41 |
| tasks_alloc/1 | 0.137223 ± 0.002510 [0.129083, 0.157391] | 0.105152 ± 0.002600 [0.100393, 0.157419] | 23.37% | 40/41 |
| tasks_alloc/4 | 0.052551 ± 0.007654 [0.039949, 0.068559] | 0.043014 ± 0.007128 [0.031536, 0.057672] | 18.15% | 36/41 |
| tasks_alloc_serial/1 | 0.138275 ± 0.003943 [0.128956, 0.272755] | 0.110391 ± 0.004563 [0.101030, 0.152197] | 20.17% | 41/41 |
| tasks_alloc_serial/4 | 0.136747 ± 0.005260 [0.128062, 0.153295] | 0.106513 ± 0.003145 [0.101075, 0.129389] | 22.11% | 40/41 |

The smaller string gain is repeated (37/41 faster paired trials on one CPU,
34/41 on four), rather than inferred from a single median or allocation count.
Median child CPU time also decreases: the repeated task case goes from 0.135834
to 0.103543 s on one CPU and 0.169717 to 0.133913 s on four. Serial controls also
improve. This does not suggest shifting cost into cross-thread allocator contention.
It is not a general contention proof or a claim about every integer distribution.

Process checks before/after each measurement pass found no visible competing
build/test/workload. Namespaces can hide host activity, so hardware exclusivity is
not established. Primary load averages were approximately 1.73/1.66/1.12 before
and 1.33/1.56/1.11 after; repeat 1.03/1.48/1.10 and 1.26/1.47/1.13. Per-child
resources and wide ranges are retained, not discarded as outliers. Cgroup CPU
limits were not exposed at `/sys/fs/cgroup` on this host. The conclusion is bounded
to these same-host inputs/builds/affinities; no absolute cross-host, allocator,
architecture or CPython-relative speedup is established.


### Unrelated arithmetic control, 41 samples

Unchanged `bench/numeric.pyr`, seed 2811, two warmups and 41 interleaved trials at
each affinity; the exact preserved compiler/archive pairs are reused. CPython 3.13
checks output; interpreter timing is not part of this additional control.
[Raw control](card811-numeric.json) retains identities and all resource fields.

| Workload / CPUs | Before | After | Median change | Faster paired trials |
|---|---|---|---:|---:|
| numeric/1 | 0.475500 ± 0.017903 [0.440144, 0.634854] | 0.472571 ± 0.017294 [0.437982, 0.585798] | -0.62% | 24/41 |
| numeric/4 | 0.464799 ± 0.012157 [0.443100, 0.574417] | 0.470815 ± 0.019000 [0.437335, 0.586822] | +1.29% | 19/41 |

The directions differ by affinity, median differences are smaller than either
condition's MAD, and paired results are mixed. No material arithmetic regression
or numeric speedup is established. The primary conversion/string gains remain the
reason for retaining this candidate. These are observations, not a universal
no-regression guarantee. The numeric-control host load/process observations remain
in its raw environment record, separate from the main passes.

## Separate mechanism and failure probes

[Raw counts/faults](card811-counts.json) use GNU linked-edge wrappers with atomic
counters and thread-local conversion scope. They are never linked into timed
binaries. All conversion workloads retain exactly two successful decimal
allocations (buffer and object) per conversion, zero decimal reallocations and
normal output. Serial: 6,400,004 total allocations, 6,400,000 decimal allocations;
tasks: 6,400,063 / 6,400,000; strings: 14,080,006 / 4,000,000; focused: 6,800,006 /
6,800,000, in both versions. The scoped decimal realloc count is zero everywhere.
Whole-process task realloc counts vary (3 vs 4 here) in scheduler/startup work;
they are not decimal reallocations. Counts used inherited 0–8 affinity, separately
from the ordinary 1/4-CPU timing matrix; instrumented wall time is not evidence.

Linked copy bytes match in serial/task (21,888,890 decimal bytes) and strings
(7,785,790). The focused case observes 22,600,000 before vs 24,200,000 after.
These are **linked call edges**, missing inlined/same-object/internal-library
copies, not total logical copied bytes. The difference is not evidence of an
extra logical copy in the new exact buffer. Allocation counts alone assign no
latency. The new formatter still copies its local decimal slice into one owned
buffer, as the original display formatter copied into its exact buffer.

Both buffer allocation (20-byte minimum-value text) and object allocation
(32 bytes here) were deterministically refused. Both versions abort (signal 6),
with Rust std allocation-failure text, empty stdout and no catchable exception.
Normal ownership/reference cleanup is checked by runtime tests and the debug
native corpus leak counter. General Unicode constructors still count code points.

## Validation

[Command results, suite summaries and full local log digests](card811-validation.json).

- Focused runtime `str::` tests: 11 passed. Every decimal boundary ±1 for 1–19
  digits, both signs, zero, i64 MIN/MAX, exact bytes/capacity, code-point lengths,
  refcount lifetime and mixed Unicode concat/index paths are covered.
- New `examples/int_format.pyr/.out`: every signed decimal boundary and both
  extrema against CPython 3.13, direct and ordered task rendering with Unicode.
- `PYTHON=/usr/bin/python3.13 CARGO_TARGET_DIR=target/card811-after cargo test --locked`:
  1,040 passed, zero failed. Five pre-existing ignored manual measurements:
  interpreter/runtime SHA compression and loaded digest (four), atomics cost (one).
  Includes all 51 conformance checks, #806 scalar pairs, #809 SHA, #810 access
  safety, #812 ownership diagnostics, debug native corpus and binding tests.
- `cargo clippy --locked --all-targets -- -D warnings`, `cargo fmt --all --check`,
  and locked release compiler/runtime builds passed. The isolated target directory
  was supplied for build/clippy/test commands.
- Absolute release `PYTHRUST_BIN=.../target/card811-after/release/pythrust`:
  full native corpus passed; expression-limit suite (3) and #807 string-chain
  suite (1) passed, including allocation/copy parity, fallback, overflow and OOM.
- CI structure checker and all 20 CI Python tests passed. All 28 benchmark harness tests and complete-bundle verification (18 timing rows
  plus count/failure evidence) passed before publication.
- Local Miri/TSan were not rerun: no borrow-cell/task/sequence synchronization or
  unsafe surface changes. Hosted Miri/race/platform/memcheck gates remain required.
  Local valgrind is unavailable. No hosted validation or independent review is
  claimed by this report; coordinator owns both and merge/Done.

## Reproduction

Use one clean isolated checkout. Build the exact base and measured candidate with
`cargo build --locked --release -p pythrust-compiler -p pythrust-rt --target-dir ...`
into separate absolute target directories. Preserve the corresponding compiler,
archive, source HEAD/tree and input hashes before switching conditions. The final
harness can be copied onto the base as measurement-only untracked `bench/` files;
no `crates/` change is allowed when `--prepare before` captures its identity.
Use the same focused input from this PR for both conditions. Do not relabel a
candidate binary as baseline, overwrite a prepared directory, or rebuild between
trials. The equivalent published candidate source snapshot above remains in this branch history.

```
python3.13 bench/int_format.py --work /absolute/new-work --prepare before --target /absolute/before-target
python3.13 bench/int_format.py --work /absolute/new-work --prepare after --target /absolute/after-target
python3.13 bench/int_format.py --work /absolute/new-work --runs 21 --output /absolute/new-work/timings.json
python3.13 bench/int_format.py --work /absolute/new-work --runs 41 --seed 1811 --output /absolute/new-work/repeat.json
python3.13 bench/int_format.py --work /absolute/new-work --counts --output /absolute/new-work/counts.json
python3.13 bench/int_format.py --work /absolute/new-work --prepare-control
python3.13 bench/int_format.py --work /absolute/new-work --numeric-control --runs 41 --seed 2811 --output /absolute/new-work/numeric.json
python3.13 bench/int_format.py --verify-bundle docs/benchmarks
```

Run each preparation while its corresponding source snapshot is current. A new
run from the published equivalent records its current commit identity; do not
rewrite historical raw IDs to make a new run pass the historical bundle gate. The
numeric control reuses the already preserved exact pairs and checks CPython output;
it does not run the slow interpreter. Bundle verification requires exact before /
after identities in every record, identical main/repeat pairs, matching numeric
compiler/archive hashes, complete finite resources/statistics/order/affinity,
unchanged sources and separate allocation/failure evidence. Corruption tests reject
omitted/malformed fields. #810's historical data/verifier remain untouched.

#396/PR119 and D29 C remain held, issue73 stays open, and this card author does not
self-approve, merge, mark Done or advance the queue.
