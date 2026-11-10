//! A comprehension's own scope (CPython 3): the names its targets bind
//! are renamed to [`comp_name`]s, which no source can spell, so they
//! neither leak out nor touch an outer variable of the same name.
use std::collections::BTreeSet;

use super::error;
use crate::ast::{comp_name, CompFor, Expr, ExprKind, Target, TargetKind};
use crate::diag::Result;

/// Renames the names `clauses` bind, wherever the comprehension starting
/// at `offset` sees them: its element, its filters, every target, and
/// every iterable but the first, which runs in the enclosing scope.
pub(super) fn own_names(elt: &mut Expr, clauses: &mut [CompFor], offset: usize) -> Result<()> {
    let mut names = BTreeSet::new();
    clauses
        .iter()
        .try_for_each(|c| bound(&c.target, &mut names))?;
    let rename = |e: &mut Expr| rename(e, &names, offset);
    rename(elt);
    for (i, c) in clauses.iter_mut().enumerate() {
        rename_target(&mut c.target, offset);
        if i > 0 {
            rename(&mut c.iter);
        }
        c.ifs.iter_mut().for_each(rename);
    }
    Ok(())
}

/// Adds the names `target` binds to `out`: a name or a tuple of them.
fn bound(target: &Target, out: &mut BTreeSet<String>) -> Result<()> {
    match &target.kind {
        TargetKind::Name(n) => {
            out.insert(n.clone());
            Ok(())
        }
        TargetKind::Tuple(items) => items.iter().try_for_each(|t| bound(t, out)),
        TargetKind::Subscript { .. } => Err(error(
            target.span,
            "a comprehension target other than a name or a tuple of names is not supported yet",
        )),
    }
}

/// Every name a target binds is the comprehension's own.
fn rename_target(target: &mut Target, offset: usize) {
    match &mut target.kind {
        TargetKind::Name(n) => *n = comp_name(n, offset),
        TargetKind::Tuple(items) => items.iter_mut().for_each(|t| rename_target(t, offset)),
        TargetKind::Subscript { .. } => {}
    }
}

/// Renames every use of `names` in `e`, nested comprehensions included:
/// their own names are already renamed, so what is left there is free.
fn rename(e: &mut Expr, names: &BTreeSet<String>, offset: usize) {
    let go = |x: &mut Expr| rename(x, names, offset);
    match &mut e.kind {
        ExprKind::Name(n) => {
            if names.contains(n.as_str()) {
                *n = comp_name(n, offset);
            }
        }
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Str(_)
        | ExprKind::Bool(_)
        | ExprKind::None => {}
        ExprKind::Tuple(items) | ExprKind::List(items) | ExprKind::Set(items) => {
            items.iter_mut().for_each(go)
        }
        ExprKind::Dict(items) => items.iter_mut().for_each(|(k, v)| {
            go(k);
            go(v);
        }),
        ExprKind::Unary { operand, .. } => go(operand),
        ExprKind::Binary { left, right, .. } | ExprKind::Logical { left, right, .. } => {
            go(left);
            go(right);
        }
        ExprKind::Compare { left, rest } => {
            go(left);
            rest.iter_mut().for_each(|(_, x)| go(x));
        }
        ExprKind::IfExp { test, body, orelse } => {
            [test, body, orelse].into_iter().for_each(|x| go(x))
        }
        ExprKind::Call { func, args } => {
            go(func);
            args.iter_mut().for_each(|a| go(&mut a.value));
        }
        ExprKind::Subscript { value, index } => {
            go(value);
            go(index);
        }
        ExprKind::Slice {
            value,
            start,
            stop,
            step,
        } => {
            go(value);
            [start, stop, step]
                .into_iter()
                .flatten()
                .for_each(|x| go(x));
        }
        ExprKind::Attr { value, .. } => go(value),
        ExprKind::ListComp { elt, clauses } => {
            go(elt);
            for c in clauses {
                go(&mut c.iter);
                c.ifs.iter_mut().for_each(go);
            }
        }
    }
}
