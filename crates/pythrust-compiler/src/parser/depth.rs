//! Bound trees at construction, including syntax parsed without recursion.
use super::{error, MAX_DEPTH};
use crate::ast::{Expr, ExprKind};
use crate::diag::Result;

/// Every child has already passed this check. The candidate can therefore
/// be at most MAX_DEPTH + 1 high, even on error: walking and dropping it is
/// bounded too. Checking only the finished expression/module would be late.
pub(super) fn bounded(expr: Expr) -> Result<Expr> {
    if height(&expr) > MAX_DEPTH {
        return Err(error(expr.span, "too deeply nested"));
    }
    Ok(expr)
}

fn height(expr: &Expr) -> usize {
    let child = match &expr.kind {
        ExprKind::Name(_)
        | ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Str(_)
        | ExprKind::Bool(_)
        | ExprKind::None => 0,
        ExprKind::Tuple(items) | ExprKind::List(items) | ExprKind::Set(items) => {
            items.iter().map(height).max().unwrap_or(0)
        }
        ExprKind::Dict(items) => items
            .iter()
            .flat_map(|(k, v)| [height(k), height(v)])
            .max()
            .unwrap_or(0),
        ExprKind::Unary { operand, .. } => height(operand),
        ExprKind::Binary { left, right, .. } | ExprKind::Logical { left, right, .. } => {
            height(left).max(height(right))
        }
        ExprKind::Compare { left, rest } => rest
            .iter()
            .map(|(_, e)| height(e))
            .max()
            .unwrap_or(0)
            .max(height(left)),
        ExprKind::IfExp { test, body, orelse } => {
            height(test).max(height(body)).max(height(orelse))
        }
        ExprKind::Call { func, args } => args
            .iter()
            .map(|a| height(&a.value))
            .max()
            .unwrap_or(0)
            .max(height(func)),
        ExprKind::Subscript { value, index } => height(value).max(height(index)),
        ExprKind::Attr { value, .. } => height(value),
    };
    child + 1
}
