//! A local module's names, qualified (step 3, D76): the program is one
//! module to every later stage, so module `a`'s global `x` becomes the
//! global `a.x`, the name `a.x` reads in its importers. A function's own
//! locals keep their names; comprehension names (`x@n`) and the names
//! of other modules (`sys.argv`, `b.f`) are never a module's globals.
//!
//! A name a module reads but does not bind is a builtin's (or no one's),
//! never the main file's: a main-file global it would meet is qualified
//! `__main__.x` instead, so each module keeps its own lookup (D76).

use std::collections::BTreeSet;

use crate::ast::{
    Expr, ExprKind, FuncDef, Ident, LocalModule, Module, Stmt, StmtKind, Target, TargetKind,
    COMP_MARK,
};
use crate::infer::scope;
use crate::span::Span;

/// The name every module reads for its own name (D83).
const NAME: &str = "__name__";

/// The main file's module name, which qualifies its globals that a
/// local module's free names would otherwise meet.
const MAIN: &str = "__main__";

/// Qualifies the globals of `module`, the local module `name`, with
/// `name.`, and binds its `__name__` first: an importer may read it.
/// The module's names, as importers see them.
pub fn qualify(module: &mut Module, name: &str) -> LocalModule {
    bind_name(module, name);
    let globals = globals(&module.body);
    let defs = module
        .body
        .iter()
        .filter_map(|s| match &s.kind {
            StmtKind::FuncDef(f) => Some(f.name.name.clone()),
            _ => None,
        })
        .collect();
    let mut rename = |n: &str| globals.contains(n).then(|| format!("{name}.{n}"));
    block(&mut module.body, &mut rename);
    LocalModule {
        name: name.to_string(),
        names: globals,
        defs,
    }
}

/// Binds `__name__` to `"__main__"` first in the main file, if it names
/// it anywhere (D83), and qualifies the main file's globals that a name
/// free in one of the local modules `parts` would otherwise resolve to.
pub fn main_name(module: &mut Module, parts: &[Module]) {
    if reads(&module.body, NAME) {
        bind_name(module, MAIN);
    }
    let free: BTreeSet<String> = parts.iter().flat_map(|p| free(&p.body)).collect();
    let clash: BTreeSet<String> = globals(&module.body).intersection(&free).cloned().collect();
    let mut rename = |n: &str| clash.contains(n).then(|| format!("{MAIN}.{n}"));
    block(&mut module.body, &mut rename);
}

/// The program as one module (D76): the bodies of `parts`, in the
/// order they run, then `main`'s; each binding's functions once.
pub fn link(parts: Vec<Module>, main: Module) -> Module {
    let mut out = Module {
        body: Vec::new(),
        stubs: Vec::new(),
        pauses: Vec::new(),
        starts: Vec::new(),
        span: main.span,
    };
    for part in parts.into_iter().chain([main]) {
        out.starts.push(out.body.len());
        out.body.extend(part.body);
        out.pauses.extend(part.pauses);
        for stub in part.stubs {
            if !out.stubs.iter().any(|s| s.name.name == stub.name.name) {
                out.stubs.push(stub);
            }
        }
    }
    out
}

/// Binds `__name__` to `name` first in `module`; a later assignment
/// still rebinds it.
fn bind_name(module: &mut Module, name: &str) {
    let at = Span {
        start: module.span.start,
        end: module.span.start,
    };
    let value = Expr {
        kind: ExprKind::Str(name.to_string()),
        span: at,
    };
    let target = Target {
        kind: TargetKind::Name(NAME.to_string()),
        span: at,
    };
    let bind = Stmt {
        kind: StmtKind::Assign { target, value },
        span: at,
    };
    module.body.insert(0, bind);
}

/// The names a qualified module's code reads outside any function's
/// locals that are not qualified: builtins', or names it never binds.
fn free(body: &[Stmt]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut look = |n: &str| {
        if !n.contains('.') && !n.contains(COMP_MARK) {
            out.insert(n.to_string());
        }
        None
    };
    block(&mut body.to_vec(), &mut look);
    out
}

/// The names the top level of `body` binds, a function's name included:
/// not a comprehension's own or another module's.
fn globals(body: &[Stmt]) -> BTreeSet<String> {
    let defs = body.iter().filter_map(|s| match &s.kind {
        StmtKind::FuncDef(f) => Some(f.name.name.clone()),
        _ => None,
    });
    scope::assigned(body)
        .into_iter()
        .chain(defs)
        .filter(|n| !n.contains(COMP_MARK) && !n.contains('.'))
        .collect()
}

/// Whether `body`, a function's included, reads the name `name`.
fn reads(body: &[Stmt], name: &str) -> bool {
    let mut found = false;
    let mut look = |n: &str| {
        found |= n == name;
        None
    };
    let mut copy = body.to_vec();
    block(&mut copy, &mut look);
    found
}

type Rename<'r> = dyn FnMut(&str) -> Option<String> + 'r;

fn block(body: &mut [Stmt], rename: &mut Rename<'_>) {
    body.iter_mut().for_each(|s| stmt(s, rename));
}

fn stmt(s: &mut Stmt, rename: &mut Rename<'_>) {
    match &mut s.kind {
        StmtKind::Assign { target: t, value }
        | StmtKind::AugAssign {
            target: t, value, ..
        } => {
            target(t, rename);
            expr(value, rename);
        }
        StmtKind::AnnAssign { target, value, .. } => {
            ident(target, rename);
            expr(value, rename);
        }
        StmtKind::Return(e) => e.iter_mut().for_each(|e| expr(e, rename)),
        StmtKind::Expr(e) | StmtKind::Raise(e) => expr(e, rename),
        StmtKind::Break | StmtKind::Continue | StmtKind::Pass => {}
        StmtKind::FuncDef(f) => func(f, rename),
        StmtKind::If {
            branch,
            elifs,
            orelse,
        } => {
            for b in std::iter::once(branch).chain(elifs) {
                expr(&mut b.test, rename);
                block(&mut b.body, rename);
            }
            block(orelse, rename);
        }
        StmtKind::While { branch, orelse } => {
            expr(&mut branch.test, rename);
            block(&mut branch.body, rename);
            block(orelse, rename);
        }
        StmtKind::For {
            target: t,
            iter,
            body,
            orelse,
        } => {
            target(t, rename);
            expr(iter, rename);
            block(body, rename);
            block(orelse, rename);
        }
        StmtKind::Del { value, index } => {
            expr(value, rename);
            expr(index, rename);
        }
        StmtKind::Try {
            body,
            handlers,
            finally,
        } => {
            block(body, rename);
            for h in handlers {
                h.classes.iter_mut().for_each(|c| ident(c, rename));
                h.binding.iter_mut().for_each(|b| ident(b, rename));
                block(&mut h.body, rename);
            }
            finally.iter_mut().for_each(|f| block(f, rename));
        }
        StmtKind::With {
            value,
            binding,
            body,
        } => {
            expr(value, rename);
            ident(binding, rename);
            block(body, rename);
        }
    }
}

/// A `def`: its name, defaults and `raises` names in the enclosing
/// scope, its body with its own locals (parameters and what it assigns)
/// kept.
fn func(f: &mut FuncDef, rename: &mut Rename<'_>) {
    ident(&mut f.name, rename);
    f.raises.iter_mut().for_each(|r| ident(r, rename));
    for p in &mut f.params {
        p.default.iter_mut().for_each(|d| expr(d, rename));
    }
    let mut locals = scope::assigned(&f.body);
    locals.extend(f.params.iter().map(|p| p.name.name.clone()));
    let mut inner = |n: &str| match locals.contains(n) {
        true => None,
        false => rename(n),
    };
    block(&mut f.body, &mut inner);
}

fn target(t: &mut Target, rename: &mut Rename<'_>) {
    match &mut t.kind {
        TargetKind::Name(n) => renamed(n, rename),
        TargetKind::Subscript { value, index } => {
            expr(value, rename);
            expr(index, rename);
        }
        TargetKind::Tuple(ts) => ts.iter_mut().for_each(|t| target(t, rename)),
    }
}

fn ident(i: &mut Ident, rename: &mut Rename<'_>) {
    renamed(&mut i.name, rename);
}

fn renamed(n: &mut String, rename: &mut Rename<'_>) {
    if let Some(new) = rename(n) {
        *n = new;
    }
}

fn expr(e: &mut Expr, rename: &mut Rename<'_>) {
    match &mut e.kind {
        ExprKind::Name(n) => renamed(n, rename),
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Str(_)
        | ExprKind::Bool(_)
        | ExprKind::None => {}
        ExprKind::Tuple(es) | ExprKind::List(es) | ExprKind::Set(es) => {
            es.iter_mut().for_each(|e| expr(e, rename));
        }
        ExprKind::Dict(pairs) => {
            for (k, v) in pairs {
                expr(k, rename);
                expr(v, rename);
            }
        }
        ExprKind::Unary { operand, .. } => expr(operand, rename),
        ExprKind::Binary { left, right, .. } | ExprKind::Logical { left, right, .. } => {
            expr(left, rename);
            expr(right, rename);
        }
        ExprKind::Compare { left, rest } => {
            expr(left, rename);
            rest.iter_mut().for_each(|(_, e)| expr(e, rename));
        }
        ExprKind::IfExp { test, body, orelse } => {
            expr(test, rename);
            expr(body, rename);
            expr(orelse, rename);
        }
        ExprKind::Call { func, args } => {
            expr(func, rename);
            args.iter_mut().for_each(|a| expr(&mut a.value, rename));
        }
        ExprKind::Subscript { value, index } => {
            expr(value, rename);
            expr(index, rename);
        }
        ExprKind::Slice {
            value,
            start,
            stop,
            step,
        } => {
            expr(value, rename);
            for part in [start, stop, step].into_iter().flatten() {
                expr(part, rename);
            }
        }
        ExprKind::Attr { value, .. } => expr(value, rename),
        ExprKind::Comp { gather, clauses } => {
            gather.exprs_mut().into_iter().for_each(|e| expr(e, rename));
            for c in clauses {
                target(&mut c.target, rename);
                expr(&mut c.iter, rename);
                c.ifs.iter_mut().for_each(|e| expr(e, rename));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer, parser};

    fn qualified(src: &str, name: &str) -> (Module, LocalModule) {
        let tokens = lexer::lex(src).unwrap();
        let mut m = parser::parse(&tokens).unwrap();
        let local = qualify(&mut m, name);
        (m, local)
    }

    fn names(m: &Module) -> Vec<String> {
        let mut out = Vec::new();
        let mut look = |n: &str| {
            out.push(n.to_string());
            None
        };
        block(&mut m.body.clone(), &mut look);
        out
    }

    #[test]
    fn globals_are_qualified_and_locals_kept() {
        let src = "X = 1\n\
                   def f(n):\n    y = n + X\n    return g(y)\n\
                   def g(v):\n    return v\n\
                   print(f(X), [X for X in [2]])\n";
        let (m, local) = qualified(src, "a");
        let want: BTreeSet<String> = ["X", "__name__", "f", "g"].map(String::from).into();
        assert_eq!(local.names, want);
        let seen = names(&m);
        for n in ["a.X", "a.f", "a.g", "print"] {
            assert!(seen.iter().any(|s| s == n), "{n} not in {seen:?}");
        }
        // A function's locals are hidden from the walk; they are kept as is.
        let tree = format!("{m:?}");
        for n in ["n", "y", "v"] {
            assert!(tree.contains(&format!("\"{n}\"")), "{n} renamed");
        }
        for n in ["X", "f", "g"] {
            assert!(!seen.iter().any(|s| s == n), "{n} left in {seen:?}");
        }
        assert!(seen.iter().any(|s| s.starts_with("X@")), "{seen:?}");
    }

    #[test]
    fn a_local_shadowing_a_global_stays_local() {
        let src = "x = 1\ndef f():\n    x = 2\n    return x\n";
        let (m, _) = qualified(src, "a");
        let StmtKind::FuncDef(f) = &m.body[2].kind else {
            panic!("a def, after `__name__` and `x`")
        };
        assert_eq!(f.name.name, "a.f");
        let StmtKind::Assign { target, .. } = &f.body[0].kind else {
            panic!("an assignment")
        };
        assert_eq!(target.kind, TargetKind::Name("x".into()));
    }

    #[test]
    fn a_program_runs_its_modules_first_and_holds_each_stub_once() {
        let stub = lexer::lex("temper def f(x: int) -> int: ...\n").unwrap();
        let binding = parser::parse_stub(&stub, "fx").unwrap();
        let a = lexer::lex("import fx\nX = fx.f(1)\n").unwrap();
        let main = lexer::lex("import a, fx\nprint(a.X, fx.f(2))\n").unwrap();
        let units = [parser::Unit {
            name: "a",
            tokens: &a,
        }];
        let m = parser::parse_program(&units, &[], &main, &[binding]).unwrap();
        assert_eq!(m.stubs.len(), 1);
        let StmtKind::Assign { target, .. } = &m.body[1].kind else {
            panic!("`a` runs first, after its `__name__`")
        };
        assert_eq!(target.kind, TargetKind::Name("a.X".into()));
        assert!(matches!(m.body[2].kind, StmtKind::Expr(_)), "then main");
    }

    #[test]
    fn a_module_always_exports_its_name() {
        let (m, local) = qualified("x = 1\n", "util");
        assert!(local.names.contains(NAME), "{:?}", local.names);
        let StmtKind::Assign { target, value } = &m.body[0].kind else {
            panic!("`__name__` bound first")
        };
        assert_eq!(target.kind, TargetKind::Name("util.__name__".into()));
        assert_eq!(value.kind, ExprKind::Str("util".into()));
    }

    fn main_of(src: &str, parts: &[Module]) -> Module {
        let tokens = lexer::lex(src).unwrap();
        let mut main = parser::parse(&tokens).unwrap();
        main_name(&mut main, parts);
        main
    }

    #[test]
    fn main_binds_its_name_first_even_when_it_rebinds_it() {
        let main = main_of("print(__name__)\n__name__ = 'x'\n", &[]);
        let StmtKind::Assign { value, .. } = &main.body[0].kind else {
            panic!("`__name__` bound first")
        };
        assert_eq!(value.kind, ExprKind::Str("__main__".into()));
        assert_eq!(main.body.len(), 3);
        assert_eq!(main_of("x = 1\n", &[]).body.len(), 1, "unread, unbound");
    }

    #[test]
    fn a_main_global_a_module_name_would_meet_is_qualified() {
        let (a, _) = qualified("def size(xs):\n    return len(xs) + n\n", "a");
        let main = main_of("def len(xs):\n    return 9\nn = 1\nm = len([])\n", &[a]);
        let seen = names(&main);
        for n in ["__main__.len", "__main__.n", "m"] {
            assert!(seen.iter().any(|s| s == n), "{n} not in {seen:?}");
        }
        assert!(!seen.iter().any(|s| s == "len" || s == "n"), "{seen:?}");
    }

    #[test]
    fn exception_names_in_handlers_and_raises_are_module_names() {
        let src = "KeyError = 1\n\
                   temper def f() -> int raises KeyError, ValueError:\n\
                   \x20   try:\n        return 1\n\
                   \x20   except (KeyError, OSError):\n        return 2\n";
        let (a, _) = qualified(src, "a");
        let tree = format!("{a:?}");
        assert!(tree.matches("\"a.KeyError\"").count() == 3, "{tree}");
        let main = main_of("ValueError = 0\nOSError = 1\nKeyError = 2\n", &[a]);
        let seen = names(&main);
        for n in ["__main__.ValueError", "__main__.OSError", "KeyError"] {
            assert!(seen.iter().any(|s| s == n), "{n} not in {seen:?}");
        }
    }
}
