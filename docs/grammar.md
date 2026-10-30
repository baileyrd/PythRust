# Loose Syntax Grammar (phase-2 subset)

Scope: the phase-2 subset only. Functions, numbers, strings, lists, control flow. Python-compatible wherever possible, so existing Python readers need no new model. Lexing and semantics follow CPython 3.13, the conformance oracle, unless listed under [Deviations from Python](#deviations-from-python).

Notation: EBNF. `A*` zero or more, `A+` one or more, `[A]` optional, `A | B` choice, `'x'` literal token.

## Lexical

- Indentation-sensitive, as in Python. The lexer emits `NEWLINE`, `INDENT`, `DEDENT`. Spaces only: a tab anywhere outside a string or comment is a lexical error.
- Line joining: implicit inside `()` and `[]`, explicit with a trailing `\`, as in Python.
- Comments: `#` to end of line.
- Tokens: `NAME` (ASCII letters, digits, `_`; not starting with a digit), `INT`, `FLOAT`, `STRING`.
  - `INT`: decimal digits, `_` separators allowed. Ints are `i64`; arithmetic overflow is a runtime error, never a silent wrap. An `INT` literal above `i64::MAX` is a compile error, except `9223372036854775808` directly under unary `-`, which the parser folds to `i64::MIN`. Arbitrary precision may be added later as an opt-in type.
  - `FLOAT`: digits with `.` or exponent.
  - `STRING`: single or double quoted, with Python's `\` escapes except `\N{...}`. An unknown escape is an error. No string prefixes (`f`, `r`, `b`, `u`) or triple quotes yet; the lexer rejects them by name.
- Keywords: `def return if elif else while for in break continue pass and or not is True False None temper try except raise as` `as` is allowed only in a handler (`except Name as e`, `except (A, B) as e`); anywhere else it is an error saying it is reserved, until `with` lands.
- Reserved for later (rejected by the lexer so programs stay forward-compatible): `finally assert class import from with lambda yield async await global nonlocal del pause shared`. `match` is not reserved: it is a common Python name, and Python's own `match` is a soft keyword.

## Program

```ebnf
module      = (stmt)* ENDMARKER ;
stmt        = simple_stmt NEWLINE | compound_stmt ;
block       = NEWLINE INDENT stmt+ DEDENT ;
```

## Statements

```ebnf
simple_stmt = assign | ann_assign | aug_assign | return_stmt | raise_stmt
            | 'break' | 'continue' | 'pass' | expr_list ;

assign      = target_list '=' expr_list ;
ann_assign  = NAME ':' type '=' expr ;
aug_assign  = target ('+=' | '-=' | '*=' | '/=' | '//=' | '%=' | '**=') expr ;   # target must not be a tuple (checked after parsing)
return_stmt = 'return' [expr_list] ;
raise_stmt  = 'raise' expr ;                              # bare `raise` is later
target_list = target (',' target)* [','] ;                # more than one, or a trailing comma, unpacks
target      = NAME | postfix | '(' target_list ')' ;      # postfix must end in a subscript_op (checked after parsing)

compound_stmt = func_def | if_stmt | while_stmt | for_stmt | try_stmt ;

func_def    = ['temper'] 'def' NAME '(' [params] ')' ['->' type] [raises] ':' block ;
raises      = 'raises' NAME (',' NAME)* ;                # contextual: `raises` stays a name elsewhere
params      = param (',' param)* [','] ;
param       = [mode] NAME [':' type] ['=' expr] ;
mode        = 'mut' | 'owned' ;                         # contextual: a mode only before a NAME

if_stmt     = 'if' expr ':' block ('elif' expr ':' block)* ['else' ':' block] ;
while_stmt  = 'while' expr ':' block ['else' ':' block] ;
for_stmt    = 'for' target_list 'in' expr ':' block ['else' ':' block] ;
try_stmt    = 'try' ':' block handler+ ;                  # `else` and `finally` are later
handler     = 'except' classes ['as' NAME] ':' block ;
classes     = NAME | '(' NAME (',' NAME)* [','] ')' ;     # each NAME is a builtin exception class (checked by inference)
```

### Notes

- `temper def f(...)` marks a function *tempered*: the compiler requires more to be written explicitly, and the meaning never changes (see [design.md](design.md#core-idea-tempering)). Only function-level tempering exists here; module- and project-level tempering come later. The keyword is a prefix, not a decorator, so the compiler sees it without resolving names.
- Annotations are optional everywhere. Loose code infers them; they are hints, never required.
- Single-line bodies (`if x: y`) are not in phase 2. One way to write a block.
- Attribute assignment (`a.b = c`) waits for classes.
- `def` appears only at module top level. Nested functions bring closure capture, which waits for ownership (phase 4).
- Since M5 (A1), a `temper def` must annotate every parameter and its return completely (no bare `list`), and may call only tempered functions and builtins (D1, D5 below). Loose code may call it freely. The other tempered rules land with the rest of M5.
- Parameter modes (phase 4, D2, A4): `mut xs` says the function may change the list `xs` (or a list inside it) in place, `owned xs` that it may also keep it after returning (store it in another list, a global, or pass it to a function that does). `read` is the default and never written; writing it is an error that says to drop it. The compiler infers what each body does to its parameters and checks it against what is written: a tempered function's unwritten mode means `read`, so it must write each `mut`/`owned` its body needs; a loose function's unwritten mode is inferred, and only the modes it writes are checked. Writing more than the body needs is allowed, as over-declaring `raises` is. Returning a list parameter is not a mode (D3 covers it); holding it in an exception is `owned`, since the exception may outlive the call. Which modes a parameter may have depends on its type (A8, Nano 2026-10-29): a *heap type* is `str` or `list` (later dict, set and objects), or a tuple or optional holding one; anything else (`int`, `float`, `bool`, `None`, `range`, a tuple or optional of them) is a *value type*, which is copied. `owned` is allowed on any heap type and required when the body keeps the parameter: for a string, or a tuple or optional holding one, that means returning it, storing it in another value or a global, or holding it in an exception; a list inside such a value keeps the list rules, and a list's elements are values of their own. `mut` is allowed only when the type's own operations reach a mutable heap value: a list, or a tuple or optional holding one (`tuple[list[int], int]`, `list[int] | None`), whose list can change in place; rebinding a tuple element stays an error. On `str`, a scalar-only tuple or optional, or an exception, `mut` is an error that names what cannot change ("strings cannot change in place; remove `mut` from `s`"). An exception is an immutable heap value: `owned` when kept, including re-raising a parameter, and never `mut`, even one carrying a list (A6 rule 4), since no operation reaches the list (revisit if `e.args` lands). Either on a value type is an error ("ints are copied; remove `owned` from `n`"). The classification is on the language-level type (`infer::Type::holds_heap` and `reaches_mutable`), not on how a backend represents it, so unboxing tuples later changes no program's legality. A generic parameter may write either. Every function whose signature is concrete is checked, called or not; a generic function is checked through the instances its calls create. A program part of which the native backend cannot yet express has its modes checked once it can (the analysis is otherwise conservative and diagnoses nothing). `check`, `run` and `build` apply the same check. Modes change no behavior: removing them leaves a passing program's output unchanged.
- Rejected by the parser, as CPython rejects them before running: `return` outside a function, `break`/`continue` outside a loop (a loop's `else` is outside it), duplicate parameter names, a parameter without a default after one with a default, a positional argument after a keyword argument, and a repeated keyword argument.
- Parameters: defaults are evaluated once, when `def` runs, as in Python. A call binds positional arguments first, then keyword arguments by name. An unknown name, a parameter bound twice, or a parameter left unbound is an error.
- Declared errors (phase 4, D8): `raises` lists the exceptions that may escape a function: builtin exception classes that nothing redefines. A class admits the classes derived from it (B4), so `raises ArithmeticError` covers a `ZeroDivisionError` and `raises Exception` covers any. A loose function without a clause infers its set from its body and its callees; a tempered function, or any function with a clause, has the set it declares, and its body may let nothing else escape. A tempered function without a clause declares that nothing escapes. Callers see the declared set, and listing more than escapes is allowed. What escapes: each `raise`, and `int(s)` / `float(s)` (`ValueError`) unless the argument is a number, minus what an enclosing `try`'s handlers catch (a class derived from one they name), plus what its handlers let escape. A call by a name that a `def` shadows counts the builtin too unless that `def` runs before any other code (before every other module statement and parameter default), since a call before the `def` runs reaches the builtin; such a `def` also means no argument is surely a number, as values its calls return are typed by the `def`. Faults never escape (D7). A raised value's class comes from the value: a call `ValueError(...)` or a name `ValueError`, an `except ValueError as e` binding (which may hold any class derived from `ValueError`), a module-level name also as the builtin it is until bound, and through variables, conditional expressions and `and`/`or`; anything else (a parameter, a function's result, a list element) is an `Exception`. An escape of a base class may be of any class derived from it, so only a handler of that class or one it derives from removes it: `except ZeroDivisionError` leaves an `ArithmeticError`, and only `except Exception` removes an `Exception`; likewise only a clause naming that class or a base admits it. A clause changes no behavior: removing it, like removing `temper`, leaves a passing program's output unchanged. A clause listing `Exception` admits any exception, so it tells callers nothing; when no escape from the body is of unknown class it draws a warning (B4 follow-up) at `Exception`, naming the clause that says more: the other classes it lists plus what escapes (`raises IndexError, ValueError`), with no repeated class and none a listed one covers, or "remove the `raises` clause" when that is nothing; applying the suggestion always checks clean, so there is no warning when that clause would name a class the module redefines (`def ValueError(): ...`). There is no way to silence it, as it fires only when the clause can narrow (revisit with a second lint). A body that needs it (re-raising a parameter, or calling a function whose own clause is `raises Exception`) draws none, so only the innermost vacuous clause warns.
- Exceptions (phase 4, D6/D7): `raise e` raises an exception value; `raise ValueError` means `raise ValueError()`. A `try` runs its first handler whose class the raised exception is an instance of: that class or one derived from it, in Python's hierarchy (B4, see [Builtins](#builtins-phase-2)), so `except Exception` catches any and a later handler a former one covers never runs, as in Python. An exception no handler catches propagates, through calls, as in Python. A tuple handler `except (A, B):` catches an instance of any class in the tuple or derived from one; the parentheses are required, as in Python before 3.14 (`except A, B:` is an error saying so), a trailing comma is allowed, and so is a repeated class. The classes are names written in the handler: an empty tuple, a nested tuple and a variable holding classes are errors (see [Deviations](#deviations-from-python)). `except Name as e` (or `except (A, B) as e`) binds `e` for the handler and unbinds it afterwards, as Python does, so reading `e` after the handler is an unbound-name error. `raise` ends a path like `return` (no fall-through, post-terminator narrowing), and a `try` ends one when its body and every handler do. Only `raise` and `int(s)` / `float(s)` of a malformed string produce a catchable exception; see [Runtime errors](#runtime-errors).

## Types (annotation syntax)

```ebnf
type        = type_atom ['|' 'None'] ;                  # `None | None` is rejected (checked after parsing)
type_atom   = (NAME | 'None') ['[' type (',' type)* ']'] ;
```

Matches Python: `int`, `str`, `list[int]`, `tuple[int, str]`, `None`. Variable-length `tuple[int, ...]` is later. Names resolve in the type namespace. `T | None` (Optional) is in phase 2, including nested positions such as `list[int | None]`; general `A | B` unions remain later. Optional is one fixed union type, not permission to change a variable's type. PR #10 implements the token/AST contracts and Optional inference at module level; PR #13 implements this type syntax and `is` / `is not` parsing, including nested annotations. PR #14 implements supported post-terminator narrowing. PR #16 adds Optional fixtures and in-process lex/parse/infer diagnostic checks. PR #17 wires CLI `check`/`run` and validates the current 35-program executable corpus; this is not exhaustive edge-case coverage. `raise` narrows like `return` since phase 4 (B1). See [open implementation work](roadmap.md#phase-2-open-implementation-work).

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
- Subscripts apply to lists, strings, and tuples. A negative index counts from the end: `xs[-1]` is the last element. An index out of range in either direction is a runtime error, except on a tuple: there the index and the length are both static, so it is a compile error, like an unpacking arity mismatch. Slicing is not in phase 2.
- Indexing a string and iterating over it (`for ch in s`) yield a length-1 `str`, counted in code points, as in CPython. Under UTF-8, indexing a non-ASCII string is O(n); the phase-3 backend deals with that.
- `in` with a `str` on the right is a substring test. The left operand must be a `str`; anything else is a compile error.
- List `==` and `!=` compare element-wise. Both lists must have the same element type, otherwise it is a compile error. Scalar `int == float` stays as in Python.
- An annotation that conflicts with the inferred type is a type error.

## Builtins (phase 2)

A closed set. Anything else is a name error.

- Functions: `print len list range str int float bool`. `list(xs)` takes exactly one list and returns a shallow copy of the same type (phase 4, D3/D4).
- Exceptions (phase 4, D6, B4): the classes `Exception ArithmeticError OverflowError ZeroDivisionError LookupError IndexError RuntimeError RecursionError ValueError`, in Python's hierarchy: `Exception` is the base of all; `ArithmeticError` of `OverflowError` and `ZeroDivisionError`; `LookupError` of `IndexError`; `RuntimeError` of `RecursionError`; `ValueError` derives from `Exception` directly. Calling one with zero or one argument of any type builds an exception value of that class; `str(e)` and `print(e)` give the argument as a string, empty without one.
- Methods: `list.append`.
- Type names: `int float str bool list tuple None Exception`. `Exception` is the one type of every exception value, so `except ValueError as e` and `except IndexError as e` can share `e`; the class of a value is known when it runs, as in Python, not from its type.

## Runtime errors

A fault (overflow, index out of range, unpacking arity, division by zero, recursion depth, ...) means a bug: `try` never catches it (D7). It stops the program, prints a diagnostic with its source span to stderr, and exits with status 1, as an uncaught CPython exception does.

Only `raise` and `int(s)` / `float(s)` of a malformed string (a `ValueError`) produce a catchable exception. One no handler catches ends the program the same way, with CPython's last traceback line as the message (`ValueError: bad`, or just `ValueError` when the message is empty) and the span of its `raise` or call. `int` of a NaN float or of a numeric string outside `i64` stays a fault.

A compile error (lexical, syntax, or type) is reported before anything runs: `pythrust run` prints the diagnostic to stderr, writes nothing to stdout, and exits with status 1.

A warning (a lint) is a note on a valid program. `run`, `check` and `build` print each to stderr as `path:line:col: warning: ...`, with the line and a caret underline, in source order, once every check passes and before anything runs; warnings never change the exit status or the program's output. A built executable prints none.

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
- An out-of-range tuple index (`(1, 2)[2]`) is a compile error, so nothing runs. Python raises an `IndexError` at that point.
- Type errors are reported before the program runs, so a program with one prints nothing. Python raises the same mistakes as a `TypeError` at runtime, if it raises them at all.
- `1 in "abc"`: a non-`str` left operand of `in` on a string is a compile error. Python raises a `TypeError`.
- **NaN in containers (Nano):** list and tuple `==`, `!=`, ordering and `in` use value equality only, with no identity shortcut, so a NaN never equals itself inside a list or tuple: for `x = float('nan')`, `[x] == [x]`, `(x,) == (x,)` and `x in [x]` are all `False`. CPython checks identity first and returns `True`; it agrees with Pythrust only when the NaNs are different objects.
- `list(x)` copies a list only: `list("ab")`, `list(range(3))`, a tuple, or no argument is a compile error. Python accepts any iterable and `list()` returns `[]`.
- List equality needs the same element type: `[1] == [1.0]` is a type error, where Python compares element-wise and returns `True`.
- `==` and `!=` between types that can never be equal (`1 == "a"`) are type errors; `int`, `float` and `bool` compare freely.
- Functions are not values yet: a `def` name may only be called. A name cannot be both a `def` and a variable, and a `def` name is defined once.
- A function that returns a value on some path must not fall off its end (that returns `None`) unless its return annotation is `T | None`.
- `is` / `is not` compare only with `None`. In an `if`/`elif`/`while` whose test is `x is not None`, and in the clauses after an `x is None` test, a `T | None` name `x` has type `T`, unless that block assigns `x`. An `if x is None:` whose body always leaves (`return`, `raise`, `break`, `continue`, also from a loop's `else` when that loop has no `break`), or an `if x is not None:` whose `else` always leaves, also narrows `x` for the rest of the enclosing block, unless the other side or the rest assigns `x`. An `if` with `elif` clauses does not. Tests compose with `and`, `or` and `not`: `a and b` holding proves what either side proves when holding, and failing proves only what both sides prove when failing; `or` is the mirror image, and `not` swaps holding and failing. So `if x is None or k > x:` narrows `x` in later clauses and `if x is None or y is None: return` narrows both. The same applies inside expressions: the right side of `a and b` sees what `a` holding proves, the right side of `a or b` what `a` failing proves, and `t if c else f` narrows `t` and `f` by `c` holding and failing. Truthiness (`if x:`) does not narrow, because `0` and `""` are false too; an error on such a name in the body suggests `x is not None` (Nano). CPython runs these programs.
- A variable first bound to `None` has type `None` unless annotated: `x = None` then `x = 5` is a type error; write `x: int | None = None`. `T` and `None` flow into `T | None` (also inside tuples); lists stay invariant, but a list or tuple literal takes its element types from where it goes (an annotation, argument, return or reassignment), so `xs: list[int | None] = [None, 1]` holds.
- A negative `float` raised to a fractional power is a runtime error; Python returns a `complex`, which the subset lacks.
- Mixed `int`/`float` arithmetic and comparison, and `int / int`, go through `f64`, so results can differ from Python's exact math for magnitudes above 2**53.
- **Faults are fatal (D7):** overflow, index out of range, division by zero, unpacking arity and recursion depth are never caught: `except IndexError` around `xs[10]` does not run, nor does `except Exception` around `10 // 0`. Only `raise` and `int(s)` / `float(s)` of a malformed string raise catchable exceptions. Fixtures `fault_not_caught` and `fault_not_caught_by_exception`.
- Bare `except:` is not supported yet (D6): it is a parse error whose fix-it is `except Exception:`, which catches any. Fixture `except_catch_all`.
- **Tuple handlers take literal names only (Nano, 2026-10-29, #56):** `except ()`, which catches nothing in Python, is a parse error (fixture `except_empty_tuple`); a nested tuple `except ((A, B), C):` is a parse error saying to list every class in one tuple; exception classes are not values, so `errs = (ValueError, IndexError)` and `except errs:` are errors saying classes can't be held in variables. Unparenthesized `except A, B:` is an error as in CPython 3.13, the oracle; 3.14 accepts it without `as` (PEP 758), which stays out for now (fixture `except_tuple_unparenthesized`).
- **Parameter modes (D2):** a tempered function whose body needs a `mut` or `owned` it does not write, or any function whose written mode its body outgrows, is a compile error, so nothing runs; so is a mode its type cannot have (`owned n: int`, `mut s: str`). CPython runs the program with the modes removed. Fixtures `modes_unwritten`, `modes_too_weak`, `modes_string_unwritten` and `modes_value_owned`.
- **Shared arguments to a tempered function (A5):** a call that may pass one list, or a list inside it, to two parameters of a tempered function, one of them `mut` or `owned`, is a compile error about the call, in loose code too; the fix-it is a copy, `list(...)`. Calls to loose functions keep Python aliasing, even when they write modes, unless the callee passes those arguments on, directly or through other functions, to such a tempered call. CPython runs the program with the modes removed. Fixtures `modes_shared_call` and `modes_shared_forward`.
- **Aliasing in tempered bodies (D3, D4):** a tempered body that changes a list through one name while another name for the same list is still used later, that changes a list a `for` loop is running over, or that returns a parameter or a value sharing a list with one, is a compile error, so nothing runs. CPython keeps Python's aliasing. Fixtures `aliasing_shared` and `aliasing_return`.
- **Declared errors (D8):** a tempered function that lets an exception escape without listing it in a `raises` clause, or any function whose clause misses one, is a compile error, so nothing runs. CPython runs the program with the clause removed. Fixtures `raises_undeclared` and `raises_late_shadow`.

## Deliberate omissions (phase 2)

Dicts, sets, slicing, starred unpacking (`a, *b = ...`), multi-index subscripts (`a[1, 2]`), comprehensions, lambda, classes, imports, `try` `else`/`finally`, bare `raise`, exception chaining and user-defined exceptions, `with`, decorators, bitwise operators, chained assignment (`a = b = 1`), bare declarations (`x: int` with no value), `tuple[int, ...]`, general `A | B` unions (except phase-2 `T | None`). Each is additive: existing phase-2 programs keep their meaning.

## Decided

- **Integers:** `i64` with checked overflow (runtime error, no silent wrap).
- **Fixed variable types (Nano):** one type per variable, inferred once; no rebinding to a different type. `total = 0` followed by `total += 1.5` is a compile error with a fix-it: "total is int here; write 0.0". Do not widen the variable from int to float: if the loop never runs, that changes the output from `0` to `0.0`, violating "tempering never changes meaning". Flow-sensitive type-changing rebinding such as `x = int(x)` is out of scope now; ship a clear error, then use the conformance corpus to decide whether to revisit it. A same-type assignment is unaffected.
- **Optional (Nano):** pull `T | None` into phase 2 so None-initialized values can be represented without arbitrary type-changing rebinding. Optional remains one fixed union type. **None-first annotation confirmed by Nano:** write `x: int | None = None` when a None-first variable must later hold an int; unannotated `x = None` then `x = 5` is an error with that fix-it. Do not relax loose-mode Optional inference yet; revisit after corpus evidence shows how often the annotation is needed.
- **Optional follow-ups (Nano, in order):** parser support for `'|' 'None'` and `is` / `is not`; then a separate PR for narrowing after a terminating `if x is None` branch (`return`, `break`, `continue`), with each implemented terminator tested and a fall-through negative case; then Optional conformance cases once parsing lands. These narrow an existing fixed union, not the variable's declared type. **Raise deferral confirmed by Nano:** implement and test `return`/`break`/`continue` narrowing now; defer the `raise` test until `raise` exists. `raise` arrived in phase 4 (B1) and narrows like `return`, with its test.
- **Lists and dyn (Nano):** homogeneous element types; `[1, "a"]` is an error unless the element type is `dyn`. A `dyn` variable may hold anything. `dyn` is the explicit Python-bridge escape hatch, not an implicit fallback.
- **Integer powers (Nano):** `int ** int` returns `int`; negative exponents fail at runtime. A literal negative exponent with an integer base fails at compile time. Both diagnostics say "use a float base". `2.0 ** -1` remains legal. Checked overflow already makes `2 ** 64` a runtime error.
- **Tempering marker:** `temper def f(...)`, a keyword prefix.
- **Tuples:** Python syntax. Fixed-length, immutable, positionally typed. Unpacking in assignment and `for`. **Tuple bounds confirmed by Nano (#12):** an out-of-range tuple index is a compile error because index and length are static, so nothing runs; list/string out-of-range indexes remain runtime errors.
- **Source file extension:** `.pyr`.

- **Pause marker** (lands with concurrency): `pause <expr>`, a loosely binding prefix covering the whole expression to its right, like Swift's `await`. Required in tempered code at every pause point; it must cover at least one. See [research/concurrency-syntax.md](research/concurrency-syntax.md).
- **Shared lock** (lands with concurrency): a builtin type `shared[T]`, built with `shared(v)` and accessed through `with x as n:`, where `n` writes through. Not a keyword.

`pause`, `shared` and `with` stay reserved until those features land, so adopting them breaks no programs. `temper` is already live in `func_def`.

### Phase 4 (M5): tempering, ownership, exceptions (Nano, 2026-10-27)

Decided, not yet implemented; [roadmap.md](roadmap.md#phase-4-m5-plan) orders the work. Ownership direction and vocabulary are in [research/ownership.md](research/ownership.md).

- **D1, what tempering requires:** a `temper def` annotates every parameter and its return, uses no `dyn` in its signature or body, and writes the `mut`/`owned` modes its body needs (D2), including `owned` on a string it keeps or returns (A8). Implemented (A1) for annotations: each must be complete, so `xs: list` is an error, and a missing one suggests the inferred type when it can be spelled, such as `n: int`. The `dyn` rule holds trivially until `dyn` lands, since no code may use it yet. Modes implemented (A4); see the notes above.
- **D2, ownership modes:** a contextual prefix on a parameter, `def grow(mut xs: list[int])`, `def keep(owned xs: list[int])`. `read` is the default and is never written. `mut` and `owned` are not reserved, so `mut = 1` stays legal. Loose functions may write modes; the compiler checks them against the body. Implemented (A4). Only a tempered callee's `mut` makes passing one list twice an error (ownership.md, decided 2); loose-to-loose calls keep Python aliasing. Extended to every heap type (Nano, 2026-10-29, A8): `owned` on any heap type, required when the body keeps the parameter, `mut` only when the type's operations reach a list, neither on a value type; see the parameter-modes note above. D3 and D4 stay list-only: a tempered function may return a string parameter (it is `owned`, no copy fix-it), and `b = a` on strings is no aliasing error. Why: one rule for what tempering writes down, keyed on what the type is rather than on its refcount representation, so a kept string is as visible in a signature as a kept list.
- **D3, returned aliases:** tempered code may not return an alias of a parameter (`return xs`); the fix-it is `return list(xs)`. Loose code may, tracked by the inferred summary. A new builtin `list(xs)` makes a shallow copy. Implemented (A6): a value that holds a parameter or a list inside one counts, as does a list the body stored in a parameter.
- **D4, aliased mutation:** in tempered code, two live names for one list with a mutation through either (`b = a`, then `b.append(1)` while `a` is used) is an error; the fix-it is `b = list(a)`. There is no share marker in phase 4. Loose code keeps Python's aliasing. Implemented (A6), non-lexically: only a name used after the change counts. A list inside another is its own list, so `r = rows[0]` then `r.append(1)` while `rows` is used is legal, but two names for `rows[0]` are two names for one list, and so are any two names of one type reached inside a value the body did not build (`pair[0]` and `pair[1][0]` of a `tuple[list[int], list[list[int]]]` may be `(xs, [xs])`). A parameter may be a global's list (`f(saved)`), so it may share lists with the globals of its types; two parameters share none, which A5 checks at the call. A module variable the body, or a function it calls, reads after the change counts as a name used later, and a call to a function that changes a module variable's list, itself or through its callees, is a change at the call. A `mut` argument is a change to the list and every list inside it. Changing a list a `for` loop is running over is the same error, with the fix-it `for x in list(xs)`. Passing one list twice to a call is A5's check.
- **D5, tempered calling loose:** a tempered function calls only tempered functions and builtins; otherwise the error says "temper `f` first". Implemented (A1), defaults included. The crossing annotation in [design.md](design.md#core-idea-tempering) waits for module tempering or the Python bridge.
- **D6, exception surface:** builtin exception names only: `ValueError`, `IndexError`, `ZeroDivisionError`, `OverflowError`, `RecursionError`; B4 adds their bases (below). `raise ValueError("msg")`; `try:` with one or more `except Name [as e]:` or `except (A, B) [as e]:` clauses (`as` is allowed only there until `with` lands); `str(e)` and `print(e)` give the message. Bare `except:`, `else`, `finally`, chaining and user-defined exceptions are later; their words stay reserved.
- **D7, what can be caught:** faults that mean a bug stay fatal as today: overflow, index out of range, division by zero, unpacking arity, recursion depth. Only `raise` and the parse builtins `int(s)` / `float(s)` (which raise `ValueError`) can be caught. A deviation from Python: `except IndexError` around `xs[10]` never catches. **Superseded (Nano, 2026-10-30, #59; not yet implemented):** division by zero (int and float `/`, `//`, `%`, a `ZeroDivisionError`), index out of range (list, string and range reads, list stores, an `IndexError`) and unpacking arity (a list or string of unknown length, a `ValueError`) become catchable at `except` sites; overflow, recursion depth and out-of-memory stay fatal. Implicit operator faults are never tracked in `raises` (an explicit `raise` and `int(s)` / `float(s)` stay tracked); a handler for one is never reported unreachable; the `raises Exception` lint ignores them; an uncaught one still prints `run`'s diagnostic and exits 1. See [research/faults.md](research/faults.md). The sections above describe D7 as implemented until that work lands.
- **D8, declared errors:** a tempered function lists the errors that escape it with a contextual `raises` clause after the return type, `temper def parse(s: str) -> int raises ValueError:`. Loose functions infer theirs. Explicit `Result[T, E]` types and a `?` operator are later.
- **B4 choices (Nano, 2026-10-29, confirmed on #55):** (1) The bases `Exception`, `ArithmeticError`, `LookupError` and `RuntimeError` are classes like the others: `except`, `raises` and `raise` take them, and `Exception("msg")` builds one, as in Python. (2) A handler catches its class and the classes derived from it; one an earlier handler covers is legal and never runs, as in Python (no error). (3) A `raises` class admits the classes derived from it, and an escape of a base class (an `except ArithmeticError as e` binding re-raised) is not removed by a handler of a subclass. (4) Faults stay fatal under every handler, `except Exception` included (D7). (5) Bare `except:`, `BaseException` and the classes no builtin raises (`KeyError`, `TypeError`, ...) stay out; bare `except:` suggests `except Exception:`. Tuple handlers were left out here and came in their own PR (B4 follow-up, #56; Nano confirmed its four choices 2026-10-29: parentheses required, `(A,)`/`(A)`/trailing comma legal with `()` an error, a repeated class legal, literal names only). `raises Exception` stays legal though it makes a declared set vacuous; a warning says so when the body does not need it (B4 follow-up, #57; Nano confirmed 2026-10-30: a warning, not an error; only when the clause can narrow, with no silencer until a second lint; the suggestion keeps the other listed classes; no warning flags yet). Revisit D7 once lowering exceptions to `Result` makes faults cheap to catch, since `except Exception` around `1 / 0` is the most visible deviation.
- **A6 choices (Nano, 2026-10-28):** four rules A6 implements, confirmed by Nano in the project thread (2026-10-28 22:05 UTC) after #48 merged, and on #49. (1) A list inside another is its own list: `r = rows[0]; r.append(1)` while `rows` is used is legal, as `for r in rows: r[0] = 0` is. (2) The caller is not a name: `ys = xs; ys.append(1)` with `xs` unused afterwards is legal, since `mut xs` already shows the caller the change. (3) Changing a list a `for` loop is running over is a D4 error, fix-it `for x in list(xs)`. (4) A list held in an exception (`raise ValueError(xs)`) is kept past the call, like one stored in a global, so the parameter needs `owned`. A copy avoids that only when it holds no list of the parameter: `ValueError(list(xs))` for `xs: list[int]` needs no mode, but for `xs: list[list[int]]` the shallow copy still holds `xs`'s inner lists, so `owned` is still required.

## Open syntax questions (M3 blockers)

None.
