# Pythrust

Python's surface, Rust's guarantees. Code starts loose (inferred, Python-feeling) and is *tempered* (hardened) one function at a time.

**Status:** design phase. No compiler yet.

**Goal:** a Python programmer can read it on day one and get real speedups without a new mental model.

- [Design sketch](docs/design.md)
- [Roadmap](docs/roadmap.md)

Layout: `crates/pythrust-compiler` (compiler, written in Rust), `docs/` (design and roadmap).
