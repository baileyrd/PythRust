//! Syntactic facts inference needs before typing: which names a block
//! binds, which top-level functions a function refers to, the order to
//! infer functions in, and whether a body can fall off its end.

use std::collections::{BTreeSet, HashMap};

use crate::ast::{
    Arg, Block, CmpOp, Expr, ExprKind, FuncDef, Stmt, StmtKind, Target, TargetKind, TypeExpr,
};

/// Names bound anywhere in `block`, as Python decides locals.
pub(crate) fn assigned(block: &[Stmt]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    walk(block, &mut |s| match &s.kind {
        StmtKind::Assign { target, .. }
        | StmtKind::AugAssign { target, .. }
        | StmtKind::For { target, .. } => target_names(target, &mut out),
        StmtKind::AnnAssign { target, .. } => {
            out.insert(target.name.clone());
        }
        _ => {}
    });
    out
}

/// Each annotated assignment in `block`: the name and its annotation.
pub(super) fn annotated(block: &Block) -> Vec<(&str, &TypeExpr)> {
    let mut out = Vec::new();
    walk(block, &mut |s| {
        if let StmtKind::AnnAssign {
            target, annotation, ..
        } = &s.kind
        {
            out.push((target.name.as_str(), annotation));
        }
    });
    out
}

/// Visits every statement of `block` and of the blocks nested in it.
fn walk<'a>(block: &'a [Stmt], f: &mut impl FnMut(&'a Stmt)) {
    for s in block {
        f(s);
        match &s.kind {
            StmtKind::For { body, orelse, .. } => {
                walk(body, f);
                walk(orelse, f);
            }
            StmtKind::While { branch, orelse } => {
                walk(&branch.body, f);
                walk(orelse, f);
            }
            StmtKind::If {
                branch,
                elifs,
                orelse,
            } => {
                std::iter::once(branch)
                    .chain(elifs)
                    .for_each(|b| walk(&b.body, f));
                walk(orelse, f);
            }
            _ => {}
        }
    }
}

fn target_names(target: &Target, out: &mut BTreeSet<String>) {
    match &target.kind {
        TargetKind::Name(name) => {
            out.insert(name.clone());
        }
        TargetKind::Tuple(items) => items.iter().for_each(|t| target_names(t, out)),
        TargetKind::Subscript { .. } => {}
    }
}

/// Functions in an order where each group's dependencies come first.
/// A group is a set of mutually recursive functions (a strongly connected
/// component of the call graph), inferred together.
pub(super) fn groups<'a>(funcs: &[&'a FuncDef]) -> Vec<Vec<&'a FuncDef>> {
    let index: HashMap<&str, usize> = funcs
        .iter()
        .enumerate()
        .map(|(i, f)| (f.name.name.as_str(), i))
        .collect();
    let edges: Vec<Vec<usize>> = funcs
        .iter()
        .map(|f| {
            let refs = references(f);
            refs.iter()
                .filter_map(|n| index.get(n.as_str()).copied())
                .collect()
        })
        .collect();
    let mut tarjan = Tarjan::new(&edges);
    (0..funcs.len()).for_each(|v| tarjan.visit(v));
    tarjan
        .groups
        .into_iter()
        .map(|g| {
            g.into_iter()
                .filter_map(|i| funcs.get(i).copied())
                .collect()
        })
        .collect()
}

/// Names `f` reads that are not its own locals: candidates for calls to
/// other top-level functions. Defaults run in module scope, so all of
/// their names count.
pub(crate) fn references(f: &FuncDef) -> BTreeSet<String> {
    let mut locals = assigned(&f.body);
    locals.extend(f.params.iter().map(|p| p.name.name.clone()));
    let mut names = BTreeSet::new();
    f.body.iter().for_each(|s| stmt_names(s, &mut names));
    names.retain(|n| !locals.contains(n));
    for default in f.params.iter().filter_map(|p| p.default.as_ref()) {
        expr_names(default, &mut names);
    }
    names
}

fn stmt_names(stmt: &Stmt, out: &mut BTreeSet<String>) {
    let mut e = |x: &Expr| expr_names(x, out);
    match &stmt.kind {
        StmtKind::Assign { target, value } | StmtKind::AugAssign { target, value, .. } => {
            e(value);
            target_exprs(target).into_iter().for_each(e);
        }
        StmtKind::AnnAssign { value, .. } | StmtKind::Expr(value) => e(value),
        StmtKind::Return(value) => value.iter().for_each(e),
        StmtKind::If {
            branch,
            elifs,
            orelse,
        } => {
            let branches: Vec<_> = std::iter::once(branch).chain(elifs).collect();
            branches.iter().for_each(|b| expr_names(&b.test, out));
            let bodies = branches.iter().flat_map(|b| &b.body);
            bodies.chain(orelse).for_each(|s| stmt_names(s, out));
        }
        StmtKind::While { branch, orelse } => {
            e(&branch.test);
            branch
                .body
                .iter()
                .chain(orelse)
                .for_each(|s| stmt_names(s, out));
        }
        StmtKind::For {
            target,
            iter,
            body,
            orelse,
        } => {
            e(iter);
            target_exprs(target).into_iter().for_each(e);
            body.iter().chain(orelse).for_each(|s| stmt_names(s, out));
        }
        StmtKind::FuncDef(_) | StmtKind::Break | StmtKind::Continue | StmtKind::Pass => {}
    }
}

/// The expressions inside subscript targets.
fn target_exprs(target: &Target) -> Vec<&Expr> {
    match &target.kind {
        TargetKind::Name(_) => vec![],
        TargetKind::Subscript { value, index } => vec![value, index],
        TargetKind::Tuple(items) => items.iter().flat_map(target_exprs).collect(),
    }
}

fn expr_names(expr: &Expr, out: &mut BTreeSet<String>) {
    match &expr.kind {
        ExprKind::Name(name) => {
            out.insert(name.clone());
        }
        ExprKind::Tuple(items) | ExprKind::List(items) => {
            items.iter().for_each(|x| expr_names(x, out))
        }
        ExprKind::Unary { operand, .. } => expr_names(operand, out),
        ExprKind::Binary { left, right, .. } | ExprKind::Logical { left, right, .. } => {
            expr_names(left, out);
            expr_names(right, out);
        }
        ExprKind::Compare { left, rest } => {
            expr_names(left, out);
            rest.iter().for_each(|(_, x)| expr_names(x, out));
        }
        ExprKind::IfExp { test, body, orelse } => {
            [test, body, orelse].iter().for_each(|x| expr_names(x, out))
        }
        ExprKind::Call { func, args } => {
            expr_names(func, out);
            args.iter().for_each(|a: &Arg| expr_names(&a.value, out));
        }
        ExprKind::Subscript { value, index } => {
            expr_names(value, out);
            expr_names(index, out);
        }
        ExprKind::Attr { value, .. } => expr_names(value, out),
        ExprKind::Int(_) | ExprKind::Float(_) | ExprKind::Str(_) | ExprKind::Bool(_) => {}
        ExprKind::None => {}
    }
}

/// Tarjan's strongly connected components. Emits each component after
/// every component it reaches, so dependencies come first.
struct Tarjan<'e> {
    edges: &'e [Vec<usize>],
    index: Vec<Option<usize>>,
    low: Vec<usize>,
    on_stack: Vec<bool>,
    stack: Vec<usize>,
    next: usize,
    groups: Vec<Vec<usize>>,
}

impl<'e> Tarjan<'e> {
    fn new(edges: &'e [Vec<usize>]) -> Self {
        let n = edges.len();
        Tarjan {
            edges,
            index: vec![None; n],
            low: vec![0; n],
            on_stack: vec![false; n],
            stack: Vec::new(),
            next: 0,
            groups: Vec::new(),
        }
    }

    fn visit(&mut self, v: usize) {
        if self.index[v].is_some() {
            return;
        }
        self.index[v] = Some(self.next);
        self.low[v] = self.next;
        self.next += 1;
        self.stack.push(v);
        self.on_stack[v] = true;
        for &w in &self.edges[v] {
            self.relax(v, w);
        }
        if Some(self.low[v]) == self.index[v] {
            self.pop_group(v);
        }
    }

    fn relax(&mut self, v: usize, w: usize) {
        match self.index[w] {
            None => {
                self.visit(w);
                self.low[v] = self.low[v].min(self.low[w]);
            }
            Some(iw) if self.on_stack[w] => self.low[v] = self.low[v].min(iw),
            Some(_) => {}
        }
    }

    fn pop_group(&mut self, root: usize) {
        let mut group = Vec::new();
        while let Some(w) = self.stack.pop() {
            self.on_stack[w] = false;
            group.push(w);
            if w == root {
                break;
            }
        }
        group.sort_unstable();
        self.groups.push(group);
    }
}

/// `x is None` gives `(x, false)`; `x is not None` gives `(x, true)`
/// (`true`: `x` is not `None` when the test holds). Either order.
pub(super) fn none_test(test: &Expr) -> Option<(String, bool)> {
    let ExprKind::Compare { left, rest } = &test.kind else {
        return None;
    };
    let [(op, right)] = rest.as_slice() else {
        return None;
    };
    let not_none = match op {
        CmpOp::Is => false,
        CmpOp::IsNot => true,
        _ => return None,
    };
    let name = match (&left.kind, &right.kind) {
        (ExprKind::Name(n), ExprKind::None) | (ExprKind::None, ExprKind::Name(n)) => n.clone(),
        _ => return None,
    };
    Some((name, not_none))
}

/// Whether control can reach the end of `block` without a `return`.
/// The name an `if` proves is not `None` for the statements after it:
/// `if x is None:` with a body that leaves, or `if x is not None:` with an
/// `else` that leaves, while the other side keeps `x`.
pub(super) fn guard(stmt: &Stmt) -> Option<String> {
    let StmtKind::If {
        branch,
        elifs,
        orelse,
    } = &stmt.kind
    else {
        return None;
    };
    if !elifs.is_empty() {
        return None;
    }
    let (name, not_none) = none_test(&branch.test)?;
    let (none_side, other) = if not_none {
        (orelse, &branch.body)
    } else {
        (&branch.body, orelse)
    };
    let proven = leaves(none_side) && !assigned(other).contains(&name);
    proven.then_some(name)
}

/// Whether control never reaches the statement after `block`: every path
/// ends in `return`, `break` or `continue`, or loops forever.
fn leaves(block: &[Stmt]) -> bool {
    exits(block, true)
}

/// Whether a function body can reach its end (and return `None`).
pub(super) fn falls_through(block: &[Stmt]) -> bool {
    !exits(block, false)
}

/// Whether every path through `block` ends in `return`, or loops
/// forever; with `jumps`, a `break` or `continue` also ends a path.
///
/// A loop with no `break` of its own either never ends or runs its
/// `else`, so it exits when it loops forever or its `else` exits. A jump
/// in its body only leaves that loop, so the body counts only for breaks.
fn exits(block: &[Stmt], jumps: bool) -> bool {
    block.iter().any(|s| match &s.kind {
        StmtKind::Return(_) => true,
        StmtKind::Break | StmtKind::Continue => jumps,
        StmtKind::If {
            branch,
            elifs,
            orelse,
        } => {
            let mut bodies = std::iter::once(branch).chain(elifs);
            bodies.all(|b| exits(&b.body, jumps)) && exits(orelse, jumps)
        }
        StmtKind::While { branch, orelse } => {
            let forever = matches!(branch.test.kind, ExprKind::Bool(true));
            !branch.body.iter().any(breaks) && (forever || exits(orelse, jumps))
        }
        StmtKind::For { body, orelse, .. } => !body.iter().any(breaks) && exits(orelse, jumps),
        _ => false,
    })
}

/// Whether `stmt` contains a `break` for the enclosing loop. A nested
/// loop's body breaks only that loop, but its `else` runs after it and
/// breaks the enclosing one.
fn breaks(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Break => true,
        StmtKind::While { orelse, .. } | StmtKind::For { orelse, .. } => orelse.iter().any(breaks),
        StmtKind::If {
            branch,
            elifs,
            orelse,
        } => {
            let bodies = std::iter::once(branch).chain(elifs).flat_map(|b| &b.body);
            bodies.chain(orelse).any(breaks)
        }
        _ => false,
    }
}
