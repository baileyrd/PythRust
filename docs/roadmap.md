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

Status against main `8cf29d1a6c7af64b561883e47c843069fad0157b` (PRs #7/#8 merged): checked items describe module-level implementation and tests, not an end-to-end compiler. Unchecked items remain work from Nano's [recorded decisions](grammar.md#decided). Use the [canonical CPython-deviations list](grammar.md#deviations-from-python) for expected differences.

- [ ] **B, with A/C contract coordination: Optional.** Implement `T | None` in phase 2 (including nested annotation positions), the `|` token, the type AST representation, parsing and inference. General unions stay out of scope. Flag grammar/contract conflicts before changing shared types.
- [ ] **Decision needed: None-first inference.** Specify how `x = None` followed by `x = 5` obtains one fixed `int | None` type. Optional's phase is decided; inference timing/constraints are not. Do not solve this by arbitrary widening or flow-sensitive rebinding.
- [x] **C: fixed-type and int-to-float diagnostic (module level).** Inference rejects type-changing binding and augmented assignment; `int_accumulator_cannot_widen_to_float` asserts a diagnostic naming `total` as int and recommending `0.0`. Homogeneous-list rejection is tested; no implicit `dyn` fallback is implemented.
- [x] **C: literal negative integer exponent (module level).** Inference checks integer-base literal negative powers and says "use a float base"; hand-built AST tests cover folded/unary negative literals, a legal float base and a nonliteral exponent. Source/CLI integration remains pending.
- [x] **D: negative integer exponent at runtime (module level).** Interpreter keeps `int ** int` integer-valued and rejects negative exponents with "use a float base"; tests cover float-base negative powers and checked power overflow. This does not establish CLI integration.
- [ ] **E: conformance and diagnostic coverage.** Add expected-difference fixtures for the canonical deviations, not ordinary CPython-equality fixtures. Keep positive fixed-type/Optional cases and a float-base negative-power regression. The differential harness compares stdout/exit status, not stderr: C/D tests must verify the required messages and fix-it.

- [ ] **B/F: parser and CLI integration.** Main parser still returns "parser not implemented"; CLI `check` does not call inference and `run` does not call the interpreter. Wire the stages and enable source-level regressions before claiming these decisions work end to end.
- [ ] **C/D/E: remaining regression coverage.** Verify the zero-iteration/nonempty-loop diagnostic and output invariant, source-level `2 ** 64`, and nonliteral negative exponent behavior through the integrated pipeline. Keep `2.0 ** -1` legal. Module-level green CI is not evidence that these source cases run.
- [ ] **B/C: Optional return interaction.** Reconcile the current fall-through rejection with Optional once its representation/inference rules are agreed; do not retain the stale premise that phase 2 has no Optional or silently choose a new return-inference policy.
