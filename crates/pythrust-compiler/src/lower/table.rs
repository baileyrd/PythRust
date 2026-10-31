//! Dicts, sets and dict views: displays, lookups and stores, methods,
//! membership, iteration, and the builtins that build lists and sets
//! from any iterable (`set`, `sorted`).

use crate::ast::{Arg, Expr, ExprKind};
use crate::diag::Result;
use crate::exc::ExcKind;
use crate::infer::Type;
use crate::ir::{CmpOp, Inst, IntOp, LocalId, Operand, Rvalue, Term, Ty, View};
use crate::span::Span;

use super::{internal, unsupported, FnBuilder};

/// What a `for` over a dict or set binds each time: a key (a set's
/// element), a value, or a `(key, value)` tuple of this type.
#[derive(Clone, Copy)]
pub(super) enum Part {
    Key,
    Value,
    Item(Ty),
}

/// The `RuntimeError` a `for` over a table whose size changed ends with.
fn changed_size(ty: Ty) -> &'static str {
    match ty {
        Ty::Set(_) => "RuntimeError: Set changed size during iteration",
        _ => "RuntimeError: dictionary changed size during iteration",
    }
}

impl FnBuilder<'_, '_> {
    /// `{k: v, ...}`: a new dict holding the pairs, a repeated key
    /// keeping its first place and its last value.
    pub(super) fn dict_display(&mut self, items: &[(Expr, Expr)], span: Span) -> Result<Operand> {
        let mut ops = Vec::new();
        for (k, v) in items {
            ops.push(self.expr(k)?);
            ops.push(self.expr(v)?);
        }
        self.bind_display(&ops, span)?;
        let ty = self.ty_at(span)?;
        let (key, value) = self.key_value(ty, span)?;
        let mut slots = Vec::new();
        for (pair, (k, v)) in ops.chunks(2).zip(items) {
            let (Some(kop), Some(vop)) = (pair.first(), pair.get(1)) else {
                return Err(internal(span, "a dict display of odd length"));
            };
            slots.push(self.stored_key(*kop, key, k.span)?);
            slots.push(self.expect(*vop, value, v.span)?);
        }
        Ok(self.temp(ty, Rvalue::NewSeq(slots), span))
    }

    /// `{a, b}`: a new set holding the items.
    pub(super) fn set_display(&mut self, items: &[Expr], span: Span) -> Result<Operand> {
        let mut ops = Vec::new();
        for item in items {
            ops.push(self.expr(item)?);
        }
        self.bind_display(&ops, span)?;
        let ty = self.ty_at(span)?;
        let elem = self.elem_of(ty, span)?;
        let mut slots = Vec::new();
        for (op, item) in ops.into_iter().zip(items) {
            slots.push(self.expect(op, elem, item.span)?);
        }
        Ok(self.temp(ty, Rvalue::NewSeq(slots), span))
    }

    /// `d` and the view if `e` is `d.keys()`, `d.values()` or
    /// `d.items()` on a dict `d`: what a `for` or `in` can read from `d`
    /// itself.
    pub(super) fn dict_view<'e>(&self, e: &'e Expr) -> Option<(&'e Expr, View)> {
        let ExprKind::Call { func, args } = &e.kind else {
            return None;
        };
        let ExprKind::Attr { value, attr } = &func.kind else {
            return None;
        };
        let view = match attr.name.as_str() {
            "keys" => View::Keys,
            "values" => View::Values,
            "items" => View::Items,
            _ => return None,
        };
        let is_dict = matches!(self.node_type(value.span), Ok(Type::Dict(..)));
        (is_dict && args.is_empty()).then_some((value.as_ref(), view))
    }

    pub(super) fn key_value(&self, ty: Ty, span: Span) -> Result<(Ty, Ty)> {
        self.cx
            .seqs
            .key_value(ty)
            .ok_or_else(|| internal(span, "a dict of no key or value type"))
    }

    /// A set's element type, or a view's item type.
    fn elem_of(&self, ty: Ty, span: Span) -> Result<Ty> {
        self.cx
            .seqs
            .elem(ty)
            .ok_or_else(|| internal(span, "a set or view of no item type"))
    }

    /// `k` as a key of type `key` to look up: a `bool` meets an `int`
    /// key as the int it is, as in Python.
    fn probe(&mut self, k: Operand, key: Ty, span: Span) -> Result<Operand> {
        let k = match (self.ty(k), key) {
            (Ty::Bool, Ty::Int) => self.as_int(k, span),
            _ => k,
        };
        self.expect(k, key, span)
    }

    /// `k` as a key of type `key` to store. A native int key cannot
    /// remember that it was stored as `True`, which a dict's `repr` and
    /// its keys show in Python, so storing a `bool` there is refused.
    fn stored_key(&mut self, k: Operand, key: Ty, span: Span) -> Result<Operand> {
        if (self.ty(k), key) == (Ty::Bool, Ty::Int) {
            return Err(unsupported(span, "a bool key stored in an int-keyed dict"));
        }
        self.expect(k, key, span)
    }

    /// `d[k]`: raises `KeyError(k)` when `k` is missing.
    pub(super) fn dict_read(&mut self, d: Operand, k: Operand, span: Span) -> Result<Operand> {
        let (key, value) = self.key_value(self.ty(d), span)?;
        let k = self.probe(k, key, span)?;
        let found = self.temp(Ty::Bool, Rvalue::Contains { hay: d, needle: k }, span);
        let (missing, ok) = (self.new_block(), self.new_block());
        self.terminate(Term::Branch {
            cond: found,
            then: ok,
            orelse: missing,
        });
        self.switch_to(missing);
        let exc = self.new_exc(ExcKind::Key, &[k], span)?;
        self.emit(Inst::Raise { exc, span });
        self.raise_path()?;
        self.switch_to(ok);
        Ok(self.temp(value, Rvalue::Index(d, k), span))
    }

    /// `d[k] = v`, which adds or replaces and never raises.
    pub(super) fn dict_store(
        &mut self,
        d: Operand,
        k: Operand,
        v: Operand,
        span: Span,
    ) -> Result<()> {
        let (key, value) = self.key_value(self.ty(d), span)?;
        let index = self.stored_key(k, key, span)?;
        let value = self.expect(v, value, span)?;
        self.emit(Inst::SetItem {
            list: d,
            index,
            value,
            span,
        });
        Ok(())
    }

    /// `d.get`, `d.keys`, `d.values`, `d.items` and `s.add`.
    pub(super) fn table_method(
        &mut self,
        t: Operand,
        attr: &str,
        args: &[Arg],
        span: Span,
    ) -> Result<Operand> {
        if args.iter().any(|a| a.name.is_some()) {
            return Err(unsupported(span, "keyword arguments"));
        }
        let ty = self.ty(t);
        let view = match (ty, attr, args) {
            (Ty::Set(_), "add", [arg]) => {
                let v = self.expr(&arg.value)?;
                let elem = self.elem_of(ty, span)?;
                let value = self.expect(v, elem, arg.span)?;
                self.emit(Inst::Append {
                    list: t,
                    value,
                    span,
                });
                return Ok(Operand::None);
            }
            (Ty::Dict(_), "get", [k, rest @ ..]) if rest.len() < 2 => {
                return self.get(t, k, rest.first(), span)
            }
            (Ty::Dict(_), "keys", []) => View::Keys,
            (Ty::Dict(_), "values", []) => View::Values,
            (Ty::Dict(_), "items", []) => View::Items,
            _ => return Err(unsupported(span, &format!("the method `{attr}` here"))),
        };
        let vty = self.ty_at(span)?;
        if self.cx.seqs.view_kind(vty) != Some(view) {
            return Err(internal(span, "a view of another kind"));
        }
        Ok(self.temp(vty, Rvalue::View(t), span))
    }

    /// `d.get(k)`, `None` when missing, or `d.get(k, default)`.
    fn get(&mut self, d: Operand, k: &Arg, default: Option<&Arg>, span: Span) -> Result<Operand> {
        let (key, value) = self.key_value(self.ty(d), span)?;
        let k = self.expr(&k.value)?;
        let k = self.probe(k, key, span)?;
        let ty = self.ty_at(span)?;
        let dest = self.pooled(ty);
        let found = self.temp(Ty::Bool, Rvalue::Contains { hay: d, needle: k }, span);
        let (hit, miss, end) = (self.new_block(), self.new_block(), self.new_block());
        self.terminate(Term::Branch {
            cond: found,
            then: hit,
            orelse: miss,
        });
        self.switch_to(hit);
        let v = self.temp(value, Rvalue::Index(d, k), span);
        self.into_dest(dest, ty, v, span)?;
        self.terminate(Term::Goto(end));
        self.switch_to(miss);
        let v = match default {
            Some(a) => self.expr(&a.value)?,
            None => Operand::None,
        };
        self.into_dest(dest, ty, v, span)?;
        self.terminate(Term::Goto(end));
        self.switch_to(end);
        Ok(Operand::Local(dest))
    }

    /// Moves `v`, fitted to `ty`, into `dest`.
    fn into_dest(&mut self, dest: LocalId, ty: Ty, v: Operand, span: Span) -> Result<()> {
        let v = self.expect(v, ty, span)?;
        self.move_into(dest, v, span);
        Ok(())
    }

    /// `x in t` of a dict or set (a key lookup), or of a view (a
    /// search; but `x in d.keys()` looks `x` up in `d`).
    pub(super) fn table_contains(&mut self, x: Operand, t: Operand, span: Span) -> Result<Operand> {
        let ty = self.ty(t);
        let needle = match ty {
            Ty::Dict(_) => {
                let (key, _) = self.key_value(ty, span)?;
                self.probe(x, key, span)?
            }
            _ => {
                let elem = self.elem_of(ty, span)?;
                self.probe(x, elem, span)?
            }
        };
        Ok(self.temp(Ty::Bool, Rvalue::Contains { hay: t, needle }, span))
    }

    /// `for` over dict or set `t`, binding `part` of each entry: by
    /// position, the size fixed when the loop starts. Each step first
    /// checks the size, so a change that adds a key stops the program
    /// with Python's `RuntimeError`, at `span`, as `run` does.
    pub(super) fn for_table(
        &mut self,
        l: &super::stmt::Looped,
        t: Operand,
        part: Part,
        span: Span,
    ) -> Result<()> {
        let t = self.own(t, span);
        let size = self.new_local(Ty::Int);
        self.assign(size, Rvalue::Len(t), span);
        let counter = self.new_local(Ty::Int);
        self.assign(counter, Rvalue::Use(Operand::Int(0)), span);
        let (n0, cur) = (Operand::Local(size), Operand::Local(counter));
        let (head, enter, latch) = (self.new_block(), self.new_block(), self.new_block());
        let (other, end) = (self.new_block(), self.new_block());
        self.terminate(Term::Goto(head));
        self.switch_to(head);
        let n = self.temp(Ty::Int, Rvalue::Len(t), span);
        self.fail_if(
            Rvalue::Cmp(CmpOp::Ne, n, n0),
            span,
            changed_size(self.ty(t)),
        );
        let going = self.temp(Ty::Bool, Rvalue::Cmp(CmpOp::Lt, cur, n0), span);
        self.terminate(Term::Branch {
            cond: going,
            then: enter,
            orelse: other,
        });
        self.switch_to(enter);
        self.bind_iteration(l.target, |b| b.entry(t, cur, part, span))?;
        self.loop_body(l.body, latch, end)?;
        self.switch_to(latch);
        self.assign(counter, Rvalue::Int(IntOp::Add, cur, Operand::Int(1)), span);
        self.terminate(Term::Goto(head));
        self.finish_loop(l.orelse, other, end)
    }

    /// The key of entry `i` of dict or set `t`, a new temporary.
    pub(super) fn entry_key(&mut self, t: Operand, i: Operand, span: Span) -> Result<Operand> {
        self.entry(t, i, Part::Key, span)
    }

    /// `part` of entry `i` of dict or set `t`, a new temporary.
    fn entry(&mut self, t: Operand, i: Operand, part: Part, span: Span) -> Result<Operand> {
        let ty = self.ty(t);
        let (key, value) = match ty {
            Ty::Dict(_) => self.key_value(ty, span)?,
            _ => (self.elem_of(ty, span)?, Ty::None),
        };
        Ok(match part {
            Part::Key => self.temp(key, Rvalue::Entry(t, i, 0), span),
            Part::Value => self.temp(value, Rvalue::Entry(t, i, 1), span),
            Part::Item(item) => {
                let k = self.temp(key, Rvalue::Entry(t, i, 0), span);
                let v = self.temp(value, Rvalue::Entry(t, i, 1), span);
                self.temp(item, Rvalue::NewSeq(vec![k, v]), span)
            }
        })
    }

    /// Calls `f` on each item `s` iterates, in order: a string's
    /// characters, a list's, tuple's or view's items, a range's ints, a
    /// dict's keys or a set's elements. Nothing `f` does changes `s`.
    pub(super) fn each(
        &mut self,
        s: Operand,
        span: Span,
        mut f: impl FnMut(&mut Self, Operand) -> Result<()>,
    ) -> Result<()> {
        let ty = self.ty(s);
        let item = match ty {
            Ty::Str => Ty::Str,
            Ty::Range => Ty::Int,
            Ty::List(_) | Ty::Set(_) | Ty::View(_) => self.elem_of(ty, span)?,
            Ty::Dict(_) => self.key_value(ty, span)?.0,
            Ty::Tuple(_) => self.tuple_item(ty, span)?,
            _ => return Err(unsupported(span, "iterating this type")),
        };
        let read: fn(Operand, Operand) -> Rvalue = match ty {
            Ty::Dict(_) | Ty::Set(_) => |t, i| Rvalue::Entry(t, i, 0),
            _ => Rvalue::Index,
        };
        let n = match ty {
            Ty::Range => self.range_len(s, span),
            _ => self.temp(Ty::Int, Rvalue::Len(s), span),
        };
        let counter = self.new_local(Ty::Int);
        self.assign(counter, Rvalue::Use(Operand::Int(0)), span);
        let cur = Operand::Local(counter);
        let (head, body, end) = (self.new_block(), self.new_block(), self.new_block());
        self.terminate(Term::Goto(head));
        self.switch_to(head);
        let going = self.temp(Ty::Bool, Rvalue::Cmp(CmpOp::Lt, cur, n), span);
        self.terminate(Term::Branch {
            cond: going,
            then: body,
            orelse: end,
        });
        self.switch_to(body);
        self.pooled_stmt(|b| {
            let x = b.temp(item, read(s, cur), span);
            f(b, x)
        })?;
        self.assign(counter, Rvalue::Int(IntOp::Add, cur, Operand::Int(1)), span);
        self.terminate(Term::Goto(head));
        self.switch_to(end);
        Ok(())
    }

    /// `set()` or `set(xs)`: a new set, of the call's type, of what `xs`
    /// iterates over.
    pub(super) fn new_set(&mut self, args: &[Operand], span: Span) -> Result<Operand> {
        let ty = self.ty_at(span)?;
        let set = self.temp(ty, Rvalue::NewSeq(Vec::new()), span);
        let Some(xs) = args.first() else {
            return Ok(set);
        };
        let elem = self.elem_of(ty, span)?;
        self.each(*xs, span, |b, x| {
            let value = b.expect(x, elem, span)?;
            b.emit(Inst::Append {
                list: set,
                value,
                span,
            });
            Ok(())
        })?;
        Ok(set)
    }

    /// `sorted(xs)`: a new list, of the call's type, of what `xs`
    /// iterates over, sorted by `<`.
    pub(super) fn sorted(&mut self, args: &[Operand], span: Span) -> Result<Operand> {
        let [xs] = args else {
            return Err(internal(span, "`sorted` of other than one argument"));
        };
        let ty = self.ty_at(span)?;
        let list = match self.ty(*xs) {
            Ty::List(_) | Ty::Dict(_) | Ty::Set(_) | Ty::View(_) => {
                self.temp(ty, Rvalue::CopyList(*xs), span)
            }
            _ => {
                let list = self.temp(ty, Rvalue::NewSeq(Vec::new()), span);
                self.each(*xs, span, |b, value| {
                    b.emit(Inst::Append { list, value, span });
                    Ok(())
                })?;
                list
            }
        };
        self.emit(Inst::Sort(list));
        Ok(list)
    }
}
