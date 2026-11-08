# Pythrust — spec
Design chat: unknown (backfilled 2026-11-03 from repo docs + remind-me; no single chat) · remind-me tag `pythrust`
Tags: [stated] [agreed] [default] [open]. Grammar details live in `docs/grammar.md` (canonical); this file points, it does not copy.

## Intent
Nano (CDTO, Python-first) wants Python's surface with Rust's guarantees: a Python programmer reads it on day one and gets real speedups with no new mental model. Code starts *loose* (inferred) and is *tempered* (`temper def`) one function at a time. [stated]

## Scope
- In: phase-2 subset (ints, floats, strings, lists, tuples, dicts, sets, control flow, `T | None`); `temper def` with modes `mut`/`owned`, `raises`; builtin exception hierarchy; Cranelift native backend (`pythrust build`); stdlib M6a (`sys`, `os`, `hashlib`, `open(p, "rb")`); Rust bindings M6b; tasks via `concurrent.futures` + `pause` M6c; package tool M6d (`pythrust.toml`, `pythrust.lock`).
- Out / later: classes, comprehensions, lambda, slicing, `try` else/finally, user exceptions, generators, text-mode/writing `open`, `shared[T]` and channels (parked), Python bridge M6e (waits for a second app), type-changing rebinding, general unions, arbitrary-precision ints. Full list: `grammar.md#deliberate-omissions-phase-2`.

## Constraints
- Tempering never changes meaning: removing `temper`/modes/`raises` from an accepted program leaves output unchanged. [stated]
- CPython 3.13 is the conformance oracle; every divergence is on `grammar.md#deviations-from-python`. [stated]
- No leaked borrow-checker vocabulary in diagnostics (`tests/vocabulary.rs`). [stated]
- Engineering rules: `CLAUDE.md` § Engineering rules (zero deps unless justified, no `unsafe` outside audited surfaces, tests for non-trivial logic, ask before hard-to-reverse changes). [stated]
- Source extension `.pyr`. [stated]
- Windows must be supported (MSVC). [stated]

## Shape
```
source -> lex -> parse -> infer -> (interp | lower -> ir -> own/rc -> codegen -> object -> cc + pythrust-rt)
```
Modular monolith: `crates/pythrust-compiler` (one module per stage; stages meet only at `span diag token ast exc ir`), `crates/pythrust-rt` (runtime native programs link). Detail: `docs/design.md`, `docs/native.md`, `docs/research/`.

## State (2026-11-08, main a6d8ca9)
M0–M5 and M6a–M6d are implemented, including shipped runtime source for Rust-binding builds and the external-copy fclone-lite driver. M6c's `shared[T]`/channels remain parked; M6e waits for a second app. Post-M6 step 1's string methods and builtins are implemented; later work remains planned. See [current handoff](../handoff.md#state) and [roadmap](../roadmap.md) for scope, recent fixes and held/unvalidated work. Driver app: fclone-lite (`apps/fclone-lite/`).

## Acceptance — first slice (for a new thread)
- [ ] `docs/roadmap.md` item for the thread is checked only with a test that holds it.
- [ ] `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` green.
- [ ] Any new CPython deviation is added to `grammar.md#deviations-from-python` with a fixture.
