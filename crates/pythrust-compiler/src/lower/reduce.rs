//! The reducing builtins (D56, D57): `abs`, `sum`, `min`, `max`, `any`
//! and `all`, as loops over what `for` iterates; `ord` and `chr` are
//! runtime calls ([`crate::stdlib::StdFn`]).

use crate::diag::Result;
use crate::exc::ExcKind;
use crate::ir::{CmpOp, FloatOp, IntOp, LocalId, Operand, Rvalue, Term, Ty};
use crate::span::Span;
use crate::stdlib::StdFn;

use super::{internal, unsupported, FnBuilder};

impl FnBuilder<'_, '_> {
    /// `abs(x)`: a bool as its int; a negative int negated, which fails
    /// for `i64::MIN` as `-x` does; a float with its sign cleared, so
    /// `abs(-0.0)` is `0.0`.
    pub(super) fn abs(&mut self, x: Operand, span: Span) -> Result<Operand> {
        let x = self.as_int(x, span);
        let ty = self.ty(x);
        let zero = match ty {
            Ty::Int => Operand::Int(0),
            Ty::Float => Operand::Float(0.0),
            other => return Err(internal(span, &format!("`abs` of a {other:?}"))),
        };
        let out = self.pooled(ty);
        let negative = self.temp(Ty::Bool, Rvalue::Cmp(CmpOp::Lt, x, zero), span);
        let kept = match ty {
            // `-0.0 + 0.0` is `0.0`; every other float stays as it is.
            Ty::Float => Rvalue::Float(FloatOp::Add, x, zero),
            _ => Rvalue::Use(x),
        };
        self.either(
            negative,
            |b| b.assign(out, Rvalue::Neg(x), span),
            |b| b.assign(out, kept, span),
        );
        Ok(Operand::Local(out))
    }

    /// `sum(xs)` or `sum(xs, start)`: ints and bools added as checked
    /// ints, floats by the runtime's compensated sum, as CPython's.
    pub(super) fn sum(&mut self, ops: &[Operand], span: Span) -> Result<Operand> {
        let (xs, start) = match ops {
            [xs] => (*xs, None),
            [xs, start] => (*xs, Some(*start)),
            _ => return Err(internal(span, "`sum` of other than 1 or 2 arguments")),
        };
        if self.ty_at(span)? == Ty::Float {
            let start = start.unwrap_or(Operand::Float(0.0));
            let v = self.temp(
                Ty::Float,
                Rvalue::Std(StdFn::FloatSum, vec![xs, start]),
                span,
            );
            return Ok(v);
        }
        let total = self.pooled(Ty::Int);
        let start = start.map_or(Operand::Int(0), |s| self.as_int(s, span));
        self.assign(total, Rvalue::Use(start), span);
        self.each(xs, span, |b, x| {
            let x = b.as_int(x, span);
            b.assign(
                total,
                Rvalue::Int(IntOp::Add, Operand::Local(total), x),
                span,
            );
            Ok(())
        })?;
        Ok(Operand::Local(total))
    }

    /// `min` (`Lt`) or `max` (`Gt`) of one iterable, raising CPython's
    /// `ValueError` when it is empty, or of two or more arguments: the
    /// first item no later one beats.
    pub(super) fn extreme(&mut self, name: &str, ops: &[Operand], span: Span) -> Result<Operand> {
        let op = if name == "min" { CmpOp::Lt } else { CmpOp::Gt };
        let ty = self.ty_at(span)?;
        let best = self.pooled(ty);
        let [xs] = ops else {
            let Some((first, rest)) = ops.split_first() else {
                return Err(internal(span, &format!("`{name}` without arguments")));
            };
            self.put(best, *first, span);
            for x in rest {
                self.beat(op, best, *x, span);
            }
            return Ok(Operand::Local(best));
        };
        let n = self.count(*xs, span);
        let message = format!("{name}() iterable argument is empty");
        let empty = Rvalue::Cmp(CmpOp::Eq, n, Operand::Int(0));
        self.raise_if(empty, ExcKind::Value, &message, span)?;
        let (item, read) = self.items_of(*xs, span)?;
        self.pooled_stmt(|b| {
            let first = b.temp(item, read(*xs, Operand::Int(0)), span);
            b.put(best, first, span);
            Ok(())
        })?;
        self.each(*xs, span, |b, x| {
            b.beat(op, best, x, span);
            Ok(())
        })?;
        Ok(Operand::Local(best))
    }

    /// `any(xs)` (`any` true) or `all(xs)`: whether some item is true,
    /// or every one. In `build`, `map`'s results are taken until one
    /// decides, as CPython stops there.
    pub(super) fn truth(&mut self, any: bool, xs: Operand, span: Span) -> Result<Operand> {
        let out = self.pooled(Ty::Bool);
        self.assign(out, Rvalue::Use(Operand::Bool(!any)), span);
        if self.ty(xs) == Ty::Results {
            let item = self.result_ty(span)?;
            self.take_each(xs, item, span, |b, x| {
                let t = b.truthy(x, span);
                b.assign(out, Rvalue::Use(t), span);
                let decided = Rvalue::Cmp(CmpOp::Eq, t, Operand::Bool(any));
                Ok(Some(b.temp(Ty::Bool, decided, span)))
            })?;
            return Ok(Operand::Local(out));
        }
        if !matches!(
            self.ty(xs),
            Ty::Str
                | Ty::Range
                | Ty::List(_)
                | Ty::Set(_)
                | Ty::View(_)
                | Ty::Dict(_)
                | Ty::Tuple(_)
        ) {
            let what = if any {
                "`any` of this type"
            } else {
                "`all` of this type"
            };
            return Err(unsupported(span, what));
        }
        self.each(xs, span, |b, x| {
            let t = b.truthy(x, span);
            let decided = b.temp(
                Ty::Bool,
                Rvalue::Cmp(CmpOp::Eq, t, Operand::Bool(any)),
                span,
            );
            b.either(
                decided,
                |b| b.assign(out, Rvalue::Use(Operand::Bool(any)), span),
                |_| {},
            );
            Ok(())
        })?;
        Ok(Operand::Local(out))
    }

    /// `best = x` when `x` beats it under `op`.
    fn beat(&mut self, op: CmpOp, best: LocalId, x: Operand, span: Span) {
        let wins = self.temp(Ty::Bool, Rvalue::Cmp(op, x, Operand::Local(best)), span);
        self.either(wins, |b| b.put(best, x, span), |_| {});
    }

    /// `dest = x`, releasing what `dest` held and keeping a reference
    /// of its own to `x`.
    fn put(&mut self, dest: LocalId, x: Operand, span: Span) {
        if !self.ty(x).is_rc() {
            self.assign(dest, Rvalue::Use(x), span);
            return;
        }
        let owned = self.own(x, span);
        self.emit(crate::ir::Inst::Release(dest));
        self.move_into(dest, owned, span);
    }

    /// Runs `then` where `cond` holds and `orelse` where it does not,
    /// continuing after both.
    fn either(
        &mut self,
        cond: Operand,
        then: impl FnOnce(&mut Self),
        orelse: impl FnOnce(&mut Self),
    ) {
        let (yes, no, join) = (self.new_block(), self.new_block(), self.new_block());
        self.terminate(Term::Branch {
            cond,
            then: yes,
            orelse: no,
        });
        self.switch_to(yes);
        then(self);
        self.terminate(Term::Goto(join));
        self.switch_to(no);
        orelse(self);
        self.terminate(Term::Goto(join));
        self.switch_to(join);
    }
}
