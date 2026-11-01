//! Tasks (M6c, `docs/research/concurrency.md`): typing an executor's
//! `submit` and `map` and a future's `result`, and, once every type is
//! known, where executors, futures and `map`'s results may go.
//!
//! They stay in the function that made them (Nano, 2026-11-01): no
//! parameter, result, global or call argument holds one, only a local
//! list may hold futures, an executor is named only by its `with` and
//! used inside it, `map`'s results are used inside the block of the
//! executor that made them, and no executor is made at module level.
//! `raises` relies on this: a body's `result()` can only take the
//! result of a task that body started.

use std::collections::BTreeSet;

use crate::ast::{Arg, Expr, ExprKind, FuncDef, Ident, Module, Stmt, StmtKind};
use crate::diag::Result;
use crate::span::Span;
use crate::stdlib::StdFn;

use super::scope::{self, Blocks, With};
use super::solve::Need;
use super::ty::{FuncType, Type};
use super::{error, Checker, Func, Types};

/// Why a value of one of these types cannot go somewhere.
const MADE: &str =
    "an executor is used only as `with concurrent.futures.ThreadPoolExecutor() as ex:`";

const STAY: &str = "executors, futures and `map`'s results stay in the function that made them";

impl Checker {
    /// `ex.submit(f, *args)`, a future of `f`'s result, or `ex.map(f,
    /// xs)`, `f`'s results over `xs`. `f` is a `def`, its arguments
    /// checked as a call's.
    pub(super) fn task(&mut self, value: &Expr, attr: &Ident, args: &[Arg]) -> Result<Type> {
        let method = attr.name.as_str();
        let receiver = self.expr(value)?;
        let needs = format!("`{method}` needs an executor");
        self.expect(value.span, &Type::Executor, &receiver, &needs)?;
        if let Some(arg) = args.iter().find(|a| a.name.is_some()) {
            let msg = format!("`{method}` takes positional arguments only yet");
            return Err(error(arg.span, msg));
        }
        let Some(first) = args.first() else {
            let msg = format!("`{method}` needs the function to run first");
            return Err(error(attr.span, msg));
        };
        let (name, sig) = self.task_func(method, first)?;
        if method == "submit" {
            let span = attr.span.merge(args[args.len() - 1].span);
            self.bind_args(&name, &sig, &args[1..], span)?;
            return Ok(Type::Future(Box::new(sig.ret)));
        }
        let [_, xs] = args else {
            let msg = format!(
                "`map` takes a function and one iterable, {} given",
                args.len()
            );
            return Err(error(attr.span, msg));
        };
        self.mapped(&name, &sig, xs)?;
        Ok(Type::Results(Box::new(sig.ret)))
    }

    /// `fut.result()`: what the task returned.
    pub(super) fn result(&mut self, at: Span, receiver: &Type) -> Result<Type> {
        let out = self.subst.fresh();
        let future = Type::Future(Box::new(out.clone()));
        self.expect(
            at,
            &future,
            receiver,
            "`result` needs a future from `submit`",
        )?;
        Ok(out)
    }

    /// The `def` a task runs, by name, and its signature at this use.
    fn task_func(&mut self, method: &str, arg: &Arg) -> Result<(String, FuncType)> {
        let not_def = || {
            let msg = format!("`{method}` takes the name of a function defined with `def`");
            error(arg.value.span, msg)
        };
        let ExprKind::Name(name) = &arg.value.kind else {
            return Err(not_def());
        };
        let variable = self.frame.locals.contains_key(name) || self.globals.contains_key(name);
        let sig = match self.funcs.get(name).cloned() {
            _ if variable => return Err(not_def()),
            Some(Func::Mono(sig)) => sig,
            Some(Func::Poly(scheme)) => self.instantiate(name, &scheme, arg.span),
            None => return Err(not_def()),
        };
        Ok((name.clone(), sig))
    }

    /// `map` calls `f` with each element of `xs`, as its one argument.
    fn mapped(&mut self, name: &str, sig: &FuncType, xs: &Arg) -> Result<()> {
        let required = sig.params.iter().filter(|p| !p.has_default).count();
        let Some(param) = sig.params.first().filter(|_| required <= 1) else {
            let msg = format!("`map` passes `{name}` one argument at a time");
            return Err(error(xs.span, msg));
        };
        let iterable = self.expr(&xs.value)?;
        let elem = self.subst.fresh();
        let need = Need::Iter {
            iterable,
            elem: elem.clone(),
        };
        self.need(need, xs.value.span);
        let what = format!("argument `{}` of `{name}`", param.name);
        self.expect(xs.value.span, &param.ty, &elem, &what)
    }
}

/// Whether a value of `t` is or holds an executor, future or `map`'s
/// results.
fn holds_task(t: &Type) -> bool {
    match t {
        Type::Executor | Type::Future(_) | Type::Results(_) => true,
        Type::List(t) | Type::Set(t) | Type::Optional(t) | Type::View(_, t) => holds_task(t),
        Type::Dict(k, v) => holds_task(k) || holds_task(v),
        Type::Tuple(ts) => ts.iter().any(holds_task),
        _ => false,
    }
}

/// Whether a local of type `t` is one of the forms a function may keep:
/// an executor, `map`'s results, a future or a list of futures, of a
/// result that holds none of them.
fn local_form(t: &Type) -> bool {
    match t {
        Type::Executor => true,
        Type::Results(r) | Type::Future(r) => !holds_task(r),
        Type::List(item) => matches!(&**item, Type::Future(r) if !holds_task(r)),
        _ => !holds_task(t),
    }
}

/// Checks where executors, futures and `map`'s results go in `module`.
pub(super) fn check(module: &Module, defs: &[&FuncDef], types: &Types) -> Result<()> {
    let top: Vec<&Stmt> = module
        .body
        .iter()
        .filter(|s| !matches!(s.kind, StmtKind::FuncDef(_)))
        .collect();
    for s in &top {
        if let Some(w) = scope::withs(std::slice::from_ref(*s)).first() {
            let msg = "an executor at module level is not supported; \
                       move the `with` into a function";
            return Err(error(w.span, msg.to_string()));
        }
    }
    for f in defs {
        signature(f, types)?;
        Body::new(f, types).check()?;
    }
    let module_exprs = top
        .iter()
        .flat_map(|s| scope::exprs(std::slice::from_ref(*s)));
    for e in module_exprs {
        if types.at(e.span).is_some_and(holds_task) {
            let msg = format!("module-level code cannot hold one: {STAY}");
            return Err(error(e.span, msg));
        }
    }
    Ok(())
}

/// No parameter or result holds one.
fn signature(f: &FuncDef, types: &Types) -> Result<()> {
    for p in &f.params {
        if types.at(p.name.span).is_some_and(holds_task) {
            let msg = format!("parameter `{}` cannot hold one: {STAY}", p.name.name);
            return Err(error(p.name.span, msg));
        }
    }
    let ret = types.func(&f.name.name).map(|sig| &sig.ret);
    if ret.is_some_and(holds_task) {
        let msg = format!("`{}` cannot return one: {STAY}", f.name.name);
        return Err(error(f.name.span, msg));
    }
    Ok(())
}

/// One function body's uses.
struct Body<'a> {
    f: &'a FuncDef,
    types: &'a Types,
    locals: BTreeSet<String>,
    exprs: Vec<&'a Expr>,
    withs: Vec<With<'a>>,
    blocks: Blocks,
}

impl<'a> Body<'a> {
    fn new(f: &'a FuncDef, types: &'a Types) -> Self {
        let mut locals = scope::assigned(&f.body);
        locals.extend(f.params.iter().map(|p| p.name.name.clone()));
        Body {
            f,
            types,
            locals,
            exprs: scope::exprs(&f.body),
            withs: scope::withs(&f.body),
            blocks: Blocks::of(&f.body),
        }
    }

    fn check(&self) -> Result<()> {
        self.fresh()?;
        for e in &self.exprs {
            self.made_by_with(e)?;
            self.name(e)?;
            self.call(e)?;
        }
        self.rebound()?;
        self.results()
    }

    fn ty(&self, e: &Expr) -> Option<&'a Type> {
        self.types.at(e.span)
    }

    /// Each `with` opens a fresh executor, so no block shuts down one
    /// that another still uses.
    fn fresh(&self) -> Result<()> {
        let fresh = |w: &&With| {
            matches!(&w.value.kind, ExprKind::Call { func, .. }
                if matches!(&func.kind, ExprKind::Name(n) if n == StdFn::Executor.name()))
        };
        if let Some(w) = self.withs.iter().find(|w| !fresh(w)) {
            return Err(error(w.value.span, MADE.to_string()));
        }
        Ok(())
    }

    /// No assignment names an executor: a name holding one always means
    /// its own `with`'s.
    fn rebound(&self) -> Result<()> {
        let rebound = scope::name_assigns(&self.f.body)
            .into_iter()
            .find(|(_, v)| self.ty(v) == Some(&Type::Executor));
        let Some((n, v)) = rebound else {
            return Ok(());
        };
        let msg = format!("`{n}` is an executor: name one only with `with ... as {n}:`");
        Err(error(v.span, msg))
    }

    /// An executor is made only as a `with`'s value.
    fn made_by_with(&self, e: &Expr) -> Result<()> {
        let ExprKind::Call { func, .. } = &e.kind else {
            return Ok(());
        };
        let made = matches!(&func.kind, ExprKind::Name(n) if n == StdFn::Executor.name());
        if !made || self.withs.iter().any(|w| w.value.span == e.span) {
            return Ok(());
        }
        Err(error(e.span, MADE.to_string()))
    }

    /// Every value is one of [`local_form`]; a name holding one is a
    /// local, and an executor is a `with`'s name, used inside its block.
    fn name(&self, e: &Expr) -> Result<()> {
        let Some(t) = self.ty(e).filter(|t| holds_task(t)) else {
            return Ok(());
        };
        let ExprKind::Name(n) = &e.kind else {
            if local_form(t) {
                return Ok(());
            }
            let msg = format!("this is {t}: only a local list may hold futures");
            return Err(error(e.span, msg));
        };
        if !self.locals.contains(n) {
            return Err(error(
                e.span,
                format!("global `{n}` cannot hold one: {STAY}"),
            ));
        }
        if !local_form(t) {
            let msg = format!("`{n}` is {t}: only a local list may hold futures");
            return Err(error(e.span, msg));
        }
        if *t != Type::Executor {
            return Ok(());
        }
        let mine: Vec<With> = self.named(n);
        if mine.is_empty() {
            let msg = format!("`{n}` is an executor: name one only with `with ... as {n}:`");
            return Err(error(e.span, msg));
        }
        if self.reaching(&mine, e.span).iter().all(|w| w.holds(e.span)) {
            return Ok(());
        }
        let msg = format!("`{n}` is used outside its `with` block");
        Err(error(e.span, msg))
    }

    /// No call takes one, but the methods that make and take them,
    /// `append` to a local list, and `len`, `list`, `sorted` and `set`, which
    /// read them where they are.
    fn call(&self, e: &Expr) -> Result<()> {
        let ExprKind::Call { func, args } = &e.kind else {
            return Ok(());
        };
        let (callee, checked) = match &func.kind {
            ExprKind::Name(n) if matches!(n.as_str(), "len" | "list" | "sorted" | "set") => {
                return Ok(())
            }
            ExprKind::Name(n) => (format!("`{n}`"), args.as_slice()),
            ExprKind::Attr { attr, .. } if attr.name == "append" => return Ok(()),
            ExprKind::Attr { attr, .. } if matches!(attr.name.as_str(), "submit" | "map") => (
                format!("a task of `{}`", attr.name),
                args.get(1..).unwrap_or(&[]),
            ),
            ExprKind::Attr { attr, .. } => (format!("`.{}`", attr.name), args.as_slice()),
            _ => return Ok(()),
        };
        let Some(arg) = checked
            .iter()
            .find(|a| self.ty(&a.value).is_some_and(holds_task))
        else {
            return Ok(());
        };
        let msg = format!("{callee} cannot take one: {STAY}");
        Err(error(arg.value.span, msg))
    }

    /// `map`'s results are bound only from `ex.map(...)`, and every use
    /// is inside the `with` block of each binding that may reach it.
    fn results(&self) -> Result<()> {
        let mut binds: Vec<(&str, Span, With<'a>)> = Vec::new();
        for (name, value) in scope::name_assigns(&self.f.body) {
            if !matches!(self.ty(value), Some(Type::Results(_))) {
                continue;
            }
            let Some(w) = self.map_with(value) else {
                let msg = format!("bind `{name}` only from `ex.map(...)`");
                return Err(error(value.span, msg));
            };
            binds.push((name, value.span, w));
        }
        for (name, _, _) in &binds {
            let mine: Vec<(Span, With)> = binds
                .iter()
                .filter(|b| b.0 == *name)
                .map(|b| (b.1, b.2))
                .collect();
            self.results_used(name, &mine)?;
        }
        Ok(())
    }

    /// Every use of `name` lies in the `with` block of each binding in
    /// `mine` (its value's span and its `with`) that may reach it.
    fn results_used(&self, name: &str, mine: &[(Span, With<'a>)]) -> Result<()> {
        let defs: Vec<Span> = mine.iter().map(|b| b.0).collect();
        let uses = self
            .exprs
            .iter()
            .filter(|e| matches!(&e.kind, ExprKind::Name(n) if n == name));
        for e in uses {
            let reach = self.blocks.reaching(&defs, e.span);
            if !reach.into_iter().all(|i| mine[i].1.holds(e.span)) {
                let msg =
                    format!("`{name}` holds `map`'s results, used only inside its `with` block");
                return Err(error(e.span, msg));
            }
        }
        Ok(())
    }

    /// The `with` whose executor `value`, a call `ex.map(...)`, uses: the
    /// latest that may reach it. [`Body::name`] has checked that each
    /// one that may holds the call.
    fn map_with(&self, value: &Expr) -> Option<With<'a>> {
        let ExprKind::Call { func, .. } = &value.kind else {
            return None;
        };
        let ExprKind::Attr { value: ex, attr } = &func.kind else {
            return None;
        };
        let ExprKind::Name(ex) = &ex.kind else {
            return None;
        };
        let reaching = self.reaching(&self.named(ex), value.span);
        let latest = reaching.into_iter().max_by_key(|w| w.span.start);
        latest.filter(|_| attr.name == "map")
    }

    /// The `with`s that bind `name`.
    fn named(&self, name: &str) -> Vec<With<'a>> {
        let mine = self.withs.iter().filter(|w| w.binding.name == name);
        mine.copied().collect()
    }

    /// Which of `withs` may have bound the executor a use at `at` names.
    fn reaching(&self, withs: &[With<'a>], at: Span) -> Vec<With<'a>> {
        let defs: Vec<Span> = withs.iter().map(|w| w.binding.span).collect();
        let reach = self.blocks.reaching(&defs, at);
        reach.into_iter().map(|i| withs[i]).collect()
    }
}

#[cfg(test)]
mod tests {
    use crate::{infer, lexer, parser};

    const HEAD: &str = "import concurrent.futures\n\
def sq(x: int) -> int:\n    return x * x\n\
def add(a: int, b: int) -> int:\n    return a + b\n\
def keep(fut):\n    pass\n";

    /// `check`'s verdict on `HEAD` and a `main` whose executor block is
    /// `body`, each line indented under the `with`.
    fn verdict(body: &str) -> Result<(), String> {
        let lines: String = body.lines().map(|l| format!("        {l}\n")).collect();
        let src = format!(
            "{HEAD}def main() -> None:\n    \
             with concurrent.futures.ThreadPoolExecutor() as ex:\n{lines}main()\n"
        );
        program(&src)
    }

    fn program(src: &str) -> Result<(), String> {
        let tokens = lexer::lex(src).map_err(|d| d.message)?;
        let module = parser::parse(&tokens).map_err(|d| d.message)?;
        infer::infer(&module).map(drop).map_err(|d| d.message)
    }

    fn err(body: &str) -> String {
        verdict(body)
            .err()
            .unwrap_or_else(|| panic!("no error for:\n{body}"))
    }

    fn rejects(body: &str, want: &str) {
        let got = err(body);
        assert!(got.contains(want), "{body}\nwant {want:?}\ngot  {got:?}");
    }

    #[test]
    fn submit_and_map_check_their_calls() {
        let fine = "futs = [ex.submit(sq, 1)]\nfuts.append(ex.submit(add, 1, 2))\n\
                    print(futs[0].result() + 1, len(futs))\n\
                    for r in ex.map(sq, range(3)):\n    print(r)\n\
                    print(list(ex.map(sq, [1])), sorted(ex.map(sq, {2})))";
        assert_eq!(verdict(fine), Ok(()));
        rejects("ex.submit(sq, \"a\")", "argument `x` of `sq`");
        rejects("ex.submit(add, 1)", "`add` is missing argument `b`");
        rejects(
            "ex.submit(sq, x=1)",
            "`submit` takes positional arguments only yet",
        );
        rejects("ex.submit()", "`submit` needs the function to run first");
        rejects(
            "ex.map(add, [1])",
            "`map` passes `add` one argument at a time",
        );
        rejects("ex.map(sq, [\"a\"])", "expected int, found str");
        rejects(
            "ex.map(sq, [1], [2])",
            "`map` takes a function and one iterable",
        );
        rejects(
            "ex.submit(len, [1])",
            "takes the name of a function defined with `def`",
        );
        rejects(
            "sq = 1\nex.submit(sq, 1)",
            "takes the name of a function defined with `def`",
        );
        rejects("[1].result()", "`result` needs a future from `submit`");
        rejects("ex.result()", "`result` needs a future from `submit`");
        rejects(
            "print(ex.submit(sq, 1).result() + \"a\")",
            "unsupported operand types for +: int and str",
        );
    }

    #[test]
    fn with_takes_an_executor() {
        let src = "def main() -> None:\n    with 3 as ex:\n        pass\n";
        let got = program(src).err().unwrap_or_default();
        assert!(got.contains("`with` takes an executor"), "{got}");
        let src = "import concurrent\n";
        let got = program(src).err().unwrap_or_default();
        assert!(got.contains("write `import concurrent.futures`"), "{got}");
        let src = "import concurrent.futures\nprint(concurrent.futures)\n";
        assert!(program(src).is_err());
    }

    #[test]
    fn a_use_names_the_latest_with_of_its_name() {
        const W: &str = "with concurrent.futures.ThreadPoolExecutor() as ex:";
        let main = |body: &str| format!("{HEAD}def main() -> None:\n{body}main()\n");
        let sequential = main(&format!(
            "    {W}\n        print(ex.submit(sq, 1).result())\n    \
             {W}\n        print(ex.submit(sq, 2).result())\n        \
             rs = ex.map(sq, [3])\n        print(list(rs))\n"
        ));
        assert_eq!(program(&sequential), Ok(()));
        let shadowed = main(&format!(
            "    {W}\n        {W}\n            print(1)\n        \
             print(ex.submit(sq, 2).result())\n"
        ));
        let got = program(&shadowed).err().unwrap_or_default();
        assert!(
            got.contains("`ex` is used outside its `with` block"),
            "{got}"
        );
        let results = main(&format!(
            "    {W}\n        rs = ex.map(sq, [1])\n        \
             {W}\n            rs = ex.map(sq, [2])\n        print(list(rs))\n"
        ));
        let got = program(&results).err().unwrap_or_default();
        assert!(got.contains("used only inside its `with` block"), "{got}");
    }

    #[test]
    fn a_use_checks_every_binding_that_may_reach_it() {
        const W: &str = "with concurrent.futures.ThreadPoolExecutor() as";
        let main = |body: &str| format!("{HEAD}def main(flag: bool) -> None:\n{body}main(False)\n");
        let branch = main(&format!(
            "    {W} outer:\n        {W} inner:\n            rs = inner.map(sq, [2])\n        \
             if flag:\n            rs = outer.map(sq, [3])\n        print(list(rs))\n"
        ));
        let got = program(&branch).err().unwrap_or_default();
        assert!(got.contains("used only inside its `with` block"), "{got}");
        let looped = main(&format!(
            "    {W} ex:\n        for i in range(2):\n            \
             print(ex.submit(sq, i).result())\n            {W} ex:\n                pass\n"
        ));
        let got = program(&looped).err().unwrap_or_default();
        assert!(
            got.contains("`ex` is used outside its `with` block"),
            "{got}"
        );
    }

    #[test]
    fn only_a_fresh_executor_opens_a_with_and_none_is_rebound() {
        const W: &str = "with concurrent.futures.ThreadPoolExecutor() as";
        let main = |body: &str| format!("{HEAD}def main() -> None:\n{body}main()\n");
        let reentered = main(&format!(
            "    {W} ex:\n        with ex as inner:\n            pass\n        \
             print(ex.submit(sq, 1).result())\n"
        ));
        let got = program(&reentered).err().unwrap_or_default();
        assert!(got.contains("an executor is used only as `with"), "{got}");
        let rebound = main(&format!(
            "    {W} a:\n        {W} b:\n            a = b\n        \
             print(a.submit(sq, 3).result())\n"
        ));
        let got = program(&rebound).err().unwrap_or_default();
        assert!(got.contains("name one only with `with ... as a:`"), "{got}");
    }

    #[test]
    fn futures_compare_by_identity_so_not_at_all() {
        let got = err("a = ex.submit(sq, 1)\nprint(a == a)");
        assert!(got.contains("Future[int] compares by identity"), "{got}");
    }

    #[test]
    fn executors_are_named_only_by_their_with_and_used_inside_it() {
        let made = format!(
            "{HEAD}def main() -> None:\n    \
             e = concurrent.futures.ThreadPoolExecutor()\n    print(e.submit(sq, 1).result())\n"
        );
        let got = program(&made).err().unwrap_or_default();
        assert!(got.contains("an executor is used only as `with"), "{got}");
        rejects(
            "e2 = ex\ne2.submit(sq, 1)",
            "`e2` is an executor: name one only with",
        );
        let after = format!(
            "{HEAD}def main() -> None:\n    \
             with concurrent.futures.ThreadPoolExecutor() as ex:\n        pass\n    \
             ex.submit(sq, 1)\n"
        );
        let got = program(&after).err().unwrap_or_default();
        assert!(
            got.contains("`ex` is used outside its `with` block"),
            "{got}"
        );
    }

    #[test]
    fn futures_and_results_stay_in_their_function() {
        rejects("keep(ex.submit(sq, 1))", "`keep` cannot take one");
        rejects("print(ex.submit(sq, 1))", "`print` cannot take one");
        rejects(
            "raise ValueError(ex.submit(sq, 1))",
            "`ValueError` cannot take one",
        );
        rejects(
            "f = ex.submit(sq, 1)\nex.submit(keep, f)",
            "a task of `submit` cannot take one",
        );
        rejects(
            "p = (ex.submit(sq, 1), 1)",
            "only a local list may hold futures",
        );
        rejects(
            "d = {1: ex.submit(sq, 1)}",
            "only a local list may hold futures",
        );
        rejects(
            "rs = ex.map(sq, [1])\nrs2 = rs",
            "bind `rs2` only from `ex.map(...)`",
        );
        let after = format!(
            "{HEAD}def main() -> None:\n    \
             with concurrent.futures.ThreadPoolExecutor() as ex:\n        \
             rs = ex.map(sq, [1])\n    print(list(rs))\n"
        );
        let got = program(&after).err().unwrap_or_default();
        assert!(got.contains("used only inside its `with` block"), "{got}");
        let inner = "rs = ex.map(sq, [1])\nfor r in rs:\n    print(r)";
        assert_eq!(verdict(inner), Ok(()));
        let global = format!(
            "{HEAD}def main() -> None:\n    \
             with concurrent.futures.ThreadPoolExecutor() as ex:\n        \
             G.append(ex.submit(sq, 1))\nG = []\nmain()\n"
        );
        let got = program(&global).err().unwrap_or_default();
        assert!(got.contains("global `G` cannot hold one"), "{got}");
        let param = format!("{HEAD}def f(fs):\n    return fs[0].result()\n");
        assert_eq!(program(&param), Ok(()), "unused generic is fine");
        let passed = format!(
            "{HEAD}def first(fs):\n    return fs[0]\n\
             def main() -> None:\n    \
             with concurrent.futures.ThreadPoolExecutor() as ex:\n        \
             print(first([ex.submit(sq, 1)]).result())\nmain()\n"
        );
        let got = program(&passed).err().unwrap_or_default();
        assert!(got.contains("`first` cannot take one"), "{got}");
    }

    #[test]
    fn no_executor_at_module_level_or_in_tempered_code() {
        let top = format!("{HEAD}with concurrent.futures.ThreadPoolExecutor() as ex:\n    pass\n");
        let got = program(&top).err().unwrap_or_default();
        assert!(
            got.contains("an executor at module level is not supported"),
            "{got}"
        );
        let tempered = format!(
            "{HEAD}temper def t() -> None:\n    \
             with concurrent.futures.ThreadPoolExecutor() as ex:\n        pass\n"
        );
        let got = program(&tempered).err().unwrap_or_default();
        assert!(got.contains("tempered `t` cannot use an executor"), "{got}");
    }
}
