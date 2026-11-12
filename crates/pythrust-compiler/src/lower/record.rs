//! Frozen records (D129-D136): construction, field reads and method
//! calls. A record is laid out as a tuple of its fields in declaration
//! order ([`crate::ir::Ty::Record`]); a method `P.m` is the top-level
//! function the parser named so, `self` its first argument.

use crate::ast::{method_name, Arg, Expr};
use crate::diag::Result;
use crate::ir::{Operand, Rvalue};
use crate::span::Span;

use super::expr::bind_slots;
use super::{internal, FnBuilder};

impl FnBuilder<'_, '_> {
    /// `P(args)`: the fields bound by position or keyword, as inference
    /// checked (D132), in a new record.
    pub(super) fn construct(&mut self, name: &str, args: &[Arg], span: Span) -> Result<Operand> {
        let names: Vec<String> = match self.cx.seqs.declared(name) {
            Some(fields) => fields.iter().map(|(n, _)| n.clone()).collect(),
            None => return Err(internal(span, &format!("`{name}` is no record"))),
        };
        // Python evaluates arguments as written, then binds them.
        let mut ops = Vec::new();
        for a in args {
            ops.push(self.expr(&a.value)?);
        }
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let slots = bind_slots(&names, 0, args, ops)?;
        let ty = self.ty_at(span)?;
        let mut fields = Vec::new();
        for (k, slot) in slots.into_iter().enumerate() {
            let op = slot.ok_or_else(|| internal(span, "a field inference did not bind"))?;
            let want = self.slot_ty(ty, k, span)?;
            fields.push(self.expect(op, want, span)?);
        }
        Ok(self.temp(ty, Rvalue::NewSeq(fields), span))
    }

    /// `value.attr`, read: a field of a record.
    pub(super) fn record_field(&mut self, value: &Expr, attr: &str, span: Span) -> Result<Operand> {
        let r = self.expr(value)?;
        let k = self.cx.seqs.field_of(self.ty(r), attr);
        let k = k.ok_or_else(|| internal(span, &format!("no record field `{attr}`")))?;
        self.field(r, k, span)
    }

    /// `value.attr(args)` on a record: a call of its method `P.attr`
    /// with the record as `self`; `None` when `value` is no record.
    pub(super) fn record_method(
        &mut self,
        value: &Expr,
        attr: &str,
        args: &[Arg],
        span: Span,
    ) -> Result<Option<Operand>> {
        if !matches!(self.node_type(value.span)?, crate::infer::Type::Record(_)) {
            return Ok(None);
        }
        let receiver = self.expr(value)?;
        let record = self.cx.seqs.record_name(self.ty(receiver));
        let record = record.ok_or_else(|| internal(value.span, "a method of no record"))?;
        let name = method_name(record, attr);
        let def = self.cx.defs.get(name.as_str()).copied();
        let def = def.ok_or_else(|| internal(span, &format!("no method `{name}`")))?;
        let mut ops = Vec::new();
        for a in args {
            ops.push(self.expr(&a.value)?);
        }
        let params: Vec<&str> = def.params.iter().map(|p| p.name.name.as_str()).collect();
        let mut slots = bind_slots(&params, 1, args, ops)?;
        if let Some(slf) = slots.first_mut() {
            *slf = Some(receiver);
        }
        let node = self.node_type(span)?;
        self.call_def(def, slots, node, span).map(Some)
    }
}
