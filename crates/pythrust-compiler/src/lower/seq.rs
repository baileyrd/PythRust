//! Lists and tuples: displays, indexing, `append`, item assignment and
//! unpacking.

use crate::ast::{Arg, Expr, ExprKind, Target, TargetKind, UnaryOp};
use crate::diag::Result;
use crate::ir::{CmpOp, Inst, Operand, Rvalue, Ty};
use crate::span::Span;

use super::{internal, unsupported, FnBuilder};

impl FnBuilder<'_, '_> {
    /// A list or tuple display: a new sequence holding the items.
    pub(super) fn display(&mut self, items: &[Expr], span: Span) -> Result<Operand> {
        let ty = self.ty_at(span)?;
        let mut ops = Vec::new();
        for (k, item) in items.iter().enumerate() {
            let op = self.expr(item)?;
            let want = self.slot_ty(ty, k, span)?;
            ops.push(self.expect(op, want, item.span)?);
        }
        Ok(self.temp(ty, Rvalue::NewSeq(ops), span))
    }

    /// The type of slot `k` of sequence type `ty`.
    pub(super) fn slot_ty(&self, ty: Ty, k: usize, span: Span) -> Result<Ty> {
        let slot = match ty {
            Ty::List(_) => self.cx.seqs.elem(ty),
            Ty::Tuple(_) => self.cx.seqs.fields(ty).and_then(|f| f.get(k).copied()),
            _ => None,
        };
        slot.ok_or_else(|| internal(span, "a sequence slot of no type"))
    }

    /// `v`, which must have type `want`: inference gives every slot its
    /// value's exact type, with no widening, but for wrapping into an
    /// optional.
    fn expect(&mut self, v: Operand, want: Ty, span: Span) -> Result<Operand> {
        let v = self.fit(v, want, span);
        if self.ty(v) != want {
            return Err(unsupported(
                span,
                "a value of another type than its list or tuple holds",
            ));
        }
        Ok(v)
    }

    /// `value[index]` on a string, list or tuple.
    pub(super) fn subscript(&mut self, value: &Expr, index: &Expr, span: Span) -> Result<Operand> {
        let s = self.expr(value)?;
        let ty = self.ty(s);
        if let Ty::Tuple(_) = ty {
            let k = self.field_index(ty, index, span)?;
            return self.field(s, k, span);
        }
        let i = self.expr(index)?;
        let item = match ty {
            Ty::Str => Ty::Str,
            Ty::List(_) => self.slot_ty(ty, 0, span)?,
            _ => return Err(unsupported(span, "indexing this type")),
        };
        if self.ty(i) != Ty::Int {
            return Err(unsupported(span, "an index that is not an int"));
        }
        Ok(self.temp(item, Rvalue::Index(s, i), span))
    }

    /// The field a tuple index names: a literal, as inference requires,
    /// counted from the end when negative.
    fn field_index(&self, ty: Ty, index: &Expr, span: Span) -> Result<usize> {
        let n = self.cx.seqs.fields(ty).map_or(0, <[Ty]>::len);
        let k = match &index.kind {
            ExprKind::Int(k) => Some(*k),
            ExprKind::Unary {
                op: UnaryOp::Neg,
                operand,
            } => match operand.kind {
                ExprKind::Int(k) => k.checked_neg(),
                _ => None,
            },
            _ => None,
        };
        let Some(k) = k else {
            return Err(unsupported(span, "a tuple index that is not a literal"));
        };
        let k = if k < 0 { k + n as i64 } else { k };
        usize::try_from(k)
            .ok()
            .filter(|k| *k < n)
            .ok_or_else(|| internal(span, "a tuple index out of range"))
    }

    /// Field `k` of tuple `t`, a new temporary.
    fn field(&mut self, t: Operand, k: usize, span: Span) -> Result<Operand> {
        let ty = self.slot_ty(self.ty(t), k, span)?;
        Ok(self.temp(ty, Rvalue::Field(t, k), span))
    }

    /// `list.append(v)`, the one method in the subset.
    pub(super) fn method(
        &mut self,
        value: &Expr,
        attr: &str,
        args: &[Arg],
        span: Span,
    ) -> Result<Operand> {
        let list = self.expr(value)?;
        let ty = self.ty(list);
        let ([arg], "append", Ty::List(_)) = (args, attr, ty) else {
            return Err(unsupported(span, "methods other than `list.append(x)`"));
        };
        if arg.name.is_some() {
            return Err(unsupported(span, "keyword arguments"));
        }
        let v = self.expr(&arg.value)?;
        let elem = self.slot_ty(ty, 0, span)?;
        let value = self.expect(v, elem, arg.span)?;
        self.emit(Inst::Append { list, value });
        Ok(Operand::None)
    }

    /// `list[index] = v`, the list and index evaluated now.
    pub(super) fn set_item(
        &mut self,
        list: &Expr,
        index: &Expr,
        v: Operand,
        span: Span,
    ) -> Result<()> {
        let list = self.expr(list)?;
        let index = self.expr(index)?;
        self.store_item(list, index, v, span)
    }

    /// `list[index] = v` on evaluated operands.
    pub(super) fn store_item(
        &mut self,
        list: Operand,
        index: Operand,
        v: Operand,
        span: Span,
    ) -> Result<()> {
        let ty = self.ty(list);
        if !matches!(ty, Ty::List(_)) || self.ty(index) != Ty::Int {
            return Err(unsupported(
                span,
                "assigning to an item of anything but a list",
            ));
        }
        let elem = self.slot_ty(ty, 0, span)?;
        let value = self.expect(v, elem, span)?;
        self.emit(Inst::SetItem {
            list,
            index,
            value,
            span,
        });
        Ok(())
    }

    /// Binds `v` to `target` as unpacking and `for` do: a name takes the
    /// value's type if it has none yet, which generic inference may not
    /// know.
    pub(super) fn bind(&mut self, target: &Target, v: Operand) -> Result<()> {
        match &target.kind {
            TargetKind::Name(name) => {
                self.declare(name, self.ty(v), target.span);
                self.write(name, v, target.span)
            }
            TargetKind::Subscript { value, index } => self.set_item(value, index, v, target.span),
            TargetKind::Tuple(targets) => self.unpack(targets, v, target.span),
        }
    }

    /// `a, b = s` for a tuple `s` of as many fields, or a list or string
    /// of as many items (checked at run time): all items are read before
    /// any target is bound.
    fn unpack(&mut self, targets: &[Target], s: Operand, span: Span) -> Result<()> {
        let ty = self.ty(s);
        let mut items = Vec::new();
        if let Ty::List(_) | Ty::Str = ty {
            self.check_unpack_len(s, targets.len(), span);
            let elem = match ty {
                Ty::Str => Ty::Str,
                _ => self.slot_ty(ty, 0, span)?,
            };
            for k in 0..targets.len() {
                let i = Operand::Int(i64::try_from(k).unwrap_or(i64::MAX));
                items.push(self.temp(elem, Rvalue::Index(s, i), span));
            }
        } else {
            let n = self.cx.seqs.fields(ty).map(<[Ty]>::len);
            if n != Some(targets.len()) {
                return Err(unsupported(
                    span,
                    "unpacking anything but a tuple of as many fields, a list or a string",
                ));
            }
            for k in 0..targets.len() {
                items.push(self.field(s, k, span)?);
            }
        }
        targets
            .iter()
            .zip(items)
            .try_for_each(|(target, v)| self.bind(target, v))
    }

    /// Fails as Python does unless list or string `s` has exactly `want`
    /// items.
    /// The shortfall is in the message, so each possible length below
    /// `want` gets its own check.
    fn check_unpack_len(&mut self, s: Operand, want: usize, span: Span) {
        let n = self.temp(Ty::Int, Rvalue::Len(s), span);
        let want_i = i64::try_from(want).unwrap_or(i64::MAX);
        let more = Rvalue::Cmp(CmpOp::Gt, n, Operand::Int(want_i));
        let message = format!("ValueError: too many values to unpack (expected {want})");
        self.fail_if(more, span, &message);
        for got in 0..want {
            let got_i = i64::try_from(got).unwrap_or(i64::MAX);
            let short = Rvalue::Cmp(CmpOp::Eq, n, Operand::Int(got_i));
            let message =
                format!("ValueError: not enough values to unpack (expected {want}, got {got})");
            self.fail_if(short, span, &message);
        }
    }
}
