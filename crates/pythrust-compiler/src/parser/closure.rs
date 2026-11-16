//! Nested `def` and closures that capture (step 6-3a, D200-D203): each
//! nested def is lifted to a hidden top-level def, as a lambda is (D195),
//! and each name its body or a lambda's reads from the enclosing function
//! becomes a leading parameter, passed by every call of the closure. So
//! inference, ownership, `raises`, `run` and lowering see ordinary defs.
//!
//! By reference as CPython, staged: a captured name is bound once in its
//! function, before the closure, so a closure can take its value as an
//! argument and never sees it change or unbound (D200, D204). A
//! closure is only called by name; its value escaping, which needs cells,
//! is the named error of step 6-3b-2.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use super::{error, Parser};
use crate::ast::{
    comp_name, source_name, Arg, Block, Expr, ExprKind, FuncDef, Ident, Mode, Param, ParamMode,
    Stmt, StmtKind, Target, TargetKind, CELL, CLOSURE,
};
use crate::diag::Result;
use crate::infer::scope;
use crate::span::Span;

/// The names each closure takes first, by the closure's hidden name.
type Captures = BTreeMap<String, Vec<String>>;

impl Parser<'_> {
    /// Closes the function that owns `body`: lifts its nested defs, then
    /// gives the closures lifted since `start` their captures.
    pub(super) fn close_function(
        &mut self,
        mode: Mode,
        params: &[Param],
        body: &mut Block,
        start: usize,
    ) -> Result<()> {
        check_nonlocal_order(params, body)?;
        let nested = extract(body)?;
        reject_def_of_nonlocal(&nested, body)?;
        reject_except_of_nonlocal(body)?;
        if !self.in_func {
            reject_nonlocal(body)?;
        }
        if mode == Mode::Tempered {
            if let Some(def) = nested.first() {
                return Err(error(def.span, TEMPERED));
            }
        }
        let own = declared_nonlocal(body);
        let mut bound = scope::assigned(body);
        bound.extend(params.iter().map(|p| p.name.name.clone()));
        // A name this function declares `nonlocal` is not its own variable.
        bound.retain(|n| !own.contains(n));
        let mut names = BTreeMap::new();
        let mut declared: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut forwarded: Vec<Ident> = Vec::new();
        for mut def in nested {
            let hidden = comp_name(&def.name.name, def.span.start);
            let (shared, outward) = nonlocals(&def, &bound);
            declared.insert(hidden.clone(), shared);
            forwarded.extend(outward);
            names.insert(def.name.name.clone(), hidden.clone());
            def.name.name = hidden;
            self.lifted.push(Stmt {
                span: def.span,
                kind: StmtKind::FuncDef(def),
            });
        }
        if let Some(clash) = names.keys().find(|n| bound.contains(*n)) {
            let msg = format!("`{clash}` is a nested `def` and also assigned or a parameter");
            return Err(error(span_of(body, clash), msg));
        }
        if let Some(first) = forwarded.first() {
            if !self.in_func {
                let msg = format!(
                    "no binding for nonlocal `{}` found in the enclosing function",
                    first.name
                );
                return Err(error(first.span, msg));
            }
            // A name of a function further out: this one forwards the cell.
            // Placed first, so no use or binding of it comes before.
            let at = Span { start: 0, end: 0 };
            body.insert(
                0,
                Stmt {
                    kind: StmtKind::Nonlocal(forwarded.clone()),
                    span: at,
                },
            );
        }
        let lifted = &mut self.lifted[start..];
        rename(body, lifted, &names)?;
        let captures = captures(lifted, &bound, &declared);
        if mode == Mode::Tempered {
            if let Some((name, _)) = captures.iter().find(|(_, c)| !c.is_empty()) {
                let at = lifted_span(lifted, name);
                return Err(error(at, TEMPERED));
            }
        }
        let celled = check_captures(&captures, &declared, &own, body, params, lifted)?;
        add_captures(body, lifted, &captures, &celled)
    }
}

/// The names `def` declares `nonlocal`, which are captured as cells, and the
/// ones of them that `bound` (the enclosing function's bindings) lacks: they
/// belong to a function further out, so the enclosing function forwards them.
fn nonlocals(def: &FuncDef, bound: &BTreeSet<String>) -> (Vec<String>, Vec<Ident>) {
    let (mut out, mut forwarded) = (Vec::new(), Vec::new());
    scope::walk(&def.body, &mut |s| {
        let StmtKind::Nonlocal(names) = &s.kind else {
            return;
        };
        for n in names {
            if !bound.contains(&n.name) {
                forwarded.push(n.clone());
            }
            out.push(n.name.clone());
        }
    });
    (out, forwarded)
}

/// Checks each `nonlocal` of a function's own source `body` (before its
/// nested defs are lifted and captures inserted): the name is no parameter,
/// and is neither read nor bound earlier in the function, as in Python.
fn check_nonlocal_order(params: &[Param], body: &Block) -> Result<()> {
    let own = declared_nonlocal(body);
    let mut clash = None;
    scope::walk(body, &mut |s| {
        // Python refuses an annotation on a nonlocal name, in either order.
        if let (StmtKind::AnnAssign { target, .. }, None) = (&s.kind, &clash) {
            if own.contains(&target.name) {
                let msg = format!("annotated name `{}` can't be nonlocal", target.name);
                clash = Some((target.span, msg));
            }
        }
    });
    scope::walk(body, &mut |s| {
        let StmtKind::Nonlocal(names) = &s.kind else {
            return;
        };
        for n in names {
            if clash.is_none() {
                clash = nonlocal_clash(params, body, n, s.span.start);
            }
        }
    });
    match clash {
        Some((span, msg)) => Err(error(span, msg)),
        None => Ok(()),
    }
}

/// Why `n`, declared `nonlocal` at `at`, is refused. A comprehension's own
/// scope does not count, but its first iterable is read here; a nested
/// def's name is bound here, its body is not read here.
fn nonlocal_clash(params: &[Param], body: &Block, n: &Ident, at: usize) -> Option<(Span, String)> {
    let name = &n.name;
    if params.iter().any(|p| &p.name.name == name) {
        return Some((n.span, format!("name `{name}` is parameter and nonlocal")));
    }
    let all = scope::exprs(body);
    let own_scope = |e: &Expr| {
        !all.iter().any(|c| {
            let ExprKind::Comp { clauses, .. } = &c.kind else {
                return false;
            };
            let inside = |outer: Span| outer.start <= e.span.start && e.span.end <= outer.end;
            inside(c.span) && !clauses.first().is_some_and(|f| inside(f.iter.span))
        })
    };
    let used = all.iter().any(|e| {
        matches!(&e.kind, ExprKind::Name(x) if x == name)
            && e.span.start < e.span.end
            && e.span.end <= at
            && own_scope(e)
    });
    if used {
        return Some((
            n.span,
            format!("name `{name}` is used prior to nonlocal declaration"),
        ));
    }
    let mut bound = false;
    scope::walk(body, &mut |s| {
        let def_name = matches!(&s.kind, StmtKind::FuncDef(d) if d.name.name == *name);
        bound |= (s.span.end <= at && (def_name || bindings(s).contains(name)))
            || header_binds(s, name, at);
    });
    bound.then(|| {
        (
            n.span,
            format!("name `{name}` is assigned to before nonlocal declaration"),
        )
    })
}

/// A nested `def` of a name the function declares `nonlocal` would replace
/// the shared variable; the lifted def makes no such store (step 6-3b-2b).
fn reject_def_of_nonlocal(nested: &[FuncDef], body: &Block) -> Result<()> {
    let own = declared_nonlocal(body);
    match nested.iter().find(|d| own.contains(&d.name.name)) {
        Some(d) => Err(error(
            d.name.span,
            format!(
                "a nested `def {}` cannot bind a `nonlocal` name yet (not supported yet)",
                d.name.name
            ),
        )),
        None => Ok(()),
    }
}

/// An `except ... as` of a `nonlocal` name would unbind the shared variable
/// when the handler ends; the cell cannot be empty yet (step 6-3b-2b).
fn reject_except_of_nonlocal(body: &Block) -> Result<()> {
    let own = declared_nonlocal(body);
    let mut bad: Option<Ident> = None;
    scope::walk(body, &mut |s| {
        let StmtKind::Try { handlers, .. } = &s.kind else {
            return;
        };
        let hit = handlers.iter().filter_map(|h| h.binding.as_ref());
        if let (Some(b), None) = (hit.into_iter().find(|b| own.contains(&b.name)), &bad) {
            bad = Some(b.clone());
        }
    });
    match bad {
        Some(b) => Err(error(
            b.span,
            format!(
                "`except ... as {}` would unbind a `nonlocal` name after the handler; that \
                 needs cells that may be unbound (not supported yet)",
                b.name
            ),
        )),
        None => Ok(()),
    }
}

/// The names `body` itself declares `nonlocal`.
fn declared_nonlocal(body: &Block) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    scope::walk(body, &mut |s| {
        if let StmtKind::Nonlocal(names) = &s.kind {
            out.extend(names.iter().map(|n| n.name.clone()));
        }
    });
    out
}

/// A `nonlocal` in a function no function encloses.
fn reject_nonlocal(body: &Block) -> Result<()> {
    let mut bad = None;
    scope::walk(body, &mut |s| {
        if let (StmtKind::Nonlocal(names), None) = (&s.kind, &bad) {
            bad = names.first().cloned();
        }
    });
    match bad {
        Some(n) => Err(error(
            n.span,
            format!(
                "no binding for nonlocal `{}` found in the enclosing function",
                n.name
            ),
        )),
        None => Ok(()),
    }
}

const TEMPERED: &str = "a closure inside a tempered function is not supported yet (step 6-3b-2)";

/// The nested defs at the top of `body`, taken out of it. A `def` in a
/// block below the top is an error: it would run conditionally or in a
/// loop (D202).
fn extract(body: &mut Block) -> Result<Vec<FuncDef>> {
    let mut defs = Vec::new();
    let mut rest = Vec::new();
    for stmt in std::mem::take(body) {
        match stmt.kind {
            StmtKind::FuncDef(def) => defs.push(def),
            kind => rest.push(Stmt { kind, ..stmt }),
        }
    }
    *body = rest;
    let mut below = None;
    scope::walk(body, &mut |s| {
        if below.is_none() && matches!(s.kind, StmtKind::FuncDef(_)) {
            below = Some(s.span);
        }
    });
    if let Some(span) = below {
        let msg = "a nested `def` must be at the top of its function body (not supported yet)";
        return Err(error(span, msg));
    }
    for def in &defs {
        if let Some(p) = def.params.iter().find(|p| p.default.is_some()) {
            let msg = "a nested `def` with a default is not supported yet";
            return Err(error(p.span, msg));
        }
    }
    Ok(defs)
}

/// The span of the first use of `name` in `body`, else of the body.
fn span_of(body: &Block, name: &str) -> Span {
    let uses = scope::exprs(body);
    let at = uses
        .into_iter()
        .find(|e| matches!(&e.kind, ExprKind::Name(n) if n == name));
    at.map_or_else(
        || body.first().map_or(Span { start: 0, end: 0 }, |s| s.span),
        |e| e.span,
    )
}

fn lifted_span(lifted: &[Stmt], name: &str) -> Span {
    let def = lifted.iter().find_map(|s| match &s.kind {
        StmtKind::FuncDef(d) if d.name.name == name => Some(d.span),
        _ => None,
    });
    def.unwrap_or(Span { start: 0, end: 0 })
}

/// Renames each nested def's source name to its hidden one in `body` and
/// the closures, where nothing local shadows it. A use before the def, or
/// inside an earlier sibling, is an error: CPython finds the name unbound
/// there (D202).
fn rename(body: &mut Block, lifted: &mut [Stmt], names: &BTreeMap<String, String>) -> Result<()> {
    if names.is_empty() {
        return Ok(());
    }
    let starts: BTreeMap<&str, usize> = lifted
        .iter()
        .filter_map(|s| match &s.kind {
            StmtKind::FuncDef(d) => Some((d.name.name.as_str(), d.span.start)),
            _ => None,
        })
        .collect();
    let starts: BTreeMap<String, usize> = names
        .iter()
        .filter_map(|(n, h)| starts.get(h.as_str()).map(|s| (n.clone(), *s)))
        .collect();
    let mut late = None;
    let none = BTreeSet::new();
    rename_in(
        body,
        &RenameCx {
            names,
            starts: &starts,
            shadows: &none,
            owner: None,
        },
        &mut late,
    );
    for stmt in lifted.iter_mut() {
        let StmtKind::FuncDef(def) = &mut stmt.kind else {
            continue;
        };
        let mut shadows = scope::assigned(&def.body);
        shadows.extend(def.params.iter().map(|p| p.name.name.clone()));
        let cx = RenameCx {
            names,
            starts: &starts,
            shadows: &shadows,
            owner: Some(def.span.start),
        };
        rename_in(&mut def.body, &cx, &mut late);
    }
    match late {
        Some((span, name)) => {
            let msg = format!("`{name}` is used before its `def` (not supported yet)");
            Err(error(span, msg))
        }
        None => Ok(()),
    }
}

struct RenameCx<'a> {
    names: &'a BTreeMap<String, String>,
    starts: &'a BTreeMap<String, usize>,
    shadows: &'a BTreeSet<String>,
    /// Where the def being renamed inside starts: its own name is fine.
    owner: Option<usize>,
}

fn rename_in(block: &mut Block, cx: &RenameCx<'_>, late: &mut Option<(Span, String)>) {
    walk_mut(block, &mut |e: &mut Expr| {
        let ExprKind::Name(n) = &mut e.kind else {
            return;
        };
        let Some(hidden) = cx.names.get(n.as_str()) else {
            return;
        };
        if cx.shadows.contains(n.as_str()) {
            return;
        }
        let start = cx.starts[n.as_str()];
        if cx.owner != Some(start) && e.span.start < start {
            *late = Some((e.span, n.clone()));
        }
        *n = hidden.clone();
    });
}

/// What each closure of `lifted` takes: the names of `bound` it reads,
/// and those of the closures it calls, to a fixed point.
fn captures(
    lifted: &[Stmt],
    bound: &BTreeSet<String>,
    declared: &BTreeMap<String, Vec<String>>,
) -> Captures {
    let defs: Vec<&FuncDef> = lifted
        .iter()
        .filter_map(|s| match &s.kind {
            StmtKind::FuncDef(d) => Some(d),
            _ => None,
        })
        .collect();
    let hidden: BTreeSet<&str> = defs.iter().map(|d| d.name.name.as_str()).collect();
    let mut direct: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    let mut calls: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for d in &defs {
        let uses: BTreeSet<&str> = scope::name_uses(d).into_iter().map(|(n, _)| n).collect();
        let reads = uses.iter().filter(|n| bound.contains(**n));
        let nonlocal = declared.get(&d.name.name).into_iter().flatten().cloned();
        direct.insert(
            &d.name.name,
            reads.map(|n| n.to_string()).chain(nonlocal).collect(),
        );
        let callees = uses.iter().filter(|n| hidden.contains(**n));
        calls.insert(&d.name.name, callees.copied().collect());
    }
    let mut taken = direct.clone();
    let mut changed = true;
    while changed {
        changed = false;
        for (d, callees) in &calls {
            let inherited: Vec<String> = callees
                .iter()
                .flat_map(|c| taken[c].iter().cloned())
                .collect();
            let mine = taken.get_mut(d).into_iter().next();
            if let Some(mine) = mine {
                let before = mine.len();
                mine.extend(inherited);
                changed |= mine.len() > before;
            }
        }
    }
    taken
        .into_iter()
        .map(|(d, names)| (d.to_string(), names.into_iter().collect()))
        .collect()
}

/// Which captured names live in cells (D209, D215): one rebound after the
/// closure, declared `nonlocal` by some closure, or bound later or only
/// conditionally (it may be unbound when read). Any other is captured by
/// value.
fn check_captures(
    captures: &Captures,
    declared: &BTreeMap<String, Vec<String>>,
    own: &BTreeSet<String>,
    body: &Block,
    params: &[Param],
    lifted: &[Stmt],
) -> Result<BTreeSet<String>> {
    let mut count = binding_counts(body, params);
    count.retain(|n, _| !own.contains(n));
    let mut celled: BTreeSet<String> = declared.values().flatten().cloned().collect();
    for (closure, names) in captures {
        let at = lifted_span(lifted, closure);
        celled.extend(
            names
                .iter()
                .filter(|n| count.get(*n).is_some_and(|c| *c > 1))
                .cloned(),
        );
        // A name bound later or only conditionally may be unbound when the
        // closure reads it (D215): it is a cell too.
        let is_param = |n: &str| params.iter().any(|p| p.name.name == n);
        celled.extend(
            names
                .iter()
                .filter(|n| count.contains_key(*n) && !is_param(n))
                .filter(|n| !assigned_before(body, n, at.start))
                .cloned(),
        );
    }
    Ok(celled)
}

/// How many times each name of the function is bound, counting twice a
/// binding that may run again or always rebinds (D204): one in a loop's
/// body, a `for` target, a `with ... as` and an `except ... as`.
fn binding_counts(body: &Block, params: &[Param]) -> BTreeMap<String, usize> {
    let mut count: BTreeMap<String, usize> = BTreeMap::new();
    for p in params {
        *count.entry(p.name.name.clone()).or_default() += 1;
    }
    scope::walk_looped(body, false, &mut |s, looped| {
        let always = matches!(
            s.kind,
            StmtKind::For { .. } | StmtKind::With { .. } | StmtKind::Try { .. }
        );
        let weight = if looped || always { 2 } else { 1 };
        for name in bindings(s) {
            *count.entry(name).or_default() += weight;
        }
    });
    count
}

/// Whether a top-level assignment of `body` that ends before `start`
/// binds `name`: one holding the closure binds only after it is made.
fn assigned_before(body: &Block, name: &str, start: usize) -> bool {
    in_loop_of(body, name, start)
        || body.iter().filter(|s| s.span.end <= start).any(|s| {
            let mut out = BTreeSet::new();
            match &s.kind {
                StmtKind::Assign { target, .. } => scope::target_names(target, &mut out),
                StmtKind::AnnAssign { target, .. } => {
                    out.insert(target.name.clone());
                }
                _ => {}
            }
            out.contains(name)
        })
}

/// Whether `start` is inside the body of a `for` whose target binds `name`:
/// the target is bound by then, and rebinds each turn (a late-binding cell).
fn in_loop_of(body: &Block, name: &str, start: usize) -> bool {
    let mut found = false;
    scope::walk(body, &mut |s| {
        let StmtKind::For { target, body, .. } = &s.kind else {
            return;
        };
        let inside = body.first().is_some_and(|b| b.span.start <= start)
            && body.last().is_some_and(|b| start < b.span.end);
        let mut out = BTreeSet::new();
        scope::target_names(target, &mut out);
        found |= inside && out.contains(name);
    });
    found
}

/// Whether the header of compound statement `s`, which holds `at`, binds
/// `name` before it: a `for` target, a `with ... as` or an `except ... as`.
fn header_binds(s: &Stmt, name: &str, at: usize) -> bool {
    match &s.kind {
        StmtKind::For { target, .. } => {
            let mut out = BTreeSet::new();
            scope::target_names(target, &mut out);
            out.contains(name) && target.span.end <= at
        }
        StmtKind::With { binding, .. } => binding.name == name && binding.span.end <= at,
        StmtKind::Try { handlers, .. } => handlers
            .iter()
            .filter_map(|h| h.binding.as_ref())
            .any(|b| b.name == name && b.span.end <= at),
        _ => false,
    }
}

/// The names a statement binds, each time it does.
fn bindings(s: &Stmt) -> Vec<String> {
    let mut out = BTreeSet::new();
    match &s.kind {
        StmtKind::Assign { target, .. }
        | StmtKind::AugAssign { target, .. }
        | StmtKind::For { target, .. } => scope::target_names(target, &mut out),
        StmtKind::AnnAssign { target, .. } => {
            out.insert(target.name.clone());
        }
        StmtKind::With { binding, .. } => {
            out.insert(binding.name.clone());
        }
        StmtKind::Try { handlers, .. } => {
            out.extend(
                handlers
                    .iter()
                    .filter_map(|h| h.binding.as_ref())
                    .map(|b| b.name.clone()),
            );
        }
        _ => {}
    }
    out.into_iter().collect()
}

/// Gives each closure its captures as leading parameters, and passes them
/// at every call of it in `body` and the closures. A closure that takes
/// some and is used other than called is the error of step 6-3b.
/// An existing `<closure>(def, inner..)` whose def gained outer captures: splice
/// them in front of the inner ones (the hidden def's parameter order) rather
/// than wrapping the def operand again.
fn widen_closure(
    args: &mut Vec<Arg>,
    captures: &Captures,
    celled: &BTreeSet<String>,
    callees: &mut HashSet<Span>,
) {
    let Some(first) = args.first() else {
        return;
    };
    let ExprKind::Name(h) = &first.value.kind else {
        return;
    };
    let Some(names) = captures.get(h.as_str()).filter(|c| !c.is_empty()) else {
        return;
    };
    callees.insert(first.value.span);
    let at = first.value.span.start + 1 + args.len();
    let extra: Vec<Arg> = names
        .iter()
        .enumerate()
        .map(|(i, n)| {
            capture_arg(
                n,
                Span {
                    start: at + i,
                    end: at + i,
                },
                celled,
            )
        })
        .collect();
    args.splice(1..1, extra);
}

fn add_captures(
    body: &mut Block,
    lifted: &mut [Stmt],
    captures: &Captures,
    celled: &BTreeSet<String>,
) -> Result<()> {
    let mut failed = None;
    let mut callees: HashSet<Span> = HashSet::new();
    let owner: std::cell::RefCell<Option<String>> = std::cell::RefCell::new(None);
    let mut go = |e: &mut Expr| match &mut e.kind {
        ExprKind::Call { func, args } => {
            if let (ExprKind::Attr { attr, .. }, Some(first)) = (&func.kind, args.first()) {
                let task = matches!(attr.name.as_str(), "submit" | "map");
                let taken = match &first.value.kind {
                    ExprKind::Name(h) => captures.get(h.as_str()).and_then(|c| c.first()),
                    _ => None,
                };
                if let (true, Some(name), None) = (task, taken, &failed) {
                    let msg = format!(
                        "a task function cannot use `{name}` from the enclosing function; \
                         pass it as an argument"
                    );
                    failed = Some((first.value.span, msg));
                }
            }
            let ExprKind::Name(h) = &func.kind else {
                return;
            };
            if h == CLOSURE {
                widen_closure(args, captures, celled, &mut callees);
                return;
            }
            let Some(names) = captures.get(h.as_str()) else {
                return;
            };
            callees.insert(func.span);
            let at = func.span.start;
            let extra: Vec<Arg> = names
                .iter()
                .enumerate()
                .map(|(i, n)| {
                    capture_arg(
                        n,
                        Span {
                            start: at + i,
                            end: at + i,
                        },
                        celled,
                    )
                })
                .collect();
            args.splice(0..0, extra);
        }
        ExprKind::Name(h) => {
            let Some(names) = captures.get(h.as_str()).filter(|c| !c.is_empty()) else {
                return;
            };
            if callees.contains(&e.span) || failed.is_some() {
                return;
            }
            if owner.borrow().as_deref() == Some(h.as_str()) {
                let msg = format!(
                    "`{}` refers to itself as a value; that needs cells (not supported yet)",
                    source_name(h)
                );
                failed = Some((e.span, msg));
                return;
            }
            *e = closure_value(h, names, celled, e.span, &mut callees);
        }
        _ => {}
    };
    walk_mut(body, &mut go);
    for stmt in lifted.iter_mut() {
        if let StmtKind::FuncDef(def) = &mut stmt.kind {
            owner.replace(Some(def.name.name.clone()));
            walk_mut(&mut def.body, &mut go);
        }
    }
    if let Some((span, msg)) = failed {
        return Err(error(span, msg));
    }
    for stmt in lifted.iter_mut() {
        let StmtKind::FuncDef(def) = &mut stmt.kind else {
            continue;
        };
        let names = captures.get(def.name.name.as_str()).into_iter().flatten();
        let lead: Vec<Param> = names
            .map(|n| Param {
                mode: ParamMode::Read,
                name: crate::ast::Ident {
                    name: n.clone(),
                    span: def.span,
                },
                annotation: None,
                default: None,
                span: def.span,
            })
            .collect();
        def.params.splice(0..0, lead);
    }
    Ok(())
}

/// The argument that passes captured variable `n`: its value, or its slot
/// when `n` is celled (D209).
fn capture_arg(n: &str, span: Span, celled: &BTreeSet<String>) -> Arg {
    let name = Expr {
        kind: ExprKind::Name(n.to_string()),
        span,
    };
    let value = if celled.contains(n) {
        let func = Expr {
            kind: ExprKind::Name(CELL.to_string()),
            span,
        };
        Expr {
            kind: ExprKind::Call {
                func: Box::new(func),
                args: vec![Arg {
                    name: None,
                    span,
                    value: name,
                }],
            },
            span,
        }
    } else {
        name
    };
    Arg {
        name: None,
        span,
        value,
    }
}

/// The value of closure `h`, which takes `names`, made where it was
/// written at `span` (D205): `<closure>(h, names...)`, a partial
/// application inference, `run` and `build` know by name.
fn closure_value(
    h: &str,
    names: &[String],
    celled: &BTreeSet<String>,
    span: Span,
    seen: &mut HashSet<Span>,
) -> Expr {
    let at = span.start;
    let name = |n: &str, k: usize| {
        let span = Span {
            start: at + k,
            end: at + k,
        };
        Expr {
            kind: ExprKind::Name(n.to_string()),
            span,
        }
    };
    let def = name(h, 1);
    seen.insert(def.span);
    let mut args = vec![Arg {
        name: None,
        span: def.span,
        value: def,
    }];
    args.extend(names.iter().enumerate().map(|(i, n)| {
        let span = Span {
            start: at + 2 + i,
            end: at + 2 + i,
        };
        capture_arg(n, span, celled)
    }));
    Expr {
        kind: ExprKind::Call {
            func: Box::new(name(CLOSURE, 0)),
            args,
        },
        span,
    }
}

/// Calls `f` on every expression of `block`, each before its parts.
pub(super) fn walk_mut(block: &mut [Stmt], f: &mut impl FnMut(&mut Expr)) {
    for s in block {
        for e in own_exprs_mut(&mut s.kind) {
            expr_mut(e, f);
        }
        for child in super::class::children_mut(&mut s.kind) {
            walk_mut(child, f);
        }
    }
}

fn expr_mut(e: &mut Expr, f: &mut impl FnMut(&mut Expr)) {
    f(e);
    for part in parts_mut(e) {
        expr_mut(part, f);
    }
}

fn parts_mut(e: &mut Expr) -> Vec<&mut Expr> {
    match &mut e.kind {
        ExprKind::Tuple(items) | ExprKind::List(items) | ExprKind::Set(items) => {
            items.iter_mut().collect()
        }
        ExprKind::Dict(items) => items.iter_mut().flat_map(|(k, v)| [k, v]).collect(),
        ExprKind::Unary { operand, .. } => vec![operand],
        ExprKind::Binary { left, right, .. } | ExprKind::Logical { left, right, .. } => {
            vec![left, right]
        }
        ExprKind::Compare { left, rest } => std::iter::once(&mut **left)
            .chain(rest.iter_mut().map(|(_, x)| x))
            .collect(),
        ExprKind::IfExp { test, body, orelse } => vec![test, body, orelse],
        ExprKind::Call { func, args } => std::iter::once(&mut **func)
            .chain(args.iter_mut().map(|a| &mut a.value))
            .collect(),
        ExprKind::Subscript { value, index } => vec![value, index],
        ExprKind::Slice {
            value,
            start,
            stop,
            step,
        } => std::iter::once(&mut **value)
            .chain(
                [start, stop, step]
                    .into_iter()
                    .filter_map(|x| x.as_deref_mut()),
            )
            .collect(),
        ExprKind::Attr { value, .. } => vec![value],
        ExprKind::Comp { gather, clauses } => {
            let mut out = Vec::new();
            for c in clauses {
                out.push(&mut c.iter);
                out.extend(c.ifs.iter_mut());
            }
            out.extend(gather.exprs_mut());
            out
        }
        ExprKind::Name(_)
        | ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Str(_)
        | ExprKind::Bool(_)
        | ExprKind::None => Vec::new(),
    }
}

/// The expressions a statement holds outside its nested blocks.
fn own_exprs_mut(kind: &mut StmtKind) -> Vec<&mut Expr> {
    match kind {
        StmtKind::Assign { target, value } | StmtKind::AugAssign { target, value, .. } => {
            let mut out = vec![value];
            target_exprs_mut(target, &mut out);
            out
        }
        StmtKind::AnnAssign { value, .. }
        | StmtKind::Expr(value)
        | StmtKind::Raise(value)
        | StmtKind::With { value, .. } => vec![value],
        StmtKind::Return(value) => value.iter_mut().collect(),
        StmtKind::Del { value, index } => vec![value, index],
        StmtKind::If { branch, elifs, .. } => std::iter::once(branch)
            .chain(elifs)
            .map(|b| &mut b.test)
            .collect(),
        StmtKind::While { branch, .. } => vec![&mut branch.test],
        StmtKind::For { target, iter, .. } => {
            let mut out = vec![iter];
            target_exprs_mut(target, &mut out);
            out
        }
        _ => Vec::new(),
    }
}

fn target_exprs_mut<'a>(t: &'a mut Target, out: &mut Vec<&'a mut Expr>) {
    match &mut t.kind {
        TargetKind::Name(_) => {}
        TargetKind::Subscript { value, index } => {
            out.push(value);
            out.push(index);
        }
        TargetKind::Attr { value, .. } => out.push(value),
        TargetKind::Tuple(items) => items.iter_mut().for_each(|i| target_exprs_mut(i, out)),
    }
}
