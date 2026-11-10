# Implementer Q&A — pythrust
Answered from repo docs and remind-me; each answer cites its source. Backfilled 2026-11-03. QA answers stay in the pack (not remind-me).

## Language
- **Q:** Are ints `i64` with checked overflow? **A:** Yes; overflow (incl. `i64::MIN // -1`, `2 ** 64`) is a runtime error; out-of-range literal is a compile error. [stated] — DECISIONS D2, grammar#lexical
- **Q:** `total = 0` then `total += 1.5`? **A:** Compile error suggesting `0.0`; no widening. [stated] — D3
- **Q:** Unannotated `x = None` later assigned? **A:** Type error with fix-it `x: int | None = None`. [stated] — D4
- **Q:** Mixed-type list? **A:** Error unless element type `dyn`; `dyn` not implemented. [stated] — D5
- **Q:** `x ** -1` with int base? **A:** Compile error if literal, runtime error otherwise; "use a float base". [stated] — D6
- **Q:** Which Python features are missing? **A:** `grammar.md#deliberate-omissions-phase-2`. [stated]
- **Q:** Where do CPython differences go? **A:** `grammar.md#deviations-from-python` only, with a fixture; Thread E treats them as expected. [stated] — D37

## Tempering and ownership
- **Q:** What does `temper` require? **A:** Full annotations, only tempered/builtin callees, written `mut`/`owned`, `raises` for escapes; loose code may write modes too. [stated] — D10, D14
- **Q:** Can tempering change output? **A:** Never; `tempered_matches_loose` holds it. [stated]
- **Q:** May a diagnostic say "borrow"/"lifetime"? **A:** No; `tests/vocabulary.rs` fails it. [stated]

## Backend
- **Q:** Which backend? **A:** Cranelift AOT via `pythrust build`, linking `pythrust-rt` with system `cc` (MSVC `link.exe` on Windows). [stated] — D9
- **Q:** Must native match `run`? **A:** Yes: stdout, exit status and stderr byte-for-byte for corpus programs (`corpus_matches_native`). [stated] — native.md
- **Q:** What is a backend gap? **A:** A named compile error "native backend does not support X yet"; never guess. A gap is not a language restriction. [stated]
- **Q:** CI targets? **A:** x86-64 and AArch64 Linux, macOS and Windows (MSVC) are configured. All six passed at PR #119 head `8ddfb76` (CI run 42421234961), including Windows PE security checks. The resumed main-integrated head needs fresh exact-head hosted validation and independent review. MinGW remains unsupported/unvalidated. See native.md § Targets for the checks and roadmap.md for status.

## Dependencies and unsafe
- **Q:** May I add a crate? **A:** Only with PR justification; `cranelift-*` accepted. [stated] — CLAUDE.md
- **Q:** Where is `unsafe` allowed? **A:** Audited C ABI of `pythrust-rt`, generated binding shims (fixed template, audit test), `pythrust-rt::sync::Slot`. Nowhere else (`forbid(unsafe_code)`). [stated] — D27

## Process
- **Q:** Branch/PR rules? **A:** One branch and PR per thread, commit small; ask before public API, dependency or deletion changes. [stated] — CLAUDE.md
- **Q:** Checks before PR? **A:** `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`; Miri job when `sync.rs` changes. [stated]
- **Q:** What goes to remind-me? **A:** Decisions and PR/merge activity immediately (`remind_me_add`), session capture at end. [stated] — CLAUDE.md
- **Q:** Who answers design questions? **A:** `project-expert` subagent first; Nano only for intent, scope, taste, or reversing a [stated] row. [stated] — D36

## Environment
- **Q:** Rust version? **A:** 1.97 floor (D34, landed in #99). [stated]
- **Q:** Repo name? **A:** `baileyrd/PythRust` (language is "Pythrust"). [stated]
