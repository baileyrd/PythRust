//! List, set and dict comprehensions (D85, D116), lowered as the nested
//! `for`/`if` loops they stand for, gathering into a fresh list, set or
//! dict: every check that runs on a loop (ownership, aliasing, tasks)
//! runs on them unchanged.

use crate::ast::{CompFor, Expr, Gather};
use crate::diag::Result;
use crate::ir::{BlockId, Inst, LocalId, Operand, Rvalue, Term};
use crate::span::Span;

use super::stmt::Looped;
use super::FnBuilder;

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
        self.comp_clauses(gather, clauses, out)?;
        Ok(Operand::Local(out))
    }

    /// The loop of the first of `clauses`, the rest nested inside it;
    /// with none left, gathers one pass into `out`.
    fn comp_clauses(&mut self, gather: &Gather, clauses: &[CompFor], out: LocalId) -> Result<()> {
        let Some((c, rest)) = clauses.split_first() else {
            return self.pooled_stmt(|b| b.gather(gather, out));
        };
        let looped = Looped::Comp {
            target: &c.target,
            ifs: &c.ifs,
            gather,
            rest,
            out,
            takes: self.takes(&c.iter)?,
        };
        self.loop_over(&looped, &c.iter)
    }

    /// One pass: appends the item, adds it to the set, or stores the
    /// key (evaluated first) and the value, as `d[k] = v` does.
    fn gather(&mut self, gather: &Gather, out: LocalId) -> Result<()> {
        let table = Operand::Local(out);
        let ty = self.ty(table);
        match gather {
            Gather::List(e) => {
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

    /// One pass of a clause whose target is bound: each filter that
    /// fails continues at `latch`; past them all, the rest of the
    /// clauses.
    pub(super) fn comp_body(
        &mut self,
        ifs: &[Expr],
        gather: &Gather,
        rest: &[CompFor],
        out: LocalId,
        latch: BlockId,
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
        self.pooled_stmt(|b| b.comp_clauses(gather, rest, out))?;
        self.terminate(Term::Goto(latch));
        Ok(())
    }
}
