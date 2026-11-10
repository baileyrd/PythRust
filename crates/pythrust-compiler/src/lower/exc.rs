//! Exceptions: `raise`, `try`/`except`, and the test after whatever can
//! raise (`ir`'s module docs). A raise goes to the innermost enclosing
//! `try`'s dispatch, which tests each handler's kind in turn and passes
//! an unmatched exception outward; with no `try` left, a function
//! returns to its caller through the exit block, which releases its
//! references, and the module's code ends with `Term::Uncaught`.

use crate::ast::{Block, Expr, ExprKind, Handler};
use crate::diag::Result;
use crate::exc::ExcKind;
use crate::ir::Rvalue::Unstash;
use crate::ir::{Inst, LocalId, Operand, Rvalue, Term, Ty};
use crate::span::Span;

use super::{internal, Fin, FnBuilder, Try, Unbind};

impl FnBuilder<'_, '_> {
    /// `raise value`; `raise ValueError` raises a new one without an
    /// argument, as `raise ValueError()` does.
    pub(super) fn raise_stmt(&mut self, value: &Expr, span: Span) -> Result<()> {
        let class = match &value.kind {
            ExprKind::Name(name) => self.exc_class(name),
            _ => None,
        };
        let exc = match class {
            Some(kind) => self.temp(Ty::Exc, Rvalue::NewExc(kind, None), value.span),
            None => self.expr(value)?,
        };
        self.emit(Inst::Raise { exc, span });
        self.raise_path()
    }

    /// `try`, its handlers and its `finally` block. The block is lowered
    /// once per way out: falling off the end, an exception on its way out
    /// (stashed while the block runs, then raised again as it was), and
    /// each `return`, `break` or `continue` leaving the `try`.
    pub(super) fn try_stmt(
        &mut self,
        body: &Block,
        handlers: &[Handler],
        finally: Option<&Block>,
    ) -> Result<()> {
        let Some(fin) = finally else {
            return self.try_except(body, handlers);
        };
        let (dispatch, end) = (self.new_block(), self.new_block());
        let (pools, stashes) = (self.pools.len(), self.stashes);
        self.finallys.push(Fin {
            body: fin.clone(),
            loops: self.loops.len(),
            tries: self.tries.len(),
            pools,
            unbinds: self.unbinds.len(),
        });
        self.tries.push(Try {
            dispatch,
            pools,
            stashes,
        });
        let lowered = match handlers {
            [] => self.block(body),
            _ => self.try_except(body, handlers),
        };
        self.tries.pop();
        self.finallys.pop();
        lowered?;
        self.block(fin)?;
        self.terminate(Term::Goto(end));
        self.switch_to(dispatch);
        self.emit(Inst::Stash);
        self.stashes += 1;
        let lowered = self.block(fin);
        self.stashes -= 1;
        lowered?;
        self.emit(Inst::Restore);
        self.raise_path()?;
        self.switch_to(end);
        Ok(())
    }

    /// On the way out of the `try` statements of `self.finallys[from..]`,
    /// innermost first: each one's statements end and its handlers'
    /// names that `left` picks are unbound, then its `finally` block runs
    /// where the `try` is, outside its body. Then the names `left` picks
    /// outside them all are unbound: a handler's name stays bound for the
    /// `finally` blocks inside the handler, as in Python. `keep` holds a
    /// `return`'s value, released if a block raises out of its `try`.
    pub(super) fn leave_finallys(
        &mut self,
        from: usize,
        left: impl Fn(&Unbind) -> bool,
        keep: Option<LocalId>,
    ) -> Result<()> {
        let mut bound = self.unbinds.len();
        for i in (from..self.finallys.len()).rev() {
            let fin = self.finallys[i].clone();
            let temps: Vec<LocalId> = self.pools[fin.pools..].concat();
            temps.into_iter().for_each(|t| self.emit(Inst::Release(t)));
            self.unbind_range(fin.unbinds..bound, &left)?;
            bound = bound.min(fin.unbinds);
            let loops = self.loops.split_off(fin.loops);
            let tries = self.tries.split_off(fin.tries);
            let finallys = self.finallys.split_off(i);
            let unbinds = self.unbinds.split_off(fin.unbinds);
            self.returning.extend(keep.map(|k| (k, fin.tries)));
            let lowered = self.block(&fin.body);
            if keep.is_some() {
                self.returning.pop();
            }
            self.loops.truncate(fin.loops);
            self.tries.truncate(fin.tries);
            self.finallys.truncate(i);
            self.unbinds.truncate(fin.unbinds);
            self.loops.extend(loops);
            self.tries.extend(tries);
            self.finallys.extend(finallys);
            self.unbinds.extend(unbinds);
            lowered?;
        }
        self.unbind_range(0..bound, &left)
    }

    /// Unbinds the names of `self.unbinds[range]` that `left` picks.
    fn unbind_range(
        &mut self,
        range: std::ops::Range<usize>,
        left: &impl Fn(&Unbind) -> bool,
    ) -> Result<()> {
        let leaving: Vec<Unbind> = self.unbinds[range]
            .iter()
            .filter(|u| left(u))
            .cloned()
            .collect();
        leaving
            .into_iter()
            .try_for_each(|u| self.unbind(&u.name, u.span))
    }

    /// `try` and its handlers. The body raises into a dispatch that runs
    /// the first handler naming the pending exception's kind.
    fn try_except(&mut self, body: &Block, handlers: &[Handler]) -> Result<()> {
        let (dispatch, end) = (self.new_block(), self.new_block());
        let pools = self.pools.len();
        let stashes = self.stashes;
        self.tries.push(Try {
            dispatch,
            pools,
            stashes,
        });
        let lowered = self.block(body);
        self.tries.pop();
        lowered?;
        self.terminate(Term::Goto(end));
        self.switch_to(dispatch);
        for h in handlers {
            let classes = h
                .classes
                .iter()
                .map(|c| {
                    ExcKind::from_name(&c.name)
                        .ok_or_else(|| internal(c.span, "a handler of no builtin exception"))
                })
                .collect::<Result<Vec<_>>>()?;
            let (catch, next) = (self.new_block(), self.new_block());
            let cond = self.temp(Ty::Bool, Rvalue::PendingIs(classes), h.span);
            self.terminate(Term::Branch {
                cond,
                then: catch,
                orelse: next,
            });
            self.switch_to(catch);
            self.handler(h)?;
            self.terminate(Term::Goto(end));
            self.switch_to(next);
        }
        // No handler matched: the exception goes on outward.
        self.raise_path()?;
        self.switch_to(end);
        Ok(())
    }

    /// Takes the pending exception and runs `h`, its `as` name bound to
    /// it, then unbinds the name, as Python does.
    fn handler(&mut self, h: &Handler) -> Result<()> {
        let e = self.temp(Ty::Exc, Rvalue::Catch, h.span);
        let Some(binding) = &h.binding else {
            if let Operand::Local(l) = e {
                self.emit(Inst::Release(l));
            }
            return self.block(&h.body);
        };
        self.write(&binding.name, e, binding.span)?;
        self.unbinds.push(Unbind {
            name: binding.name.clone(),
            span: binding.span,
            loops: self.loops.len(),
            tries: self.tries.len(),
        });
        let lowered = self.block(&h.body);
        self.unbinds.pop();
        lowered?;
        self.unbind(&binding.name, binding.span)
    }

    /// After something that may have raised, at `span`: goes the way a
    /// raise does if it did.
    pub(super) fn check_raised(&mut self, span: Span) -> Result<()> {
        let cond = self.temp(Ty::Bool, Rvalue::Raised, span);
        let (raised, ok) = (self.new_block(), self.new_block());
        self.terminate(Term::Branch {
            cond,
            then: raised,
            orelse: ok,
        });
        self.switch_to(raised);
        self.raise_path()?;
        self.switch_to(ok);
        Ok(())
    }

    /// Ends the current block where a raise goes: the innermost `try`'s
    /// dispatch, after releasing the temporaries of the statements left
    /// and unbinding the handler names left, else the unwind block, whose
    /// exit releases everything (or the program ends).
    pub(super) fn raise_path(&mut self) -> Result<()> {
        self.abandon_returns();
        let Some(t) = self.tries.last().copied() else {
            self.drop_stashes(0);
            let unwind = self.unwind_block();
            self.jump_away(Term::Goto(unwind));
            return Ok(());
        };
        self.drop_stashes(t.stashes);
        let left: Vec<LocalId> = self.pools[t.pools..].concat();
        left.into_iter().for_each(|l| self.emit(Inst::Release(l)));
        let depth = self.tries.len();
        self.unbind_left(|u| u.tries >= depth)?;
        self.jump_away(Term::Goto(t.dispatch));
        Ok(())
    }

    /// Releases the return values a raise from here abandons: those whose
    /// running `finally` block it leaves. A `try` inside the block catches
    /// without abandoning, and the `return` goes on.
    fn abandon_returns(&mut self) {
        let depth = self.tries.len();
        let left: Vec<LocalId> = self
            .returning
            .iter()
            .filter(|(_, tries)| depth <= *tries)
            .map(|(l, _)| *l)
            .collect();
        left.into_iter().for_each(|l| self.emit(Inst::Release(l)));
    }

    /// Drops the exceptions stashed above the first `kept`: an exception
    /// raised in a `finally` block replaces the one it was running for
    /// (D88).
    fn drop_stashes(&mut self, kept: usize) {
        for _ in kept..self.stashes {
            let span = Span { start: 0, end: 0 };
            if let Operand::Local(e) = self.temp(Ty::Exc, Unstash, span) {
                self.emit(Inst::Release(e));
            }
        }
    }

    /// Unbinds the names of the handlers being left: those `left` picks.
    pub(super) fn unbind_left(&mut self, left: impl Fn(&Unbind) -> bool) -> Result<()> {
        self.unbind_range(0..self.unbinds.len(), &left)
    }

    /// Unbinds variable `name`: a later read fails as unbound.
    fn unbind(&mut self, name: &str, span: Span) -> Result<()> {
        let var = *self
            .names
            .get(name)
            .ok_or_else(|| internal(span, "unbinding a variable never bound"))?;
        if let Some(flag) = var.flag {
            self.assign(flag, Rvalue::Use(Operand::Bool(false)), span);
        }
        self.emit(Inst::Release(var.local));
        self.always_bound.remove(&var.local);
        self.block_bound.remove(&var.local);
        self.mirror_unbind(name);
        Ok(())
    }

    /// Where an exception no `try` here catches goes: back to the
    /// caller through the exit block, or out of the program.
    fn unwind_block(&mut self) -> super::BlockId {
        if let Some(b) = self.unwind {
            return b;
        }
        let b = self.new_block();
        let term = match self.defined {
            Some(_) => Term::Uncaught,
            None => Term::Goto(self.exit),
        };
        self.blocks[b.0].term = Some(term);
        self.unwind = Some(b);
        b
    }

    /// The builtin exception `name` names here, unless a variable or
    /// function shadows it.
    pub(super) fn exc_class(&self, name: &str) -> Option<ExcKind> {
        let shadowed =
            self.scope.contains(name) || self.cx.defs.contains_key(name) || self.cx.is_shared(name);
        ExcKind::from_name(name).filter(|_| !shadowed)
    }

    /// `kind(args)`: a new exception holding the one argument, if any.
    pub(super) fn new_exc(
        &mut self,
        kind: ExcKind,
        args: &[Operand],
        span: Span,
    ) -> Result<Operand> {
        let arg = match args {
            [] => None,
            [arg] => Some(*arg),
            _ => return Err(internal(span, "an exception of several arguments")),
        };
        Ok(self.temp(Ty::Exc, Rvalue::NewExc(kind, arg), span))
    }

    /// Whether a call to top-level `name` may raise: its `raises` set
    /// is not empty, or unknown, or it may fault catchably.
    pub(super) fn may_raise(&self, name: &str) -> bool {
        self.cx.faulting.contains(name) || self.cx.types.raises(name).is_none_or(|r| !r.is_empty())
    }

    /// Raises `kind` holding `message` at `span` when the `Bool` `test`
    /// holds: a catchable fault lowering detects itself.
    pub(super) fn raise_if(
        &mut self,
        test: Rvalue,
        kind: ExcKind,
        message: &str,
        span: Span,
    ) -> Result<()> {
        let cond = self.temp(Ty::Bool, test, span);
        let (raise, ok) = (self.new_block(), self.new_block());
        self.terminate(Term::Branch {
            cond,
            then: raise,
            orelse: ok,
        });
        self.switch_to(raise);
        let text = self.temp(Ty::Str, Rvalue::StrLit(message.to_string()), span);
        let exc = self.new_exc(kind, &[text], span)?;
        self.emit(Inst::Raise { exc, span });
        self.raise_path()?;
        self.switch_to(ok);
        Ok(())
    }

    /// Whether a value of type `ty` is, or may hold, one of a type
    /// `leaf` picks.
    pub(super) fn holds(&self, ty: Ty, leaf: fn(Ty) -> bool) -> bool {
        crate::ir::holds(self.cx.seqs.all(), ty, leaf)
    }
}
