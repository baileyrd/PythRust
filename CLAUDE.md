# CLAUDE.md

Pythrust: Python's surface, Rust's guarantees. Code starts *loose* (inferred) and is *tempered* (hardened) one function at a time.

## Memory: rusty-remind-me (required)
remind-me is the shared memory across all Claude surfaces (claude.ai, voice, Claude Code). Treat it as the source of truth for prior decisions and context.

Tools load on demand. If remind_me_* tools are not visible, run a tool search for remind_me before doing anything else.

### 1. Search first
• Before answering or acting on anything about Nano, this repo, a past decision, or a named crate/PR/issue, call remind_me_search.

• Use the project name, crate, or PR number in the query (e.g. rusty_mux phase 1, #686 T6).

• Ground the work in what it returns. If results conflict with the repo, the repo wins; note the drift.

### 2. Save as you go
• When a decision is made or a durable fact is learned, call remind_me_add immediately — do not batch to the end.

• category: fact or preference. memory_type: decision for decisions.

• Always set tags (project, crate, PR/issue ids) and entities (e.g. {kind: project, name: rusty_mux}).

• Put the why in metadata.rationale.

### 3. Capture at the end
• At the end of any substantive session, call remind_me_auto_capture with a descriptive title, tags, the full dialog, and a terse summary: topics, decisions, facts, action items, PRs opened/merged.

• If the call fails, tool-search remind_me_auto_capture and retry once.

### Rules
• Never store secrets, tokens, or credentials.

• Keep entries terse. One fact per remind_me_add.

• PR and merge activity (opened, merged, blocked, why) always gets saved — other surfaces rely on it to know current state.

## Read first, in order

1. `README.md`
2. `docs/design.md`
3. `docs/grammar.md` (the spec for phase 2; if code and spec disagree, fix one and say which)
4. `docs/roadmap.md`
5. `docs/handoff.md` (parallel threads and milestones)

## Layout

`crates/pythrust-compiler`, one module per stage. Shared contract types: `span`, `diag`, `token`, `ast`, `exc` (builtin exception classes), `ir`. Stages: `lexer`, `parser`, `infer`, `interp`, `own` (ownership summaries over the IR), and for native code `lower`, `rc` (refcount elision), `codegen` (`docs/native.md`). Stages meet only at the contract types. `crates/pythrust-rt` is the runtime native programs link against.

## Engineering rules

- Small, terse, modular, pragmatic. Modular monolith. No abstraction before two real call sites.
- Explicit over implicit. Types everywhere. Flat control flow: guard clauses, early returns, at most 3 levels of nesting. Short single-purpose functions.
- `Result` and `?`. No `unwrap()`/`expect()` outside tests. Never swallow errors; propagate with context.
- Domain logic free of I/O. Immutable data; make illegal states unrepresentable.
- Zero dependencies unless justified in the PR (the `cranelift-*` crates are accepted). No `unsafe` (`forbid(unsafe_code)`), except the audited C ABI surface of `pythrust-rt`.
- Tests for all non-trivial logic: happy path plus failure and boundary cases.
- Ask before large or hard-to-reverse changes (public API, dependency bumps, deletions). Keep diffs focused. State the why for non-obvious choices.
- One branch and one PR per thread. Commit small.

## Language decisions to preserve

Follow `docs/grammar.md#decided`: one fixed type per variable, no int-to-float variable widening, homogeneous lists with the explicit `dyn` bridge escape hatch, phase-2 `T | None`, and checked integer powers. Type-changing flow-sensitive rebinding is out of scope. The `total = 0` / `total += 1.5` error must suggest `0.0`; negative integer exponent errors must say "use a float base". Check a literal negative exponent at compile time only when the base is integer; `2.0 ** -1` stays legal.

None-first Optional is decided: require `x: int | None = None` and offer that annotation as the fix-it for an unannotated None-first binding that later receives a value. Do not relax loose-mode Optional inference yet. Follow the ordered parser, post-terminator narrowing (separate PR), and conformance work in `docs/roadmap.md#optional-follow-ups-in-order`; module implementation is not parser/CLI completion.

The single CPython-deviations list is `docs/grammar.md#deviations-from-python`; Thread E records these as expected diffs, not bugs. Pending implementation and regression coverage are in `docs/roadmap.md#phase-2-open-implementation-work`. A recorded language decision is not evidence that code implements it.

## Open decisions (do not decide silently; propose and ask)

None. Source file extension: `.pyr`. Do not silently infer new widening rules. Decided in `docs/research/`: Cranelift backend, `pause <expr>`, `shared[T]` builtin type, ownership vocabulary, CFG IR, `pythrust-rt` runtime crate.

## Checks

`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
