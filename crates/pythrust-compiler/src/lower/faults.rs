//! Which functions may return with a catchable fault pending: a division
//! by zero, an index out of range, a wrong unpacking arity
//! (docs/research/faults.md) or a read of a closed file. Such a fault is never in a `raises` set,
//! so lowering tests the pending flag after a call to one of these as it
//! does after a call whose set is not empty.
//!
//! Syntactic and conservative: `/`, `//` and `%` by anything but a
//! nonzero literal, an index into anything but a tuple, and unpacking
//! anything but a tuple fault, as does any `.read()` (a file's, whatever
//! the receiver's inferred type); a call by name to a function that faults
//! does too, closed over the call graph.

use std::collections::{HashMap, HashSet};

use crate::ast::{BinOp, Expr, ExprKind, FuncDef, Stmt, StmtKind, Target, TargetKind, UnaryOp};
use crate::infer::{Type, Types};

/// The functions in `defs` that may leave a catchable fault pending.
pub(super) fn faulting<'a>(
    defs: &HashMap<&'a str, &'a FuncDef>,
    types: &'a Types,
) -> HashSet<&'a str> {
    let mut sites: HashMap<&'a str, Sites<'a>> = HashMap::new();
    for (name, def) in defs {
        let mut s = Sites {
            types,
            faults: false,
            calls: HashSet::new(),
        };
        s.block(&def.body);
        sites.insert(name, s);
    }
    let mut out: HashSet<&'a str> = sites
        .iter()
        .filter(|(_, s)| s.faults)
        .map(|(n, _)| *n)
        .collect();
    let mut changed = true;
    while changed {
        changed = false;
        for (name, s) in &sites {
            if !out.contains(name) && s.calls.iter().any(|c| out.contains(c)) {
                out.insert(name);
                changed = true;
            }
        }
    }
    out
}

/// What one body does: a fault site of its own, and the names it calls.
struct Sites<'a> {
    types: &'a Types,
    faults: bool,
    calls: HashSet<&'a str>,
}

impl<'a> Sites<'a> {
    fn block(&mut self, block: &'a [Stmt]) {
        block.iter().for_each(|s| self.stmt(s));
    }

    fn stmt(&mut self, s: &'a Stmt) {
        match &s.kind {
            StmtKind::Assign { target, value } => {
                self.expr(value);
                self.target(target, Some(value));
            }
            StmtKind::AugAssign { target, op, value } => {
                self.expr(value);
                self.target(target, None);
                self.faults |= divides(*op, value);
            }
            StmtKind::AnnAssign { value, .. } | StmtKind::Expr(value) | StmtKind::Raise(value) => {
                self.expr(value);
            }
            StmtKind::Return(value) => value.iter().for_each(|v| self.expr(v)),
            StmtKind::If {
                branch,
                elifs,
                orelse,
            } => {
                for b in std::iter::once(branch).chain(elifs) {
                    self.expr(&b.test);
                    self.block(&b.body);
                }
                self.block(orelse);
            }
            StmtKind::While { branch, orelse } => {
                self.expr(&branch.test);
                self.block(&branch.body);
                self.block(orelse);
            }
            StmtKind::For {
                target,
                iter,
                body,
                orelse,
            } => {
                self.expr(iter);
                self.target(target, None);
                self.block(body);
                self.block(orelse);
            }
            StmtKind::Try { body, handlers } => {
                self.block(body);
                handlers.iter().for_each(|h| self.block(&h.body));
            }
            StmtKind::FuncDef(_) | StmtKind::Break | StmtKind::Continue | StmtKind::Pass => {}
        }
    }

    /// A store to `target`, of `value` when the statement names one.
    fn target(&mut self, target: &'a Target, value: Option<&Expr>) {
        match &target.kind {
            TargetKind::Name(_) => {}
            TargetKind::Subscript { value, index } => {
                self.expr(value);
                self.expr(index);
                self.faults = true;
            }
            TargetKind::Tuple(items) => {
                self.faults |= !value.is_some_and(|v| self.is_tuple(v));
                items.iter().for_each(|t| self.target(t, None));
            }
        }
    }

    fn expr(&mut self, e: &'a Expr) {
        match &e.kind {
            ExprKind::Call { func, args } => {
                args.iter().for_each(|a| self.expr(&a.value));
                match &func.kind {
                    ExprKind::Name(name) => {
                        self.calls.insert(name);
                    }
                    ExprKind::Attr { value, attr } => {
                        self.faults |= attr.name == "read";
                        self.expr(value);
                    }
                    _ => self.expr(func),
                }
            }
            ExprKind::Binary { op, left, right } => {
                self.faults |= divides(*op, right);
                self.expr(left);
                self.expr(right);
            }
            ExprKind::Subscript { value, index } => {
                self.faults |= !self.is_tuple(value);
                self.expr(value);
                self.expr(index);
            }
            ExprKind::Tuple(items) | ExprKind::List(items) | ExprKind::Set(items) => {
                items.iter().for_each(|x| self.expr(x))
            }
            ExprKind::Dict(items) => items
                .iter()
                .flat_map(|(k, v)| [k, v])
                .for_each(|x| self.expr(x)),
            ExprKind::Unary { operand, .. } => self.expr(operand),
            ExprKind::Logical { left, right, .. } => {
                self.expr(left);
                self.expr(right);
            }
            ExprKind::Compare { left, rest } => {
                self.expr(left);
                rest.iter().for_each(|(_, x)| self.expr(x));
            }
            ExprKind::IfExp { test, body, orelse } => {
                [test, body, orelse].into_iter().for_each(|x| self.expr(x));
            }
            ExprKind::Attr { value, .. } => self.expr(value),
            ExprKind::Name(_)
            | ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Str(_)
            | ExprKind::Bool(_)
            | ExprKind::None => {}
        }
    }

    fn is_tuple(&self, e: &Expr) -> bool {
        matches!(self.types.at(e.span), Some(Type::Tuple(_)))
    }
}

/// Whether `_ op divisor` may divide by zero.
fn divides(op: BinOp, divisor: &Expr) -> bool {
    matches!(op, BinOp::Div | BinOp::FloorDiv | BinOp::Mod) && !nonzero_literal(divisor)
}

/// A literal number other than zero, maybe negated.
fn nonzero_literal(e: &Expr) -> bool {
    match &e.kind {
        ExprKind::Int(n) => *n != 0,
        ExprKind::Float(f) => *f != 0.0,
        ExprKind::Unary {
            op: UnaryOp::Neg | UnaryOp::Pos,
            operand,
        } => nonzero_literal(operand),
        _ => false,
    }
}
