//! Syntactic facts inference needs before typing: which names a block
//! binds, which top-level functions a function refers to, the order to
//! infer functions in, and whether a body can fall off its end.

use std::collections::{BTreeSet, HashMap};

use crate::ast::{Arg, Block, Expr, ExprKind, FuncDef, Stmt, StmtKind, Target, TargetKind};

/// Names bound anywhere in `block`, as Python decides locals.
pub(super) fn assigned(block: &Block) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    block.iter().for_each(|s| assigned_stmt(s, &mut out));
    out
}

fn assigned_stmt(stmt: &Stmt, out: &mut BTreeSet<String>) {
    match &stmt.kind {
        StmtKind::Assign { target, .. } | StmtKind::AugAssign { target, .. } => {
            target_names(target, out)
        }
        StmtKind::AnnAssign { target, .. } => {
            out.insert(target.name.clone());
        }
        StmtKind::For {
            target,
            body,
            orelse,
            ..
        } => {
            target_names(target, out);
            body.iter()
                .chain(orelse)
                .for_each(|s| assigned_stmt(s, out));
        }
        StmtKind::If {
            branch,
            elifs,
            orelse,
        } => {
            let bodies = std::iter::once(branch).chain(elifs).flat_map(|b| &b.body);
            bodies.chain(orelse).for_each(|s| assigned_stmt(s, out));
        }
        StmtKind::While { branch, orelse } => {
            branch
                .body
                .iter()
                .chain(orelse)
                .for_each(|s| assigned_stmt(s, out));
        }
        _ => {}
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
fn references(f: &FuncDef) -> BTreeSet<String> {
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

/// Whether control can reach the end of `block` without a `return`.
pub(super) fn falls_through(block: &Block) -> bool {
    !block.iter().any(diverges)
}

fn diverges(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Return(_) => true,
        StmtKind::If {
            branch,
            elifs,
            orelse,
        } => {
            let all_branches = std::iter::once(branch)
                .chain(elifs)
                .all(|b| !falls_through(&b.body));
            all_branches && !orelse.is_empty() && !falls_through(orelse)
        }
        StmtKind::While { branch, .. } => {
            let forever = matches!(branch.test.kind, ExprKind::Bool(true));
            forever && !branch.body.iter().any(breaks)
        }
        _ => false,
    }
}

/// Whether `stmt` contains a `break` for the enclosing loop (not one
/// belonging to a nested loop).
fn breaks(stmt: &Stmt) -> bool {
    match &stmt.kind {
        StmtKind::Break => true,
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
