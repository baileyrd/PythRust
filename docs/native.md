# Native backend (phase 3, M4)

How `pythrust build` works, what it covers today, and the order the rest lands in. Decisions behind it: [research/backend.md](research/backend.md) (Cranelift, AOT) and [research/ir.md](research/ir.md) (one CFG IR, `pythrust-rt`).

## Pipeline

```
source -> lex -> parse -> infer -> lower -> ir::Program -> codegen -> object -> cc + libpythrust_rt.a -> executable
```

| Piece | Where | Does |
|---|---|---|
| IR | `pythrust-compiler/src/ir.rs` | Contract type. Basic blocks over typed, monomorphic locals; not SSA. |
| Lowering | `src/lower/` | Typed AST to IR. Monomorphizes generic functions per call-site types; desugars `for`/`range`, chained compares, `and`/`or`, tuple-display assignment. |
| Codegen | `src/codegen.rs` | IR to an object file with Cranelift (`opt_level=speed`, PIC). Checked arithmetic and Python floor `//` and `%` are emitted inline. |
| Runtime | `crates/pythrust-rt` | Static library: `main`, buffered stdout, `print` formatting, `pt_fail`. The project's only `unsafe`, each item marked `allow(unsafe_code)` with its reason. |
| Driver | `src/main.rs` | `pythrust build f.pythrust` writes `f`, linking with the system `cc`. |

The compiled binary prints what `pythrust run` prints. A runtime error's message is rendered at compile time (`path:line:col`, source line, carets) and stored in the binary, then printed by `pt_fail`, which flushes stdout and exits 1.

`pythrust build` finds `libpythrust_rt.a` next to its own executable, where `cargo build` puts it, or at `$PYTHRUST_RT`.

## Covered now

Ints (checked `i64`), bools, `None`; top-level functions with positional arguments, recursion and generic instantiation; `if`/`elif`/`else`, `while` and `for x in range(...)` with `else`, `break`, `continue`; `+ - * // %`, unary `- + not`, chained comparisons, `and`/`or`, conditional expressions, `is None`; `print` with positional arguments.

Semantics kept from `run`: a read of a variable no assignment has reached fails (`UnboundLocalError` in a function, `NameError` at module level), via a per-variable flag checked unless the read follows an assignment in the entry block or earlier in its own block. A module-level call before the function's `def` raises `NameError`; a call that could reach a function not yet defined is rejected. More than 1000 active calls is a `RecursionError` (the interpreter's `MAX_DEPTH`), counted in a module-private counter around each call. A `def` with defaults is rejected, since defaults run at definition time.

Everything else is a compile error, `backend error: the native backend does not support X yet`, pointing at X. Nothing is guessed. `corpus_matches_native` in `tests/conformance.rs` builds every corpus program, runs what builds against its golden, and requires the programs in `NATIVE` to build.

## Choices made inside the settled design

- **Recursion limit matches `run`** (1000 active calls), pending Nano's call on the PR; it is one constant.
- **`None` has no machine value.** A function returning `None` returns nothing. (ir.md: zero-sized.)
- **Monomorphization from signatures.** An instance's type variables are bound by matching the generic signature against the call's argument and result types. A generic body whose inner types are not determined by its signature is a backend error for now; the fix is for inference to expose per-instance types, which ir.md anticipated.
- **`target-lexicon` as a direct dependency** for the host triple. It is already a dependency of the accepted `cranelift-*` crates, so no new crate enters the tree. `cranelift-native` (CPU feature detection) is left out so binaries run on any machine of the same architecture.
- **Runtime lookup next to the compiler**, because Cargo cannot yet express "this binary needs that static library" (artifact dependencies are unstable). The conformance test builds `pythrust-rt` itself, with the profile and target of the `pythrust` under test, since `cargo test` does not.
- **Objects are written to a fresh owner-only temporary directory**, created exclusively and removed after linking.

## Next, in order (one small PR each)

1. **Strings.** Immutable refcounted heap strings in the runtime; literals as static data; `print`, `+`, `*`, `str()`, `len`, comparisons, `in`. Brings in `Copy`/`Move`, `RcInc` and `Drop` from ir.md.
2. **Floats.** `f64`, `/`, `**` (int and float), Python's shortest `repr` in the runtime, int/float mixing.
3. **Lists and tuples.** Refcounted shared lists (loose aliasing), unboxed tuples, indexing with bounds checks, `for` over lists, `append`, `len`, unpacking from any tuple.
4. **The rest of the subset.** Globals read from functions, default and keyword arguments, `T | None`, `print(sep=, end=)`.
5. **Benchmarks** against CPython: a numeric loop, string building, list sorting; numbers reported honestly, including where the gain is small.

## Known gaps

- **Binary size** is dominated by the runtime and Rust's standard library (about 5 MB in debug builds).
- **Targets:** CI covers Linux x86-64 only. macOS and AArch64 are expected to work (Cranelift and `cc` serve them) but are untested.
- **Diagnostics:** the conformance harness compares stdout and exit status, not stderr, so native error text and spans are not yet checked against `run`.
