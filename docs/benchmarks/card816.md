# Constant-divisor call-site clones #816

#814 left the tuple workload's time in the D63 scalar-pair clone of
`div_mod(a, b)`, called as `div_mod(n, 10)`: the divisor is a parameter,
so the helper runs the general checked floor division and remainder
(three `idiv` sites). This card tests one bounded candidate (D69): a
direct call passing an int literal other than 0 and -1 to a parameter
the callee never redefines and divides by may target a private clone
of the callee whose body binds that parameter at entry. The call stays
a real call with the same signature, arguments, depth check, spans,
pending-fault tests and cleanup; only the divisor becomes a constant that
Cranelift strength-reduces. At most four clones per callee, and only
callees of at most 64 IR instructions. No general inliner, no call
elimination, no IR type, runtime, binding or task ABI change.

The #814 source-inline diagnostic is not repeated and not counted here.

## Protocol

Baseline: main `02c7816` built from a clean worktree. Candidate: branch
commit `24fd9db` (clean tree). Both built `--locked --release` with
separate target directories; the four `.pyr` workloads are unchanged.
Preparation (`bench/three_way.py --prepare` for the baseline,
`bench/division_campaign.py prepare` for the candidate) checks each
program's stdout, then `division_campaign.py pair` times both
conditions in seeded balanced order after two warmups each, pinned to
CPU 0, with nothing else built or run during timings. The harness now
takes the candidate's declared build-input delta (`--delta`, default
#814's) and checks it exactly: no deleted inputs, only declared added
ones, and every declared input changed. Raw samples, host records and
identities are in [24 rounds](card816-pair24.json),
[42 rounds](card816-pair42.json) and the
[no-D63 attribution pass](card816-nopair24.json).

Host: Linux x86-64 container, Intel Xeon @ 2.10 GHz, 4 CPUs, shared and
uncontrolled (frequency, tenants). This is a different host from #814's,
so absolute times are not comparable with card814.md.

## Results

Median ± MAD seconds; "faster" counts the rounds where the candidate's
sample was below the baseline's in the same round.

| Workload | 24 rounds: main → candidate | Faster | 42 rounds: main → candidate | Faster |
|---|---:|---:|---:|---:|
| tuples | 0.076055 ± 0.001675 → 0.026360 ± 0.000740 | 24/24 | 0.076064 ± 0.002359 → 0.026513 ± 0.001083 | 42/42 |
| numeric | 0.386172 ± 0.009731 → 0.379521 ± 0.014653 | 13/24 | 0.386018 ± 0.017556 → 0.387443 ± 0.009290 | 22/42 |
| sorting | 0.065949 ± 0.001555 → 0.065740 ± 0.000566 | 12/24 | 0.065681 ± 0.003280 → 0.065408 ± 0.003567 | 19/42 |
| strings | 0.240079 ± 0.007505 → 0.241296 ± 0.007111 | 11/24 | 0.240286 ± 0.006270 → 0.240885 ± 0.013839 | 19/42 |
| startup | 0.002259 ± 0.000092 → 0.001342 ± 0.000266 | 16/24 | 0.002142 ± 0.000415 → 0.001435 ± 0.000374 | 23/42 |

The numeric, sorting and strings executables are instruction-identical
between the conditions (their `objdump -d` text differs only in
addresses; the binaries differ in the embedded source path), and the
startup executables are byte-identical. Control differences, including
the large relative startup shifts on identical bytes, are consistent with
shared-host and path variability; instruction-identical code can still
differ through layout and path effects. These campaigns show no
repeatable material control slowdown.

### Attribution: the clone alone and combined with D63

To separate the clone from D63, both compilers were rebuilt with D63's
call-site selection disabled (one-line experiment patch in `pair.rs`,
not shipped), so `div_mod` returns a boxed tuple in both.

| tuples, 24 rounds | Baseline | Candidate | Faster |
|---|---:|---:|---:|
| D63 on (shipped): main → clone | 0.076055 ± 0.001675 | 0.026360 ± 0.000740 | 24/24 |
| D63 off: main → clone | 0.528846 ± 0.015893 | 0.536779 ± 0.022387 | 9/24 |

Without D63 the boxed tuple's allocation and refcount work dominates and
the clone shows no gain beyond spread. The gain exists only combined
with D63, where the helper's division was the remaining cost.

## Generated code and size

[Selected disassembly](card816-code/) of the tuple program's generated
functions for all four builds. The candidate's
`pt_const_2_1_10_pair` has no `idiv`: a multiply-high by the magic
constant, shifts, and `imul $0xa` for the remainder, plus the existing
floor corrections. The zero-divisor and `MIN // -1` checks fold away
because the bound divisor is the constant 10.

| tuples (bytes) | main | candidate | main, no D63 | candidate, no D63 |
|---|---:|---:|---:|---:|
| helper the hot call targets | 424 (`pt_scalar_pair_2`) | 104 (`pt_const_2_1_10_pair`) | 544 (`py2_div_mod`) | 224 (`pt_const_2_1_10`) |
| caller `digits` (pair clone / boxed) | 272 | 272 | 469 | 469 |
| ELF `size` text | 377686 | 377782 | 377830 | 378054 |

Text grows 96 bytes in the shipped configuration: the clone is added
while the now-unused D63 pair clone of `div_mod` is still emitted.
Pruning unused clones is left out to keep the change narrow. The numeric
control's text is unchanged (320710 both).

## Parity and checks

- `examples/const_divisor.pyr` (in `NATIVE`, golden from CPython 3.13):
  extreme and signed numerators against literal divisors 10, -10, 3, -7,
  1, 2, `i64::MAX` and `i64::MIN`; the fallbacks -1, 0 and dynamic
  divisors; a redefined parameter; recursion with a literal divisor; a
  try/except inside a clone; side-effecting and failing arguments.
  Helpers called only with `i64::MAX` and `i64::MIN` keep those
  divisors under the per-callee cap, so their boxed and pair clones are
  generated and checked (a unit test pins that selection). The
  native build matches, and runs clean under `valgrind --leak-check=full`.
- `tests/const_divisor.rs`: native output, stderr and exit status equal
  `run`'s for overflow inside a clone and inside a pair clone's second
  field, unbounded recursion through a clone (`RecursionError` at the
  same depth), an uncaught `ZeroDivisionError` in a clone, and an
  argument that fails before the call. CI also runs it against the
  release compiler.
- Unit tests: clone selection (the tuple workload selects exactly
  `div_mod` bound to 10 in its pair variant; one clone per value and
  variant; 0, -1, dynamic, redefined, non-divisor, float and default
  arguments select none; the per-callee cap), and optimized machine code
  of `n // d`, `n % d` and both together has no hardware division once
  `d` is bound to 10, 3, 7, -10, `i64::MAX` or `i64::MIN`, but does when
  unbound. That test failed with the binding disabled.

## Final head

After the measurements, `Codegen::define` was changed to take its
translation context as one argument (a Clippy argument-count fix, no
behaviour change). Rebuilt from the final head and compiled from the
same source paths, the generated functions of all four workloads are
instruction-identical to the measured candidate's. Locally: `cargo fmt
--check`, Clippy with warnings denied, and `cargo test` (1,049 passed,
5 pre-existing ignored) pass; the release compiler passes
`corpus_matches_native` and `const_divisor`; the bench unit tests pass.

## Limits

One shared host with a millisecond-level noise floor; no hardware
counters or profiles were taken for this card. Only the tuple workload
has an eligible call; the other workloads are controls. Clone symbols
appear only in the object's local symbol table, never in program output.
