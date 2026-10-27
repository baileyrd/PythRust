# Roadmap

Each phase ends in something runnable.

1. **Spec.** Formal grammar for loose syntax. Exact syntax for tempering annotations, pause markers, and the shared-lock keyword. Precise enough to write a parser.
2. **Vertical slice.** Parser and type inferencer in Rust for a small subset (functions, numbers, strings, lists, control flow). Interpreter first, to prove the loose syntax feels right.
3. **Native backend.** Cranelift (decided). Generate native code for the subset. Benchmark against Python.
4. **Core differentiators.** Ownership inference, borrow checking, tempering, exception-to-Result error lowering. The hardest phase.
5. **Ecosystem.** Python typed bridge, Rust binary-level interop, unified package tool (one manifest, one lockfile), concurrency.

Concurrency and interop come last because they depend on everything beneath them.

Gut estimate: phases 1-3 (a prototype) about a quarter; a language trusted for real work, one to two years.

## Phase-2 open implementation work

Status against main `1202fc46e2a14527f1002fd757f4291443fabec3` (including merged #18): checked items state their validated scope. CLI `check`/`run` and the current 35-program executable corpus are now wired and green; this is not exhaustive regression coverage. Unchecked items remain work from Nano's [recorded decisions](grammar.md#decided). Use the [canonical CPython-deviations list](grammar.md#deviations-from-python) for expected differences.

- [x] **A/C/D: Optional module support.** PR #10 implements the `|`/`is` token and type/compare contracts, Optional inference, contextual literals, per-block None narrowing and interpreter None comparisons. Parser and CLI/conformance completion are tracked separately below.
- [x] **None-first annotation decision and module diagnostic.** Nano confirmed PR #10: a None-first variable that later holds a value requires an explicit annotation (`x: int | None = None`). The module test `none_first_without_annotation_asks_for_one` asserts the fix-it. Do not relax loose-mode inference yet; revisit after corpus evidence.
- [x] **C: fixed-type and int-to-float diagnostic (module level).** Inference rejects type-changing binding and augmented assignment; `int_accumulator_cannot_widen_to_float` asserts a diagnostic naming `total` as int and recommending `0.0`. Homogeneous-list rejection is tested; no implicit `dyn` fallback is implemented.
- [x] **C: literal negative integer exponent (module level).** Inference checks integer-base literal negative powers and says "use a float base"; hand-built AST tests cover folded/unary negative literals, a legal float base and a nonliteral exponent. CLI integration is complete under #17; additional dedicated exponent regressions remain tracked below.
- [x] **D: negative integer exponent at runtime (module level).** Interpreter keeps `int ** int` integer-valued and rejects negative exponents with "use a float base"; tests cover float-base negative powers and checked power overflow. CLI integration is separately validated under #17.
- [x] **E: Optional fixture/type-check diagnostic coverage.** PR #16 adds Optional basics/narrowing/loop-target fixtures and None-first/un-narrowed Optional error cases. `corpus_checks` runs lex, parse and infer over the corpus, asserting `# expect-error:` diagnostic substrings or successful type checking. It does not execute the interpreter.
- [x] **E/F: executable conformance.** PR #17 enables non-ignored `corpus_matches_pythrust` against Cargo's built binary (`PYTHRUST_BIN` may override it). The current 35-program corpus passes stdout/exit-status comparison; CI reports 268 tests and zero ignored. The executable harness does not compare stderr; retain targeted stage diagnostics.

- [x] **B: baseline parser.** PR #6 supplies recursive-descent parsing and parser-error checks for the original subset. Optional parsing landed separately in PR #13 (see the ordered follow-ups below).
- [x] **F: CLI integration.** PR #17 wires `check` through lex/parse/infer and `run` through lex/parse/infer/interp. Tests cover program output, compile errors producing no output, output retained before runtime errors, and check never executing. Runtime errors exit 1; stdout is flushed before diagnostics.
- [ ] **C/D/E: remaining regression coverage.** Verify the zero-iteration/nonempty-loop diagnostic and output invariant, source-level `2 ** 64`, and nonliteral negative exponent behavior through the integrated pipeline. Keep `2.0 ** -1` legal. The passing 35-program corpus does not by itself establish coverage for each of these dedicated regressions.
- [x] **C: Optional return interaction (module level).** An explicit `T | None` return annotation permits fall-through to `None`; PR #10's `optional_return_may_fall_through` covers this. No arbitrary union inference is implied.

- [x] **E: index/string/list-equality spec and corpus.** PR #11 records negative list/string/tuple indexing, Unicode code-point strings, substring membership, same-element-type list equality and compile-error-before-output behavior. Its 29-program corpus includes `tuple_index_error`, `tuple_index_error_negative`, `str_in_non_str`, `list_eq_mismatch` and `list_eq_int_float`; see [corpus instructions](../examples/README.md). These fixtures are now included in #17's executable corpus validation.
- [x] **C/E: tuple-bounds decision/spec/corpus alignment.** Nano explicitly confirmed #12: out-of-range tuple indexes in either direction are compile errors, so nothing runs. Merged #12 aligns the grammar with existing inference and changes `tuple_index_error` / `tuple_index_error_negative` to documented deviations with empty stdout goldens. The prior tuple timing mismatch is resolved; list/string bounds remain runtime errors.
- [x] **E: corpus compile-error diagnostics.** PR #16 adds `# expect-error:` tags to the tuple, list-equality and string-membership compile-error fixtures as well as Optional cases; `corpus_checks` asserts their diagnostic substrings in process. This closes the prior 'wrong error could pass via empty stdout/status' gap for these checks, not executable CLI verification.

### Optional follow-ups in order

1. [x] **B: parser, immediately after #10.** PR #13 implements `'|' 'None'` in types and `is` / `is not` in comparisons, with source-level tests including nested type positions. The inference restriction that `is` comparisons involve `None` remains; general unions remain out of scope. CLI and executable conformance were subsequently validated under #17.
2. [x] **C: supported post-terminator narrowing, separate PR #14.** After a qualifying None branch always leaves via `return`, `break` or `continue`, the remaining block sees `x` as `T`; the reversed `is not None`/terminating-else form is supported too. Tests cover these terminators, fall-through (no narrowing), assignments and loop/control-flow boundaries. An `if` with `elif` clauses is not narrowed past; preserve the exact block/assignment rules in [the grammar](grammar.md#deviations-from-python). This entry records inference/source-test scope; #17 separately validates the current executable corpus. **Nano confirmed deferring the `raise` test until `raise` exists**; `raise` arrived in phase 4 (B1), which narrows after it and tests that.
3. [x] **E: Optional conformance fixtures and type-checks after parsing.** PR #16 covers Optional basics and narrowing, for-target narrowing, None-first annotation error/fix-it, non-None identity rejection and arithmetic on un-narrowed Optional. Its non-ignored `corpus_checks` test checks lex/parse/infer diagnostics and successful programs; CPython checks validate goldens/deviations. PR #17 also enables `corpus_matches_pythrust` and validates these fixtures through the executable pipeline. Do not relax loose-mode Optional inference until corpus evidence motivates revisiting the annotation requirement.
4. [x] **C: narrowing through `and`, `or`, `not` and conditional expressions.** `if best is None or k > best:` type-checks, as do compound guards and `x + 1 if x is not None else 0`; see the rule in [the grammar](grammar.md#deviations-from-python). A truth test (`if x:`) does not narrow; the error suggests `x is not None`. Fixtures `optional_short_circuit`, `optional_short_circuit_unnarrowed` and `optional_truth_test`.

- [ ] **F: final-flush and cross-stream ordering regressions.** #17 implements final-flush error handling and stdout flushing before diagnostics, but the inspected tests do not directly inject a final-flush failure or assert process-level stdout/stderr ordering. Keep these coverage limits explicit; corpus stdout/status success does not prove them.

## Phase-3 native status (M4 done)

M4 is done, as Nano chose (2026-10-26): fix the last subset gaps (#29, #30, #31, and bools as ints in #32), then close it with the follow-ups below. Both exit criteria hold:

- [x] **The subset compiles to a native binary.** `pythrust build` (Cranelift lowering and codegen, `pythrust-rt`) covers ints, floats, bools, `None`, strings, lists, tuples, ranges, Optional (`T | None`), functions with keyword and default arguments, bools computing as ints (#32), generic instances (#30), module variables read from functions, and the subset's control flow (#18 to #25, #29 to #32). Every corpus program that type-checks builds natively and matches `run`'s stdout, exit status and diagnostics (`corpus_matches_native`); the programs outside `NATIVE` are the compile-error fixtures. This is corpus parity, not proof that every valid program builds: [native.md](native.md#known-gaps) lists what is still a backend error.
- [x] **Benchmarked against CPython on four programs** (#28): numeric loop about 20x, list sorting about 8x, string building about 1.3x, multiple return with tuple unpacking about 1.8x, with method, spread and confounders in [benchmarks.md](benchmarks.md).

Follow-ups, none blocking M4:

- [x] **Targets:** PR #33 runs CI, including the native corpus, on x86-64 Linux, AArch64 Linux and AArch64 macOS; PR #42 adds x86-64 Windows, linking with MSVC's `link.exe` (Nano, 2026-10-27: Windows must be supported). x86-64 macOS and AArch64 Windows are untested.
- [x] **Use-after-free checking:** CI's `memcheck` job runs the native corpus under valgrind (`PYTHRUST_VALGRIND=1`), on x86-64 Linux only.
- [x] **Binary size:** about 0.5 MB on x86-64 Linux, down from 5.5 to 6.3 MB, by linking with dead-code removal and without debug info ([native.md](native.md#choices-made-inside-the-settled-design)).
- [x] **Generic functions where no `return` decides the result type** build: one declared `-> list | None` that only ever returns `None` (or has no `return`), and one whose every `return` needs a recursive call first, are settled (see the monomorphization note in [native.md](native.md)). Nano accepted deferring this class at closure (2026-10-26); `generic_undecided`, `generic_never_returns` and `generic_spin` in the corpus cover it.
- [ ] **An operation on the result of a generic function that never returns**, such as `return f(n - 1) + 1`, is still a backend error: the settled `None` meets an `int` operation in lowering, though the operation never runs. Found in review of #36; the fix needs a divergence analysis that conditionals and `and`/`or` do not break.

Native validation evidence (the required native corpus subset and diagnostics compared with `run`) covers x86-64 Linux, AArch64 Linux, AArch64 macOS and x86-64 Windows; x86-64 macOS and AArch64 Windows are not tested. Preserve the wider interpreter language decisions when extending the backend; a backend limitation is not a new language restriction.

## Phase 4 (M5) plan

Language decisions D1 to D8 are in [grammar.md](grammar.md#phase-4-m5-tempering-ownership-exceptions-nano-2026-10-27) (Nano, 2026-10-27). Exit (from [handoff.md](handoff.md)): a tempered function rejects what a loose one accepts, with diagnostics a Python programmer can act on, and no leaked borrow-checker vocabulary. One PR per item, in order within a track; track B needs only A1, and A6 waits for B3.

Track A, tempering and ownership:

- [x] **A0. Generic results no `return` decides** build natively (#36), so lowering covers nearly every checked program. The ownership analysis runs on the IR; the remaining gap above (an operation on a never-returning generic result) must be closed or handled before A3 relies on lowering.
- [x] **A1. Tempered signatures (D1, D5).** Required annotations, no `dyn`, tempered calls only tempered functions and builtins. Corpus property: removing `temper` from a passing program leaves its output unchanged; the harness runs CPython on loosened programs and `tempered_matches_loose` checks Pythrust too.
- [x] **A2. `list(xs)` copy builtin (D3, D4)** in inference, interpreter and native (#38): a shallow copy of a list only (other iterables are a recorded deviation), natively the runtime call `pt_list_copy`.
- [ ] **A3. Parameter mode inference.** Wire `check` and `run` through lowering (analysis only; `run` still interprets), behind a coverage gate: every program `check`/`run` accepts today keeps passing, so a lowering gap is never a new language restriction. A function lowering cannot express gets a conservative summary (every parameter `owned`, result may alias any parameter), which is sound for loose code. Known gaps to keep as regression fixtures: a call to a builtin before the `def` that shadows it (`x = print(7)` then `def print(n): return 1`, rejected in `lower/expr.rs`), an operation on a never-returning generic result, and `try`/`raise` until B3. Then an `own` stage over the IR: per-function `read`/`mut`/`owned` per parameter and whether the result aliases one, bottom-up with a fixpoint per strongly connected component. No user-visible change.
- [ ] **A4. Mode syntax (D2).** Parse contextual `mut`/`owned`; check written modes against inferred ones; tempered functions must write them. Mode diagnostics come only from precise summaries, never from A3's conservative fallback: for a body that does not lower yet (one with `try`/`raise` before B3), checking its modes waits until it lowers.
- [ ] **A5. Loose call-site aliasing.** `f(xs, xs)` against a `mut` parameter of a **tempered** callee is an error about the call ([ownership.md](research/ownership.md#decided)). Loose-to-loose calls keep Python aliasing, even when the loose callee writes its modes.
- [ ] **A6. Tempered body checks (D3, D4).** Non-lexical alias check over the CFG, diagnostics in Python terms. Needs every tempered body to lower: lands after B3 so tempered `try`/`raise` lower, and closes any other lowering gap a tempered body can reach instead of rejecting it.
- [ ] **A7. Refcount elision in native.** Use the summaries and last-use analysis to drop `Retain`/`Release` pairs; rerun the benchmarks and valgrind the corpus. Can follow A3 in parallel with A4 to A6.

Track B, exceptions:

- [ ] **B1. Front end and interpreter (D6, D7).** Unreserve `try`/`except`/`raise`, and allow `as` in `except Name as e` only (it stays reserved elsewhere until `with`), with a regression test that `e` binds the exception; parse, infer and run them; `int(s)`/`float(s)` raise `ValueError`. CPython-differential corpus programs; deviations recorded in grammar.md.
- [ ] **B2. Error inference and `raises` (D8).** Inferred escaping errors for loose functions; tempered ones must declare them.
- [ ] **B3. Native lowering.** Calls return a tag plus value and branch on it ([ir.md](research/ir.md)); native corpus parity for every B1 program.

- [ ] **M5 close.** A test that no diagnostic says "borrow", "lifetime" or "move out of"; docs updated; exit criteria checked.
