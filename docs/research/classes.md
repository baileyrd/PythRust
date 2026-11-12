# Post-M6 step 5: mutable classes

**Status: decided 2026-11-12.** Nano chose the first cut on 2026-11-03 (gap list, decisions 4 and 5); the expert settled the rest as [default] rows D144-D153. Nano answered Q22 on 2026-11-12 (D154): `from __future__ import annotations`, in a card after 5-2. Code follows in the [PR order](#pr-order) below.

## Objective

Records (step 4) are frozen: fields are read, never stored. Step 5 adds the mutable objects Python programs are built from: a plain `class` with `__init__`, fields and methods, and the non-frozen `@dataclass`. Every accepted program stays a CPython 3.13 program with the same output (the oracle); each refused form is a named error or a listed deviation.

## Nano's first cut (2026-11-03)

Recorded from the gap list's "Decided" line and remind-me (paraphrases, so the rows cite them as sources rather than as `[stated]` rows):

- Fields, methods and `__init__`. No inheritance, except from builtin exception classes (step 4-3). Static dispatch. Protocols come later (design.md).
- A method that changes `self` needs `mut self` in tempered code and has it inferred in loose code, as D2's parameters do.
- Dunders other than `__init__` and the printable form are named errors. 5-6 then admits exactly `__enter__` and `__exit__`.
- The non-frozen `@dataclass` becomes legal (a named error since 4-1, D38).
- Cycles: `build` refuses a class type that can reach itself through a mutable field, and `run` accepts it. This is a named gap with an allow-listed fixture and a stale-gap test. A cycle collector limited to those types gates "classes done".

## Decided

### Declaring a class (D144, D145)

```python
class Node:
    def __init__(self, value: int):
        self.value = value
        self.next: Node | None = None

    def push(self, value: int):
        n = Node(value)
        n.next = self.next
        self.next = n
```

- `class P:` with no base and no decorator is a mutable class. It is defined at the top level only, imported, and protected from rebinding as a record is (D136).
- Its body holds methods, an optional leading docstring and `pass`.
- Its fields are exactly the names `__init__` stores as `self.x = v` or `self.x: T = v`. Each field has one fixed type (D3), inferred from every store in the program.
- `__init__` annotations are not evaluated by CPython, so `self.next: Node | None` may name the class itself.
- `__init__` is optional. Without it, `P()` takes no arguments and the class has no fields.
- `P(...)` binds `__init__`'s parameters after `self` as a function call does.
- Every field is assigned on every path out of `__init__`. Until then, `self` only stores fields and reads fields assigned so far. So every object a program can see has all its fields: a field read never fails, and a native object has no unset state.

Named errors:
- a class-body annotation (`x: int`);
- a class attribute (`k = 3`, D132's message);
- a store to a name that is not a field (`p.extra = 1`);
- a field not assigned on every path;
- `self` used before every field is assigned;
- calling `__init__` directly;
- a generic class.

### Non-frozen dataclasses (D146)

- Bare `@dataclass` is the one spelling.
- Its body rules are a record's (D132), and so are its `__init__`, `==` and printing (D133, D134).
- Printing a cycle shows `...` (CPython's recursion guard).
- Fields may be stored through any name.
- It is unhashable, as in CPython.
- It cannot hold its own class: field defaults are refused, and its eager annotations cannot name it. So a self-referential type is a plain class (D148).

### Operations and dunders (D147)

- A plain class may define `__repr__` and `__str__`, each returning `str`, with CPython's fallback between them. No other dunder is accepted, except `__enter__`/`__exit__` in 5-6.
- `==` between two objects of one plain class is identity, as in CPython.
- Printing a plain object with neither dunder is a named error, a listed deviation, because CPython prints an address.
- An object as a dict key or set element is a named error, a listed deviation.
- `is`, ordering, truthiness, `isinstance` and `type` follow records (D134).
- A method or a class is not a value (step 6).

### None-first fields (D148)

D4 applies to a field as it does to a variable. An unannotated `self.next = None` that later receives a `Node` is D4's error, with the fix-it `self.next: Node | None = None`.

### Methods and `self` (D149)

- Methods are as on records (D131): top-level `P.m`, `self` first and unannotated, static dispatch.
- A method that changes `self` (a field store, or a change reaching through a field or a call) needs `mut self`. One that keeps `self` needs `owned self`.
- Both modes are written in tempered code and inferred in loose code. The missing-mode error is D2's.
- Frozen records keep D131 unchanged.

### Ownership (D150)

- An object is a mutable heap container whatever its fields. A field store is a container store: it changes the object and keeps the stored value, as `append` does.
- Grammar D3, D4 and A5, and tasks rule 4, cover objects as they cover dicts and sets. Their fix-its are prose, since objects have no copy.
- In the summaries (A8), an object is one abstract container whose contents are its fields' values. An object reached through a field is its own container (A6 choice 1).
- Tasks marking walks fields, and a store into a marked object marks what it stores.

### Cycles and native code (D151, D152)

- A class is **cycle-capable** when its type graph reaches itself through field types, list elements, dict values, tuple and record parts, or optionals. This includes mutual recursion. Records alone form a DAG (D129), so every such cycle passes through a mutable class.
- Until 5-5, `build` fails at the first construction of a cycle-capable class with a named gap, extending `codegen::may_cycle`. `check` and `run` accept the class. Each such fixture is in `NATIVE_GAPS`, and the stale-gap check fails once it builds.
- A method annotation naming its own class stays D129's "not defined yet" error, as CPython raises `NameError`. So tempered methods cannot take their own class until D154's card (#1454) lands.
- The collector (5-5) is synchronous Bacon-Rajan trial deletion, over cycle-capable values only:
  - A value is buffered as a candidate when its count drops to nonzero.
  - Collection runs past a threshold and at exit.
  - It runs only where counts are exact (no elided reference live, no `Slot` borrow held), never during an executor block.
  - The details go back to the expert in 5-5's PR.

### User context managers (D153)

- `__enter__(self)` and CPython's four-parameter `__exit__(self, exc_type, exc, tb)`. `exc_type` and `tb` are only compared with `None`; `exc` is `Exception | None`.
- `__exit__` runs on every way out of the block, and a `True` result suppresses the exception.
- `raises` counts the block's escapes plus those of `__enter__` and `__exit__`.
- A tempered `__exit__`, or a `with` on an object in tempered code, is "not supported yet".

## Decided after the note

- **D154 (Q22):** a module whose first statement is `from __future__ import annotations` has lazy annotations, so methods may name their own or a later class. It gets its own card (#1454) after 5-2. Until then D129's error stays.

## PR order

One PR each (cards under step 5):

1. **Design note (#1260):** this note, the grammar entries, the roadmap items, D144-D154.
2. **Front end and `run` (#1262):** plain classes and `@dataclass` through `parse`, `infer` and `interp` (D144-D149), with `build` refusing classes as a named gap. Exit: CPython-checked corpus programs (fields, methods, `__repr__`/`__str__`, a None-first field, a module importing a class); each named error has a fixture beside an accepted neighbour.
3. **Future annotations (#1454):** D154. Recognize `from __future__ import annotations` as a module's first statement and make that module's annotations lazy. Exit: CPython-checked corpus programs with a method taking and returning its own class, and a fixture for the error without the import.
4. **Ownership for objects (#1264):** D150 in `own` and the tasks rules. Exit: fixtures for D3, D4, A5 and rule 4 through fields, each with an accepted neighbour, plus inferred `mut self`/`owned self` and the tempered missing-mode error.
5. **Native classes (#1266):** objects lower to the runtime, and `build` refuses only cycle-capable classes (D151). Exit: the 5-2 corpus in `NATIVE` (valgrind-clean), and cycle-capable fixtures in `NATIVE_GAPS` with the stale-gap check.
6. **Cycle collector (#1268):** D152, with the details agreed with the expert in the PR. Exit: the cycle-capable fixtures build and run memcheck-clean, `NATIVE_GAPS` drops them, and the benchmarks are reported as tables. Classes are done.
7. **User context managers (#1270):** D153. Exit: CPython-checked corpus programs for `with` on an object (normal exit, an exception, suppression, `raises`), with the named errors beside accepted neighbours.

## Deviations

These are listed in [grammar.md](../grammar.md#deviations-from-python) as planned now, and get fixtures as each lands:
- a store to a non-field;
- a field not assigned on every path;
- printing a plain object with neither `__repr__` nor `__str__`;
- an object as a dict key or set element;
- the class-body and dunder errors.
