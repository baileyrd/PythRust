# Research G.1: LLVM vs Cranelift (phase 3)

**Status: proposal.** Nothing here is decided. The backend stays an open decision until Nano picks.

**Recommendation: Cranelift for M4**, behind a backend-neutral IR ([ir.md](ir.md)), with LLVM kept as a possible later release backend if M4 benchmarks show code quality is the bottleneck.

## What M4 needs

M4 compiles the phase-2 subset to a native binary and benchmarks it against CPython on a numeric loop, string building, and list sorting. That means: integer and float arithmetic with checked `i64` overflow, branches and loops, calls, and calls into a small runtime (strings, lists, `print`, refcounting). No vectorization, no exotic targets, no debugger story yet.

## Comparison

| | Cranelift | LLVM (via `inkwell` or `llvm-sys`) |
|---|---|---|
| Compile speed | Built for fast codegen (Wasmtime, rustc's debug backend). Several times faster than LLVM at comparable settings. | Slow at `-O2`/`-O3`; still the slower of the two at `-O0`. |
| Code quality | Good, not great. Roughly LLVM `-O1`. E-graph mid-end (GVN, constant folding, LICM). Little or no auto-vectorization; limited inlining. | Best available. Inlining, vectorization, LTO, PGO. |
| Rust integration | Pure Rust crates from cargo (`cranelift-codegen`, `-frontend`, `-module`, `-object`). Builds with `cargo build`, nothing else to install. | Needs a matching system LLVM (hundreds of MB). `inkwell` pins LLVM versions with feature flags; CI must install LLVM. |
| Compiler binary size | Adds a few MB. | Adds tens of MB if linked statically. |
| Output binary size | Similar for small programs; the runtime dominates. | Smaller on large programs with `-Os` and LTO. |
| Targets | x86-64, AArch64, RISC-V 64, s390x. | Everything, including Wasm. |
| Debug info | Partial DWARF. | Mature DWARF. |
| Checked overflow | Overflow-reporting add/sub/mul ops plus traps (confirm on the version we pin). | `llvm.sadd.with.overflow` and friends. |
| Maintenance | One cargo dependency to bump. API changes between releases are real but local. | LLVM major release every ~6 months; wrapper crates lag; build breakage across machines is common. |
| `forbid(unsafe_code)` | AOT via `cranelift-object` plus the system linker needs no `unsafe` in our crate. JIT (`cranelift-jit`) would: calling a generated function pointer is `unsafe`. | `inkwell` exposes mostly safe APIs; JIT has the same `unsafe` call problem. |

## Why Cranelift

1. **Speed comes from elsewhere first.** The M4 win over CPython is unboxed `i64`, no dispatch loop, and static types. That gap is far larger than the Cranelift-to-LLVM gap. String building and sorting run mostly inside the runtime, which rustc compiles with LLVM anyway.
2. **Compile speed is a stated goal** (design.md, "own compile speed").
3. **Zero-friction build.** `cargo build` on any contributor machine and in CI, no LLVM install, no version matrix. This matches "minimal dependencies" better than LLVM does, even though it is still a dependency.
4. **Reversible.** With a backend-neutral IR, adding LLVM later is a second lowering, not a rewrite. rustc itself ships both.

## Costs we accept

- Numeric hot loops may run slower than an LLVM build would. M4 must report this honestly, per the roadmap.
- We inline small functions ourselves in the IR if benchmarks need it.
- Partial debug info until it matters.

## What Nano is deciding

- The backend for M4: Cranelift (recommended) or LLVM.
- Accepting the project's first dependencies: the four `cranelift-*` crates (plus their transitive `target-lexicon` and friends). Justification goes in the M4 PR.
- AOT only for M4 (object file plus system `cc` link), which keeps `unsafe` out of the compiler crate. See the runtime question in [ir.md](ir.md#runtime).

## Spike before committing (1 to 2 days)

Hand-lower `fib(30)` and a summing loop to Cranelift IR, emit an object, link it with a stub runtime, and record: codegen time, binary size, runtime vs CPython. If the numbers contradict this doc, the doc changes.
