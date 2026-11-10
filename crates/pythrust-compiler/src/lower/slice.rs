//! Slice reads and positional tuple selection.
use super::{internal, FnBuilder};
use crate::ast::{Expr, ExprKind, UnaryOp};
use crate::diag::Result;
use crate::ir::{Inst, Operand, Rvalue, Ty};
use crate::span::Span;

impl FnBuilder<'_, '_> {
    /// Receiver and components are evaluated once, before observing its length.
    pub(super) fn slice_read(
        &mut self,
        value: &Expr,
        bounds: [Option<&Expr>; 3],
        span: Span,
    ) -> Result<Operand> {
        let source = self.expr(value)?;
        let mut ops = [Operand::Int(0), Operand::Int(0), Operand::Int(1)];
        let mut present = 0;
        for (k, expr) in bounds.iter().enumerate() {
            if let Some(expr) = expr {
                let op = self.expr(expr)?;
                ops[k] = self.as_int(op, expr.span);
                present |= 1 << k;
            }
        }
        let ty = self.ty(source);
        if matches!(ty, Ty::Tuple(_)) && ops[2] == Operand::Int(0) {
            let text = self.temp(
                Ty::Str,
                Rvalue::StrLit("slice step cannot be zero".to_string()),
                span,
            );
            let exc = self.new_exc(crate::exc::ExcKind::Value, &[text], span)?;
            self.emit(Inst::Raise { exc, span });
            self.raise_path()?;
            return Err(self.diverge(span));
        }
        self.raise_if(
            Rvalue::Cmp(crate::ir::CmpOp::Eq, ops[2], Operand::Int(0)),
            crate::exc::ExcKind::Value,
            "slice step cannot be zero",
            span,
        )?;
        if let Ty::Tuple(_) = ty {
            let fields = self
                .cx
                .seqs
                .fields(ty)
                .ok_or_else(|| internal(span, "tuple fields"))?
                .to_vec();
            let literal = |expr: Option<&Expr>| -> Result<Option<i64>> {
                expr.map(|e| slice_literal(e).ok_or_else(|| internal(span, "computed tuple slice")))
                    .transpose()
            };
            let [start, stop, step] = bounds.map(literal);
            let indices = crate::slice::indices(fields.len(), start?, stop?, step?.unwrap_or(1));
            let mut items = Vec::new();
            let mut types = Vec::new();
            for k in indices.into_iter().flatten() {
                items.push(self.field(source, k, span)?);
                types.push(fields[k]);
            }
            let result = self.cx.seqs.tuple(types);
            return Ok(self.temp(result, Rvalue::NewSeq(items), span));
        }
        Ok(self.temp(
            ty,
            Rvalue::Slice {
                source,
                bounds: ops,
                present,
            },
            span,
        ))
    }
}

fn slice_literal(expr: &Expr) -> Option<i64> {
    match &expr.kind {
        ExprKind::Int(n) => Some(*n),
        ExprKind::Unary {
            op: UnaryOp::Neg,
            operand,
        } => match operand.kind {
            ExprKind::Int(n) => n.checked_neg(),
            _ => None,
        },
        ExprKind::Unary {
            op: UnaryOp::Pos,
            operand,
        } => match operand.kind {
            ExprKind::Int(n) => Some(n),
            _ => None,
        },
        _ => None,
    }
}
