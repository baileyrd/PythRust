//! Comparisons of tuples with different positional types or lengths.

use crate::diag::Result;
use crate::ir::{CmpOp, Operand, Rvalue, Term, Ty};
use crate::span::Span;

use super::{internal, FnBuilder};

impl FnBuilder<'_, '_> {
    /// Compare only the shared prefix, stopping at the first unequal
    /// field. Each field keeps its own type (notably int/float/bool);
    /// equal prefixes compare by length. Operands are already evaluated.
    pub(super) fn tuple_cmp(
        &mut self,
        op: CmpOp,
        l: Operand,
        r: Operand,
        span: Span,
    ) -> Result<Operand> {
        let fields = |v| {
            self.cx
                .seqs
                .fields(self.ty(v))
                .map(<[Ty]>::to_vec)
                .ok_or_else(|| internal(span, "a tuple comparison without fields"))
        };
        let (left, right) = (fields(l)?, fields(r)?);
        let (ln, rn) = (left.len(), right.len());
        // Equality is also needed for nested tuple prefixes, whose
        // lengths may differ even when their outer tuples match.
        if matches!(op, CmpOp::Eq | CmpOp::Ne) && ln != rn {
            return Ok(Operand::Bool(op == CmpOp::Ne));
        }
        let dest = self.pooled(Ty::Bool);
        let end = self.new_block();
        for (k, (lt, rt)) in left.into_iter().zip(right).enumerate() {
            let a = self.temp(lt, Rvalue::Field(l, k), span);
            let b = self.temp(rt, Rvalue::Field(r, k), span);
            let eq = self.equal(a, b, span)?;
            let (next, different) = (self.new_block(), self.new_block());
            self.terminate(Term::Branch {
                cond: eq,
                then: next,
                orelse: different,
            });
            self.switch_to(different);
            let result = match op {
                CmpOp::Eq => Operand::Bool(false),
                CmpOp::Ne => Operand::Bool(true),
                _ => self.cmp_values(op, a, b, span)?,
            };
            self.assign(dest, Rvalue::Use(result), span);
            self.terminate(Term::Goto(end));
            self.switch_to(next);
        }
        let result = match op {
            CmpOp::Eq => ln == rn,
            CmpOp::Ne => ln != rn,
            CmpOp::Lt => ln < rn,
            CmpOp::Le => ln <= rn,
            CmpOp::Gt => ln > rn,
            CmpOp::Ge => ln >= rn,
        };
        self.assign(dest, Rvalue::Use(Operand::Bool(result)), span);
        self.terminate(Term::Goto(end));
        self.switch_to(end);
        Ok(Operand::Local(dest))
    }
}
