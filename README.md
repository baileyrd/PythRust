# Pythrust

Python's surface, Rust's guarantees. Code starts loose (inferred, Python-feeling) and is *tempered* (hardened) one function at a time.

**Status:** design phase. No compiler yet.

**Goal:** a Python programmer can read it on day one and get real speedups without a new mental model.

- [Design sketch](docs/design.md)
- [Roadmap](docs/roadmap.md)
- [Loose syntax grammar](docs/grammar.md)
- [Claude Code handoff](docs/handoff.md)
- Research (decided 2026-10-25): [backend](docs/research/backend.md), [ownership](docs/research/ownership.md), [pause and shared syntax](docs/research/concurrency-syntax.md), [IR](docs/research/ir.md)

Layout: `crates/pythrust-compiler` (compiler, written in Rust), `docs/` (design and roadmap).
