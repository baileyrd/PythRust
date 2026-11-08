# Portable SHA-256 pilot (#809)

One safe scalar compressor variant is retained for independent review: a 16-word
circular schedule with statically expanded rounds. Fresh measurements show lower
loaded-SHA and full large-file fclone latency on this host. This does not close
the gap to CPython/OpenSSL or establish a general language or I/O speedup.

## Implementation and correctness

`crates/pythrust-rt/src/sha256.rs::compress` loads 16 big-endian words. Before
round t >= 16, the overwritten slot holds W[t-16]; the other reads hold W[t-15],
W[t-7], W[t-2]. Eight rounds rotate the variable names back to their original
order; eight groups execute exactly 64 rounds. All schedule, round and final
state additions explicitly wrap modulo 2^32. Macros expand constant indexes and
constants without dynamic dispatch, heap allocation, unsafe code or intrinsics.
This trades a longer compressor for less schedule storage and explicit round
expansion; timing, rather than an assumed compiler transformation, justifies
keeping this single variant. No alternatives were searched in this task.

Padding remains one/two blocks at the 56-byte boundary, with the existing low
64 bits of byte-length times eight. `bit_length` factors that same wrapping
operation so 2^61-1, 2^61, 2^61+1 and u64::MAX can be tested without huge slices.
This preserves existing behavior, rather than introducing a new length error.
Existing empty/abc/million-a/multiblock FIPS/CPython vectors remain. New
CPython-generated `(i*37+11)%256` vectors cover lengths 0, 1, 55, 56, 63, 64,
65, 119, 120, 127, 128, 129, 1024 and 65537. A conformance test reads those
synthetic files and repeats each hash through both interpreter and native bytes
ABIs against CPython 3.13. Existing file failure/closed-file tests remain.

The interpreter includes this exact source (`interp/stdlib.rs`); the binding
runtime embeds it (`interop/runtime.rs`). Public/runtime/binding ABIs, whole-file
reads, hexadecimal output, scheduler, dependency graph and D22 are unchanged.
[#566/PR69](https://github.com/baileyrd/PythRust/pull/69) rejected adding sha2
without a measured gain on the available non-SHA-NI host; it was closed unmerged.
This pilot adds no dependency, OpenSSL route, SIMD or alternate hash.

## Reproducibility and identities

- Fresh locked release baseline: `81ec33e3c990ff012cd016dd155416476ca33ea0`,
  tree `c4863a696307608b4de9e7111db6f063f5eaa7e0`.
- Measured candidate: `6fe2eb52c5586f08119300aa04e41e9829e1e144`,
  tree `c4fb3da215bcae0c0152295f3c8a04c893975cb6`. Later changes add evidence,
  result verification, CI harness tests and docs; the measured SHA source is
  unchanged. GitHub connector publication reconstructs the measured tree as
  commit `d5dfb5281706356830ad2202ae2ac2cf54495dda` (same tree
  `c4fb3da215bcae0c0152295f3c8a04c893975cb6`). This preserves a remotely
  reproducible measured source despite changed commit metadata. The draft PR
  records the final tested/published tree.
- [Primary raw evidence](card809.json) records source/compiler/runtime/binary,
  Cargo.lock and fixture-manifest SHA-256 identities, exact generated sources,
  commands, all samples, medians/MAD/ranges and runtime order. Both baseline and
  candidate were built fresh in isolated checkouts, with matching adjacent
  runtime archives explicitly selected for native compilation. Cargo.lock hashes
  match. The source and binary identity verifier also checks current artifacts.
- Linux x86-64, AMD EPYC 9V74, Rust 1.99.1/LLVM 23.1.1, CPython 3.13.5 at
  `/usr/bin/python3.13`, `_hashlib`/OpenSSL 3.5.7. Full observed tool versions,
  permitted affinity and filesystem are in the raw environment record.
- Fifteen ordinary samples after one warmup, with baseline/candidate/CPython
  order shuffled per round using seed 809. Affinity is CPU 0 or CPUs 0–3 for
  every runtime in a row. Pool limits remain their existing cpus+4 behavior.
  Every timed process must succeed, print exactly its oracle output and have
  empty stderr. Interpreter parity runs separately outside timing; fclone output
  also matches the independently generated manifest/group oracle. Printed fixture
  prefixes are normalized to `<DATA>` after parity; raw stdout hashes precede
  normalization. Result verification reconstructs normalized groups/digests.
- Fixtures regenerate #808's deterministic eight-file ~32 MiB `large_dup` and
  `large_unique` trees and 256-file ~1 MiB small controls. Loaded-SHA uses the
  exact #808 source: 32 MiB preload, eight hashes per buffer (256 MiB hashed),
  plus process startup, discovery and shutdown. Discovery, reading and startup
  are independent controls; no durations are subtracted.
- Fixtures are task-owned synthetic warm tmpfs data, cleaned after each run.
  No user files, cache eviction, security changes, hardware-counter or tracing
  retries, paid services or new packages. Shared-host load and frequency remain
  uncontrolled; ranges expose occasional outliers. MAD is descriptive spread,
  not a confidence interval. Historical #808 timings are not used as a speedup.

From the final branch, create an unchanged baseline checkout and build both:

```sh
git worktree add --detach ../pythrust-809-base 81ec33e3c990ff012cd016dd155416476ca33ea0
(cd ../pythrust-809-base && cargo build --locked --release -p pythrust-compiler -p pythrust-rt)
cargo build --locked --release -p pythrust-compiler -p pythrust-rt
/usr/bin/python3.13 bench/sha256_pilot.py --baseline ../pythrust-809-base --runs 15 --output /tmp/card809-rerun.json
/usr/bin/python3.13 bench/repeat_sha256_controls.py /tmp/card809-rerun.json --output /tmp/card809-controls-rerun.json
/usr/bin/python3.13 bench/verify_sha256_pilot.py /tmp/card809-rerun.json --identities --controls /tmp/card809-controls-rerun.json
/usr/bin/python3.13 -m unittest discover -s bench -p test_sha256_pilot.py -v
/usr/bin/python3.13 -m unittest discover -s bench -p test_profile_workloads.py -v
```

For the exact measured prototype, check out the candidate commit above; the final
branch preserves its compressor/source byte-for-byte. Standalone microbenchmarks
are opt-in tests in `sha256.rs`; the harness appends the same microbenchmark
module to unchanged baseline source and compiles each with `rustc --edition=2021
-O --test`. They use `black_box`, run on CPU 0, and retain both internal elapsed
and whole-process times separately. Compression processes 2,097,152 fixed blocks
(128 MiB); loaded digest hashes a preallocated 32 MiB buffer four times. These
are separate from full-workload timing, with no instrumentation-based correction.

## Fresh ordinary results

Seconds, median ± MAD. CPU column is permitted affinity width, not a promise of
parallel execution. Raw ranges and all 15 samples are retained in JSON.

| Workload | CPUs | Baseline | Candidate | CPython | Candidate elapsed reduction |
| --- | ---: | ---: | ---: | ---: | ---: |
| `startup` | 1 | 0.001876 ± 0.000119 | 0.001946 ± 0.000178 | 0.010655 ± 0.000294 | -3.8% |
| `startup` | 4 | 0.001928 ± 0.000151 | 0.002149 ± 0.000152 | 0.011890 ± 0.000907 | -11.5% |
| `phase_discover` | 1 | 0.001979 ± 0.000105 | 0.002108 ± 0.000155 | 0.015431 ± 0.000507 | -6.5% |
| `phase_discover` | 4 | 0.002362 ± 0.000489 | 0.001974 ± 0.000174 | 0.016585 ± 0.001933 | 16.4% |
| `phase_read` | 1 | 0.014041 ± 0.000494 | 0.014313 ± 0.000684 | 0.029458 ± 0.001355 | -1.9% |
| `phase_read` | 4 | 0.016006 ± 0.000496 | 0.015837 ± 0.000907 | 0.030564 ± 0.001378 | 1.1% |
| `phase_loaded_sha256` | 1 | 1.161688 ± 0.050972 | 0.866901 ± 0.070030 | 0.208430 ± 0.005998 | 25.4% |
| `phase_loaded_sha256` | 4 | 1.186202 ± 0.077975 | 0.857537 ± 0.043002 | 0.214222 ± 0.009340 | 27.7% |
| `fclone_lite / large_dup` | 1 | 0.151338 ± 0.007478 | 0.110976 ± 0.007121 | 0.048344 ± 0.003238 | 26.7% |
| `fclone_lite / large_dup` | 4 | 0.156828 ± 0.006790 | 0.117426 ± 0.005073 | 0.053339 ± 0.004466 | 25.1% |
| `fclone_lite / large_unique` | 1 | 0.153007 ± 0.009017 | 0.107073 ± 0.002162 | 0.046306 ± 0.001923 | 30.0% |
| `fclone_lite / large_unique` | 4 | 0.153862 ± 0.005917 | 0.117444 ± 0.004992 | 0.050754 ± 0.002368 | 23.7% |
| `fclone_lite / small_dup` | 1 | 0.009173 ± 0.000717 | 0.007531 ± 0.000445 | 0.020577 ± 0.001618 | 17.9% |
| `fclone_lite / small_dup` | 4 | 0.008801 ± 0.000255 | 0.007574 ± 0.000394 | 0.020630 ± 0.001252 | 13.9% |
| `fclone_lite / small_unique` | 1 | 0.009319 ± 0.000585 | 0.007597 ± 0.000350 | 0.020253 ± 0.000808 | 18.5% |
| `fclone_lite / small_unique` | 4 | 0.008808 ± 0.000353 | 0.007801 ± 0.000294 | 0.020411 ± 0.001148 | 11.4% |
| `fclone_tasks / large_dup` | 1 | 0.163210 ± 0.004800 | 0.123891 ± 0.005126 | 0.072895 ± 0.002445 | 24.1% |
| `fclone_tasks / large_dup` | 4 | 0.075224 ± 0.006189 | 0.058405 ± 0.007476 | 0.051398 ± 0.002455 | 22.4% |
| `fclone_tasks / large_unique` | 1 | 0.168294 ± 0.006569 | 0.125952 ± 0.004115 | 0.076596 ± 0.005771 | 25.2% |
| `fclone_tasks / large_unique` | 4 | 0.072154 ± 0.003998 | 0.057766 ± 0.002839 | 0.051119 ± 0.005047 | 19.9% |
| `fclone_tasks / small_dup` | 1 | 0.009713 ± 0.000358 | 0.008600 ± 0.000606 | 0.037831 ± 0.001134 | 11.5% |
| `fclone_tasks / small_dup` | 4 | 0.005957 ± 0.000333 | 0.005904 ± 0.000372 | 0.053770 ± 0.002260 | 0.9% |
| `fclone_tasks / small_unique` | 1 | 0.009676 ± 0.000368 | 0.008547 ± 0.000359 | 0.037823 ± 0.001658 | 11.7% |
| `fclone_tasks / small_unique` | 4 | 0.006322 ± 0.000221 | 0.006423 ± 0.000881 | 0.055449 ± 0.004611 | -1.6% |

Loaded SHA falls by 25–28%; large-file serial/task fclone medians fall by 20–30%
across both trees and affinities. These gains repeat across full-workload and
isolated controls, beyond their typical MADs. The candidate still trails
CPython/OpenSSL: identical digest algorithms and inputs do not imply identical
implementations, and OpenSSL's selected instruction path was not measured.

The initial four-CPU small-unique task difference was +0.10 ms (1.6%), smaller
than candidate MAD 0.88 ms; the corresponding duplicate case was also near noise.
A [separate longer repeat](card809-controls.json) uses 51 samples, a new order seed
8091 and regenerated fixtures, with no code/build change or prior-sample deletion.
It supports no meaningful regression; millisecond percentages remain fragile.
Its discovery/read controls use the small-duplicate tree (unlike the primary
large tree); compare baseline and candidate only within each row.

| Repeated control | CPUs | Baseline s ± MAD | Candidate s ± MAD |
| --- | ---: | ---: | ---: |
| `startup` | 1 | 0.002006 ± 0.000131 | 0.001973 ± 0.000176 |
| `startup` | 4 | 0.002028 ± 0.000234 | 0.002071 ± 0.000237 |
| `phase_discover` | 1 | 0.002520 ± 0.000137 | 0.002623 ± 0.000196 |
| `phase_discover` | 4 | 0.002952 ± 0.000258 | 0.002980 ± 0.000246 |
| `phase_read` | 1 | 0.003614 ± 0.000503 | 0.003384 ± 0.000337 |
| `phase_read` | 4 | 0.003609 ± 0.000219 | 0.003555 ± 0.000230 |
| `fclone_tasks / small_dup` | 1 | 0.009700 ± 0.000507 | 0.008423 ± 0.000524 |
| `fclone_tasks / small_dup` | 4 | 0.006182 ± 0.000423 | 0.005824 ± 0.000376 |
| `fclone_tasks / small_unique` | 1 | 0.009448 ± 0.000347 | 0.008150 ± 0.000405 |
| `fclone_tasks / small_unique` | 4 | 0.005906 ± 0.000284 | 0.005537 ± 0.000380 |

| Isolated microbenchmark, internal s | Baseline median ± MAD | Candidate median ± MAD |
| --- | ---: | ---: |
| `compression` | 0.596316 ± 0.033833 | 0.406093 ± 0.019515 |
| `loaded_digest` | 0.542386 ± 0.020612 | 0.386529 ± 0.011161 |

## Validation and disposition

Retain the one safe scalar production variant based on repeated loaded-SHA,
large duplicate/unique serial/task and isolated-digest improvements, with exact
output parity and no material regression in longer small-file controls. Do not
infer cross-architecture gains until measured there. No further variants are
authorized by this result. #810/#811 remain separate queued pilots; #806/#807,
#767's mapping, held #396/PR119 and open issue73 remain unchanged.

Final validation status is recorded with exact head/tree in the draft PR.
Locally required checks include digest/FIPS/boundary/wrapping and two-backend
CPython parity, independent raw-statistics/group/artifact verification, both
harness test suites, fmt, locked clippy/full tests, locked release compiler/runtime
build and release native corpus/expression/scalar-pair/string-chain suites.
The two manual microbenchmark tests are intentionally ignored during ordinary
correctness tests and executed explicitly by the timing harness. No sync/task/
borrow-cell code changes require a local Miri change run. Local Valgrind/perf
are unavailable; no local TSan/Miri/calibration result is claimed. Existing hosted
memcheck, race/calibration, Miri, target and aggregate gates remain mandatory,
along with independent technical review of the unchanged PR head.


Local validation completed on Linux x86-64 with CPython 3.13.5:

| Command/check | Result |
| --- | --- |
| `cargo fmt --all --check` | passed |
| `cargo clippy --locked --all-targets -- -D warnings` | passed |
| `PYTHON=/usr/bin/python3.13 cargo test --locked` | 1022 passed, 0 failed, 5 ignored: 4 copies of the 2 opt-in microbenchmarks plus existing `sync::tests::atomics_cost` |
| `cargo build --locked --release -p pythrust-compiler -p pythrust-rt` | passed |
| `cargo test --locked --test conformance corpus_matches_native` | passed, release compiler/runtime and CPython 3.13 |
| `cargo test --locked --test expression_limits` | 3 passed, release compiler/runtime and CPython 3.13 |
| `cargo test --locked --test scalar_pair --test string_chain` | 2 passed, release compiler/runtime and CPython 3.13; linked allocation/copy/fault probes included |
| `cargo test --locked --test conformance sha256_patterns_match_cpython` | passed additionally with release compiler/runtime and CPython 3.13 |
| `python3.13 -m unittest discover -s bench -p test_sha256_pilot.py -v` | 6 passed, including saved-evidence corruption checks |
| `python3.13 -m unittest discover -s bench -p test_profile_workloads.py -v` | 7 passed |
| `python3.13 bench/verify_sha256_pilot.py docs/benchmarks/card809.json --identities --controls docs/benchmarks/card809-controls.json` | passed: 24 main cases, 2 microbenchmarks and 10 longer controls |
| `git diff --check`; unchanged locked dependency files | passed |

All release test commands above set `PYTHON=/usr/bin/python3.13` and
`PYTHRUST_BIN` to the absolute `target/release/pythrust` path in the candidate
checkout. The conformance harness builds/selects the matching runtime archive.
Hosted CI, platform validation, memcheck/race/Miri gates and independent review
remain pending; a locally passing test that skips absent instrumentation is not
reported as a local sanitizer run.
