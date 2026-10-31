# M6b: Rust binary-level interop

**Status: decided 2026-10-31** (Nano approved all five recommendations of the scope proposal, with the conditions folded in below). Not yet implemented; [roadmap.md](../roadmap.md#phase-5-m6-plan) orders the work.

A Pythrust program calls functions of a Rust *binding crate* the app owns, declared to Pythrust by a *stub* of tempered signatures. `pythrust build` generates one shim per stub function and has cargo compile the shims, the binding and the runtime into one static library; rustc thereby checks every stub against the real Rust signature. The compiler gains no dependency. A binding's crates.io dependencies are the app's, not Pythrust's.

## Decided

1. **Binding crate plus stub.** The binding is ordinary safe Rust: no `unsafe`, no macros, no Pythrust dependency beyond `pythrust_rt::abi::Raise` when it raises a named builtin. Its stub is a `.pyr` file of `temper def` signatures whose bodies are `...`, allowed only in a stub. A proc-macro export (adds `syn`/`quote`) and calling arbitrary crate functions directly (a `u128` result does not fit `int`) were rejected.
2. **`run` is native for interop programs.** The interpreter cannot call Rust: loading a library at run time needs `unsafe` in the compiler, which stays `forbid(unsafe_code)`. `run` of a program that imports a binding builds it natively and executes it, printing one line on stderr every run saying so. Like `cargo run`, this runs the binding's build scripts and fetches its dependencies: the same trust as building it.
3. **Naming a binding: `--rust name=path`** on `check`, `run` and `build`. Interim: M6d's manifest supersedes it.
4. **Driver:** fclone-lite hashing with xxh3 through a `fastfs` binding (below).
5. **Toolchain and cache.** cargo and rustc are needed only for programs that import a binding; every other program builds exactly as today. The generated crate and cargo's target directory live in `.pythrust/` beside the program (ignored by git), so a rebuild recompiles only what changed.

## How a call works

```
apps/fclone-lite/fastfs/Cargo.toml     # depends on xxhash-rust and rayon
apps/fclone-lite/fastfs/Cargo.lock     # committed; required
apps/fclone-lite/fastfs/src/lib.rs     # pub fn file_xxh3(path: &str) -> std::io::Result<String>
apps/fclone-lite/fastfs/fastfs.pyr     # temper def file_xxh3(path: str) -> str raises OSError: ...
```

The program writes `import fastfs` and calls `fastfs.file_xxh3(p)`. Imports keep M6a's rules: first in the module, no `from`, a module is not a value.

1. `check` reads the stub and checks calls against it like any tempered function. No cargo.
2. `build` writes a generated crate into `.pythrust/`: one `extern "C"` shim per stub function (`pt_rs_fastfs_file_xxh3`) that converts the arguments, calls `fastfs::file_xxh3`, and converts the result. It depends on the binding and on `pythrust-rt` (as an rlib) and is built as the one staticlib the program links in place of the plain runtime. Two Rust staticlibs in one executable would each carry std, so there is exactly one.
3. **Locked builds.** The binding's committed `Cargo.lock` is copied into `.pythrust/` and cargo runs with `--locked`. A binding without a `Cargo.lock` is a named error ("binding `fastfs` has no Cargo.lock; run `cargo generate-lockfile` in it and commit the file").
4. rustc compiles the shims against the real signatures, so a stub saying `bytes` where Rust takes `&str` fails the build. Pythrust reports "`fastfs.pyr:3`: the stub for `file_xxh3` does not match the Rust function" and shows rustc's output after it, as Rust's output, not as a Pythrust diagnostic (the vocabulary test keeps scanning Pythrust's own strings).
5. Codegen calls a shim like any fallible runtime function: scalars and pointers, and the status-byte convention for `raises`, so the C ABI is unchanged.

## Unsafe

The compiler stays `forbid(unsafe_code)`. The generated shims are the only new `unsafe`: `#[unsafe(no_mangle)]` and the pointer reads at the boundary, all from one fixed template in the compiler, audited like `pythrust-rt`'s C surface. Conversions live in a new safe module, `pythrust-rt::abi`, so the template is a few lines per function. A test generates the shims for the probe binding's stub and checks that the source holds only the template's expected `unsafe` items, so the template cannot grow new ones silently. CLAUDE.md's exception names the generated binding shims.

## Types across the boundary

Only plain values cross, and every parameter is `read`: Rust borrows for the call or gets a copy, and cannot keep a Pythrust value, since a borrowed `&str` or `&[u8]` cannot outlive the call.

| Pythrust | Rust parameter | Rust result | Notes |
| --- | --- | --- | --- |
| `int` | `i64` | `i64`; `u64`, `usize`, `i32`, `u32` | A result outside `i64` is the fatal integer overflow, as `2 ** 64` is |
| `float` | `f64` | `f64` | |
| `bool` | `bool` | `bool` | |
| `None` | (none) | `()` | |
| `str` | `&str` | `String`, `&'static str` | No copy in; a fresh string out |
| `bytes` | `&[u8]` | `Vec<u8>` | No copy in |
| `list[T]`, `T` one of the above | `Vec<T>` | `Vec<T>` | A copy each way |

**Threads.** Pythrust values never reach Rust threads. Every argument is converted on the calling thread before the binding runs: a list becomes a `Vec` of plain Rust values (`String`, not a runtime string), and a `&str` or `&[u8]` borrow ends when the call returns, so a binding that spawns threads (rayon) sees only Rust-owned data. Results are converted back on the calling thread. The runtime's refcounts therefore stay single-threaded until M6c. The probe binding includes a function that spawns threads over a copied list, tested on every CI platform and under memcheck.

**Errors.** A result of `Result<T, E>` raises when it is `Err`, and the stub must say `raises`, as any tempered function must. `E = std::io::Error` raises the `OSError` subclass its errno names, with CPython's `str`, reusing M6a's mapping. `E = pythrust_rt::abi::Raise` (an exception kind and a message) raises a named builtin such as `ValueError`. The shim checks the raised class against the stub's `raises`; an undeclared one stops the program with a fatal error naming the stub, the cheap check at the crossing that [design.md](../design.md#interop) calls for.

**Panics.** Each shim catches a panic, which would otherwise abort across `extern "C"`, and stops the program as a fault does: the panic's message, exit 1. A panic is never catchable, like overflow.

**Ownership.** Every result is a new value owned by the caller, so `own` and `rc` treat a shim call as a runtime constructor. Tuples stay out of the ABI, as Nano required for the boxed layout ([native.md](../native.md)).

## Oracle

CPython cannot import a Rust binding, so an interop program has no CPython oracle. Its tests compare its output with a pure-Pythrust program doing the same work; for the driver, `fclone_lite.py`. Interop is a Pythrust extension, listed as such in [grammar.md](../grammar.md#deviations-from-python), not a deviation of shared behavior.

## Driver: fast fclone-lite

M6a left native fclone-lite hash-bound (see the roadmap's M6a hashing entry). rusty_fclone's edge is xxh3, partial hashing and rayon, which a binding can call. The driver is `apps/fclone-lite/fclone_fast.pyr`: the same walk and grouping in Pythrust, hashing through `fastfs`.

1. `fastfs.file_xxh3(path: str) -> str raises OSError`: xxh3-128 of the file streamed in Rust, as 32 hex digits. It exercises `str` in and out and `io::Error` to `OSError`.
2. `fastfs.group_xxh3(paths: list[str]) -> list[str]`: the same for a whole size group, hashed in parallel with rayon, one digest per path. A file that fails to read gives `""`; a comment on the stub's signature says so. The driver re-hashes each `""` entry with `file_xxh3`, so its `OSError` surfaces exactly as in the serial driver. It exercises `list[str]` both ways and the thread rule above.

Done means: output identical to `fclone_lite.py`'s on the M6a trees and on a tree holding an unreadable file (an xxh3 collision is ignored, as rusty_fclone does); benchmark rows for native fclone-lite, `fclone_fast` with each function, CPython and rusty_fclone; and xxhash-rust and rayon justified in that PR as the app's dependencies. No speedup is claimed before the benchmark runs.

## Not in this cut

Each waits for an app that needs it: Rust objects held by Pythrust (a hasher with `update`), callbacks from Rust into Pythrust, `dict`, `set` and tuples crossing, `mut` parameters (`&mut Vec`), generic functions, naming crates.io crates directly in `import`, and the manifest (M6d).

## Risks

- **Build time.** A first build compiles the runtime and the binding with cargo: likely tens of seconds, then incremental. Measured in the native PR.
- **Runtime source.** The generated crate needs `pythrust-rt` as source. Inside the repo that is a path dependency; an installed `pythrust` must ship it, which is M6d's problem.
- **Windows.** The combined staticlib must link with MSVC as today's runtime does; the existing Windows job covers it.
