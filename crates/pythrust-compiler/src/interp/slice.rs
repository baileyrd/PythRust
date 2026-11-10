//! Slice reads: keep the receiver alive while bounds may mutate it.
use super::{raised, rt_err, ExcKind, Interp, Unwind, Value};
use crate::ast::Expr;
use crate::span::Span;

impl<'m> Interp<'m, '_> {
    pub(super) fn slice_read(
        &mut self,
        value: &'m Expr,
        components: [Option<&'m Expr>; 3],
        span: Span,
    ) -> std::result::Result<Value<'m>, Unwind<'m>> {
        let container = self.eval(value)?;
        let mut bounds = [None; 3];
        for (dest, expr) in bounds.iter_mut().zip(components) {
            if let Some(expr) = expr {
                *dest = Some(match self.eval(expr)? {
                    Value::Int(n) => n,
                    Value::Bool(b) => i64::from(b),
                    _ => {
                        return Err(
                            rt_err(expr.span, "slice bounds must be integers".to_string()).into(),
                        )
                    }
                });
            }
        }
        let [start, stop, step] = bounds;
        let step = step.unwrap_or(1);
        if step == 0 {
            return Err(raised(
                ExcKind::Value,
                "slice step cannot be zero".to_string(),
                span,
            ));
        }
        let result = match &container {
            Value::Str(s) => {
                let chars: Vec<_> = s.chars().collect();
                let out: String = crate::slice::indices(chars.len(), start, stop, step)
                    .into_iter()
                    .flatten()
                    .map(|k| chars[k])
                    .collect();
                Value::str(&out)
            }
            Value::List(xs) => {
                let xs = xs.borrow();
                Value::list(
                    crate::slice::indices(xs.len(), start, stop, step)
                        .into_iter()
                        .flatten()
                        .map(|k| xs[k].clone())
                        .collect(),
                )
            }
            Value::Tuple(xs) => Value::Tuple(
                crate::slice::indices(xs.len(), start, stop, step)
                    .into_iter()
                    .flatten()
                    .map(|k| xs[k].clone())
                    .collect(),
            ),
            _ => return Err(rt_err(span, "unsupported slice receiver".to_string()).into()),
        };
        Ok(result)
    }
}
