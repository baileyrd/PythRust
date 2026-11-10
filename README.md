# Pythrust

Python's surface, Rust's guarantees. Code starts loose (inferred, Python-feeling) and is *tempered* (hardened) one function at a time.

**Status:** M0–M3 (contract, interpreter/checker and syntax), the native backend (M4) and tempering/ownership (M5) are implemented. M6a–M6d deliver the fclone-lite stdlib, Rust bindings, scoped tasks with `pause`, and the manifest/lockfile tool; the Python bridge (M6e) waits for a second app, while `shared[T]` and channels remain parked. Post-M6 step 1's string methods and builtins are implemented; later roadmap work remains planned ([roadmap](docs/roadmap.md), [handoff](docs/handoff.md#state)). Native coverage and named backend gaps are in [native.md](docs/native.md); CPython differences have one [canonical list](docs/grammar.md#deviations-from-python).

`pythrust check file.pyr` checks without executing; `pythrust run file.pyr` interprets; `pythrust build file.pyr` emits a native executable. Ordinary native builds need the runtime archive beside the compiler or at `PYTHRUST_RT`; Rust-binding builds instead compile extracted embedded runtime source ([runtime setup](docs/native.md#runtime-resolution)). Exceptions use per-thread pending state and explicit branches, rather than a second return value.

**Goal:** a Python programmer can read it on day one and get real speedups without a new mental model.

- [Design sketch](docs/design.md)
- [Roadmap](docs/roadmap.md)
- [Loose syntax grammar](docs/grammar.md)
- [Claude Code handoff](docs/handoff.md)
- Research (decided 2026-10-25): [backend](docs/research/backend.md), [ownership](docs/research/ownership.md), [pause and shared syntax](docs/research/concurrency-syntax.md), [IR](docs/research/ir.md); M6b (decided 2026-10-31): [Rust interop](docs/research/rust-interop.md)

Layout: `crates/pythrust-compiler` (compiler, written in Rust), `crates/pythrust-rt` (runtime linked into native programs), `docs/` (design and roadmap).
