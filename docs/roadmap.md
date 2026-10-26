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

Status against main `ed8ce138ae792b113ef1198d1b7487f8c2fd0e0c` (PRs #6/#7/#8/#10/#11 merged): checked items describe module-level implementation and tests, not an end-to-end compiler. Unchecked items remain work from Nano's [recorded decisions](grammar.md#decided). Use the [canonical CPython-deviations list](grammar.md#deviations-from-python) for expected differences.

- [x] **A/C/D: Optional module support.** PR #10 implements the `|`/`is` token and type/compare contracts, Optional inference, contextual literals, per-block None narrowing and interpreter None comparisons. Parser and CLI/conformance completion remain separate below.
- [x] **None-first annotation decision and module diagnostic.** Nano confirmed PR #10: a None-first variable that later holds a value requires an explicit annotation (`x: int | None = None`). The module test `none_first_without_annotation_asks_for_one` asserts the fix-it. Do not relax loose-mode inference yet; revisit after corpus evidence.
- [x] **C: fixed-type and int-to-float diagnostic (module level).** Inference rejects type-changing binding and augmented assignment; `int_accumulator_cannot_widen_to_float` asserts a diagnostic naming `total` as int and recommending `0.0`. Homogeneous-list rejection is tested; no implicit `dyn` fallback is implemented.
- [x] **C: literal negative integer exponent (module level).** Inference checks integer-base literal negative powers and says "use a float base"; hand-built AST tests cover folded/unary negative literals, a legal float base and a nonliteral exponent. Source/CLI integration remains pending.
- [x] **D: negative integer exponent at runtime (module level).** Interpreter keeps `int ** int` integer-valued and rejects negative exponents with "use a float base"; tests cover float-base negative powers and checked power overflow. This does not establish CLI integration.
- [ ] **E: conformance and diagnostic coverage.** Add expected-difference fixtures for the canonical deviations, not ordinary CPython-equality fixtures. Keep positive fixed-type/Optional cases and a float-base negative-power regression. The differential harness compares stdout/exit status, not stderr: C/D tests must verify the required messages and fix-it.

- [x] **B: baseline parser.** PR #6 supplies recursive-descent parsing and parser-error checks for the original subset. This does not include Optional.
- [ ] **F: CLI integration.** CLI `check` does not call inference and `run` does not call the interpreter. Wire the stages and enable source-level regressions before claiming these decisions work end to end.
- [ ] **C/D/E: remaining regression coverage.** Verify the zero-iteration/nonempty-loop diagnostic and output invariant, source-level `2 ** 64`, and nonliteral negative exponent behavior through the integrated pipeline. Keep `2.0 ** -1` legal. Module-level green CI is not evidence that these source cases run.
- [x] **C: Optional return interaction (module level).** An explicit `T | None` return annotation permits fall-through to `None`; PR #10's `optional_return_may_fall_through` covers this. No arbitrary union inference is implied.

- [x] **E: index/string/list-equality spec and corpus.** PR #11 records negative list/string/tuple indexing, Unicode code-point strings, substring membership, same-element-type list equality and compile-error-before-output behavior. Its 29-program corpus includes `tuple_index_error`, `tuple_index_error_negative`, `str_in_non_str`, `list_eq_mismatch` and `list_eq_int_float`; see [corpus instructions](../examples/README.md). This is spec/fixture completion, not Pythrust end-to-end validation.
- [ ] **C/F/E: tuple bounds and diagnostic gap.** Preserve the specified runtime error for out-of-range tuple indexes in either direction. Current inference rejects statically out-of-range tuple indexes before runtime; reconcile implementation with the spec without silently changing the rule. The two tuple fixtures must retain output printed before the runtime error. Keep `corpus_matches_pythrust` pending until CLI integration; add targeted diagnostic assertions because stdout/status alone cannot establish which error fired.

### Optional follow-ups in order

1. [ ] **B: parser, immediately after #10.** Parse `'|' 'None'` in types and `is` / `is not` in comparisons; retain the inference restriction that `is` comparisons involve `None`. Add source-level tests, including nested type positions. General unions remain out of scope.
2. [ ] **C: post-terminator narrowing, its own PR.** After `if x is None: return` (or `raise` / `break` / `continue`), the remaining block sees `x` as `T`. Test each terminator and a branch that can fall through (no narrowing). Respect the terminator's control-flow target. `raise` is still reserved in the current syntax: flag that prerequisite rather than silently adding exception syntax.
3. [ ] **E: Optional conformance after parsing lands.** Cover narrowing, the None-first annotation error/fix-it, and arithmetic on an un-narrowed `T | None`. Retain diagnostic assertions as well as stdout/status checks. Do not relax loose-mode Optional inference until corpus evidence motivates revisiting the annotation requirement.
