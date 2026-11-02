//! The `pause` check (M6c, `docs/research/concurrency.md#the-rest-of-m6c-decided-nano-2026-11-02`).
//!
//! A pause point waits on a task: `fut.result()`, each step of a `for`
//! over `map`'s results, `list`, `sorted`, `set` or `in` on them, and a
//! call to a function that pauses. A function pauses when its body
//! holds a pause point or an executor's `with`, whose end waits for its
//! tasks; `submit` and `map` themselves wait for nothing. Tempered code
//! marks each pause point with `pause`, the block's end excepted, and
//! every `pause` must cover one. The marker changes nothing a program
//! does.

use std::collections::BTreeSet;

use crate::ast::{CmpOp, Expr, ExprKind, FuncDef, Mode, Module, Pause, Stmt};
use crate::diag::Result;
use crate::span::Span;

use super::{error, scope, Type, Types};

/// A place that waits on a task, and how a diagnostic names it.
struct Point {
    span: Span,
    what: String,
}

/// Checks `module`'s `pause` markers against its pause points: first
/// that each marker covers one, then that tempered code marks each.
pub(super) fn check(module: &Module, defs: &[&FuncDef], types: &Types) -> Result<()> {
    let pausing = pausing(defs, types);
    let each: Vec<(&FuncDef, Vec<Point>)> = defs
        .iter()
        .map(|f| (*f, points(&f.body, &pausing, types)))
        .collect();
    let top = points(&module.body, &pausing, types);
    let all: Vec<&Point> = top.iter().chain(each.iter().flat_map(|(_, p)| p)).collect();
    covered(&module.pauses, &all)?;
    let tempered = each.iter().filter(|(f, _)| f.mode == Mode::Tempered);
    tempered
        .map(|(f, mine)| marked(f, mine, &module.pauses))
        .collect()
}

/// The functions of `defs` that pause, directly or through a call.
fn pausing<'a>(defs: &[&'a FuncDef], types: &Types) -> BTreeSet<&'a str> {
    let mut out = BTreeSet::new();
    loop {
        let grew: Vec<&str> = defs
            .iter()
            .filter(|f| !out.contains(f.name.name.as_str()))
            .filter(|f| pauses(f, &out, types))
            .map(|f| f.name.name.as_str())
            .collect();
        if grew.is_empty() {
            return out;
        }
        out.extend(grew);
    }
}

/// Whether `f` pauses, given the functions known to.
fn pauses(f: &FuncDef, pausing: &BTreeSet<&str>, types: &Types) -> bool {
    !scope::withs(&f.body).is_empty() || !points(&f.body, pausing, types).is_empty()
}

/// The pause points of `block`, nested blocks included.
fn points(block: &[Stmt], pausing: &BTreeSet<&str>, types: &Types) -> Vec<Point> {
    let steps = scope::for_iters(block)
        .into_iter()
        .filter(|e| results(e, types))
        .map(|e| Point {
            span: e.span,
            what: "each step of this loop".to_string(),
        });
    let waits = scope::exprs(block).into_iter().filter_map(|e| {
        let what = wait(e, pausing, types)?;
        Some(Point { span: e.span, what })
    });
    steps.chain(waits).collect()
}

/// How a diagnostic names `e`, if evaluating it waits on a task beyond
/// what its parts wait on.
fn wait(e: &Expr, pausing: &BTreeSet<&str>, types: &Types) -> Option<String> {
    match &e.kind {
        ExprKind::Call { func, args } => match &func.kind {
            ExprKind::Attr { value, attr } if attr.name == "result" => {
                let future = matches!(types.at(value.span), Some(Type::Future(_)));
                future.then(|| "`result()`".to_string())
            }
            ExprKind::Name(n) if pausing.contains(n.as_str()) => Some(format!("calling `{n}`")),
            ExprKind::Name(n) if matches!(n.as_str(), "list" | "sorted" | "set") => {
                let reads = args.iter().any(|a| results(&a.value, types));
                reads.then(|| format!("`{n}` of `map`'s results"))
            }
            _ => None,
        },
        ExprKind::Compare { rest, .. } => {
            let member = |op: &CmpOp| matches!(op, CmpOp::In | CmpOp::NotIn);
            let reads = rest.iter().any(|(op, x)| member(op) && results(x, types));
            reads.then(|| "`in` on `map`'s results".to_string())
        }
        _ => None,
    }
}

/// Whether `e` is `map`'s results.
fn results(e: &Expr, types: &Types) -> bool {
    matches!(types.at(e.span), Some(Type::Results(_)))
}

/// Tempered `f` marks each of its pause points.
fn marked(f: &FuncDef, points: &[Point], pauses: &[Pause]) -> Result<()> {
    let bare = points
        .iter()
        .find(|p| !pauses.iter().any(|m| scope::within(p.span, m.covers)));
    let Some(p) = bare else {
        return Ok(());
    };
    let msg = format!(
        "{} waits on a task; tempered `{}` marks it: write `pause` before it",
        p.what, f.name.name
    );
    Err(error(p.span, msg))
}

/// Every `pause` covers a pause point.
fn covered(pauses: &[Pause], points: &[&Point]) -> Result<()> {
    let empty = pauses
        .iter()
        .find(|m| !points.iter().any(|p| scope::within(p.span, m.covers)));
    match empty {
        Some(m) => Err(error(m.keyword, "nothing here can pause".to_string())),
        None => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use crate::{infer, lexer, parser};

    const HEAD: &str = "import concurrent.futures\n\
temper def sq(x: int) -> int:\n    return x * x\n";

    /// `check`'s verdict on `HEAD` then `src`.
    fn check(src: &str) -> Result<(), String> {
        let src = format!("{HEAD}{src}");
        let tokens = lexer::lex(&src).map_err(|d| d.message)?;
        let module = parser::parse(&tokens).map_err(|d| d.message)?;
        infer::infer(&module).map(drop).map_err(|d| d.message)
    }

    /// A function `main`, tempered or loose, whose executor block is
    /// `body`, each line indented under the `with`.
    fn main(temper: bool, body: &str) -> String {
        let lines: String = body.lines().map(|l| format!("        {l}\n")).collect();
        let mode = if temper { "temper " } else { "" };
        format!(
            "{mode}def main() -> None:\n    \
             with concurrent.futures.ThreadPoolExecutor() as ex:\n{lines}"
        )
    }

    fn rejects(src: &str, want: &str) {
        let got = check(src).err().unwrap_or_default();
        assert!(got.contains(want), "{src}\nwant {want:?}\ngot  {got:?}");
    }

    /// Each pause point in tempered code needs its marker, and the marked
    /// form is accepted.
    #[test]
    fn tempered_code_marks_each_pause_point() {
        let cases = [
            (
                "print(ex.submit(sq, 1).result())",
                "print(pause ex.submit(sq, 1).result())",
                "`result()`",
            ),
            (
                "for r in ex.map(sq, [1]):\n    print(r)",
                "for r in pause ex.map(sq, [1]):\n    print(r)",
                "each step of this loop",
            ),
            (
                "rs = ex.map(sq, [1])\nfor r in rs:\n    print(r)",
                "rs = ex.map(sq, [1])\nfor r in pause rs:\n    print(r)",
                "each step of this loop",
            ),
            (
                "print(list(ex.map(sq, [1])))",
                "print(pause list(ex.map(sq, [1])))",
                "`list` of `map`'s results",
            ),
            (
                "print(sorted(ex.map(sq, [1])))",
                "print(pause sorted(ex.map(sq, [1])))",
                "`sorted` of `map`'s results",
            ),
            (
                "print(1 in ex.map(sq, [1]))",
                "print(pause 1 in ex.map(sq, [1]))",
                "`in` on `map`'s results",
            ),
        ];
        for (bare, marked, what) in cases {
            let want = format!("{what} waits on a task; tempered `main` marks it");
            rejects(&main(true, bare), &want);
            assert_eq!(check(&main(true, marked)), Ok(()), "{marked}");
            assert_eq!(check(&main(false, bare)), Ok(()), "loose: {bare}");
        }
    }

    /// Leaving the block waits for its tasks unmarked, but makes the
    /// function pause, so a tempered caller marks the call.
    #[test]
    fn a_function_with_an_executor_pauses_for_its_callers() {
        let inner = main(true, "ex.submit(sq, 1)");
        assert_eq!(check(&inner), Ok(()));
        let caller = "temper def top() -> None:\n    main()\n";
        rejects(
            &format!("{inner}{caller}"),
            "calling `main` waits on a task; tempered `top` marks it",
        );
        let marked = "temper def top() -> None:\n    pause main()\n";
        assert_eq!(check(&format!("{inner}{marked}")), Ok(()));
        let further = "temper def up() -> None:\n    pause top()\n";
        assert_eq!(check(&format!("{inner}{marked}{further}")), Ok(()));
        assert_eq!(check(&format!("{inner}pause main()\nmain()\n")), Ok(()));
    }

    /// A marker covering no pause point is an error, loose or tempered;
    /// `submit` and `map` wait for nothing, and `list(pause ...)` covers
    /// only the argument.
    #[test]
    fn a_pause_must_cover_a_pause_point() {
        let empty = [
            "print(pause sq(1))",
            "f = pause ex.submit(sq, 1)",
            "rs = pause ex.map(sq, [1])\nprint(pause list(rs))",
            "print(list(pause ex.map(sq, [1])))",
        ];
        for body in empty {
            for temper in [false, true] {
                rejects(&main(temper, body), "nothing here can pause");
            }
        }
        rejects("x = pause 1\n", "nothing here can pause");
    }

    /// Nested markers and a marker covering several points are accepted.
    #[test]
    fn one_marker_may_cover_several_points() {
        let body = "a = ex.submit(sq, 1)\nb = ex.submit(sq, 2)\n\
                    print(pause a.result() + b.result())\nprint(pause sq(pause a.result()))";
        assert_eq!(check(&main(true, body)), Ok(()));
    }
}
