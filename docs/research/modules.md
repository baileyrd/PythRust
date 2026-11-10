# Post-M6 step 3: multi-file modules

**Status: decided 2026-11-10, except module-level tempering (open, Q19).** Nano chose the import rules on 2026-11-03 (D75); the expert settled the rest as [default] rows D76-D84. Code follows in the [PR order](#pr-order) below.

## Objective

A program is one file today: `import` names only a standard module (M6a) or a Rust binding (M6b, M6d). Step 3 lets a program span several `.pyr` files of its own, with Python's import forms and Python's run order, so the second app and later programs can be split the way Python programs are. The CPython oracle keeps working: the same tree runs under CPython 3.13.

## Decided

### Nano's rules (D75, [stated])

- Modules are found from the **project root**: the directory of `pythrust.toml`, else the main file's directory. There is no search path and no `..`.
- The forms are `import a`, `import a.b`, `from a import x` and `as`.
- Top-level code runs **once, in import order**, as in CPython.
- A **circular import** is a compile error, a listed deviation.
- A local module **named like a stdlib module** is a named error: stdlib wins.
- Local modules need **no manifest entry**, so `[pythrust]` in `pythrust.toml` stays reserved (M6d).

### Resolution (D78, D81)

A dotted name `a.b` is the file `<root>/a/b.pyr`. A name resolves in this order:

1. A **stdlib module**: any top-level name in CPython 3.13's `sys.stdlib_module_names`, kept as a static table. A module Pythrust does not implement yet (`import json`) is "`json` is not supported yet", and a local `json.pyr` is refused now, not when `json` lands.
2. A **local module** file under the root.
3. A **Rust binding** in `[rust]` of the root `pythrust.toml`, the project's one manifest.

Named errors:

- A local module named like a stdlib module (D75).
- A name that is both a local module and a `[rust]` binding.
- A file `a.pyr` beside a directory `a/`, naming both (CPython would take the file and then fail on `a.b`).
- A file name that differs from the import's spelling, including case, even on a case-insensitive file system, as CPython requires.
- An `__init__.pyr`: "packages with `__init__.pyr` are not supported yet". Directories are namespace packages (CPython 3.3+), so no package initialization exists. A listed deviation.
- A relative import (`from . import x`, `from .a import x`): "relative imports are not supported yet", a listed deviation.

Finding the root only checks that `pythrust.toml` exists. The manifest is still parsed only when some module imports a name that is neither stdlib nor local. Any module, not only the main file, may import a binding the root manifest names, and M6d's "build only imported bindings" (D32) counts imports across all modules.

### Forms and names (D79, D80, D83, D84)

| Form | Binds | Uses |
|---|---|---|
| `import a` | `a` | `a.f()`, `a.X` |
| `import a.b` | `a` | `a.b.f()` |
| `import a.b as c` | `c` | `c.f()` |
| `from a import x` | `x`, the same object as `a.x` | `x` |
| `from a import x as y` | `y` | `y` |

- Imports still come first in each module, so every module name is known while parsing, as M6a resolves `sys.argv` today.
- `from a import x` where `a` has no `x` is a compile error.
- `from a import x` binds a new global holding the same object, so a list is shared, as in CPython. D3's one fixed type applies to rebinding `x`.
- `from a import *` is the named error "`import *` is not supported; import the names you use", a listed deviation, for stdlib and local modules alike (D79).
- Privacy is not enforced, as in CPython: `from a import _x` and `a._x` work. `__all__` is an ordinary global with no effect (D80).
- A local module is not a value (`m = a` is an error), as a stdlib module is not.
- Assigning another module's global (`a.X = 1`) is "not supported yet": attribute stores wait for classes. A listed deviation.
- `__name__` is a `str` global: `"__main__"` in the main file and the module's dotted name elsewhere, so `if __name__ == "__main__":` works as in CPython.
- `from` and `as` also work on stdlib modules, for every name Pythrust's module exposes (D84): `from concurrent.futures import ThreadPoolExecutor`, `from os import path` (binds the module name `path`, which is not a value), `from os.path import join`, `import os.path`, `import sys as s`. M6c's executor rules are unchanged. A name the module does not expose keeps today's "not supported" error.

### Run order (D75, D83)

Modules run depth-first in the order of their import statements, each at most once, as CPython runs them. Importing the main file always closes a cycle, so it is the circular-import error. Pythrust refuses every cycle because it cannot match CPython's partially initialized modules.

```python
# main.pyr          # a.pyr            # b.pyr
import a            import b           print("b")
import b            print("a")
print("main")
```

prints `b`, `a`, `main`, as CPython does.

### Analysis (D76)

The compiler loads the whole module graph and analyzes it as one program. `own` summaries, `raises` sets and generic schemes are keyed by qualified name (`a.f`), as module values are already globals. There is no separate compilation and no per-module summary file. A tempered function in any module is checked as at top level today, and D44 holds: a lowering gap in a program with a `temper def` in any module is a named error. D5 holds across modules: a tempered `b.g` calling a loose `a.f` gets "temper `a.f` first".

### Native code (D77)

`build` lowers all modules into one IR program, one object and one executable. Each function's symbol is mangled from its qualified name, so `a.f` and `b.f` never collide. Diagnostics and tracebacks keep the source name `f` and the file of the module that defines it.

### Diagnostics (D82)

`span::Span` stays `{start, end}`. M6b's `Sources` (`bindings.rs`) already gives each stub a base offset and maps a span back to its file. It moves into its own module, shared by stubs and modules, and each local module gets a base too. No contract type changes. Diagnostics stay `path:line:col: ...`: the main file's path as given, and a module's path joined to the root and shown relative to the current directory where possible.

### CPython oracle

A multi-file corpus program is a directory holding the main file and its modules. The oracle runs CPython with the root on `sys.path` (D78), so the same tree runs under both. Programs that import a Rust binding still have no CPython oracle (M6b).

## Open

- **Module- and project-level tempering (Q19, escalated to Nano).** Step 3 adds none: only function-level `temper def` exists, in any module. design.md says tempering "cascades upward to module and project", and grammar.md says it "comes later". Until Nano answers, nothing about it is decided.

## PR order

One PR each (cards under #1143):

1. **Design note (#1216):** this note, the grammar entries, the roadmap items, D75-D84 and Q19.
2. **Sources for modules (#1218, D82):** move `Sources` out of `bindings.rs` and generalize it to any number of files; no surface change. Exit: a unit test with two module sources renders each span with its own file; every existing diagnostic unchanged.
3. **`import mymod` (#1220):** the module graph from the root, parsed once per file and ordered depth-first; the resolution order and its named errors; qualified names through `infer`, `own` and `lower`; `__name__`; one executable. Exit: CPython-checked corpus directories with two and three files (diamond imports, top-level effects printing in order, a module with a tempered function and one importing a binding), the circular-import, stdlib-shadow, binding-shadow, `__init__.pyr` and file-beside-directory errors, each with an accepted neighbour; `run` and `build` agree.
4. **`from`, dotted and `as` (#1222):** the remaining forms for local and stdlib modules (D83, D84), `import *` and relative imports refused, missing names a compile error. Exit: CPython-checked corpus programs for each form, aliasing and shared lists through `from`, the named errors with accepted neighbours; grammar.md's omissions list drops the three forms; step 3 ticked.

## Deviations

Listed in [grammar.md](../grammar.md#deviations-from-python) as planned now, and given fixtures as each lands: circular imports (D75), stdlib-shadow and binding-shadow errors (D75, D81), `__init__.pyr` and relative imports (D78), `import *` (D79), and `a.X = v` (D83). The existing "a module is not a value" entry extends to local modules.
