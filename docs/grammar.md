# Loose Syntax Grammar (phase 1 draft)

Scope: the phase-2 subset only. Functions, numbers, strings, lists, control flow. Python-compatible wherever possible, so existing Python readers need no new model.

Notation: EBNF. `A*` zero or more, `A+` one or more, `[A]` optional, `A | B` choice, `'x'` literal token.

## Lexical

- Indentation-sensitive, as in Python. The lexer emits `NEWLINE`, `INDENT`, `DEDENT`. Spaces only; a tab is a lexical error.
- Comments: `#` to end of line.
- Tokens: `NAME`, `INT`, `FLOAT`, `STRING`.
  - `INT`: decimal digits, `_` separators allowed. Arbitrary-size ints are a later question; phase 2 uses i64.
  - `FLOAT`: digits with `.` or exponent.
  - `STRING`: single or double quoted, with `\` escapes. No f-strings, triple quotes, or byte strings yet.
- Keywords: `def return if elif else while for in break continue pass and or not True False None`.
- Reserved for later (rejected by the lexer so programs stay forward-compatible): `try except raise class import from as with match lambda async await global nonlocal del temper pause shared`.

## Program

```ebnf
module      = (stmt)* ENDMARKER ;
stmt        = simple_stmt NEWLINE | compound_stmt ;
block       = NEWLINE INDENT stmt+ DEDENT ;
```

## Statements

```ebnf
simple_stmt = assign | ann_assign | aug_assign | return_stmt
            | 'break' | 'continue' | 'pass' | expr ;

assign      = target '=' expr ;
ann_assign  = NAME ':' type ['=' expr] ;
aug_assign  = target ('+=' | '-=' | '*=' | '/=' | '//=' | '%=') expr ;
return_stmt = 'return' [expr] ;
target      = NAME | postfix ;      # postfix must end in a subscript_op (checked after parsing)

compound_stmt = func_def | if_stmt | while_stmt | for_stmt ;

func_def    = 'def' NAME '(' [params] ')' ['->' type] ':' block ;
params      = param (',' param)* [','] ;
param       = NAME [':' type] ['=' expr] ;

if_stmt     = 'if' expr ':' block ('elif' expr ':' block)* ['else' ':' block] ;
while_stmt  = 'while' expr ':' block ['else' ':' block] ;
for_stmt    = 'for' NAME 'in' expr ':' block ['else' ':' block] ;
```

Notes:
- Annotations are optional everywhere. Loose code infers them; they are hints, never required.
- Single-line bodies (`if x: y`) are not in phase 2. One way to write a block.
- Attribute assignment (`a.b = c`) waits for classes.

## Types (annotation syntax)

```ebnf
type        = NAME ['[' type (',' type)* ']'] ;
```

Matches Python: `int`, `str`, `list[int]`. Names resolve in the type namespace. `A | B` unions and `T | None` are a later addition.

## Expressions

Lowest to highest precedence:

```ebnf
expr        = or_expr ['if' or_expr 'else' expr] ;     # conditional expression
or_expr     = and_expr ('or' and_expr)* ;
and_expr    = not_expr ('and' not_expr)* ;
not_expr    = 'not' not_expr | comparison ;
comparison  = sum (comp_op sum)* ;                       # chained, as in Python
comp_op     = '==' | '!=' | '<' | '<=' | '>' | '>=' | 'in' | 'not' 'in' ;
sum         = term (('+' | '-') term)* ;
term        = factor (('*' | '/' | '//' | '%') factor)* ;
factor      = '-' factor | power ;
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
            | '[' [expr (',' expr)* [',']] ']' ;         # list literal
```

Semantic rules (not grammar):
- `/` is true division (float); `//` is floor division. Same as Python.
- `and`/`or` short-circuit and return an operand, as in Python.
- Comparison chains evaluate each operand once.
- A bare `expr` statement is allowed (calls for effect).

## Deliberate omissions (phase 2)

Tuples, dicts, sets, slicing, comprehensions, lambda, classes, imports, exceptions, `with`, decorators, bitwise operators. Each is additive and none changes the grammar above.

## Open syntax questions (phase 1 blockers)

These are the three the spec still owes, plus two surfaced while drafting:

1. **Tempering annotation.** How a function is marked tempered. Candidates: a decorator-style prefix (`@temper`), a keyword (`temper def f(...)`), or a module-level pragma. Prefer whichever Python tooling already ignores gracefully.
2. **Pause marker.** The explicit interleave point required in tempered mode. Candidate: a keyword prefix on the call (`pause fetch(x)`).
3. **Shared-lock keyword.** One short keyword for the built-in locked wrapper. Candidate: `shared`.
4. **Tuples.** Needed almost immediately (multiple returns, `for k, v in ...`). Likely the first addition after the subset.
5. **Integer width.** Fixed i64 versus Python-style arbitrary precision. Affects speed claims and loose-mode semantics. Needs an explicit decision.

The keywords `temper`, `pause`, and `shared` are already reserved above so adopting them later breaks no programs.
