//! Variables: Python's local-name rule, and run-time boundness.
//!
//! A variable read before any assignment reached it is an error in
//! Python (`UnboundLocalError` in a function, `NameError` at module
//! level), so each named local that might be read unbound carries a
//! `Bool` flag, false on entry and set by every assignment, checked on
//! reads that are not known to follow an assignment.

use crate::diag::{DiagKind, Diagnostic, Result};
use crate::ir::{CmpOp, Inst, LocalId, Operand, Rvalue, Term, Ty};
use crate::span::Span;

use super::{unsupported, FnBuilder};

/// A named local. `flag` is `None` for parameters, which are always bound.
#[derive(Debug, Clone, Copy)]
pub(super) struct Var {
    pub(super) local: LocalId,
    pub(super) flag: Option<LocalId>,
}

impl FnBuilder<'_, '_> {
    /// Reads variable `name`, failing at `span` if it is unbound.
    pub(super) fn read(&mut self, name: &str, span: Span) -> Result<Operand> {
        let var = self.var(name, span)?;
        let Some(flag) = var.flag else {
            return Ok(Operand::Local(var.local));
        };
        let known = self.always_bound.contains(&var.local) || self.block_bound.contains(&var.local);
        if !known {
            let unbound = Rvalue::Cmp(CmpOp::Eq, Operand::Local(flag), Operand::Bool(false));
            let message = self.unbound_message(name);
            self.fail_if(unbound, span, &message);
            self.block_bound.insert(var.local);
        }
        Ok(Operand::Local(var.local))
    }

    /// Assigns `value` to `name` and marks it bound from here on. The
    /// value must have the variable's type: the IR is typed, and a
    /// mismatch (inference and lowering disagreeing) is refused here
    /// rather than miscompiled.
    pub(super) fn write(&mut self, name: &str, value: Operand, span: Span) -> Result<()> {
        let var = self.var(name, span)?;
        if self.ty(value) != self.locals[var.local.0] {
            let what = format!("assigning `{name}` a value of another type than inferred");
            return Err(unsupported(span, &what));
        }
        if let Some(flag) = var.flag {
            self.assign(flag, Rvalue::Use(Operand::Bool(true)), span);
        }
        if self.cur.0 == 0 {
            self.always_bound.insert(var.local);
        }
        self.block_bound.insert(var.local);
        self.assign(var.local, Rvalue::Use(value), span);
        Ok(())
    }

    fn unbound_message(&self, name: &str) -> String {
        match self.defined {
            Some(_) => format!("NameError: name '{name}' is not defined"),
            None => format!(
                "UnboundLocalError: cannot access local variable '{name}' where it is not associated with a value"
            ),
        }
    }

    /// The variable `name`, created on first use with the type of the
    /// node at `span`, and its flag cleared in the entry block.
    fn var(&mut self, name: &str, span: Span) -> Result<Var> {
        if let Some(var) = self.names.get(name) {
            return Ok(*var);
        }
        if !self.scope.contains(name) {
            let what = format!("reading `{name}`, a global, from a function");
            return Err(unsupported(span, &what));
        }
        let ty = self.ty_at(span)?;
        let local = self.new_local(ty);
        let flag = self.new_local(Ty::Bool);
        let clear = Inst::Assign {
            dest: flag,
            value: Rvalue::Use(Operand::Bool(false)),
            span,
        };
        self.blocks[0].insts.insert(0, clear);
        let var = Var {
            local,
            flag: Some(flag),
        };
        self.names.insert(name.to_string(), var);
        Ok(var)
    }

    /// Fails with `message` at `span` when the `Bool` `test` holds.
    pub(super) fn fail_if(&mut self, test: Rvalue, span: Span, message: &str) {
        let cond = self.temp(Ty::Bool, test, span);
        let (fail, ok) = (self.new_block(), self.new_block());
        self.terminate(Term::Branch {
            cond,
            then: fail,
            orelse: ok,
        });
        self.switch_to(fail);
        self.terminate(Term::Fail(runtime(span, message)));
        self.switch_to(ok);
    }
}

pub(super) fn runtime(span: Span, message: &str) -> Diagnostic {
    Diagnostic {
        span,
        message: message.to_string(),
        kind: DiagKind::Runtime,
    }
}
