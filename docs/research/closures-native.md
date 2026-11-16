# Step 6-3c: native closures

**Status: approved by project-expert (D207, D208); parked until 6-3b-2 merges (Nano, 2026-11-16: "queue it after").** Amends D197. Nano asked for 6-3c on 2026-11-16; project-expert read that as "now, ahead of 6-3b-2" (a reading, not a confirmed instruction). Cells (6-3b-2) only add a box to a captured slot, so nothing here is redone when they land.

## Objective

`build` refuses every closure value today: the gap "closures that escape" (`closure_values`, `closure_levels`). 6-3c makes a closure that outlives its creating frame (returned, stored in a list, dict, tuple or field, passed) build and run as `run` does, leaving no gap for the 6-3b-1 corpus. No new CPython deviation.

## What stays

- The parser's lift (D201): a hidden def takes its captures as leading `read`/`mut` parameters, and `<closure>(def, captures...)` is the value (D205). Capture is by value, equal to CPython under D204.
- `Callable[[A], R]` carries no captures, modes or `raises` (D189, D191); a call through a value raises `Exception`; `own/values.rs` still joins one dispatcher per signature.
- D193 errors (compare, print, hash), D196/D203 task errors, and the `a closure that changes its parameter` rule stay front-end errors.

## Proposed

### 1. One representation for every function value (amends D197)

D197's bare code word cannot hold captures, and a `list[Callable]` may mix `double` with a closure, so a function value is one heap object for all: a runtime sequence whose slot 0 is the address of a **thunk** (a plain word, never counted) and whose remaining slots are the captured values, each typed as any slot is. A non-capturing def (`f = double`) is the same object with one slot. The runtime ABI needs one change (below); nothing else in `pythrust-rt` does.

Why not a code/env pair of words: containers and records hold one slot per value, so a pair would need descriptor and walker changes in `seq`, `table` and `gc` for every container (the reason D197 stayed a word). Why not a bare word for non-capturing defs: two representations behind one type would need a tag test at each call. The cost is one allocation per `f = double`, which `rc` does not elide (a value read from a variable is borrowed as any heap value).

### 2. Thunks and calls

For each hidden def (and each top-level def whose address is taken) codegen emits one thunk `thunk(env, args...)` that loads the captured slots from `env`, borrowed, and calls the def with them first. `CallValue(f, args)` becomes `call_indirect` of slot 0 with `f` as the first argument, followed by the usual pending-exception check. The thunk adds one call, the price of one signature per `Callable` type regardless of captures. Captures are never `owned` (D201), so the thunk never moves a slot out of the env; `mut` captures are the shared heap value itself.

### 3. IR

`Rvalue::FuncRef(def)` becomes `Rvalue::MakeClosure(def, captures)` (empty `captures` for a plain def): an owned new reference whose captured heap values each gain a count, which is "storing an alias" for `own`, D3 and D4 (D206(a)). `CallValue` borrows its callee and takes no new rule. Lowering gives the gap at `unsupported(span, "closures that escape")` its first real code path; `own` treats creating a closure as storing each heap capture into a fresh object, and a call through a value as possibly changing any heap value some closure of that signature captured (D201), conservative per signature, as D206(a), through `own/values.rs`'s dispatcher. A `mut` capture changed through the closure while the caller iterates the same list raises as any `mut` call does (D4, the `Slot` borrow); a fixture covers it.

### 4. Descriptor and runtime (the one change)

A slot holding a function value gets descriptor `F`. The element descriptor cannot describe each env's layout, so each env carries its own descriptor (a tuple's: `t`, a word slot `i` for the thunk, then the capture types), and `F` tells the runtime to drop, mark for tasks (D150) and trace for the collector through the object's own descriptor instead of the slot's. Equality and printing never reach it (D193). `F` is one dispatch arm with span 1, in `seq.rs`'s slot dispatch (including clone and copy), `table.rs` (dict values) and `gc.rs`'s traversal. Tests run under Miri whenever `gc` changes (D176; CI's command also covers the `seq` hold tests). Any need for `unsafe` in `seq` or `table` outside the audited surface is stop-and-ask, since it would reverse D27. D197's "the runtime does not change" is gone, so the debug-runtime size is measured against Q20's `MAX_NATIVE_BYTES`; the audited ABI list gains no function. D27 holds: the runtime reads the thunk word as an int and never calls it.

### 5. Counts and cycles

The env owns one count per captured heap value and drops them with itself. A closure stored in a list that the closure captures is a cycle (`log.append(f)` inside `f`'s own scope): D202 already treats a type containing `Callable` as possibly cyclic, so a descriptor holding `F` carries the `&` back-reference and an env is a collector candidate iff some captured value's type is cycle-capable (D152's predicate), so an env capturing only a `list[int]` is not. `rc::elide` is off for `Callable` types (D175(2)): a function value was an uncounted word and is now counted, and the benchmark includes that cost. Self-reference as a value stays the 6-3b-2 error.

### 6. Tempered code and tasks

Tempered functions take and return `Callable` already (6-2c); with an owned object they need `owned` when they keep or return one, as for any heap parameter (A8), checked by the existing mode rules. A task still cannot receive a closure (D196, D203); tasks over captures are 6-3d.

## PR plan

One PR (D202 keeps 6-3c separate; it touches `lower`, `codegen` and runtime):

1. `MakeClosure`, thunks, the uniform object, descriptor `F` with its runtime tests (Miri), `rc` and `own` for the new rvalue.
2. Corpus: `closure_values` and `closure_levels` move from `NATIVE_GAPS` to `NATIVE` (valgrind-clean); `fn_values`, `lambda_value` and `fn_value_record_field` close too, since containers of function values need no walker change beyond `F`; new fixtures for a closure returned from a function and called after its frame, one stored in a dict and a record field, a capture of a heap value mutated through the closure, `in`, `index`, `count` and `remove` on a `list[Callable]` as front-end errors (D193), a `mut` capture changed while the caller iterates its list, and a closure in a cycle collected by `gc` (`log: list[Callable]`, `log.append(f)` for an `f` that captures `log`). A benchmark table of `f = double` called in a loop and of `Callable`-typed locals (the D175(2) counting cost), before and after (no headline speedup).
3. Docs: `docs/native.md`, grammar's closure entry, roadmap 6-3c ticked, D197 amended by the expert.

## Answered

- Allocation on `f = double` is accepted; a static immortal object is reopened only if the benchmark shows a visible regression (D208a).
- All container gaps close in this PR (D208b). "Closure-free generic instantiation across modules" (D197(2)) stays a named gap unless a corpus program needs it (D207e).
