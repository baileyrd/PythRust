# Pythrust

Python's surface, Rust's guarantees. Code starts loose (inferred, Python-feeling) and is *tempered* (hardened) one function at a time.

**Status:** phase 2 (interpreter vertical slice) wrapping up; phase 3's first milestone (M4) is done: `pythrust build` compiles the subset to native code, and every corpus program that type-checks builds and matches `run` ([native backend](docs/native.md), [benchmarks](docs/benchmarks.md)). Phase 4's milestone (M5) is done too: `temper def` enforces annotations, parameter modes and declared `raises`, with Python-phrased diagnostics, and exceptions lower to native tag-plus-branch returns ([roadmap](docs/roadmap.md#phase-4-m5-plan)).

**Goal:** a Python programmer can read it on day one and get real speedups without a new mental model.

- [Design sketch](docs/design.md)
- [Roadmap](docs/roadmap.md)
- [Loose syntax grammar](docs/grammar.md)
- [Claude Code handoff](docs/handoff.md)
- Research (decided 2026-10-25): [backend](docs/research/backend.md), [ownership](docs/research/ownership.md), [pause and shared syntax](docs/research/concurrency-syntax.md), [IR](docs/research/ir.md)

Layout: `crates/pythrust-compiler` (compiler, written in Rust), `crates/pythrust-rt` (runtime linked into native programs), `docs/` (design and roadmap).
