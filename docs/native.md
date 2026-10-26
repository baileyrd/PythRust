# Native backend (phase 3, M4)

How `pythrust build` works, what it covers today, and the order the rest lands in. Decisions behind it: [research/backend.md](research/backend.md) (Cranelift, AOT) and [research/ir.md](research/ir.md) (one CFG IR, `pythrust-rt`).

## Pipeline

```
source -> lex -> parse -> infer -> lower -> ir::Program -> codegen -> object -> cc + libpythrust_rt.a -> executable
```

| Piece | Where | Does |
|---|---|---|
| IR | `pythrust-compiler/src/ir.rs` | Contract type. Basic blocks over typed, monomorphic locals; not SSA. |
| Lowering | `src/lower/` | Typed AST to IR. Monomorphizes generic functions per call-site types; desugars `for` (over `range`, strings and lists), chained compares, `and`/`or`, unpacking; interns list and tuple types; makes reference ownership explicit. |
| Codegen | `src/codegen/` | IR to an object file with Cranelift (`opt_level=speed`, PIC). Checked int arithmetic, Python floor `//` and `%` on ints, and float `+ - * /` and comparisons are emitted inline; `num.rs` holds the numeric rvalues, `seq.rs` lists and tuples. |
| Runtime | `crates/pythrust-rt` | Static library: `main`, buffered stdout, `print` formatting, `pt_fail`, refcounted strings (`str.rs`), float `repr`, `**` and parsing (`num.rs`), lists and tuples (`seq.rs`). The project's only `unsafe`, each item marked `allow(unsafe_code)` with its reason. |
| Driver | `src/main.rs` | `pythrust build f.pyr` writes `f`, linking with the system `cc`. |

The compiled binary prints what `pythrust run` prints. A runtime error's message is rendered at compile time (`path:line:col`, source line, carets) and stored in the binary, then printed by `pt_fail`, which flushes stdout and exits 1.

`pythrust build` finds `libpythrust_rt.a` next to its own executable, where `cargo build` puts it, or at `$PYTHRUST_RT`.

## Covered now

Ints (checked `i64`), floats (`f64`), bools, `None`, strings; top-level functions with positional and keyword arguments (evaluated as written, then bound to parameters) and defaults, reading module variables, recursion and generic instantiation; `if`/`elif`/`else`, `while` and `for x in range(...)` with `else`, `break`, `continue`; `+ - * / // % **`, unary `- + not`, chained comparisons, `and`/`or`, conditional expressions, `is None`; `print` with `sep` and `end`. Numbers follow Python's tower: `/`, or an int meeting a float, computes in floats, and mixed comparisons compare as floats (as `run` does). `int()` of a float or a string, `float()` of an int, bool or string, and `str()` and `print` of a float (Python's shortest `repr`). On strings: literals, `+`, `*` by an int, `s[i]` (negative too), `len`, `str()` of ints, bools and `None`, comparisons in code point order, `in` and `not in`, truthiness, `for ch in s`. Lists and tuples: displays, `xs[i]` and `xs[i] = v` (negative too), `t[k]` by a literal, `xs[i] op= v`, `append`, `len`, `str()` and `print` (Python's `repr`), `==` and lexicographic ordering, `in` on lists, truthiness, `for` over a list or a tuple (with unpacking in the target), and unpacking a tuple, list or string in assignment (a list or string of the wrong length fails as in Python). Optionals (`T | None`) of every type: passing, returning (falling off an optional function returns `None`), storing in lists and tuples, `is None` / `is not None` and the narrowing inference does, `==` / `!=`, truthiness, `and` / `or`, `str()` and `print`; a value adapts structurally where an optional is declared, so `(None, "a")` passes as a `tuple[int | None, str] | None`.

Semantics kept from `run`: a read of a variable no assignment has reached fails (`UnboundLocalError` in a function, `NameError` at module level), via a per-variable flag checked unless the read follows an assignment in the entry block or earlier in its own block. A module-level call before the function's `def` raises `NameError`; a call that could reach a function not yet defined is rejected. More than 1000 active calls is a `RecursionError` (the interpreter's `MAX_DEPTH`), counted in a module-private counter around each call. Defaults are evaluated when the `def` runs, once, so a list default is shared across calls. A function reading a module variable the module has not assigned yet raises `NameError`. An out-of-range string index is an `IndexError` and a repeat too large to allocate a `MemoryError`, as in `run`; so are float division by zero, `**` out of range, a negative `int ** int` exponent, and failed conversions, each with `run`'s message. A message with a run-time part (`could not convert string to float: 'x'`, the `2.0 ** -1` fix-it) is rendered at compile time around a marker, and the runtime fills the marker in (`pt_fail_mid`).

### Strings, lists and tuples

A string is a pointer to an immutable runtime object holding a count, its length in chars (so ASCII strings index in O(1)) and its UTF-8 text. Ownership is decided in lowering and spelled out in the IR, so codegen only translates:

- A string, list or tuple local (`Ty::is_rc`) is null or owns one reference. Operands are borrowed.
- `Retain` adds a reference, `Release` drops one and nulls the local, `Forget` nulls it without dropping (the reference moved).
- Temporaries belong to their statement (a condition counts as one) and are released where it ends. A temporary can be moved out of, which nulls it.
- Assigning a variable takes an owned value (moved from a temporary, or a retained copy), then releases the old one.
- Calls take their reference arguments owned and return an owned result. Every `return` goes through one exit block that releases the function's strings except the result.
- `for x in s` holds its own reference to `s`. Over a list it re-reads the length each time, so appending in the body extends the loop, as in Python. Binding the target is a statement of its own each iteration, so the item, the fields an unpacking reads and the list an indexed target evaluates are released before the body runs.
- `break` and `continue` release the temporaries of every statement they leave, such as a nested `for`'s snapshot when they sit in its `else`.

A list or tuple is a pointer to one runtime object: a count, a descriptor, and a vector of 64-bit slots. A slot holds an int, a float's bits, a bool or `None` as 0 or 1, or a pointer the sequence owns a reference to. The descriptor is a prefix code the compiler emits as read-only data (`i f b n s`, `l` and the element, `t`, a field-count byte and the fields, `o` and an optional's present type), which tells the runtime how to compare, print and drop slots. Runtime functions borrow their arguments and give new references, like the string ones. Static types rule out a sequence containing itself, so counts never form a cycle.

A runtime error exits without releasing; nothing else does. The runtime counts live strings and sequences, and a debug build of it fails a program that ends normally with any still live (`pythrust-rt: N objects leaked`, exit 1), so the native corpus run checks for leaks.

Everything else is a compile error, `backend error: the native backend does not support X yet`, pointing at X. Nothing is guessed. `corpus_matches_native` in `tests/conformance.rs` builds every corpus program, runs what builds against its golden, and requires the programs in `NATIVE` to build.

## Choices made inside the settled design

- **Recursion limit matches `run`** (1000 active calls), as Nano chose; it is one constant.
- **`Retain`/`Release`/`Forget` instead of ir.md's `Copy`/`Move` operands with `RcInc`/`Drop`.** The same information: a move is a `Use` then a `Forget`, a copy a `Use` then a `Retain`. Keeping operands plain kept every existing int rvalue unchanged, and M5's ownership pass can still delete the pairs it proves unnecessary.
- **Numeric semantics are copied into the runtime, not shared with the interpreter.** Float `repr`, floor division, `**` and number parsing exist in `interp` and in `pythrust-rt/src/num.rs`. Sharing them needs a crate both depend on (the runtime cannot be a library of the compiler: it defines `main`). The copies are small and the native corpus test holds them to the same output; a shared crate is a follow-up if they grow.
- **Fallible runtime calls report a status byte** through a stack slot rather than returning a pair, which keeps the C ABI to scalars and pointers.
- **Module variables functions read are mirrored, not moved.** The module's code keeps such a variable in a local, as before, and each assignment also stores it (with a bound flag) in a module-private global, which holds its own reference. Functions only read the global: they cannot assign module variables without `global`, which the subset reserves. Module-level loops therefore stay on locals. Only names some function refers to get a mirror. A default is a global of its own, stored when its `def` runs. The module's end drops what the globals own, so the leak check still sees zero.
- **Tuples are boxed**, not unboxed as planned: one runtime object like a list, so a tuple in a list slot, a tuple argument and a tuple result need no multi-value ABI. A tuple's fields are read by literal index (inference requires one), so unboxing stays an optimization M5 can make without changing the IR's types. Nano accepted this on the condition that the layout stays out of every ABI (`pythrust-rt`'s C surface and future Rust interop see no tuple representation), and asked for a tuple-heavy benchmark to measure its allocation cost.
- **List and tuple types are interned** in `Program::types`, so `ir::Ty` stays `Copy`. An empty list whose element type nothing constrains (`not []`) gets `None` slots; it never holds an element.
- **An optional is one nullable reference**, as Nano chose: null is `None`; a present string, list or tuple is that pointer; a present int, float or bool is boxed in a one-field tuple. One layout everywhere (locals, slots, arguments, globals) reuses the sequence runtime and reference counting, at the cost of an allocation per present scalar. Lowering wraps a value where an optional is expected and unwraps a read inference narrowed; a tuple display whose fields are narrower than its declared type (`(None, 4)` for `tuple[str | None, int | None]`) is rebuilt as that type.
- **`None` has no machine value.** A function returning `None` returns nothing. (ir.md: zero-sized.)
- **Monomorphization from signatures.** An instance's type variables are bound by matching the generic signature against the call's argument and result types. In generic code a call's result type may be open: the signature does not fix it (`def square(n): return n * n` has a result variable of its own), and the call's node type is in the callee's variables. Such an instance is lowered at the call, and its first `return` decides the result type in the declared shape (inference made every `return` agree; `[x * x]` from a `-> list | None` function makes it `list[int] | None`) and binds the signature's result variable, so later recursive calls resolve; the caller binds the call's own open variables from it too. A display whose item types are open (`[n * n]`) binds them from its lowered items. A statement that needs a result not yet known (a call back into an instance being lowered, directly or through another open instance, or a `return None` whose optional is still open) is skipped, and the instance is lowered again once a `return` has decided (`rec(n - 1, x) + x` before `return x * x`). An instance whose every `return` needs such a result stays a backend error at the outermost call. Inference exposing per-instance types, as ir.md anticipated, would replace this.
- **`target-lexicon` as a direct dependency** for the host triple. It is already a dependency of the accepted `cranelift-*` crates, so no new crate enters the tree. `cranelift-native` (CPU feature detection) is left out so binaries run on any machine of the same architecture.
- **Runtime lookup next to the compiler**, because Cargo cannot yet express "this binary needs that static library" (artifact dependencies are unstable). The conformance test builds `pythrust-rt` itself, with the profile and target of the `pythrust` under test, since `cargo test` does not.
- **Objects are written to a fresh owner-only temporary directory**, created exclusively and removed after linking.

## Benchmarks

M4's four benchmarks are in `bench/`, with results and analysis in [benchmarks.md](benchmarks.md). Scalar code runs about 20x faster than CPython, list sorting about 8x, string building about the same, and tuple-heavy code about 1.8x; a tuple-free control of the same work runs about 6.8x faster than the tuple version, most of it plausibly boxed tuples (see the confounders there).

## Known gaps

- **Reference ownership** is checked for leaks in CI (the debug runtime's count) but for use-after-free only by running the examples under valgrind by hand.
- **`range` outside a `for` header** (`len(range(n))`, `range(n)[i]`, `x in range(n)`, unpacking one) is a backend error.
- **Binary size** is dominated by the runtime and Rust's standard library (about 5 MB in debug builds).
- **Targets:** CI covers Linux x86-64 only. macOS and AArch64 are expected to work (Cranelift and `cc` serve them) but are untested.
- **Diagnostics:** for a native program that fails, `corpus_matches_native` requires its stderr to equal `run`'s byte for byte (message, location, carets). This is only checked on Linux x86-64 in CI.
