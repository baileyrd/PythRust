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

use super::{unsupported, FnBuilder, Hint, Hints};

/// A named local. `flag` is `None` for parameters no handler unbinds,
/// which are always bound.
#[derive(Debug, Clone, Copy)]
pub(super) struct Var {
    pub(super) local: LocalId,
    pub(super) flag: Option<LocalId>,
}

impl FnBuilder<'_, '_> {
    /// Reads variable `name`, failing at `span` if it is unbound.
    pub(super) fn read(&mut self, name: &str, span: Span) -> Result<Operand> {
        if self.defined.is_none() && !self.scope.contains(name) && self.cx.is_shared(name) {
            return self.read_global(name, span);
        }
        if let Some(Hint::Unbound) = self.hints.get(name) {
            // Never assigned by code that completes: the read fails.
            let message = self.unbound_message(name);
            self.jump_away(Term::Fail(runtime(span, &message)));
            return Err(self.diverge(span));
        }
        let var = self.var(name, span)?;
        let v = Operand::Local(var.local);
        let Some(flag) = var.flag else {
            return self.narrow(v, span);
        };
        let known = self.always_bound.contains(&var.local) || self.block_bound.contains(&var.local);
        if !known {
            let unbound = Rvalue::Cmp(CmpOp::Eq, Operand::Local(flag), Operand::Bool(false));
            let message = self.unbound_message(name);
            self.fail_if(unbound, span, &message);
            self.block_bound.insert(var.local);
        }
        self.narrow(v, span)
    }

    /// Assigns `value` to `name` and marks it bound from here on; a
    /// string's old value is released. The
    /// value must have the variable's type: the IR is typed, and a
    /// mismatch (inference and lowering disagreeing) is refused here
    /// rather than miscompiled.
    pub(super) fn write(&mut self, name: &str, value: Operand, span: Span) -> Result<()> {
        let var = self.var_of(name, span, Some(self.ty(value)))?;
        let value = self.fit(value, self.locals[var.local.0], span);
        if self.ty(value) != self.locals[var.local.0] {
            let what = format!("assigning `{name}` a value of another type than inferred");
            return Err(unsupported(span, &what));
        }
        if let Some(flag) = var.flag {
            self.assign(flag, Rvalue::Use(Operand::Bool(true)), span);
        }
        // A handler's name is unbound again when it ends.
        if self.cur.0 == 0 && !self.caught.contains(name) {
            self.always_bound.insert(var.local);
        }
        self.block_bound.insert(var.local);
        if !self.ty(value).is_rc() {
            self.assign(var.local, Rvalue::Use(value), span);
        } else {
            // Owned first: `value` may be this variable.
            let owned = self.own(value, span);
            self.emit(Inst::Release(var.local));
            self.move_into(var.local, owned, span);
        }
        self.mirror_write(name, Operand::Local(var.local), span)
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
        self.var_of(name, span, None)
    }

    /// [`var`](Self::var), for an assignment of a `value` of that type:
    /// a live value types a variable live code has not typed yet.
    fn var_of(&mut self, name: &str, span: Span, value: Option<Ty>) -> Result<Var> {
        if let Some(var) = self.names.get(name) {
            return Ok(*var);
        }
        if !self.scope.contains(name) {
            let what = format!("reading `{name}`, a global, from a function");
            return Err(unsupported(span, &what));
        }
        let node = self.node_type(span)?;
        match self.hints.get(name).copied() {
            Some(Hint::Typed(ty)) => {
                // As the assignment that gave it would have, bind what it
                // decides (an element type), for the code before it.
                super::bind(node, ty, &self.cx.seqs, &mut self.subst);
                return Ok(self.new_var(name, ty, span));
            }
            Some(Hint::Unbound) => {
                return Err(super::internal(span, "assigning a variable never assigned"))
            }
            None => {}
        }
        let ty = match (self.settled_ty_at(span)?, value) {
            (Some(ty), _) => ty,
            (None, Some(ty)) => {
                super::bind(node, ty, &self.cx.seqs, &mut self.subst);
                ty
            }
            (None, None) => {
                // Live code may type it later; typing it now from code
                // that never runs could reject that code.
                self.pending.push(name.to_string());
                self.stuck = true;
                return Err(unsupported(span, "a variable typed only by later code"));
            }
        };
        Ok(self.new_var(name, ty, span))
    }

    /// Hints for the variables first met untyped: the types live code
    /// gave them since; if it gave none and the body otherwise lowered
    /// (`lowered`), that they are never assigned. `None` if no variable
    /// was met untyped, or the body's own error stands.
    pub(super) fn pending_hints(&self, lowered: bool) -> Option<Hints> {
        if self.pending.is_empty() {
            return None;
        }
        let typed: Hints = self
            .pending
            .iter()
            .filter_map(|name| {
                let var = self.names.get(name)?;
                Some((name.clone(), Hint::Typed(self.locals[var.local.0])))
            })
            .collect();
        if !typed.is_empty() {
            return Some(typed);
        }
        if !lowered {
            return None;
        }
        let unbound = self.pending.iter().map(|n| (n.clone(), Hint::Unbound));
        Some(unbound.collect())
    }

    /// Creates variable `name` of type `ty` unless it exists: for a loop
    /// target whose type lowering knows better than generic inference.
    pub(super) fn declare(&mut self, name: &str, ty: Ty, span: Span) {
        if !self.names.contains_key(name) {
            self.new_var(name, ty, span);
        }
    }

    /// A new variable, its flag cleared in the entry block.
    fn new_var(&mut self, name: &str, ty: Ty, span: Span) -> Var {
        let local = self.new_local(ty);
        let var = Var {
            local,
            flag: Some(self.flag(false, span)),
        };
        self.names.insert(name.to_string(), var);
        var
    }

    /// A new boundness flag, set to `bound` first thing in the entry
    /// block.
    pub(super) fn flag(&mut self, bound: bool, span: Span) -> LocalId {
        let flag = self.new_local(Ty::Bool);
        let set = Inst::Assign {
            dest: flag,
            value: Rvalue::Use(Operand::Bool(bound)),
            span,
        };
        self.blocks[0].insts.insert(0, set);
        flag
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
