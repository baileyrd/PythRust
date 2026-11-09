//! The list methods that read or change their list, the dict methods
//! that change their dict, and `del` (D70, D71): list `pop`, `extend`,
//! `insert`, `remove`, `index`, `count` and `sort`; dict `pop`,
//! `setdefault` and `update`; `del xs[i]` and `del d[k]`.

use crate::ast::{Arg, Expr};
use crate::diag::Result;
use crate::exc::ExcKind;
use crate::ir::{CmpOp, Inst, IntOp, Operand, Rvalue, Term, Ty};
use crate::span::Span;
use crate::stdlib::StdFn;

use super::{internal, unsupported, FnBuilder};

impl FnBuilder<'_, '_> {
    /// A list method but `append`, on `list`. Inference checked the
    /// arguments; only `sort` has a keyword, `reverse=`.
    pub(super) fn list_method(
        &mut self,
        list: Operand,
        attr: &str,
        args: &[Arg],
        span: Span,
    ) -> Result<Operand> {
        let mut ops = Vec::new();
        for a in args {
            ops.push(self.expr(&a.value)?);
        }
        let elem = self.slot_ty(self.ty(list), 0, span)?;
        match (attr, ops.as_slice()) {
            ("pop", [i]) => self.pop(list, Some(*i), elem, span),
            ("pop", []) => self.pop(list, None, elem, span),
            ("extend", [other]) => self.extend(list, *other, elem, span),
            ("insert", [i, x]) => {
                let index = self.as_int(*i, span);
                let value = self.expect(*x, elem, span)?;
                self.emit(Inst::Insert {
                    list,
                    index,
                    value,
                    span,
                });
                Ok(Operand::None)
            }
            ("remove", [x]) => {
                let x = self.needle(*x, elem, span)?;
                let at = self.temp(Ty::Int, Rvalue::Std(StdFn::ListFind, vec![list, x]), span);
                let missing = Rvalue::Cmp(CmpOp::Lt, at, Operand::Int(0));
                let message = "list.remove(x): x not in list";
                self.raise_if(missing, ExcKind::Value, message, span)?;
                self.emit(Inst::Remove {
                    list,
                    index: at,
                    span,
                });
                Ok(Operand::None)
            }
            ("index", [x]) => {
                // The error shows `x` as written, so it is not converted.
                if self.ty(*x) != elem && matches!(self.ty(*x), Ty::Int | Ty::Bool) {
                    let what = "`list.index` of a number of another type than the list's items";
                    return Err(unsupported(span, what));
                }
                let x = self.needle(*x, elem, span)?;
                let at = self.temp(Ty::Int, Rvalue::Std(StdFn::ListIndex, vec![list, x]), span);
                self.check_raised(span)?;
                Ok(at)
            }
            ("count", [x]) => {
                let x = self.needle(*x, elem, span)?;
                Ok(self.temp(Ty::Int, Rvalue::Std(StdFn::ListCount, vec![list, x]), span))
            }
            ("sort", _) => {
                let (_, reverse) = self.sort_args(args, &ops, span)?;
                self.emit(Inst::Sort {
                    list,
                    reverse,
                    span,
                });
                Ok(Operand::None)
            }
            _ => Err(unsupported(span, &format!("the list method `{attr}` here"))),
        }
    }

    /// `sorted`'s or `sort`'s positional operands, and its `reverse=` as
    /// a `Bool` (false without one).
    pub(super) fn sort_args(
        &mut self,
        args: &[Arg],
        ops: &[Operand],
        span: Span,
    ) -> Result<(Vec<Operand>, Operand)> {
        let (mut positional, mut reverse) = (Vec::new(), Operand::Bool(false));
        for (a, op) in args.iter().zip(ops) {
            match &a.name {
                None => positional.push(*op),
                Some(kw) if kw.name == "reverse" => reverse = self.truthy(*op, span),
                Some(kw) => {
                    let what = format!("the keyword `{}=` of a sort", kw.name);
                    return Err(unsupported(span, &what));
                }
            }
        }
        Ok((positional, reverse))
    }

    /// `x` as an item of a list of `elem`s, to compare: a bool meets a
    /// number as the int it is and an int a float as a float, as `==`.
    fn needle(&mut self, x: Operand, elem: Ty, span: Span) -> Result<Operand> {
        let x = match (self.ty(x), elem) {
            (Ty::Bool, Ty::Int | Ty::Float) => self.as_int(x, span),
            _ => x,
        };
        let x = match (self.ty(x), elem) {
            (Ty::Int, Ty::Float) => self.as_float(x, span),
            _ => x,
        };
        let x = self.fit(x, elem, span);
        if self.ty(x) != elem {
            let what = "comparing a list's items with a value of another type";
            return Err(unsupported(span, what));
        }
        Ok(x)
    }

    /// `list.pop()` (the last item) or `list.pop(i)`: removes and gives
    /// the item, raising CPython's `IndexError` (D71).
    fn pop(
        &mut self,
        list: Operand,
        index: Option<Operand>,
        elem: Ty,
        span: Span,
    ) -> Result<Operand> {
        let n = self.temp(Ty::Int, Rvalue::Len(list), span);
        let empty = Rvalue::Cmp(CmpOp::Eq, n, Operand::Int(0));
        self.raise_if(empty, ExcKind::Index, "pop from empty list", span)?;
        let at = match index {
            Some(i) => self.position(i, n, "pop index out of range", span)?,
            None => self.temp(Ty::Int, Rvalue::Int(IntOp::Sub, n, Operand::Int(1)), span),
        };
        let v = self.temp(elem, Rvalue::Index(list, at), span);
        self.emit(Inst::Remove {
            list,
            index: at,
            span,
        });
        Ok(v)
    }

    /// `i` as a position in a sequence of length `n`, counting a
    /// negative one from the end; out of range raises `IndexError`
    /// holding `message`.
    fn position(&mut self, i: Operand, n: Operand, message: &str, span: Span) -> Result<Operand> {
        let i = self.as_int(i, span);
        let at = self.new_local(Ty::Int);
        let negative = self.temp(Ty::Bool, Rvalue::Cmp(CmpOp::Lt, i, Operand::Int(0)), span);
        self.either(
            negative,
            |b| b.assign(at, Rvalue::Int(IntOp::Add, i, n), span),
            |b| b.assign(at, Rvalue::Use(i), span),
        );
        let at = Operand::Local(at);
        let below = Rvalue::Cmp(CmpOp::Lt, at, Operand::Int(0));
        self.raise_if(below, ExcKind::Index, message, span)?;
        let above = Rvalue::Cmp(CmpOp::Ge, at, n);
        self.raise_if(above, ExcKind::Index, message, span)?;
        Ok(at)
    }

    /// `list.extend(other)`: a list's items read in full first, so
    /// `xs.extend(xs)` doubles `xs`; any other iterable's in turn.
    fn extend(&mut self, list: Operand, other: Operand, elem: Ty, span: Span) -> Result<Operand> {
        if let Ty::List(_) = self.ty(other) {
            self.emit(Inst::Extend { list, other, span });
            return Ok(Operand::None);
        }
        self.each(other, span, |b, x| {
            let value = b.expect(x, elem, span)?;
            b.emit(Inst::Append { list, value, span });
            Ok(())
        })?;
        Ok(Operand::None)
    }

    /// `del value[index]`: removes a list's item, raising CPython's
    /// `IndexError` out of range, or a dict's key, raising `KeyError`
    /// when missing (D71).
    pub(super) fn del_stmt(&mut self, value: &Expr, index: &Expr, span: Span) -> Result<()> {
        let c = self.expr(value)?;
        let i = self.expr(index)?;
        match self.ty(c) {
            Ty::Dict(_) => {
                let (key, _) = self.key_value(self.ty(c), span)?;
                let probe = self.probe(i, key, span)?;
                self.require_key(c, i, probe, span)?;
                self.emit(Inst::Remove {
                    list: c,
                    index: probe,
                    span,
                });
                Ok(())
            }
            Ty::List(_) => {
                let n = self.temp(Ty::Int, Rvalue::Len(c), span);
                let message = "list assignment index out of range";
                let at = self.position(i, n, message, span)?;
                self.emit(Inst::Remove {
                    list: c,
                    index: at,
                    span,
                });
                Ok(())
            }
            _ => Err(unsupported(span, "`del` of an item of this type")),
        }
    }

    /// `d.pop(k)`, raising `KeyError(k)` when missing, or
    /// `d.pop(k, default)`.
    pub(super) fn dict_pop(
        &mut self,
        d: Operand,
        k: &Arg,
        default: Option<&Arg>,
        span: Span,
    ) -> Result<Operand> {
        let (key, value) = self.key_value(self.ty(d), span)?;
        let k = self.expr(&k.value)?;
        let probe = self.probe(k, key, span)?;
        let Some(default) = default else {
            self.require_key(d, k, probe, span)?;
            return Ok(self.take_entry(d, probe, value, span));
        };
        let default = self.expr(&default.value)?;
        let dest = self.pooled(value);
        let (hit, miss, end) = self.found(d, probe, span);
        self.switch_to(hit);
        let v = self.take_entry(d, probe, value, span);
        self.store_dest(dest, value, v, span)?;
        self.terminate(Term::Goto(end));
        self.switch_to(miss);
        self.store_dest(dest, value, default, span)?;
        self.terminate(Term::Goto(end));
        self.switch_to(end);
        Ok(Operand::Local(dest))
    }

    /// `d.setdefault(k)` or `d.setdefault(k, default)`: `d[k]`, first
    /// storing the default (`None` without one) when `k` is missing.
    pub(super) fn setdefault(
        &mut self,
        d: Operand,
        k: &Arg,
        default: Option<&Arg>,
        span: Span,
    ) -> Result<Operand> {
        let (key, value) = self.key_value(self.ty(d), span)?;
        let k = self.expr(&k.value)?;
        let probe = self.probe(k, key, span)?;
        let default = match default {
            Some(a) => self.expr(&a.value)?,
            None => Operand::None,
        };
        let default = self.expect(default, value, span)?;
        let dest = self.pooled(value);
        let (hit, miss, end) = self.found(d, probe, span);
        self.switch_to(hit);
        let v = self.temp(value, Rvalue::Index(d, probe), span);
        self.store_dest(dest, value, v, span)?;
        self.terminate(Term::Goto(end));
        self.switch_to(miss);
        let index = self.stored(k, key, span)?;
        self.emit(Inst::SetItem {
            list: d,
            index,
            value: default,
            span,
        });
        self.store_dest(dest, value, default, span)?;
        self.terminate(Term::Goto(end));
        self.switch_to(end);
        Ok(Operand::Local(dest))
    }

    /// `d.update(other)` of a dict of `d`'s type: each of `other`'s
    /// pairs stored in `d` in order.
    pub(super) fn update(&mut self, d: Operand, other: &Arg, span: Span) -> Result<Operand> {
        let o = self.expr(&other.value)?;
        if self.ty(o) != self.ty(d) {
            return Err(internal(span, "`update` of a dict of another type"));
        }
        let (_, value) = self.key_value(self.ty(d), span)?;
        self.each(o, span, |b, k| {
            let v = b.temp(value, Rvalue::Index(o, k), span);
            b.emit(Inst::SetItem {
                list: d,
                index: k,
                value: v,
                span,
            });
            Ok(())
        })?;
        Ok(Operand::None)
    }

    /// Branches on whether dict `d` holds key `probe`: the blocks where
    /// it does and does not, and the one after both.
    fn found(
        &mut self,
        d: Operand,
        probe: Operand,
        span: Span,
    ) -> (crate::ir::BlockId, crate::ir::BlockId, crate::ir::BlockId) {
        let found = Rvalue::Contains {
            hay: d,
            needle: probe,
        };
        let cond = self.temp(Ty::Bool, found, span);
        let (hit, miss, end) = (self.new_block(), self.new_block(), self.new_block());
        self.terminate(Term::Branch {
            cond,
            then: hit,
            orelse: miss,
        });
        (hit, miss, end)
    }

    /// Removes key `probe`, which `d` holds, giving its value.
    fn take_entry(&mut self, d: Operand, probe: Operand, value: Ty, span: Span) -> Operand {
        let v = self.temp(value, Rvalue::Index(d, probe), span);
        self.emit(Inst::Remove {
            list: d,
            index: probe,
            span,
        });
        v
    }
}
