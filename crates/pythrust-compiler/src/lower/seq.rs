//! Lists, tuples and ranges: displays, indexing, `append`, item
//! assignment and unpacking.

use crate::ast::{Arg, Expr, ExprKind, Target, TargetKind, UnaryOp};
use crate::diag::Result;
use crate::infer::Type;
use crate::ir::{CmpOp, Inst, Operand, Rvalue, Ty};
use crate::span::Span;

use super::{internal, unsupported, FnBuilder};

impl FnBuilder<'_, '_> {
    /// A list or tuple display: a new sequence holding the items.
    pub(super) fn display(&mut self, items: &[Expr], span: Span) -> Result<Operand> {
        let mut ops = Vec::new();
        for item in items {
            ops.push(self.expr(item)?);
        }
        self.bind_display(&ops, span)?;
        let ty = self.ty_at(span)?;
        let mut slots = Vec::new();
        for (k, (op, item)) in ops.into_iter().zip(items).enumerate() {
            let want = self.slot_ty(ty, k, span)?;
            slots.push(self.expect(op, want, item.span)?);
        }
        Ok(self.temp(ty, Rvalue::NewSeq(slots), span))
    }

    /// In generic code, binds the variables a display's type leaves open
    /// from its items' types, as `[n * n]` in `def squares(n)` needs.
    fn bind_display(&mut self, ops: &[Operand], span: Span) -> Result<()> {
        let node = self.node_type(span)?;
        if super::is_closed(node, &self.subst) {
            return Ok(());
        }
        let slots: Vec<&Type> = match node {
            Type::List(elem) => vec![elem.as_ref(); ops.len()],
            Type::Tuple(fields) => fields.iter().collect(),
            _ => Vec::new(),
        };
        for (t, op) in slots.into_iter().zip(ops) {
            super::bind(t, self.ty(*op), &self.cx.seqs, &mut self.subst);
        }
        Ok(())
    }

    /// `len(r)` of range `r`, failing as `run` does when it does not fit
    /// in an int.
    pub(super) fn range_len(&mut self, r: Operand, span: Span) -> Operand {
        let n = self.temp(Ty::Int, Rvalue::Len(r), span);
        let message = "OverflowError: length does not fit in an int";
        self.fail_if(Rvalue::Cmp(CmpOp::Lt, n, Operand::Int(0)), span, message);
        n
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

    /// The item type of a string, list or range `ty`, as indexing reads.
    fn item_ty(&self, ty: Ty, span: Span) -> Result<Ty> {
        match ty {
            Ty::Str => Ok(Ty::Str),
            Ty::Range => Ok(Ty::Int),
            Ty::List(_) => self.slot_ty(ty, 0, span),
            _ => Err(unsupported(span, "indexing this type")),
        }
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

    /// `value[index]` on a string, list, tuple or range.
    pub(super) fn subscript(&mut self, value: &Expr, index: &Expr, span: Span) -> Result<Operand> {
        let s = self.expr(value)?;
        let ty = self.ty(s);
        if let Ty::Tuple(_) = ty {
            let k = self.field_index(ty, index, span)?;
            return self.field(s, k, span);
        }
        let i = self.expr(index)?;
        let item = self.item_ty(ty, span)?;
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

    /// `a, b = s` for a tuple `s` of as many fields, or a list, string
    /// or range of as many items (checked at run time): all items are
    /// read before any target is bound.
    fn unpack(&mut self, targets: &[Target], s: Operand, span: Span) -> Result<()> {
        let ty = self.ty(s);
        let mut items = Vec::new();
        if let Ty::List(_) | Ty::Str | Ty::Range = ty {
            self.check_unpack_len(s, targets.len(), span);
            let elem = self.item_ty(ty, span)?;
            for k in 0..targets.len() {
                let i = Operand::Int(i64::try_from(k).unwrap_or(i64::MAX));
                items.push(self.temp(elem, Rvalue::Index(s, i), span));
            }
        } else {
            let n = self.cx.seqs.fields(ty).map(<[Ty]>::len);
            if n != Some(targets.len()) {
                return Err(unsupported(
                    span,
                    "unpacking anything but a tuple of as many fields, a list, a string or a range",
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

    /// Fails as Python does unless list, string or range `s` has exactly
    /// `want` items.
    /// The shortfall is in the message, so each possible length below
    /// `want` gets its own check. A range too long for an int has length
    /// -1 here: too many.
    fn check_unpack_len(&mut self, s: Operand, want: usize, span: Span) {
        let n = self.temp(Ty::Int, Rvalue::Len(s), span);
        let want_i = i64::try_from(want).unwrap_or(i64::MAX);
        let message = format!("ValueError: too many values to unpack (expected {want})");
        self.fail_if(
            Rvalue::Cmp(CmpOp::Gt, n, Operand::Int(want_i)),
            span,
            &message,
        );
        if self.ty(s) == Ty::Range {
            self.fail_if(Rvalue::Cmp(CmpOp::Lt, n, Operand::Int(0)), span, &message);
        }
        for got in 0..want {
            let got_i = i64::try_from(got).unwrap_or(i64::MAX);
            let short = Rvalue::Cmp(CmpOp::Eq, n, Operand::Int(got_i));
            let message =
                format!("ValueError: not enough values to unpack (expected {want}, got {got})");
            self.fail_if(short, span, &message);
        }
    }
}
