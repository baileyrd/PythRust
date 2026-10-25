# Research G.3: Pause marker and shared-lock syntax

**Status: decided 2026-10-25.** Nano accepted both recommendations (B and B). `pause`, `shared` and `with` stay reserved until the features land, so phase-2 programs cannot use them as names.

## Pause marker

**What it marks.** A pause point is a call where other tasks may interleave: I/O, a channel receive, a lock acquire, or any function that transitively reaches one. Functions are uncolored, so the compiler infers which calls can pause. Loose code needs no marker. Tempered code must mark every pause point. The marker changes nothing at runtime; it is documentation the compiler checks.

### Options

| | Syntax | For | Against |
|---|---|---|---|
| A | `pause fetch(x)`, covering one call | Precise. Reads as English. Already reserved. | Noisy in I/O-heavy code: `pause a(pause b())`. |
| B | `pause` covering the expression to its right, Swift-style: `pause total(fetch(a), fetch(b))` | One marker per statement in practice. Swift's `await` and `try` work this way at scale. | Less precise: the reader sees the statement pauses, not which call. |
| C | `await f(x)` | Familiar keyword, already reserved. | Python users read `await` as "this is an `async def`", which is the coloring we removed. |
| D | Postfix sigil: `fetch(x)!` | Terse. | Un-Pythonic, easy to miss, and crowds the planned `?` unwrap operator. |

**Recommendation: B.** It marks the statement a reader needs to slow down for, without one marker per call. Rules:

- `pause` takes an expression and must cover at least one pause point; a `pause` with none is an error ("nothing here can pause"), so markers do not rot.
- Grammar sketch (not proposed for `grammar.md` until decided): `expr = 'pause' expr | or_expr ['if' or_expr 'else' expr]`. Like Python's `await`, it is a prefix keyword, but it binds loosely, covering the whole expression.
- It composes with the shared lock: holding a `shared` lock across a pause point is an error in tempered code and a warning in loose code (latency and deadlock hazard).

## Shared-lock wrapper

**What it is.** Tier 2 of the three sharing tiers in design.md: a value that several tasks may read and write, guarded by a lock. Tier 1 (move or lend read-only) and tier 3 (channels) need no new syntax.

The access syntax matters more than the keyword. Implicit per-access locking (`counter += 1` locks for that statement) is rejected: it invites check-then-act races (`if c > 0: c -= 1`), the exact bug the lock exists to prevent.

### Options

| | Create | Access | For | Against |
|---|---|---|---|---|
| A | `counter = shared 0` (keyword) | `with counter as n: n += 1` | Matches design.md's "one short keyword". | A keyword where a type would do; new expression grammar. |
| B | `counter = shared(0)`, type `shared[int]` (builtin type, no keyword) | `with counter as n: n += 1` | No grammar change; reads like `list(...)` and `list[int]`. The compiler still treats it specially, as it does `list`. | Departs from "one short keyword"; `shared` moves from reserved word to builtin name. |
| C | `counter = Lock(0)` | `with counter.lock() as n: n.value += 1` | Closest to Python's `threading`. | Verbose; `.value` everywhere. |

**Recommendation: B.** The lock is a container type, and Python spells containers as lowercase builtin types. It needs no new grammar beyond `with`.

Two points for Nano either way:

- **`with` leaves the reserved list when the shared lock lands.** Both A and B need `with ... as name:` for scoped access. It is reserved today, so adding it is additive.
- **`n` writes through.** Inside the block, `n` is an exclusive (`mut`) view of the locked value, so `n += 1` updates the shared value. In Python, rebinding an `as` name is local. Here it is not, which is a deliberate deviation that would go under "Deviations from Python". The alternative, C's `.value`, avoids the deviation at the cost of noise.

## Decided

1. Pause marker: B, an expression-covering `pause <expr>` that must cover at least one pause point.
2. Shared lock: B, a builtin type `shared[T]` built with `shared(v)`, not a keyword.
3. Inside `with x as n:` on a shared value, `n` writes through. Listed as a deviation from Python when it lands.
