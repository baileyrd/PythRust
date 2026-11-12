//! Frozen records (D129-D136) and objects (D150, D157): construction,
//! field reads and stores, and method calls. A record is laid out as a
//! tuple of its fields in declaration order ([`crate::ir::Ty::Record`]),
//! an object as a container of them ([`crate::ir::Ty::Obj`]); a method
//! `P.m` is the top-level function the parser named so, `self` its first
//! argument.

use crate::ast::{method_name, Arg, Expr};
use crate::diag::Result;
use crate::infer::Type;
use crate::ir::{Inst, Operand, Rvalue, Ty};
use crate::span::Span;

/// What `__init__` returns, the node type its call takes.
const NONE: Type = Type::None;
/// What `__str__` and `__repr__` return.
const STR: Type = Type::Str;

use super::expr::bind_slots;
use super::{internal, FnBuilder};

impl<'a> FnBuilder<'_, 'a> {
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
                *slf = Some(self.another(obj, span));
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

    /// `v` as `str` and `print` render it (D167): a call of its own
    /// `__str__` or `__repr__` (D147's lookup), a new `Str`. Otherwise
    /// `v` itself, rendered as built in, after a call that passes it to
    /// each `__repr__` of the classes its type reaches, so their
    /// summaries apply to it; there are none in most programs.
    pub(super) fn render(&mut self, v: Operand, span: Span) -> Result<Operand> {
        let ty = self.ty(v);
        if let Some(def) = self
            .dunder(ty, "__str__")
            .or_else(|| self.dunder(ty, "__repr__"))
        {
            return self.call_def(def, vec![Some(v)], &STR, span);
        }
        for t in crate::ir::reachable(self.cx.seqs.all(), ty) {
            if let Some(def) = self.dunder(t, "__repr__") {
                self.pass_to(def, t, v, span)?;
            }
        }
        Ok(v)
    }

    /// The class method `name` of record or object type `ty`, if any.
    fn dunder(&self, ty: Ty, name: &str) -> Option<&'a crate::ast::FuncDef> {
        let class = self.cx.seqs.record_name(ty)?;
        self.cx.defs.get(method_name(class, name).as_str()).copied()
    }

    /// A call of `def`'s instance for `self` of type `ty` that passes
    /// `v`, a value holding such objects, as its `self`: what rendering
    /// `v` does to it, for the analysis (D167). The result is unused.
    fn pass_to(
        &mut self,
        def: &'a crate::ast::FuncDef,
        ty: Ty,
        v: Operand,
        span: Span,
    ) -> Result<()> {
        let id = self.cx.instance(def, vec![ty], Ty::Str, span)?;
        let arg = self.another(v, span);
        self.temp(Ty::Str, Rvalue::Call(id, vec![arg]), span);
        if let Operand::Local(l) = arg {
            self.emit(Inst::Forget(l));
        }
        if self.may_raise(&def.name.name) {
            self.check_raised(span)?;
        }
        Ok(())
    }

    /// A temporary with a reference of its own to object `v`, which
    /// stays in use after the call this is passed to takes it.
    fn another(&mut self, v: Operand, span: Span) -> Operand {
        let copy = self.temp(self.ty(v), Rvalue::Use(v), span);
        if let Operand::Local(l) = copy {
            self.emit(Inst::Retain(l));
        }
        copy
    }
}
