# Loose Syntax Grammar (phase-2 subset)

Scope: the phase-2 subset only. Functions, numbers, strings, lists, control flow. Python-compatible wherever possible, so existing Python readers need no new model. Lexing and semantics follow CPython 3.12 unless listed under [Deviations from Python](#deviations-from-python).

Notation: EBNF. `A*` zero or more, `A+` one or more, `[A]` optional, `A | B` choice, `'x'` literal token.

## Lexical

- Indentation-sensitive, as in Python. The lexer emits `NEWLINE`, `INDENT`, `DEDENT`. Spaces only: a tab anywhere outside a string or comment is a lexical error.
- Line joining: implicit inside `()` and `[]`, explicit with a trailing `\`, as in Python.
- Comments: `#` to end of line.
- Tokens: `NAME` (ASCII letters, digits, `_`; not starting with a digit), `INT`, `FLOAT`, `STRING`.
  - `INT`: decimal digits, `_` separators allowed. Ints are `i64`; arithmetic overflow is a runtime error, never a silent wrap. An `INT` literal above `i64::MAX` is a compile error, except `9223372036854775808` directly under unary `-`, which the parser folds to `i64::MIN`. Arbitrary precision may be added later as an opt-in type.
  - `FLOAT`: digits with `.` or exponent.
  - `STRING`: single or double quoted, with Python's `\` escapes except `\N{...}`. An unknown escape is an error. No string prefixes (`f`, `r`, `b`, `u`) or triple quotes yet; the lexer rejects them by name.
- Keywords: `def return if elif else while for in break continue pass and or not is True False None temper`.
- Reserved for later (rejected by the lexer so programs stay forward-compatible): `try except finally raise assert class import from as with lambda yield async await global nonlocal del pause shared`. `match` is not reserved: it is a common Python name, and Python's own `match` is a soft keyword.

## Program

```ebnf
module      = (stmt)* ENDMARKER ;
stmt        = simple_stmt NEWLINE | compound_stmt ;
block       = NEWLINE INDENT stmt+ DEDENT ;
```

## Statements

```ebnf
simple_stmt = assign | ann_assign | aug_assign | return_stmt
            | 'break' | 'continue' | 'pass' | expr_list ;

assign      = target_list '=' expr_list ;
ann_assign  = NAME ':' type '=' expr ;
aug_assign  = target ('+=' | '-=' | '*=' | '/=' | '//=' | '%=' | '**=') expr ;   # target must not be a tuple (checked after parsing)
return_stmt = 'return' [expr_list] ;
target_list = target (',' target)* [','] ;                # more than one, or a trailing comma, unpacks
target      = NAME | postfix | '(' target_list ')' ;      # postfix must end in a subscript_op (checked after parsing)

compound_stmt = func_def | if_stmt | while_stmt | for_stmt ;

func_def    = ['temper'] 'def' NAME '(' [params] ')' ['->' type] ':' block ;
params      = param (',' param)* [','] ;
param       = NAME [':' type] ['=' expr] ;

if_stmt     = 'if' expr ':' block ('elif' expr ':' block)* ['else' ':' block] ;
while_stmt  = 'while' expr ':' block ['else' ':' block] ;
for_stmt    = 'for' target_list 'in' expr ':' block ['else' ':' block] ;
```

### Notes

- `temper def f(...)` marks a function *tempered*: the compiler requires more to be written explicitly, and the meaning never changes (see [design.md](design.md#core-idea-tempering)). Only function-level tempering exists here; module- and project-level tempering come later. The keyword is a prefix, not a decorator, so the compiler sees it without resolving names.
- Annotations are optional everywhere. Loose code infers them; they are hints, never required.
- Single-line bodies (`if x: y`) are not in phase 2. One way to write a block.
- Attribute assignment (`a.b = c`) waits for classes.
- `def` appears only at module top level. Nested functions bring closure capture, which waits for ownership (phase 4).
- Phase 2 parses and records `temper`; nothing about it is enforced until M5.
- Rejected by the parser, as CPython rejects them before running: `return` outside a function, `break`/`continue` outside a loop (a loop's `else` is outside it), duplicate parameter names, a parameter without a default after one with a default, a positional argument after a keyword argument, and a repeated keyword argument.
- Parameters: defaults are evaluated once, when `def` runs, as in Python. A call binds positional arguments first, then keyword arguments by name. An unknown name, a parameter bound twice, or a parameter left unbound is an error.

## Types (annotation syntax)

```ebnf
type        = type_atom ['|' 'None'] ;
type_atom   = (NAME | 'None') ['[' type (',' type)* ']'] ;
```

Matches Python: `int`, `str`, `list[int]`, `tuple[int, str]`, `None`. Variable-length `tuple[int, ...]` is later. Names resolve in the type namespace. `T | None` (Optional) is in phase 2, including nested positions such as `list[int | None]`; general `A | B` unions remain later. Optional is one fixed union type, not permission to change a variable's type. PR #10 implements the token/AST contracts and Optional inference at module level; parsing this syntax and end-to-end CLI/conformance work remain pending. See [open implementation work](roadmap.md#phase-2-open-implementation-work).

For example, `x: int | None = None` followed by `x = 5` keeps the same `int | None` type. Nano confirmed that a None-first variable intended to hold a value requires an explicit annotation. Without it, `x = None` fixes the type as `None`, so a later `x = 5` is a type error with a fix-it suggesting `x: int | None = None`. Do not relax loose-mode Optional inference yet; revisit only after the corpus shows how often the annotation is needed.

## Expressions

Lowest to highest precedence:

```ebnf
expr_list   = expr (',' expr)* [','] ;                  # a comma makes a tuple
expr        = or_expr ['if' or_expr 'else' expr] ;     # conditional expression
or_expr     = and_expr ('or' and_expr)* ;
and_expr    = not_expr ('and' not_expr)* ;
not_expr    = 'not' not_expr | comparison ;
comparison  = sum (comp_op sum)* ;                       # chained, as in Python
comp_op     = '==' | '!=' | '<' | '<=' | '>' | '>=' | 'in' | 'not' 'in'
            | 'is' | 'is' 'not' ;                       # `is` only with a None operand (checked by inference)
sum         = term (('+' | '-') term)* ;
term        = factor (('*' | '/' | '//' | '%') factor)* ;
factor      = ('-' | '+') factor | power ;
power       = postfix ['**' factor] ;
postfix     = atom (call | subscript_op | attr)* ;
call        = '(' [args] ')' ;
args        = arg (',' arg)* [','] ;
arg         = [NAME '='] expr ;                          # keyword args by name
subscript_op = '[' expr ']' ;
attr        = '.' NAME ;
atom        = NAME | INT | FLOAT | STRING
            | 'True' | 'False' | 'None'
            | '(' expr ')'
            | '(' ')'                                    # empty tuple
            | '(' expr ',' [expr (',' expr)* [',']] ')'  # tuple: (a,) (a, b) (a, b,)
            | '[' [expr (',' expr)* [',']] ']' ;         # list literal
```

### Semantic rules

- `/` is true division (float); `//` is floor division. Same as Python.
- `and`/`or` short-circuit and return an operand, as in Python.
- Comparison chains evaluate each operand once.
- A bare `expr_list` statement is allowed (calls for effect).
- Tuples are fixed-length, immutable values. Subscript assignment on a tuple is an error. Element types are positional, so `(1, "a")` is `tuple[int, str]`.
- Unpacking arity must match. A mismatch is a compile error when both lengths are statically known, otherwise a runtime error.
- Call arguments and list elements are comma lists but never tuples: `f(1, 2)` is two arguments, `f((1, 2))` is one tuple.
- `a, b = b, a` evaluates the right side fully before assigning.
- Lists are homogeneous: every element has one type. `[1, "a"]` is a type error unless the element type is `dyn`. A variable of type `dyn`, the Python-bridge escape hatch, may hold anything; this does not implicitly turn heterogeneous lists or ordinary variables into `dyn`. The bridge remains phase 5; this decision does not claim `dyn` is implemented in phase 2.
- A variable has one fixed type, inferred once (or constrained by its annotation); assignment and augmented assignment cannot change it. See [Decided](#decided) for the no-widening diagnostic and Optional boundary.
- `int ** int` returns `int`. A negative integer exponent is a runtime error, except that a literal negative exponent with an integer base is a compile-time error. Both diagnostics say "use a float base"; `2.0 ** -1` is legal.
- A tuple subscript must be an integer literal, optionally negated (`t[-1]`), so the element type is known statically.
- Subscripts apply to lists, strings, and tuples. A negative index counts from the end: `xs[-1]` is the last element. An index out of range in either direction is a runtime error, for tuples too. Slicing is not in phase 2.
- Indexing a string and iterating over it (`for ch in s`) yield a length-1 `str`, counted in code points, as in CPython. Under UTF-8, indexing a non-ASCII string is O(n); the phase-3 backend deals with that.
- `in` with a `str` on the right is a substring test. The left operand must be a `str`; anything else is a compile error.
- List `==` and `!=` compare element-wise. Both lists must have the same element type, otherwise it is a compile error. Scalar `int == float` stays as in Python.
- An annotation that conflicts with the inferred type is a type error.

## Builtins (phase 2)

A closed set. Anything else is a name error.

- Functions: `print len range str int float bool`.
- Methods: `list.append`.
- Type names: `int float str bool list tuple None`.

## Runtime errors

Uncatchable in phase 2 (there is no `try`). A runtime error (overflow, index out of range, unpacking arity, division by zero, ...) stops the program, prints a diagnostic with its source span to stderr, and exits with status 1, as an uncaught CPython exception does.

A compile error (lexical, syntax, or type) is reported before anything runs: `pythrust run` prints the diagnostic to stderr, writes nothing to stdout, and exits with status 1.

## Deviations from Python

This is the canonical documented deviations-from-CPython list. Thread E treats these as expected differences, not bugs, and links deviation fixtures here. The [Decided](#decided) section supplies rationale; other docs should reference this list rather than maintain another one.

- Spaces only: a tab outside a string or comment is an error, not just in indentation.
- An unknown string escape (`'\q'`) is an error; CPython keeps the backslash and warns.
- `\N{name}` escapes are not supported.
- Strings hold Unicode scalar values: a surrogate escape (`'\uD800'`) is an error.
- String prefixes and triple quotes are rejected with a dedicated error.
- `NAME` is ASCII only.
- **Checked overflow:** ints are `i64`; overflow is a runtime error, including `i64::MIN // -1`, `-i64::MIN` and `2 ** 64`. An out-of-range literal is a compile error (with the unary-minus boundary exception specified above).
- **Fixed variable types:** a variable cannot be rebound to a different type. Flow-sensitive type-changing rebinding is outside the current subset; `dyn` is the explicit Python-bridge escape hatch.
- **No int-to-float variable widening:** `total = 0` followed by `total += 1.5` is a compile error, with a fix-it to initialize with `0.0`. Mixed arithmetic within an expression still promotes as in Python; the variable's type does not widen.
- **Negative integer exponents:** `int ** int` returns `int`; a negative exponent is a runtime error. With an integer base, a literal negative exponent (`x ** -1`) is a compile-time error. Both messages say "use a float base". `2.0 ** -1` stays legal.
- Lists are homogeneous; tuple subscripts are integer literals, optionally negated.
- Type errors are reported before the program runs, so a program with one prints nothing. Python raises the same mistakes as a `TypeError` at runtime, if it raises them at all.
- `1 in "abc"`: a non-`str` left operand of `in` on a string is a compile error. Python raises a `TypeError`.
- List equality needs the same element type: `[1] == [1.0]` is a type error, where Python compares element-wise and returns `True`.
- `==` and `!=` between types that can never be equal (`1 == "a"`) are type errors; `int`, `float` and `bool` compare freely.
- Functions are not values yet: a `def` name may only be called. A name cannot be both a `def` and a variable, and a `def` name is defined once.
- A function that returns a value on some path must not fall off its end (that returns `None`) unless its return annotation is `T | None`.
- `is` / `is not` compare only with `None`. In an `if`/`elif`/`while` whose test is `x is not None`, and in the clauses after an `x is None` test, a `T | None` name `x` has type `T`, unless that block assigns `x`. Currently narrowing is per block; `if x is None: return` does not yet narrow the code after it. Post-terminator narrowing is a decided follow-up, not an implemented feature (see [roadmap](roadmap.md#optional-follow-ups-in-order)).
- A variable first bound to `None` has type `None` unless annotated: `x = None` then `x = 5` is a type error; write `x: int | None = None`. `T` and `None` flow into `T | None` (also inside tuples); lists stay invariant, but a list or tuple literal takes its element types from where it goes (an annotation, argument, return or reassignment), so `xs: list[int | None] = [None, 1]` holds.
- A negative `float` raised to a fractional power is a runtime error; Python returns a `complex`, which the subset lacks.
- Mixed `int`/`float` arithmetic and comparison, and `int / int`, go through `f64`, so results can differ from Python's exact math for magnitudes above 2**53.

## Deliberate omissions (phase 2)

Dicts, sets, slicing, starred unpacking (`a, *b = ...`), multi-index subscripts (`a[1, 2]`), comprehensions, lambda, classes, imports, exceptions, `with`, decorators, bitwise operators, chained assignment (`a = b = 1`), bare declarations (`x: int` with no value), `tuple[int, ...]`, general `A | B` unions (except phase-2 `T | None`). Each is additive: existing phase-2 programs keep their meaning.

## Decided

- **Integers:** `i64` with checked overflow (runtime error, no silent wrap).
- **Fixed variable types (Nano):** one type per variable, inferred once; no rebinding to a different type. `total = 0` followed by `total += 1.5` is a compile error with a fix-it: "total is int here; write 0.0". Do not widen the variable from int to float: if the loop never runs, that changes the output from `0` to `0.0`, violating "tempering never changes meaning". Flow-sensitive type-changing rebinding such as `x = int(x)` is out of scope now; ship a clear error, then use the conformance corpus to decide whether to revisit it. A same-type assignment is unaffected.
- **Optional (Nano):** pull `T | None` into phase 2 so None-initialized values can be represented without arbitrary type-changing rebinding. Optional remains one fixed union type. **None-first annotation confirmed by Nano:** write `x: int | None = None` when a None-first variable must later hold an int; unannotated `x = None` then `x = 5` is an error with that fix-it. Do not relax loose-mode Optional inference yet; revisit after corpus evidence shows how often the annotation is needed.
- **Optional follow-ups (Nano, in order):** parser support for `'|' 'None'` and `is` / `is not`; then a separate PR for narrowing after a terminating `if x is None` branch (`return`, `break`, `continue`), with each implemented terminator tested and a fall-through negative case; then Optional conformance cases once parsing lands. These narrow an existing fixed union, not the variable's declared type. **Raise deferral confirmed by Nano:** implement and test `return`/`break`/`continue` narrowing now; defer the `raise` test until `raise` exists. `raise` remains reserved; do not expand exception syntax here.
- **Lists and dyn (Nano):** homogeneous element types; `[1, "a"]` is an error unless the element type is `dyn`. A `dyn` variable may hold anything. `dyn` is the explicit Python-bridge escape hatch, not an implicit fallback.
- **Integer powers (Nano):** `int ** int` returns `int`; negative exponents fail at runtime. A literal negative exponent with an integer base fails at compile time. Both diagnostics say "use a float base". `2.0 ** -1` remains legal. Checked overflow already makes `2 ** 64` a runtime error.
- **Tempering marker:** `temper def f(...)`, a keyword prefix.
- **Tuples:** Python syntax. Fixed-length, immutable, positionally typed. Unpacking in assignment and `for`.

- **Pause marker** (lands with concurrency): `pause <expr>`, a loosely binding prefix covering the whole expression to its right, like Swift's `await`. Required in tempered code at every pause point; it must cover at least one. See [research/concurrency-syntax.md](research/concurrency-syntax.md).
- **Shared lock** (lands with concurrency): a builtin type `shared[T]`, built with `shared(v)` and accessed through `with x as n:`, where `n` writes through. Not a keyword.

`pause`, `shared` and `with` stay reserved until those features land, so adopting them breaks no programs. `temper` is already live in `func_def`.

## Open syntax questions (M3 blockers)

1. **Source file extension.** Placeholder: `.pythrust`.
