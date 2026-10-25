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
type        = (NAME | 'None') ['[' type (',' type)* ']'] ;
```

Matches Python: `int`, `str`, `list[int]`, `tuple[int, str]`, `None`. Variable-length `tuple[int, ...]` is later. Names resolve in the type namespace. `A | B` unions and `T | None` are a later addition.

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
- Lists are homogeneous: every element has one type.
- A tuple subscript must be an integer literal, so the element type is known statically.
- An annotation that conflicts with the inferred type is a type error.

## Builtins (phase 2)

A closed set. Anything else is a name error.

- Functions: `print len range str int float bool`.
- Methods: `list.append`.
- Type names: `int float str bool list tuple None`.

## Runtime errors

Uncatchable in phase 2 (there is no `try`). A runtime error (overflow, index out of range, unpacking arity, division by zero, ...) stops the program, prints a diagnostic with its source span to stderr, and exits with status 1, as an uncaught CPython exception does.

## Deviations from Python

- Spaces only: a tab outside a string or comment is an error, not just in indentation.
- An unknown string escape (`'\q'`) is an error; CPython keeps the backslash and warns.
- `\N{name}` escapes are not supported.
- Strings hold Unicode scalar values: a surrogate escape (`'\uD800'`) is an error.
- String prefixes and triple quotes are rejected with a dedicated error.
- `NAME` is ASCII only.
- Ints are `i64`: overflow is a runtime error, including `i64::MIN // -1` and `-i64::MIN`. An out-of-range literal is a compile error.
- Lists are homogeneous; tuple subscripts are integer literals.
- A variable has one type in its scope. `int` never widens to `float` on assignment: `total = 0` then `total += 0.5` is a type error (write `0.0`). Mixed arithmetic in an expression still promotes, as in Python.
- `int ** int` is `int`. A negative exponent is a compile error when it is a literal (`2 ** -1`) and a runtime error otherwise; the fix is a float base (`2.0 ** -1`). Python returns a `float`.
- `==` and `!=` between types that can never be equal (`1 == "a"`) are type errors; `int`, `float` and `bool` compare freely.
- Functions are not values yet: a `def` name may only be called. A name cannot be both a `def` and a variable, and a `def` name is defined once.
- A function that returns a value on some path must not fall off its end (that returns `None`) unless its return annotation is `T | None`.
- `is` / `is not` compare only with `None`. In an `if`/`elif`/`while` whose test is `x is not None`, and in the clauses after an `x is None` test, a `T | None` name `x` has type `T`, unless that block assigns `x`. Narrowing is per block; `if x is None: return` does not narrow the code after it.
- A variable first bound to `None` has type `None` unless annotated: `x = None` then `x = 5` is a type error; write `x: int | None = None`. `T` and `None` flow into `T | None` (also inside tuples); lists stay invariant.
- A negative `float` raised to a fractional power is a runtime error; Python returns a `complex`, which the subset lacks.
- Mixed `int`/`float` arithmetic and comparison, and `int / int`, go through `f64`, so results can differ from Python's exact math for magnitudes above 2**53.

## Deliberate omissions (phase 2)

Dicts, sets, slicing, starred unpacking (`a, *b = ...`), multi-index subscripts (`a[1, 2]`), comprehensions, lambda, classes, imports, exceptions, `with`, decorators, bitwise operators, chained assignment (`a = b = 1`), bare declarations (`x: int` with no value), `tuple[int, ...]`, `A | B` unions. Each is additive: existing phase-2 programs keep their meaning.

## Decided

- **Integers:** `i64` with checked overflow (runtime error, no silent wrap).
- **Tempering marker:** `temper def f(...)`, a keyword prefix.
- **Tuples:** Python syntax. Fixed-length, immutable, positionally typed. Unpacking in assignment and `for`.

- **Pause marker** (lands with concurrency): `pause <expr>`, a loosely binding prefix covering the whole expression to its right, like Swift's `await`. Required in tempered code at every pause point; it must cover at least one. See [research/concurrency-syntax.md](research/concurrency-syntax.md).
- **Shared lock** (lands with concurrency): a builtin type `shared[T]`, built with `shared(v)` and accessed through `with x as n:`, where `n` writes through. Not a keyword.

`pause`, `shared` and `with` stay reserved until those features land, so adopting them breaks no programs. `temper` is already live in `func_def`.

## Open syntax questions (M3 blockers)

1. **Source file extension.** Placeholder: `.pythrust`.
