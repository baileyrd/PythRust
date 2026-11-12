//! Frozen records (D129-D136) and objects (D150, D157): construction,
//! field reads and stores, and method calls. A record is laid out as a
//! tuple of its fields in declaration order ([`crate::ir::Ty::Record`]),
//! an object as a container of them ([`crate::ir::Ty::Obj`]); a method
//! `P.m` is the top-level function the parser named so, `self` its first
//! argument.

use crate::ast::{method_name, Arg, Expr};
use crate::diag::Result;
use crate::infer::Type;
use crate::ir::{Inst, Operand, Rvalue};
use crate::span::Span;

/// What `__init__` returns, the node type its call takes.
const NONE: Type = Type::None;

use super::expr::bind_slots;
use super::{internal, FnBuilder};

impl FnBuilder<'_, '_> {
    /// `P(args)`: the fields bound by position or keyword, as inference
    /// checked (D132), in a new record.
    pub(super) fn construct(&mut self, name: &str, args: &[Arg], span: Span) -> Result<Operand> {
        if self.cx.seqs.is_class(name) {
            return self.construct_object(name, args, span);
        }
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

    /// `P(args)` of a mutable class: a new object, then its `__init__`
    /// with it as `self`, or for a dataclass, which has none, its fields
    /// stored in declaration order (D146).
    fn construct_object(&mut self, name: &str, args: &[Arg], span: Span) -> Result<Operand> {
        let ty = self.ty_at(span)?;
        let obj = self.temp(ty, Rvalue::NewObj, span);
        let mut ops = Vec::new();
        for a in args {
            ops.push(self.expr(&a.value)?);
        }
        let init = method_name(name, "__init__");
        if let Some(def) = self.cx.defs.get(init.as_str()).copied() {
            let params: Vec<&str> = def.params.iter().map(|p| p.name.name.as_str()).collect();
            let mut slots = bind_slots(&params, 1, args, ops)?;
            if let Some(slf) = slots.first_mut() {
                *slf = Some(obj);
            }
            self.call_def(def, slots, &NONE, span)?;
            return Ok(obj);
        }
        let names: Vec<String> = match self.cx.seqs.declared(name) {
            Some(fields) => fields.iter().map(|(n, _)| n.clone()).collect(),
            None => return Err(internal(span, &format!("`{name}` is no class"))),
        };
        let names: Vec<&str> = names.iter().map(String::as_str).collect();
        let slots = bind_slots(&names, 0, args, ops)?;
        for (field, slot) in slots.into_iter().enumerate() {
            let value = slot.ok_or_else(|| internal(span, "a field inference did not bind"))?;
            let want = self.slot_ty(ty, field, span)?;
            let value = self.expect(value, want, span)?;
            self.emit(Inst::SetField {
                obj,
                field,
                value,
                span,
            });
        }
        Ok(obj)
    }

    /// `value.attr`, read: a field of a record or object.
    pub(super) fn record_field(&mut self, value: &Expr, attr: &str, span: Span) -> Result<Operand> {
        let r = self.expr(value)?;
        self.attr(r, attr, span)
    }

    /// Field `attr` of record or object `r`.
    pub(super) fn attr(&mut self, r: Operand, attr: &str, span: Span) -> Result<Operand> {
        let k = self.cx.seqs.field_of(self.ty(r), attr);
        let k = k.ok_or_else(|| internal(span, &format!("no field `{attr}`")))?;
        self.field(r, k, span)
    }

    /// `obj.attr = v` on an object (D150): a container store.
    pub(super) fn store_field(
        &mut self,
        obj: Operand,
        attr: &str,
        v: Operand,
        span: Span,
    ) -> Result<()> {
        let ty = self.ty(obj);
        let field = self.cx.seqs.field_of(ty, attr);
        let field = field.ok_or_else(|| internal(span, &format!("no field `{attr}`")))?;
        let want = self.slot_ty(ty, field, span)?;
        let value = self.expect(v, want, span)?;
        self.emit(Inst::SetField {
            obj,
            field,
            value,
            span,
        });
        Ok(())
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
