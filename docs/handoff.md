# Pythrust: Claude Code Handoff

You are picking up Pythrust, a new language: Python's surface, Rust's guarantees. Code starts *loose* (inferred, Python-feeling) and is *tempered* (hardened) one function at a time. Goal: a Python programmer reads it on day one and gets real speedups with no new mental model.

Read first, in order: `README.md`, `docs/design.md`, `docs/grammar.md`, `docs/roadmap.md`. The grammar is the spec for phase 2. If code and spec disagree, fix one and say which.

## State

- Done: design sketch, roadmap, loose-syntax grammar (functions, ints, floats, strings, lists, tuples, control flow, optional annotations).
- Exists in code: an empty `crates/pythrust-compiler` lib crate (`forbid(unsafe_code)`). Nothing else.
- Decided: ints are `i64` with checked overflow; tempering is `temper def f(...)`; tuples are fixed-length, immutable, positionally typed.
- Open: pause marker (`pause f(x)`?), shared-lock keyword (`shared`?), source file extension (placeholder: `.pythrust`), LLVM vs Cranelift. Do not decide these silently; propose and ask.

## Engineering rules

- Small, terse, modular, pragmatic. Modular monolith: one crate, clear modules. No abstraction before two real call sites.
- Explicit over implicit. Types everywhere. Flat control flow: guard clauses, early returns, at most 3 levels of nesting. Short single-purpose functions.
- `Result` and `?`, never `unwrap()`/`expect()` outside tests. Never swallow errors; propagate with context.
- Domain logic free of I/O. Immutable data; make illegal states unrepresentable.
- Zero dependencies unless justified in the PR. No `unsafe`.
- Tests for all non-trivial logic: happy path plus failure and boundary cases.
- Ask before large or hard-to-reverse changes (public API, dependency bumps, deletions). Keep diffs focused. State the why for non-obvious choices.
- One branch and one PR per thread. Commit small.

## Step 0 (serial, do first): the contract

Parallel threads need agreed types. One PR, small, merged before anything else:

- `span.rs`: `Span { start: usize, end: usize }` (byte offsets).
- `diag.rs`: `Diagnostic { span, message, kind }` and a `Result<T> = std::result::Result<T, Diagnostic>`. Hand-written, no error-crate dependency.
- `token.rs`: `Token { kind: TokenKind, span }`. `TokenKind` covers every keyword, operator, literal, `NEWLINE`, `INDENT`, `DEDENT`, `ENDMARKER`.
- `ast.rs`: `Module`, `Stmt`, `Expr`, `Param`, `TypeExpr`, every node carrying a `Span`. Mirrors `docs/grammar.md` one-to-one. Immutable, owned, `Box` for recursion.
- `lib.rs` re-exports; module stubs for `lexer`, `parser`, `infer`, `interp`.

Exit: compiles, no logic, reviewed by Nano.

## Parallel threads (after Step 0)

Each thread owns one module and one test directory. They meet only at the Step 0 types.

| Thread | Owns | Input to output | Develops against |
|---|---|---|---|
| A. Lexer | `lexer.rs` | source text to `Vec<Token>` | raw strings |
| B. Parser | `parser.rs` | `Vec<Token>` to `Module` | hand-built token vectors until A lands |
| C. Inference | `infer.rs` | `Module` to diagnostics and types | hand-built ASTs until B lands |
| D. Interpreter | `interp.rs` | `Module` to program output | hand-built ASTs until B lands |
| E. Conformance | `tests/`, `examples/` | corpus of programs plus differential harness | CPython as oracle |
| F. CLI and CI | `src/main.rs`, `.github/` | `pythrust run <file>`, CI pipeline | stubs |
| G. Research | `docs/` | spikes feeding phases 3 and 4 | no code dependency |

Thread notes:

Nano's typing/power decisions are authoritative in [grammar.md, Decided](grammar.md#decided); implementation remains tracked in [roadmap.md](roadmap.md#phase-2-open-implementation-work). This handoff's original state snapshot above is historical, not a current implementation inventory. At main `ed8ce138ae792b113ef1198d1b7487f8c2fd0e0c`, the baseline parser and C/D module-level fixed-type/fix-it and integer-power rules have landed. PR #11 adds index/string/list-equality spec and corpus coverage. PR #10 adds Optional module support and the confirmed explicit None-first annotation/fix-it. Optional parsing, post-terminator narrowing, conformance and CLI stage wiring remain pending; inference still conflicts with the runtime tuple-bounds rule. See the roadmap for checked versus open work.

- **A. Lexer.** Indentation tokens (spaces only; a tab is an error), inconsistent dedent errors, `#` comments, `_` in ints, string escapes, the keyword set and reserved-for-later set from the grammar. Edge cases: blank and comment-only lines, dedent to EOF, line continuation inside brackets.
- **B. Parser.** Extend type syntax for phase-2 `T | None`, coordinating the `|` token and type AST with A/C; flag grammar/contract conflicts, and do not silently introduce general unions. Recursive descent with precedence climbing, one function per grammar rule. Tuple versus parenthesized-expression disambiguation. Assignment targets validated after parsing. Errors carry spans and say what was expected.
- **C. Inference.** Implement fixed-type-per-variable, the int-to-float fix-it (`0.0`), and the literal-negative-exponent check for integer bases only. Preserve legal float-base powers. Optional is a fixed union, not arbitrary rebinding; None-first values require an explicit Optional annotation and a fix-it on error; do not relax loose-mode inference. Follow the ordered parser, post-terminator narrowing and conformance tasks in the roadmap. Hindley-Milner-style local inference over the subset. Annotations are hints. Reports type errors with spans. Produces a typed view the phase-3 backend will consume. Must not block D: the interpreter runs untyped AST.
- **D. Interpreter.** Enforce a negative integer exponent runtime error saying "use a float base"; keep `int ** int` integer-valued and `2.0 ** -1` legal. Tree-walking. Checked `i64` arithmetic (overflow is a runtime error), true versus floor division, short-circuit `and`/`or` returning operands, chained comparisons evaluating each operand once, tuple unpacking with arity errors. Treat it as the semantic reference, not the fast path.
- **E. Conformance.** Because the subset is Python-compatible, run every example under CPython and under `pythrust run`, then diff stdout. Divergences listed in the [canonical CPython-deviations list](grammar.md#deviations-from-python) are expected diffs, not bugs; add targeted fixtures for Nano's decisions. Other divergences require investigation. Keep diagnostic text/fix-it assertions in C/D tests because the differential harness does not compare stderr. Seed the corpus: fizzbuzz, fibonacci, primes, sorting, string handling, tuple swaps, nested control flow.
- **F. CLI and CI.** `pythrust run file`, `pythrust check file` (parse and infer only). GitHub Actions: `cargo fmt --check`, `clippy -D warnings`, `cargo test`. No other tooling.
- **G. Research (each produces a short doc, no code):**
  1. LLVM vs Cranelift for phase 3: compile speed, binary size, Rust integration, maintenance burden. Recommend one.
  2. Ownership inference approach for phase 4: what the compiler can infer, where Rust-style annotations must surface, how tempered and loose code meet at a boundary.
  3. Pause-marker and shared-lock syntax proposals with tradeoffs.
  4. IR design: what the backend consumes between typed AST and native code.

Merge order: Step 0, then any order, with B before integration tests, and A plus B before E and F can run end to end.

## Roadmap

Each milestone ends in something runnable.

**M0. Contract.** Step 0 merged. Exit: all threads unblocked.

**M1. Subset runs (phase 2).** A, B, D integrated: `pythrust run hello.pythrust` works for the subset. Exit: fizzbuzz and fibonacci run and match CPython output.

**M2. Subset checked (phase 2).** C integrated, E corpus at 20+ programs, F and CI green. Exit: `pythrust check` catches type errors with good spans; differential tests pass in CI.

**M3. Syntax closed (phase 1 tail).** Pause marker, shared-lock keyword, and file extension decided and written into the grammar. Exit: no open syntax questions in `docs/grammar.md`.

**M4. Native code (phase 3).** Backend chosen per G.1 and G.4. Compile the subset to a native binary. Exit: benchmark against CPython on 3 programs (a numeric loop, string building, list sorting); report the real speedup honestly, including where it is small.

**M5. Tempering and ownership (phase 4).** `temper def` enforced: required annotations, explicit error types, borrow by default with inferred ownership, exception-to-Result lowering. Exit: a tempered function rejects what a loose one accepts, with diagnostics a Python programmer can act on, and no leaked Rust borrow-checker vocabulary.

**M6. Interop and ecosystem (phase 5).** Python typed bridge (dynamic import, stubs, runtime check at the crossing), Rust binary-level interop, one tool with one manifest and one lockfile for all three dependency kinds, uncolored concurrency with channels and the shared-lock wrapper. Exit: a real script that imports a Python package, calls a Rust crate, and runs tasks concurrently.

Estimates (gut, not a plan): M0 to M3 a few weeks; M4 about a month; M5 several months; M6 open-ended.

## First actions

1. Create `CLAUDE.md` summarizing the engineering rules and the read-first list above.
2. Do Step 0 and open the PR.
3. Start threads A, B, C, D, E, F, and G as separate branches once Step 0 merges.
4. Report back with: what merged, what diverged from the grammar, and any decision you need from Nano.

Design decisions and rationale are also in the remind-me memory store, tagged `language-design` and `pythrust`.
