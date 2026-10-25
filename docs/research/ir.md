# Research G.4: IR design

**Status: decided 2026-10-25** (Nano accepted the recommendations). Type sketches are illustrative, not code to land.

**Recommendation:** one mid-level IR between the typed AST and the backend: a control-flow graph of basic blocks over typed, monomorphic locals, MIR-style, not in SSA form. Build it in M4. It is the single place for desugaring, checked arithmetic, refcount insertion, and later borrow checking and exception lowering.

## Pipeline

```
source -> lexer -> parser -> ast::Module -> infer -> (AST + types by Span) -> lower -> IR -> backend -> object -> cc link -> binary
```

The tree-walking interpreter (thread D) keeps running the AST and stays the semantic reference. The IR is only for compiling.

## Why one IR, and why now

| Option | Verdict |
|---|---|
| Typed AST straight to Cranelift | Smallest for M4, but desugaring (`for` over `range` and lists, chained compares, short-circuit, unpacking) lands in backend code and must be redone for borrow checking in M5 and for any second backend. |
| **One CFG IR** | Two real consumers by M5: the backend lowering and the ownership checker (non-lexical borrow checking needs a CFG). Backend translation becomes mechanical. |
| HIR plus MIR plus LIR | Speculative. Add a layer when a second real consumer appears. |

Not SSA: locals are mutable slots. Cranelift's `cranelift-frontend` (`Variable`) and LLVM's `mem2reg` build SSA for us, and a borrow checker reads places more easily than phi nodes.

## Shape

```text
Program   { funcs: Vec<Func>, strings: Vec<String> }
Func      { name, params: Vec<LocalId>, ret: Ty, locals: Vec<Local>, blocks: Vec<Block>, span }
Local     { ty: Ty, span }                       // named or temporary
Block     { insts: Vec<Inst>, term: Term }
Inst      { kind: Assign(Place, Rvalue) | Call { dest: Place, callee, args: Vec<Operand> } | RcInc(Place) | Drop(Place), span }
Term      Goto(BlockId) | Branch(Operand, BlockId, BlockId) | Return(Operand) | Fail(RuntimeError, span)
Place     { local: LocalId, proj: Vec<Index(Operand) | Field(u32)> }
Operand   Copy(Place) | Move(Place) | Const(Const)
Rvalue    Use(Operand) | Checked(BinOp, Operand, Operand) | FloatOp(..) | Unary(..) | Cmp(CmpOp, ..) | Tuple(Vec<Operand>) | List(Vec<Operand>)
Ty        Int | Float | Bool | None | Str | List(Box<Ty>) | Tuple(Vec<Ty>)
```

Key choices:

- **Monomorphic.** Inferred generic functions are specialized per type during lowering (design.md: "specialized per type by default"). The backend never sees a type variable.
- **Checked arithmetic is explicit.** `Checked(Add, a, b)` lowers to an overflow-reporting op plus a branch to `Fail(Overflow, span)`. Division by zero, `i64::MIN // -1`, index bounds, and unpacking arity are explicit checks with `Fail` too.
- **No unwinding.** Runtime errors are `Fail`: print the diagnostic with its span and exit 1, matching grammar.md. In M5, exceptions lower to Result-style returns: a call returns a tag plus value and the IR branches on the tag. No landing pads in either backend, ever.
- **Spans everywhere.** Every `Inst` and `Fail` keeps its span, so runtime errors print source locations from a table in the binary.
- **Moves and refcounts are visible.** `Move` vs `Copy`, `RcInc`, and `Drop` are IR instructions. M4 inserts them conservatively; M5's ownership pass deletes the ones it proves unnecessary ([ownership.md](ownership.md)).

## Value representation

| Type | Representation |
|---|---|
| `int` | `i64` in a register |
| `float` | `f64` |
| `bool` | `i8` |
| `None` | zero-sized |
| tuple | unboxed fields, by value (immutable, so aliasing is unobservable) |
| `str` | pointer to an immutable, refcounted heap string |
| `list` | pointer to a refcounted heap buffer (shared, because Python lists alias) |

## Runtime

Strings, lists, `print`, `len`, `str(...)`, refcounting, and `Fail` reporting live in a small runtime library that compiled programs link against. Writing it in Rust is the natural choice, but exporting functions to generated code needs `extern "C"` and `#[unsafe(no_mangle)]`, which `forbid(unsafe_code)` disallows. That makes the runtime **a second crate with a narrow, audited `unsafe` FFI surface**, which is a decision for Nano. The alternative, a runtime written in Pythrust IR or C, is more work and moves the `unsafe` elsewhere.

## Contract notes for thread C

- Lowering looks up each expression's type by `Span` (the agreed keying). Two type-bearing nodes must never share a span. A parenthesized `(x)` should return the inner node, not wrap it. Worth a test in thread C.
- Lowering needs, per function, the inferred signature and, per generic function, the set of instantiations. The typed view should expose both.

## Decided

1. The CFG IR is built in M4.
2. The runtime is a second crate, `pythrust-rt`, with a small, audited `unsafe` FFI surface. The compiler crate keeps `forbid(unsafe_code)`.
