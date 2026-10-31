# Pythrust: Design Sketch

First-pass design (2026-10-24). Every item is a decision unless marked open.

## Goal

A language a Python programmer can read on day one that runs faster than Python, with Rust-style safety. It must not be a lateral move (the C-to-Java test): no new mental model required to get started.

## Core idea: tempering

Code starts **loose** (Python-feeling, inferred) and is **tempered** (hardened, more spelled out) one function at a time.

- Tempering is set per function and cascades upward to module and project. The narrower setting wins.
- Tempering never changes meaning. It only makes the compiler require more to be written explicitly (ownership, error types, pause markers).
- Loose code may call tempered code freely. Tempered code calling loose code needs a small annotation at the crossing, or the compiler refuses.
- The compiler can suggest which hot functions are worth tempering.

## Surface and types

- Python-like syntax, inferred static types, ahead-of-time compiled.
- Structural typing: duck typing in loose mode, with the compiler inferring required abilities. Tempered mode names traits explicitly in signatures. Python Protocols are the familiar on-ramp.
- Generics are specialized per type by default, with opt-in dynamic dispatch where binary size matters.
- A dynamic escape-hatch type exists from day one (needed for the Python bridge).

## Ownership and memory

- Borrow by default when passing to functions. Small values are copied.
- Ownership and lifetime syntax appears only when the compiler cannot infer it.
- Values are freed deterministically when their owner goes out of scope. No GC by default.
- Shared objects use reference counting, with weak references to break cycles.
- **Open:** loose mode may eventually need a real garbage collector.

## Error handling

- Pythonic try/except on the surface. The compiler lowers exceptions to Result-style returns underneath.
- Loose mode infers which errors can escape. Tempered mode requires declared error types in signatures.
- Explicit Result return types are also available. Loose callers use try/except on them; tempered callers unwrap with a `?`-style operator.
- Accepted costs: invisible control flow in loose mode, compiler complexity, inference ripple in loose mode.

## Concurrency

- I/O concurrency and true CPU parallelism, under one syntax family.
- Functions are uncolored: no async/sync split. The runtime parks lightweight tasks on blocking calls.
- Tempered mode requires an explicit marker at pause points (where other tasks could interleave).
- Shared data has three tiers: (1) default, move into a task or lend read-only; (2) a built-in locked wrapper, the builtin type `shared[T]` with scoped `with` access; (3) channels, where ownership moves with the message.
- Actors are not a language feature. The owner-object pattern is built from channels.

## Interop

- **Python libraries:** a typed bridge. Python modules import as a dynamic type by default. Tempering the boundary means adding type stubs. The compiler trusts a stub but inserts a cheap runtime check at the crossing so a wrong stub fails loudly. Existing type hints and `.pyi` stub packages are read directly.
- **Rust crates:** called at the binary level through a shared calling convention. The app owns a small binding crate and declares its functions in a Pythrust stub; the compiler generates the shims, and rustc checks each stub against the Rust signature at build time, so there is no dynamic boundary. Plain values cross, a `Result` error is a `raises`, and a panic is fatal ([research/rust-interop.md](research/rust-interop.md)).
- Rejected: embed-CPython-only, and full source-compatibility with all of Python.

## Compiler

- Own compiler, not a transpiler to Rust, with a native backend: Cranelift, behind a backend-neutral IR so LLVM can be added later ([research/backend.md](research/backend.md)).
- Reasons: own diagnostics (no leaked borrow-checker errors about code the user never wrote), own compile speed, own semantics, compiler-driven tempering hints.
- Written in Rust. Self-hosting is a long-term goal, attempted only once the language is stable.

## Tooling and library

- One official tool (Cargo/uv style) builds, tests, manages dependencies, and publishes.
- One manifest and one lockfile cover all three dependency kinds: Pythrust packages, Rust crates, Python packages. No manual virtual environments.
- The tool reports typed versus untyped dependencies and points at boundaries worth tempering.
- Standard library is a focused core: files, paths, JSON, dates, regex, basic networking, concurrency primitives. Everything else lives in packages.

## Name

- **Pythrust** (Python + Rust, with a hint of thrust). No programming language uses the name. A PyPI package "pythrust" and a GitHub project "PyThrust" (unrelated) exist, so published tooling may need a variant such as `pythrust-lang`.
- **Temper** is the vocabulary for the strict dial: loose is untempered, hardening a function is tempering it.

## Prior art

Mojo, Nim, Codon, TypeScript (adoption path), Swift (reference counting, error lowering), Go and Java (uncolored concurrency), Cargo and uv (tooling).

## Next steps

- Formal grammar for the loose syntax.
- Prototype parser and type inferencer for a tiny subset.
- Specify syntax for tempering annotations (ownership modes start from `read` / `mut` / `owned`, see [research/ownership.md](research/ownership.md)).
