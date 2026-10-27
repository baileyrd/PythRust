# Research G.2: Ownership inference (phase 4)

**Status: decided 2026-10-25** (direction and vocabulary; Nano accepted the recommendations). M5 syntax and scope were decided 2026-10-27; see [grammar.md](../grammar.md#phase-4-m5-tempering-ownership-exceptions-nano-2026-10-27).

**Recommendation:** loose code keeps Python's aliasing meaning, implemented as reference counting that the compiler elides wherever it can prove a single owner. Ownership is inferred per function as a *summary* of how each parameter is used. Tempered code must have every value resolve to a single owner or an explicit borrow; anything that would need a hidden refcount is either spelled out or rejected.

## The constraint that shapes everything

Python lists alias:

```python
a = [1]
b = a
b.append(2)
print(a)   # [1, 2]
```

"Tempering never changes meaning" and the CPython differential tests (thread E) both require this output. So lists cannot be Rust-style values that move or copy on assignment. Mutable value semantics (Hylo, Swift arrays) is rejected for this reason: it would change output on ordinary Python code.

Immutable values (`int`, `float`, `bool`, `None`, `str`, tuples) can be copied or shared freely, because aliasing is unobservable without `is` (reserved, not in the subset).

## Model

- **Loose:** every mutable heap value (`list` today, objects later) is refcounted. The compiler removes refcount operations where it proves a value has one owner (Lobster- and Perceus-style elision). Correct by default, fast where provable.
- **Tempered:** the same program, but the compiler must prove a single owner or a scoped borrow for every value. Where it cannot, it asks the user to write the sharing down. Meaning is unchanged; only what must be written grows.

## What the compiler infers

Per function, one **parameter mode** for each parameter, from the body:

| Mode | Inferred when the body... | Caller effect |
|---|---|---|
| `read` | only reads it | shared borrow, no refcount |
| `mut` | mutates it in place (`xs.append`, `xs[i] = v`) | exclusive borrow |
| `owned` | returns it, stores it in another value, or keeps it past the call | move, or refcount increment if the caller still uses it |

Plus, per function, whether the return value **aliases** a parameter (`def first(xs): return xs`).

Summaries are computed bottom-up over the call graph; mutual recursion is solved as a fixpoint over each strongly connected component. Inside a body: last-use analysis turns copies into moves, and a non-lexical borrow check runs over the IR's control-flow graph ([ir.md](ir.md)).

The phase-2 subset needs **no lifetime annotations**: there are no classes, closures, or stored references, and every return is an owned value or an alias tracked by the summary.

## Where annotations must surface (tempered only)

1. **Mutable aliasing** the compiler cannot rule out: `f(xs, xs)` where one parameter is `mut`. Loose: refcount and Python behaviour. Tempered: error, or an explicit copy at the call (no share marker in phase 4, per D4).
2. **A shared mutable value kept in two places**: `b = a` followed by mutation through either while both are live. ~~Tempered asks for an explicit share marker on `b = a`.~~ Superseded for phase 4 by grammar.md D4: tempered rejects it and the fix-it is `b = list(a)`; no share marker.
3. **Returning an alias of a parameter**: ~~the signature must say so (a mode on the return type), so callers do not depend on an inferred fact that can change.~~ Superseded for phase 4 by grammar.md D3: tempered code may not return a parameter's alias (fix-it `return list(xs)`); no return mode.
4. **Later phases:** closures capturing mutable state, borrowed values stored in objects, Rust crate calls.

Vocabulary: `read`, `mut`, `owned` as parameter modifiers (Mojo uses the same three words). No lifetimes, no `&`, no `'a`. Syntax decided 2026-10-27 (grammar.md D2): contextual prefixes, `read` implicit.

## Loose and tempered at a boundary

- **Loose calls tempered:** free. The tempered signature is the contract. The loose caller adapts: it borrows from a refcounted value when the mode is `read` or `mut`, and increments when it is `owned`. If a loose caller passes the same list to two parameters and one is `mut`, the call site fails to compile even in loose code, with a message about the call, not the callee. This is a narrow exception to design.md's "loose code may call tempered code freely" (see question 2).
- **Tempered calls loose:** the loose callee's summary is inferred and can change when its body changes (the "inference ripple" from design.md). The crossing annotation pins it: the tempered caller declares the modes it expects, the compiler checks the callee's inferred summary against that, and a later change fails at the crossing instead of somewhere deep in the tempered code. **Superseded for phase 4 by grammar.md D5:** tempered code calls only tempered functions and builtins; the crossing annotation is deferred to module tempering or the Python bridge.

## Diagnostics without borrow-checker vocabulary

M5 exits on "no leaked Rust borrow-checker vocabulary". Diagnostics speak in Python terms: what is changed, by whom, and while what else is looking at it.

> `xs` is changed by `grow(xs, ...)` while `total(xs)` is still reading it. Pass a copy (`list(xs)`), or mark `grow`'s first parameter `mut` and call it once.

No "borrow", "lifetime", or "move out of" in user-facing text.

## Prior art

Swift (ARC with `borrowing`/`consuming` and the law of exclusivity, checked statically with a dynamic fallback), Mojo (`read`/`mut`/`owned` argument conventions), Lobster (compile-time refcount elision), Koka's Perceus (precise refcounting with reuse), Rust NLL (CFG-based borrow checking).

## Decided

1. Loose code keeps Python aliasing via refcounting, with compile-time elision.
2. A loose call that passes one value to two parameters of a tempered function, one of them `mut`, is a compile error, not a silent copy.
3. `read` / `mut` / `owned` is the starting vocabulary for M5 syntax.
