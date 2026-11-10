//! List comprehensions (D85), lowered as the nested `for`/`if` loops
//! they stand for, appending to a fresh list: every check that runs on
//! a loop (ownership, aliasing, tasks) runs on them unchanged.

use crate::ast::{CompFor, Expr};
use crate::diag::Result;
use crate::ir::{BlockId, Inst, LocalId, Operand, Rvalue, Term};
use crate::span::Span;

use super::stmt::Looped;
use super::FnBuilder;

impl FnBuilder<'_, '_> {
    /// `[elt for ...]` at `span`: a new list, filled by the loops.
    pub(super) fn list_comp(
        &mut self,
        elt: &Expr,
        clauses: &[CompFor],
        span: Span,
    ) -> Result<Operand> {
        let ty = self.ty_at(span)?;
        let out = self.pooled(ty);
        self.assign(out, Rvalue::NewSeq(Vec::new()), span);
        self.comp_clauses(elt, clauses, out)?;
        Ok(Operand::Local(out))
    }

    /// The loop of the first of `clauses`, the rest nested inside it;
    /// with none left, appends `elt` to `out`.
    fn comp_clauses(&mut self, elt: &Expr, clauses: &[CompFor], out: LocalId) -> Result<()> {
        let Some((c, rest)) = clauses.split_first() else {
            return self.comp_append(elt, out);
        };
        let looped = Looped::Comp {
            target: &c.target,
            ifs: &c.ifs,
            elt,
            rest,
            out,
            takes: self.takes(&c.iter)?,
        };
        self.loop_over(&looped, &c.iter)
    }

    fn comp_append(&mut self, elt: &Expr, out: LocalId) -> Result<()> {
        self.pooled_stmt(|b| {
            let v = b.expr(elt)?;
            let list = Operand::Local(out);
            let want = b.slot_ty(b.ty(list), 0, elt.span)?;
            let value = b.expect(v, want, elt.span)?;
            let span = elt.span;
            b.emit(Inst::Append { list, value, span });
            Ok(())
        })
    }

    /// One pass of a clause whose target is bound: each filter that
    /// fails continues at `latch`; past them all, the rest of the
    /// clauses.
    pub(super) fn comp_body(
        &mut self,
        ifs: &[Expr],
        elt: &Expr,
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
        self.pooled_stmt(|b| b.comp_clauses(elt, rest, out))?;
        self.terminate(Term::Goto(latch));
        Ok(())
    }
}
