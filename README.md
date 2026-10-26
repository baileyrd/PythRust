# Pythrust

Python's surface, Rust's guarantees. Code starts loose (inferred, Python-feeling) and is *tempered* (hardened) one function at a time.

**Status:** phase 2 (interpreter vertical slice) wrapping up; phase 3 started: `pythrust build` compiles an int, float, string, list, tuple and control-flow subset to native code ([native backend](docs/native.md)).

**Goal:** a Python programmer can read it on day one and get real speedups without a new mental model.

- [Design sketch](docs/design.md)
- [Roadmap](docs/roadmap.md)
- [Loose syntax grammar](docs/grammar.md)
- [Claude Code handoff](docs/handoff.md)
- Research (decided 2026-10-25): [backend](docs/research/backend.md), [ownership](docs/research/ownership.md), [pause and shared syntax](docs/research/concurrency-syntax.md), [IR](docs/research/ir.md)

Layout: `crates/pythrust-compiler` (compiler, written in Rust), `crates/pythrust-rt` (runtime linked into native programs), `docs/` (design and roadmap).
