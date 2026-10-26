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

Status against main `6e78ff23c8724c6cbd382e41e469f8f20f8f6911` (including merged #12, #15 and #16): checked items describe module-level implementation and tests, not an end-to-end compiler. Unchecked items remain work from Nano's [recorded decisions](grammar.md#decided). Use the [canonical CPython-deviations list](grammar.md#deviations-from-python) for expected differences.

- [x] **A/C/D: Optional module support.** PR #10 implements the `|`/`is` token and type/compare contracts, Optional inference, contextual literals, per-block None narrowing and interpreter None comparisons. Parser completion and pending CLI/conformance work are tracked separately below.
- [x] **None-first annotation decision and module diagnostic.** Nano confirmed PR #10: a None-first variable that later holds a value requires an explicit annotation (`x: int | None = None`). The module test `none_first_without_annotation_asks_for_one` asserts the fix-it. Do not relax loose-mode inference yet; revisit after corpus evidence.
- [x] **C: fixed-type and int-to-float diagnostic (module level).** Inference rejects type-changing binding and augmented assignment; `int_accumulator_cannot_widen_to_float` asserts a diagnostic naming `total` as int and recommending `0.0`. Homogeneous-list rejection is tested; no implicit `dyn` fallback is implemented.
- [x] **C: literal negative integer exponent (module level).** Inference checks integer-base literal negative powers and says "use a float base"; hand-built AST tests cover folded/unary negative literals, a legal float base and a nonliteral exponent. Source/CLI integration remains pending.
- [x] **D: negative integer exponent at runtime (module level).** Interpreter keeps `int ** int` integer-valued and rejects negative exponents with "use a float base"; tests cover float-base negative powers and checked power overflow. This does not establish CLI integration.
- [x] **E: Optional fixture/type-check diagnostic coverage.** PR #16 adds Optional basics/narrowing/loop-target fixtures and None-first/un-narrowed Optional error cases. `corpus_checks` runs lex, parse and infer over the corpus, asserting `# expect-error:` diagnostic substrings or successful type checking. It does not execute the interpreter.
- [ ] **E/F: executable conformance.** Wire the CLI stages and enable `corpus_matches_pythrust`; verify runtime stdout/exit status independently of the in-process type-check harness. Preserve required runtime messages/fix-its in targeted stage tests.

- [x] **B: baseline parser.** PR #6 supplies recursive-descent parsing and parser-error checks for the original subset. Optional parsing landed separately in PR #13 (see the ordered follow-ups below).
- [ ] **F: CLI integration.** CLI `check` calls inference; `run` does not yet call the interpreter. Wire `run` and enable executable corpus regressions before claiming runtime end-to-end completion.
- [ ] **C/D/E: remaining regression coverage.** Verify the zero-iteration/nonempty-loop diagnostic and output invariant, source-level `2 ** 64`, and nonliteral negative exponent behavior through the integrated pipeline. Keep `2.0 ** -1` legal. Module-level green CI is not evidence that these source cases run.
- [x] **C: Optional return interaction (module level).** An explicit `T | None` return annotation permits fall-through to `None`; PR #10's `optional_return_may_fall_through` covers this. No arbitrary union inference is implied.

- [x] **E: index/string/list-equality spec and corpus.** PR #11 records negative list/string/tuple indexing, Unicode code-point strings, substring membership, same-element-type list equality and compile-error-before-output behavior. Its 29-program corpus includes `tuple_index_error`, `tuple_index_error_negative`, `str_in_non_str`, `list_eq_mismatch` and `list_eq_int_float`; see [corpus instructions](../examples/README.md). This is spec/fixture completion, not Pythrust end-to-end validation.
- [x] **C/E: tuple-bounds decision/spec/corpus alignment.** Nano explicitly confirmed #12: out-of-range tuple indexes in either direction are compile errors, so nothing runs. Merged #12 aligns the grammar with existing inference and changes `tuple_index_error` / `tuple_index_error_negative` to documented deviations with empty stdout goldens. The prior tuple timing mismatch is resolved; list/string bounds remain runtime errors.
- [x] **E: corpus compile-error diagnostics.** PR #16 adds `# expect-error:` tags to the tuple, list-equality and string-membership compile-error fixtures as well as Optional cases; `corpus_checks` asserts their diagnostic substrings in process. This closes the prior 'wrong error could pass via empty stdout/status' gap for these checks, not executable CLI verification.

### Optional follow-ups in order

1. [x] **B: parser, immediately after #10.** PR #13 implements `'|' 'None'` in types and `is` / `is not` in comparisons, with source-level tests including nested type positions. The inference restriction that `is` comparisons involve `None` remains; general unions remain out of scope. This parser completion does not imply CLI or conformance completion.
2. [x] **C: supported post-terminator narrowing, separate PR #14.** After a qualifying None branch always leaves via `return`, `break` or `continue`, the remaining block sees `x` as `T`; the reversed `is not None`/terminating-else form is supported too. Tests cover these terminators, fall-through (no narrowing), assignments and loop/control-flow boundaries. An `if` with `elif` clauses is not narrowed past; preserve the exact block/assignment rules in [the grammar](grammar.md#deviations-from-python). This is inference/source-test completion, not CLI/conformance completion. **Nano confirmed deferring the `raise` test until `raise` exists**; keep `raise` reserved.
3. [x] **E: Optional conformance fixtures and type-checks after parsing.** PR #16 covers Optional basics and narrowing, for-target narrowing, None-first annotation error/fix-it, non-None identity rejection and arithmetic on un-narrowed Optional. Its non-ignored `corpus_checks` test checks lex/parse/infer diagnostics and successful programs; CPython checks validate goldens/deviations. `corpus_matches_pythrust` remains ignored: executable/CLI/interpreter integration is still pending. Do not relax loose-mode Optional inference until corpus evidence motivates revisiting the annotation requirement.
