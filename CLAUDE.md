# CLAUDE.md

Pythrust: Python's surface, Rust's guarantees. Code starts *loose* (inferred) and is *tempered* (hardened) one function at a time.

## Read first, in order

1. `README.md`
2. `docs/design.md`
3. `docs/grammar.md` (the spec for phase 2; if code and spec disagree, fix one and say which)
4. `docs/roadmap.md`
5. `docs/handoff.md` (parallel threads and milestones)

## Layout

One crate, `crates/pythrust-compiler`, one module per stage. Shared contract types: `span`, `diag`, `token`, `ast`. Stages: `lexer`, `parser`, `infer`, `interp`. Stages meet only at the contract types.

## Engineering rules

- Small, terse, modular, pragmatic. Modular monolith. No abstraction before two real call sites.
- Explicit over implicit. Types everywhere. Flat control flow: guard clauses, early returns, at most 3 levels of nesting. Short single-purpose functions.
- `Result` and `?`. No `unwrap()`/`expect()` outside tests. Never swallow errors; propagate with context.
- Domain logic free of I/O. Immutable data; make illegal states unrepresentable.
- Zero dependencies unless justified in the PR. No `unsafe` (`forbid(unsafe_code)`).
- Tests for all non-trivial logic: happy path plus failure and boundary cases.
- Ask before large or hard-to-reverse changes (public API, dependency bumps, deletions). Keep diffs focused. State the why for non-obvious choices.
- One branch and one PR per thread. Commit small.

## Language decisions to preserve

Follow `docs/grammar.md#decided`: one fixed type per variable, no int-to-float variable widening, homogeneous lists with the explicit `dyn` bridge escape hatch, phase-2 `T | None`, and checked integer powers. Type-changing flow-sensitive rebinding is out of scope. The `total = 0` / `total += 1.5` error must suggest `0.0`; negative integer exponent errors must say "use a float base". Check a literal negative exponent at compile time only when the base is integer; `2.0 ** -1` stays legal.

The single CPython-deviations list is `docs/grammar.md#deviations-from-python`; Thread E records these as expected diffs, not bugs. Pending implementation and regression coverage are in `docs/roadmap.md#phase-2-open-implementation-work`. A recorded language decision is not evidence that code implements it.

## Open decisions (do not decide silently; propose and ask)

Source file extension (placeholder `.pythrust`); how None-first inference obtains a fixed Optional type without flow-sensitive rebinding. Do not silently infer new widening rules. Decided in `docs/research/`: Cranelift backend, `pause <expr>`, `shared[T]` builtin type, ownership vocabulary, CFG IR, `pythrust-rt` runtime crate.

## Checks

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
