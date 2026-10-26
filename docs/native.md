# Native backend (phase 3, M4)

How `pythrust build` works, what it covers today, and the order the rest lands in. Decisions behind it: [research/backend.md](research/backend.md) (Cranelift, AOT) and [research/ir.md](research/ir.md) (one CFG IR, `pythrust-rt`).

## Pipeline

```
source -> lex -> parse -> infer -> lower -> ir::Program -> codegen -> object -> cc + libpythrust_rt.a -> executable
```

| Piece | Where | Does |
|---|---|---|
| IR | `pythrust-compiler/src/ir.rs` | Contract type. Basic blocks over typed, monomorphic locals; not SSA. |
| Lowering | `src/lower/` | Typed AST to IR. Monomorphizes generic functions per call-site types; desugars `for` (over `range` and strings), chained compares, `and`/`or`, tuple-display assignment; makes string ownership explicit. |
| Codegen | `src/codegen.rs` | IR to an object file with Cranelift (`opt_level=speed`, PIC). Checked arithmetic and Python floor `//` and `%` are emitted inline. |
| Runtime | `crates/pythrust-rt` | Static library: `main`, buffered stdout, `print` formatting, `pt_fail`, refcounted strings (`str.rs`). The project's only `unsafe`, each item marked `allow(unsafe_code)` with its reason. |
| Driver | `src/main.rs` | `pythrust build f.pyr` writes `f`, linking with the system `cc`. |

The compiled binary prints what `pythrust run` prints. A runtime error's message is rendered at compile time (`path:line:col`, source line, carets) and stored in the binary, then printed by `pt_fail`, which flushes stdout and exits 1.

`pythrust build` finds `libpythrust_rt.a` next to its own executable, where `cargo build` puts it, or at `$PYTHRUST_RT`.

## Covered now

Ints (checked `i64`), bools, `None`, strings; top-level functions with positional arguments, recursion and generic instantiation; `if`/`elif`/`else`, `while` and `for x in range(...)` with `else`, `break`, `continue`; `+ - * // %`, unary `- + not`, chained comparisons, `and`/`or`, conditional expressions, `is None`; `print` with positional arguments. On strings: literals, `+`, `*` by an int, `s[i]` (negative too), `len`, `str()` of ints, bools and `None`, comparisons in code point order, `in` and `not in`, truthiness, `for ch in s`.

Semantics kept from `run`: a read of a variable no assignment has reached fails (`UnboundLocalError` in a function, `NameError` at module level), via a per-variable flag checked unless the read follows an assignment in the entry block or earlier in its own block. A module-level call before the function's `def` raises `NameError`; a call that could reach a function not yet defined is rejected. More than 1000 active calls is a `RecursionError` (the interpreter's `MAX_DEPTH`), counted in a module-private counter around each call. A `def` with defaults is rejected, since defaults run at definition time. An out-of-range string index is an `IndexError` and a repeat too large to allocate a `MemoryError`, as in `run`.

### Strings

A string is a pointer to an immutable runtime object holding a count, its length in chars (so ASCII strings index in O(1)) and its UTF-8 text. Ownership is decided in lowering and spelled out in the IR, so codegen only translates:

- A `Str` local is null or owns one reference. Operands are borrowed.
- `Retain` adds a reference, `Release` drops one and nulls the local, `Forget` nulls it without dropping (the reference moved).
- Temporaries belong to their statement (a condition counts as one) and are released where it ends. A temporary can be moved out of, which nulls it.
- Assigning a variable takes an owned value (moved from a temporary, or a retained copy), then releases the old one.
- Calls take their string arguments owned and return an owned result. Every `return` goes through one exit block that releases the function's strings except the result.
- `for ch in s` iterates an owned snapshot of `s`.
- `break` and `continue` release the temporaries of every statement they leave, such as a nested `for`'s snapshot when they sit in its `else`.

A runtime error exits without releasing; nothing else does. The runtime counts live strings, and a debug build of it fails a program that ends normally with any still live (`pythrust-rt: N strings leaked`, exit 1), so the native corpus run checks for leaks.

Everything else is a compile error, `backend error: the native backend does not support X yet`, pointing at X. Nothing is guessed. `corpus_matches_native` in `tests/conformance.rs` builds every corpus program, runs what builds against its golden, and requires the programs in `NATIVE` to build.

## Choices made inside the settled design

- **Recursion limit matches `run`** (1000 active calls), as Nano chose; it is one constant.
- **`Retain`/`Release`/`Forget` instead of ir.md's `Copy`/`Move` operands with `RcInc`/`Drop`.** The same information: a move is a `Use` then a `Forget`, a copy a `Use` then a `Retain`. Keeping operands plain kept every existing int rvalue unchanged, and M5's ownership pass can still delete the pairs it proves unnecessary.
- **`None` has no machine value.** A function returning `None` returns nothing. (ir.md: zero-sized.)
- **Monomorphization from signatures.** An instance's type variables are bound by matching the generic signature against the call's argument and result types. A generic body whose inner types are not determined by its signature is a backend error for now; the fix is for inference to expose per-instance types, which ir.md anticipated.
- **`target-lexicon` as a direct dependency** for the host triple. It is already a dependency of the accepted `cranelift-*` crates, so no new crate enters the tree. `cranelift-native` (CPU feature detection) is left out so binaries run on any machine of the same architecture.
- **Runtime lookup next to the compiler**, because Cargo cannot yet express "this binary needs that static library" (artifact dependencies are unstable). The conformance test builds `pythrust-rt` itself, with the profile and target of the `pythrust` under test, since `cargo test` does not.
- **Objects are written to a fresh owner-only temporary directory**, created exclusively and removed after linking.

## Next, in order (one small PR each)

1. **Floats.** `f64`, `/`, `**` (int and float), Python's shortest `repr` in the runtime, int/float mixing.
2. **Lists and tuples.** Refcounted shared lists (loose aliasing), unboxed tuples, indexing with bounds checks, `for` over lists, `append`, `len`, unpacking from any tuple.
3. **The rest of the subset.** Globals read from functions, default and keyword arguments, `T | None`, `print(sep=, end=)`.
4. **Benchmarks** against CPython: a numeric loop, string building, list sorting; numbers reported honestly, including where the gain is small.

## Known gaps

- **String ownership** is checked for leaks in CI (the debug runtime's count) but for use-after-free only by running the examples under valgrind by hand.
- **Binary size** is dominated by the runtime and Rust's standard library (about 5 MB in debug builds).
- **Targets:** CI covers Linux x86-64 only. macOS and AArch64 are expected to work (Cranelift and `cc` serve them) but are untested.
- **Diagnostics:** for a native program that fails, `corpus_matches_native` requires its stderr to equal `run`'s byte for byte (message, location, carets). This is only checked on Linux x86-64 in CI.
