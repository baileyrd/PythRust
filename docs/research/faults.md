# D7 revisit: can a `try` catch a fault?

**Status: proposal, for Nano's decision** (requested 2026-10-30, after M5 closed with D7 as is). No code. Numbers come from the corpus at `main` after #58 and from two native micro-benchmarks, both described under [Method](#method).

**Recommendation: option 3, the subset, kept out of `raises`.** `ZeroDivisionError` and `IndexError` become catchable at `except` sites. They stay out of `raises` sets, and lowering tracks them with a "may fault" fact of its own. Overflow, recursion depth, unpacking arity, `MemoryError` and unbound names stay fatal. Do the inline pending-flag change described below first, whichever option is chosen.

## Today (D7)

A fault is a bug: overflow, index out of range, division by zero, unpacking arity, recursion depth, a failed conversion, an unbound name. It stops the program with `run`'s diagnostic. No handler sees it, `except Exception` included (fixtures `fault_not_caught`, `fault_not_caught_by_exception`). Only `raise` and `int(s)` / `float(s)` of a malformed string raise catchable exceptions.

In native code a fault is a check whose failing branch calls the runtime's `pt_fail`, which never returns. The program exits without releasing anything (`Term::Fail`, failing rvalues). An exception is the runtime's *pending* exception. After each call whose `raises` set is not empty, and after `int(s)` / `float(s)`, the caller tests `Rvalue::Raised` and branches to its `try`'s dispatch, or releases its references and returns ([native.md](../native.md#exceptions)). So the `raises` sets decide where the tests go, and a call to a function that cannot raise costs nothing.

That last point is the whole question. A catchable fault must reach the handler without unwinding, since ir.md rules out landing pads. So every frame between the fault and the `try` must test after its call and return, and that frame's callee may have an empty `raises` set.

## The corpus, in numbers

Over the 108 corpus programs that lower (1227 function instances, `main` included, 549 calls):

| | count |
|---|---|
| Calls tested today (a `raises` set or a parse builtin) | 61 |
| Calls whose callee may fault, transitively, any fault | 375 (68%) |
| ... counting `RecursionError`, which any call can raise | 549 (100%) |
| Calls whose callee may raise `ZeroDivisionError` or `IndexError` | 98 (18%) |
| Functions that may fault, any fault | 826 (67%) |
| Functions that may raise `ZeroDivisionError` or `IndexError` | 276 (22%) |
| Fault sites: overflow checks / indexing / division / `Term::Fail` | 434 / 225 / 16 / 342 |

Most `Term::Fail`s are unbound-name checks: 180 `NameError` and 113 `UnboundLocalError`. D7's list does not name these, and `exc::ExcKind` has no class for them.

## What a test costs

`pt_raised` is an out-of-line runtime call that takes the pending exception's mutex. Over-declaring `raises ValueError` on a function makes every call to it tested, which measures the price directly (release build, median of 9 runs):

| program | no test | test after every call | per test |
|---|---|---|---|
| `fib(32)`, about 7M calls | 15.9 ms | 87.1 ms (5.5x) | about 10 ns |
| `bench/numeric.pyr`, 1M calls in a hot loop | 489 ms | 516 ms (+5%) | about 27 ns |

The pending exception is per process, so the flag behind `pt_raised` can be a plain static byte that codegen reads inline: one load and one branch, with no call and no lock. That change stands on its own. It speeds every program that raises today, and it makes the per-call cost of options 2 and 3 small. It should land first, whichever option wins.

A fault site already has its check and its cold failing branch, so making the fault catchable adds nothing to the path that doesn't fail. The failing branch changes target. Instead of `pt_fail`, it builds the exception from the message it would have printed, makes it pending, and goes to the dispatch or the unwind exit, releasing the statement's temporaries on the way as a `raise` does. The cost is code size for those exits. A fault then releases instead of exiting with references live, so memcheck and the leak counter cover fault paths too.

## The options

### 1. Faults stay fatal (D7 as is)

- **`raises`:** unchanged. A set lists exactly what can be caught, so a tempered function's declaration is complete.
- **Native cost:** none. 61 tested calls.
- **Deviation:** stays. `except Exception` around `1 / 0` or `xs[10]` never runs, which is the most visible difference from Python and the first thing an EAFP habit (`try: xs[i] except IndexError:`) hits.

### 2. Every fault catchable at `except` sites, never in `raises`

- **`raises`:** unchanged in syntax, but it no longer lists everything a caller can catch. It lists what the code raises, while faults escape implicitly, like a panic next to a `Result` in Rust. Listing faults is not an option: with 67% of functions able to fault, every tempered signature would carry them. The lowering needs its own fact, "this call may return with a fault pending", computed bottom-up over the call graph from the fault sites. That is the untracked error channel, and it is separate from `infer::Types::raises`.
- **Native cost:** a test after every call, because any call can hit the recursion limit: 549 tests instead of 61. With today's `pt_raised` that is up to 5.5x on call-heavy code. With the inline flag it is a load and a branch per call. One possible refinement is to keep fatal any fault in a function that no `try` can reach (no `try` in it or any caller). That is sound, but it gives each function two behaviors depending on the whole program, so it is not proposed.
- **Deviation:** gone for the D7 faults. Two new ones appear instead. Catching `OverflowError` from `i64` arithmetic is a Pythrust-only behavior, since CPython's ints don't overflow. And `RecursionError` caught at depth 1000 leaves the handler running near the limit. Unbound names would still be fatal, or would need `NameError` and `UnboundLocalError` classes, which is a separate decision.

### 3. A subset catchable: `ZeroDivisionError` and `IndexError`; overflow and recursion stay fatal

- **`raises`:** as in option 2, kept out of `raises`. Tracking them in `raises` would make 22% of functions declare `raises IndexError` or `ZeroDivisionError` (every tempered function that indexes or divides). The "may fault" fact is narrower: only division and indexing sites, and calls into functions that reach one.
- **Native cost:** 98 tested calls (18%) instead of 61. Each test is a load and a branch with the inline flag. Recursion stays fatal, so a call does not need a test merely for being a call.
- **Deviation:** the two visible cases match CPython: `except ZeroDivisionError` / `except ArithmeticError` / `except Exception` around `1 / 0`, and `except IndexError` / `except LookupError` around `xs[10]`. Three remain, recorded in grammar.md's deviations:
  - Overflow is fatal. It is already a deviation of its own, since CPython has no int overflow.
  - Recursion depth is fatal.
  - Unpacking arity is fatal although it is a `ValueError`. This is the one I would ask about separately: `except ValueError` already catches `int(s)`, so a handler that catches one `ValueError` and not another is surprising. It could join the subset at the cost of the unpacking sites.
  - One wrinkle: `except ArithmeticError` would catch `1 / 0` but not an overflow, though `OverflowError` derives from it. A fatal class inside a catchable base is legal under B4, since a handler never sees a fault, but grammar.md should say so.

## What stays true under every option

- A `raises` clause changes no behavior, and removing `temper` or a clause leaves a passing program's output unchanged (`tempered_matches_loose`).
- The interpreter is still the reference. Its faults are already `Err(Diagnostic)` values carrying the class name in the message, so catching them in `run` is a local change at the `try`.
- The ownership summaries are unaffected. A caught fault's exception holds only its message string.
- No unwinding and no landing pads (ir.md).

## Method

- Corpus counts: every `examples/*.pyr` that lowers, counted over the unelided IR. Fault sites are checked int arithmetic and negation, `**`, float `/ // %` and `**`, `Index`, `SetItem`, `Repeat`, `FloatToInt`, and each `Term::Fail` by its message's class. "May fault" is closed over the call graph. Division by a nonzero literal other than -1 is not counted.
- Benchmarks: the release `pythrust` and runtime, `fib(32)` and `bench/numeric.pyr`, each built with and without `raises ValueError` on the called function, so the only difference is the `pt_raised` test after each call. Same output; median wall time of 9 runs on the session's Linux x86-64 container.
