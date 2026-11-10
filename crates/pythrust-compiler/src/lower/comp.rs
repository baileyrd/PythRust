//! List, set and dict comprehensions (D85, D116), lowered as the nested
//! `for`/`if` loops they stand for, gathering into a fresh list, set or
//! dict: every check that runs on a loop (ownership, aliasing, tasks)
//! runs on them unchanged. A generator expression, a reducer's argument
//! (D119), is the same loop feeding the reducer: `sum` adds each item as
//! it comes, `any`/`all` stop at the item that decides, and the others,
//! which read every item before anything observable, gather a list.

use crate::ast::{Arg, CompFor, Expr, ExprKind, Gather};
use crate::diag::Result;
use crate::ir::{BlockId, CmpOp, Inst, IntOp, LocalId, Operand, Rvalue, Term, Ty};
use crate::span::Span;

use super::stmt::Looped;
use super::FnBuilder;

/// Where each pass of a comprehension goes.
#[derive(Clone, Copy)]
pub(super) enum Sink {
    /// Into a fresh list, set or dict.
    Into(LocalId),
    /// Added to an int total, as `sum` adds; `span` is the call's.
    Sum(LocalId, Span),
    /// `any` (`any` true) or `all`: `out` starts `!any` and becomes
    /// `any` at the item that decides, which ends every loop.
    Truth { any: bool, out: LocalId, span: Span },
}

impl FnBuilder<'_, '_> {
    /// The comprehension at `span`: a new list, set or dict, filled by
    /// the loops.
    pub(super) fn comp(
        &mut self,
        gather: &Gather,
        clauses: &[CompFor],
        span: Span,
    ) -> Result<Operand> {
        let ty = self.ty_at(span)?;
        let out = self.pooled(ty);
        self.assign(out, Rvalue::NewSeq(Vec::new()), span);
        self.comp_clauses(gather, clauses, Sink::Into(out))?;
        Ok(Operand::Local(out))
    }

    /// `any`, `all` or an int `sum` (`name`) of a generator expression
    /// (D119), run as its loop, a `sum`'s `start` read first (D123);
    /// `None` for any other call, whose generator expression gathers a
    /// list as a list comprehension does.
    pub(super) fn reduce_gen(
        &mut self,
        name: &str,
        args: &[Arg],
        span: Span,
    ) -> Result<Option<Operand>> {
        let Some((arg, rest)) = args.split_first() else {
            return Ok(None);
        };
        let ExprKind::Comp {
            gather: gather @ Gather::Gen(_),
            clauses,
        } = &arg.value.kind
        else {
            return Ok(None);
        };
        let (sink, out) = match (name, rest) {
            ("any" | "all", []) => {
                let out = self.pooled(Ty::Bool);
                let any = name == "any";
                self.assign(out, Rvalue::Use(Operand::Bool(!any)), span);
                (Sink::Truth { any, out, span }, out)
            }
            ("sum", [] | [_]) if self.ty_at(span)? == Ty::Int => {
                let start = match rest.first() {
                    Some(a) => {
                        let v = self.expr(&a.value)?;
                        self.as_int(v, a.value.span)
                    }
                    None => Operand::Int(0),
                };
                let out = self.pooled(Ty::Int);
                self.assign(out, Rvalue::Use(start), span);
                (Sink::Sum(out, span), out)
            }
            _ => return Ok(None),
        };
        self.comp_clauses(gather, clauses, sink)?;
        Ok(Some(Operand::Local(out)))
    }

    /// The loop of the first of `clauses`, the rest nested inside it;
    /// with none left, sends one pass to `sink`.
    fn comp_clauses(&mut self, gather: &Gather, clauses: &[CompFor], sink: Sink) -> Result<()> {
        let Some((c, rest)) = clauses.split_first() else {
            return self.pooled_stmt(|b| b.gather(gather, sink));
        };
        let looped = Looped::Comp {
            target: &c.target,
            ifs: &c.ifs,
            gather,
            rest,
            sink,
            takes: self.takes(&c.iter)?,
        };
        self.loop_over(&looped, &c.iter)
    }

    /// One pass: appends the item, adds it to the set, stores the key
    /// (evaluated first) and the value, as `d[k] = v` does, or hands the
    /// item to its reducer.
    fn gather(&mut self, gather: &Gather, sink: Sink) -> Result<()> {
        let out = match sink {
            Sink::Into(out) => out,
            Sink::Sum(total, span) => {
                let e = gen_item(gather, span)?;
                return self.sum_item(e, total, span);
            }
            Sink::Truth { any, out, span } => {
                let e = gen_item(gather, span)?;
                return self.truth_item(e, any, out);
            }
        };
        let table = Operand::Local(out);
        let ty = self.ty(table);
        match gather {
            Gather::List(e) | Gather::Gen(e) => {
                let v = self.expr(e)?;
                let want = self.slot_ty(ty, 0, e.span)?;
                let value = self.expect(v, want, e.span)?;
                self.emit(Inst::Append {
                    list: table,
                    value,
                    span: e.span,
                });
            }
            Gather::Set(e) => {
                let v = self.expr(e)?;
                let elem = self.elem_of(ty, e.span)?;
                let value = self.stored(v, elem, e.span)?;
                self.emit(Inst::Append {
                    list: table,
                    value,
                    span: e.span,
                });
            }
            Gather::Dict(k, v) => {
                let key = self.expr(k)?;
                let value = self.expr(v)?;
                self.dict_store(table, key, value, k.span.merge(v.span))?;
            }
        }
        Ok(())
    }

    /// `total += item`, checked as `sum`'s ints are: an overflow is the
    /// call's (`span`), as in `run`.
    fn sum_item(&mut self, e: &Expr, total: LocalId, span: Span) -> Result<()> {
        let v = self.expr(e)?;
        let x = self.as_int(v, e.span);
        let added = Rvalue::Int(IntOp::Add, Operand::Local(total), x);
        self.assign(total, added, span);
        Ok(())
    }

    /// `out = any` when the item's truth is `any`: it decides.
    fn truth_item(&mut self, e: &Expr, any: bool, out: LocalId) -> Result<()> {
        let v = self.expr(e)?;
        let t = self.truthy(v, e.span);
        let decides = Rvalue::Cmp(CmpOp::Eq, t, Operand::Bool(any));
        let decided = self.temp(Ty::Bool, decides, e.span);
        self.either(
            decided,
            |b| b.assign(out, Rvalue::Use(Operand::Bool(any)), e.span),
            |_| {},
        );
        Ok(())
    }

    /// One pass of a clause whose target is bound: each filter that
    /// fails continues at `latch`; past them all, the rest of the
    /// clauses, after which a decided `any`/`all` leaves for `end`.
    pub(super) fn comp_body(
        &mut self,
        ifs: &[Expr],
        gather: &Gather,
        rest: &[CompFor],
        sink: Sink,
        (latch, end): (BlockId, BlockId),
    ) -> Result<()> {
        for test in ifs {
            let pass = self.new_block();
            let cond = self.cond(test, false)?;
            self.terminate(Term::Branch {
                cond,
                then: pass,
                orelse: latch,
            });
            self.switch_to(pass);
        }
        // As a statement of the body: what a nested loop holds (its
        // iterable) is released each pass.
        self.pooled_stmt(|b| b.comp_clauses(gather, rest, sink))?;
        let Sink::Truth { any, out, span } = sink else {
            self.terminate(Term::Goto(latch));
            return Ok(());
        };
        let decided = Rvalue::Cmp(CmpOp::Eq, Operand::Local(out), Operand::Bool(any));
        let cond = self.temp(Ty::Bool, decided, span);
        self.terminate(Term::Branch {
            cond,
            then: end,
            orelse: latch,
        });
        Ok(())
    }
}

/// The item a generator expression yields: only `Gen` reaches a reducer
/// (inference refuses a generator expression anywhere else, D121).
fn gen_item(gather: &Gather, span: Span) -> Result<&Expr> {
    match gather {
        Gather::Gen(e) => Ok(e),
        _ => Err(super::internal(
            span,
            "a reducer fed by other than a generator expression",
        )),
    }
}
