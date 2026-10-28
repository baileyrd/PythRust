//! Optionals (`T | None`): null is `None`. A present `Str`, list or tuple
//! is the pointer itself; a present `Int`, `Float` or `Bool` is boxed in
//! a one-field tuple, so every optional is one reference-counted word.

use cranelift_codegen::ir::{InstBuilder, Value};

use super::seq::descriptor;
use super::{backend, rc_fns, FnGen, Rt};
use crate::diag::Result;
use crate::ir::{Compound, LocalId, Operand, Ty};
use crate::span::Span;

impl FnGen<'_, '_, '_> {
    /// The present type of optional `ty`.
    pub(super) fn present(&self, ty: Ty) -> Result<Ty> {
        match self.types.get(opt_id(ty)?) {
            Some(Compound::Opt(present)) => Ok(*present),
            _ => Err(backend("an optional of no present type".to_string())),
        }
    }

    /// `v`, a present value or `None`, as optional `ty`: a new reference.
    pub(super) fn wrap(&mut self, v: Operand, ty: Ty) -> Result<Value> {
        let present = self.present(ty)?;
        match self.func.ty(v) {
            Ty::None => Ok(self.b.ins().iconst(self.cg.ptr, 0)),
            p if p.is_rc() => {
                let s = self.operand(v);
                self.call_rt(rc_fns(self.types, p).0, &[s]);
                Ok(s)
            }
            _ => {
                let mut text = String::from("t\u{1}");
                descriptor(self.types, present, &mut text)?;
                let (desc, len) = self.data(text)?;
                let one = self.b.ins().iconst(cranelift_codegen::ir::types::I64, 1);
                let cell = self.call_rt_value(Rt::SeqNew, &[desc, len, one])?;
                let slot = self.slot(v);
                self.call_rt(Rt::SeqPush, &[cell, slot]);
                Ok(cell)
            }
        }
    }

    /// The present value of optional `v`, not null, into `dest`: a new
    /// reference if reference-counted.
    pub(super) fn unwrap(&mut self, v: Operand, dest: LocalId) -> Result<Value> {
        let present = self.present(self.func.ty(v))?;
        if !present.is_rc() {
            return self.field(v, 0, dest);
        }
        let s = self.operand(v);
        self.call_rt(rc_fns(self.types, present).0, &[s]);
        Ok(s)
    }

    /// The present value of non-null optional `v`, borrowed.
    fn peek(&mut self, v: Value, present: Ty) -> Result<Value> {
        if present.is_rc() {
            return Ok(v);
        }
        let zero = self.b.ins().iconst(cranelift_codegen::ir::types::I64, 0);
        let (slot, _) = self.with_status(Rt::SeqGet, &[v, zero])?;
        Ok(self.unslot(slot, present))
    }

    /// Runs `none` when optional `op` is null, else `present` on its
    /// borrowed present value.
    fn branch_opt(
        &mut self,
        op: Operand,
        none: impl FnOnce(&mut Self) -> Result<()>,
        present: impl FnOnce(&mut Self, Ty, Value) -> Result<()>,
    ) -> Result<()> {
        let ty = self.present(self.func.ty(op))?;
        let v = self.operand(op);
        let (some, null, end) = (
            self.b.create_block(),
            self.b.create_block(),
            self.b.create_block(),
        );
        self.b.ins().brif(v, some, &[], null, &[]);
        self.b.switch_to_block(null);
        none(self)?;
        self.b.ins().jump(end, &[]);
        self.b.switch_to_block(some);
        let p = self.peek(v, ty)?;
        present(self, ty, p)?;
        self.b.ins().jump(end, &[]);
        self.b.switch_to_block(end);
        Ok(())
    }

    /// `print`'s text for optional `op`.
    pub(super) fn write_opt(&mut self, op: Operand) -> Result<()> {
        self.branch_opt(
            op,
            |g| {
                g.call_rt(Rt::WriteNone, &[]);
                Ok(())
            },
            |g, ty, v| g.write_value(ty, v),
        )
    }

    /// `str(op)` of optional `op`, into `dest`.
    pub(super) fn opt_to_str(&mut self, op: Operand, dest: LocalId, span: Span) -> Result<Value> {
        let var = self.vars[dest.0];
        self.branch_opt(
            op,
            |g| {
                let s = g.call_rt_value(Rt::StrNone, &[])?;
                g.b.def_var(var, s);
                Ok(())
            },
            |g, ty, v| {
                let s = g.str_of_value(ty, v, span)?;
                g.b.def_var(var, s);
                Ok(())
            },
        )?;
        Ok(self.b.use_var(var))
    }
}

fn opt_id(ty: Ty) -> Result<usize> {
    match ty {
        Ty::Opt(id) => Ok(id.0),
        _ => Err(backend("an optional operation on another type".to_string())),
    }
}
