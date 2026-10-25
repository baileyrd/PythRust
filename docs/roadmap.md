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

These are pending work items from Nano's [recorded decisions](grammar.md#decided), not completed features. Use the [canonical CPython-deviations list](grammar.md#deviations-from-python) for expected differences.

- [ ] **B, with A/C contract coordination: Optional.** Implement `T | None` in phase 2 (including nested annotation positions), the `|` token, the type AST representation, parsing and inference. General unions stay out of scope. Flag grammar/contract conflicts before changing shared types.
- [ ] **Decision needed: None-first inference.** Specify how `x = None` followed by `x = 5` obtains one fixed `int | None` type. Optional's phase is decided; inference timing/constraints are not. Do not solve this by arbitrary widening or flow-sensitive rebinding.
- [ ] **C: fixed-type and int-to-float diagnostic.** Reject type-changing assignment and augmented assignment, with a source-span diagnostic and fix-it: "total is int here; write 0.0". Test both a zero-iteration and a nonempty loop; changing inference must not change the zero-iteration output. Preserve homogeneous lists and do not infer `dyn` as a fallback.
- [ ] **C: literal negative integer exponent.** Reject an integer base with a literal negative exponent at compile time; say "use a float base". Test the legal `2.0 ** -1` case and nonliteral exponents separately.
- [ ] **D: negative integer exponent at runtime.** Keep `int ** int` integer-valued; a negative exponent gives a source-span runtime error saying "use a float base". Test negative nonliteral exponents, legal float-base powers and checked overflow (`2 ** 64`).
- [ ] **E: conformance and diagnostic coverage.** Add expected-difference fixtures for the canonical deviations, not ordinary CPython-equality fixtures. Keep positive fixed-type/Optional cases and a float-base negative-power regression. The differential harness compares stdout/exit status, not stderr: C/D tests must verify the required messages and fix-it.
