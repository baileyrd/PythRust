//! Optionals (`T | None`): wrapping a value where an optional is
//! expected, unwrapping a narrowed one, and the tests that see `None`.

use crate::diag::Result;
use crate::infer::Type;
use crate::ir::{BlockId, CmpOp, Operand, Rvalue, Term, Ty};
use crate::span::Span;

use super::{unsupported, FnBuilder};

impl FnBuilder<'_, '_> {
    /// `v` as type `want`: wrapped when `want` is an optional of `v`'s
    /// type, or `v` is `None`. Any other `v` is left for the caller's
    /// own type check.
    ///
    /// A tuple whose fields fit `want`'s is rebuilt as `want`, as for
    /// `t: tuple[int | None, str] = (None, "a")`.
    pub(super) fn fit(&mut self, v: Operand, want: Ty, span: Span) -> Operand {
        let ty = self.ty(v);
        if let (Ty::Tuple(_), Ty::Tuple(_)) = (ty, want) {
            return self.fit_tuple(v, want, span);
        }
        if ty == want || !matches!(want, Ty::Opt(_)) {
            return v;
        }
        if ty == Ty::None || self.cx.seqs.present(want) == Some(ty) {
            return self.temp(want, Rvalue::Wrap(v), span);
        }
        v
    }

    fn fit_tuple(&mut self, v: Operand, want: Ty, span: Span) -> Operand {
        let have = self.cx.seqs.fields(self.ty(v)).map(<[Ty]>::to_vec);
        let wanted = self.cx.seqs.fields(want).map(<[Ty]>::to_vec);
        let (Some(have), Some(wanted)) = (have, wanted) else {
            return v;
        };
        if have == wanted || have.len() != wanted.len() {
            return v;
        }
        let mut items = Vec::new();
        for (k, (h, w)) in have.into_iter().zip(wanted).enumerate() {
            let f = self.temp(h, Rvalue::Field(v, k), span);
            let f = self.fit(f, w, span);
            if self.ty(f) != w {
                return v;
            }
            items.push(f);
        }
        self.temp(want, Rvalue::NewSeq(items), span)
    }

    /// Argument `v` for a parameter of declared type `param`: wrapped if
    /// the parameter is optional.
    pub(super) fn fit_param(&mut self, v: Operand, param: &Type, span: Span) -> Result<Operand> {
        if !matches!(param, Type::Optional(_)) {
            return Ok(v);
        }
        let want = match self.ty(v) {
            Ty::Opt(_) => return Ok(v),
            // The present type comes from the callee's signature, which
            // must not depend on its other arguments.
            Ty::None => self.concrete(param, span).map_err(|_| {
                unsupported(span, "`None` for a parameter of generic optional type")
            })?,
            ty => self.cx.seqs.optional(ty),
        };
        Ok(self.fit(v, want, span))
    }

    /// A read of optional `v` where inference narrowed it to its present
    /// type (the node at `span`): the present value.
    pub(super) fn narrow(&mut self, v: Operand, span: Span) -> Result<Operand> {
        let ty = self.ty(v);
        if !matches!(ty, Ty::Opt(_)) {
            return Ok(v);
        }
        let seen = self.ty_at(span)?;
        if self.cx.seqs.present(ty) != Some(seen) {
            return Ok(v);
        }
        Ok(self.temp(seen, Rvalue::Unwrap(v), span))
    }

    /// Whether `v` is `None`, as a `Bool`.
    pub(super) fn is_none(&mut self, v: Operand, span: Span) -> Operand {
        match self.ty(v) {
            Ty::None => Operand::Bool(true),
            Ty::Opt(_) => self.temp(Ty::Bool, Rvalue::IsNone(v), span),
            _ => Operand::Bool(false),
        }
    }

    /// `l op r`, `==` or `!=`, where a side is optional: `None` equals
    /// only `None`, and present values compare as their type does.
    pub(super) fn opt_eq(
        &mut self,
        op: CmpOp,
        l: Operand,
        r: Operand,
        span: Span,
    ) -> Result<Operand> {
        if !matches!(op, CmpOp::Eq | CmpOp::Ne) {
            return Err(unsupported(span, "ordering an optional"));
        }
        let (ln, rn) = (self.is_none(l, span), self.is_none(r, span));
        if self.ty(l) == Ty::None || self.ty(r) == Ty::None {
            return Ok(self.temp(Ty::Bool, Rvalue::Cmp(op, ln, rn), span));
        }
        let dest = self.pooled(Ty::Bool);
        let (check_r, none, present, end) = (
            self.new_block(),
            self.new_block(),
            self.new_block(),
            self.new_block(),
        );
        self.branch(ln, none, check_r);
        self.switch_to(check_r);
        self.branch(rn, none, present);
        self.switch_to(none);
        self.assign(dest, Rvalue::Cmp(op, ln, rn), span);
        self.terminate(Term::Goto(end));
        self.switch_to(present);
        let (a, b) = (self.present(l, span), self.present(r, span));
        let c = self.cmp_values(op, a, b, span)?;
        self.assign(dest, Rvalue::Use(c), span);
        self.terminate(Term::Goto(end));
        self.switch_to(end);
        Ok(Operand::Local(dest))
    }

    /// `l is r` (`negate`: `is not`), where a side is `None`: inference
    /// allows `is` only there.
    pub(super) fn is(&mut self, l: Operand, r: Operand, negate: bool, span: Span) -> Operand {
        let op = if negate { CmpOp::Ne } else { CmpOp::Eq };
        match (self.is_none(l, span), self.is_none(r, span)) {
            (Operand::Bool(a), Operand::Bool(b)) => Operand::Bool((a == b) != negate),
            (a, b) => self.temp(Ty::Bool, Rvalue::Cmp(op, a, b), span),
        }
    }

    /// Truthiness of optional `v`: present and truthy.
    pub(super) fn opt_truthy(&mut self, v: Operand, span: Span) -> Operand {
        let dest = self.pooled(Ty::Bool);
        let n = self.is_none(v, span);
        let (none, present, end) = (self.new_block(), self.new_block(), self.new_block());
        self.branch(n, none, present);
        self.switch_to(none);
        self.assign(dest, Rvalue::Use(Operand::Bool(false)), span);
        self.terminate(Term::Goto(end));
        self.switch_to(present);
        let u = self.present(v, span);
        let t = self.truthy(u, span);
        self.assign(dest, Rvalue::Use(t), span);
        self.terminate(Term::Goto(end));
        self.switch_to(end);
        Operand::Local(dest)
    }

    /// `v`'s present value if optional, which the caller proved.
    fn present(&mut self, v: Operand, span: Span) -> Operand {
        let ty = self.ty(v);
        match self.cx.seqs.present(ty) {
            Some(p) => self.temp(p, Rvalue::Unwrap(v), span),
            None => v,
        }
    }

    fn branch(&mut self, cond: Operand, then: BlockId, orelse: BlockId) {
        self.terminate(Term::Branch { cond, then, orelse });
    }
}
