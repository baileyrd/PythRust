//! `enumerate(...)` and `zip(...)` (D51, D58), which are never values:
//! a `for` steps one in place, and a builtin that reads every item
//! collects them first. Each step pulls the sources left to right, as
//! CPython's iterators do, so `zip` stops at the first one exhausted and
//! a list that grows in the loop body is read to its new end.

use crate::ast::{Arg, Expr, ExprKind};
use crate::diag::Result;
use crate::exc::ExcKind;
use crate::ir::{BlockId, CmpOp, Inst, IntOp, LocalId, Operand, Rvalue, Term, Ty, View};
use crate::span::Span;

use super::stmt::Looped;
use super::table::{changed_size, ItemRead, Part};
use super::FnBuilder;

/// An `enumerate(...)` or `zip(...)` whose arguments are evaluated.
pub(super) struct Zip {
    sources: Vec<Source>,
    /// `enumerate`'s start; `None` for `zip`.
    start: Option<Operand>,
    /// The tuple type of each item.
    item: Ty,
    span: Span,
}

/// One argument of a [`Zip`], read at the loop's step count: a nested
/// one steps exactly as often as the loop does.
enum Source {
    /// A string, list, tuple, view or range, read by index. The count is
    /// re-read each step but for a range's, fixed when the loop starts.
    Seq {
        s: Operand,
        item: Ty,
        read: ItemRead,
        fixed: Option<Operand>,
        span: Span,
    },
    /// A dict's keys, values or items, or a set's elements; a size
    /// change stops the program as `for` over it does.
    Table {
        t: Operand,
        size: Operand,
        part: Part,
        span: Span,
    },
    /// `map`'s results in `build`: each step takes one.
    Results {
        rs: Operand,
        item: Ty,
        span: Span,
    },
    Zip(Box<Zip>),
}

/// An evaluated argument of a [`Zip`], not yet a [`Source`]: a nested
/// one, or a value with the part of it a dict view passed directly reads.
enum Pending {
    Zip(Zip),
    Value(Operand, Option<Part>, Span),
}

impl FnBuilder<'_, '_> {
    /// `e` as an `enumerate(...)` or `zip(...)` of the builtin, its
    /// arguments evaluated; `None` for any other expression.
    pub(super) fn zip_of(&mut self, e: &Expr) -> Result<Option<Zip>> {
        let ExprKind::Call { func, args } = &e.kind else {
            return Ok(None);
        };
        match &func.kind {
            ExprKind::Name(n) if self.is_zip(n) => self.zip(n, args, e.span).map(Some),
            _ => Ok(None),
        }
    }

    /// Whether a call of `name` is the builtin `enumerate` or `zip`.
    pub(super) fn is_zip(&self, name: &str) -> bool {
        matches!(name, "enumerate" | "zip") && !self.cx.defs.contains_key(name)
    }

    /// `name(args)`, the builtin `enumerate` or `zip`: its arguments
    /// evaluated in order, `enumerate`'s start (by position or keyword)
    /// as an int.
    pub(super) fn zip(&mut self, name: &str, args: &[Arg], span: Span) -> Result<Zip> {
        let list = self.ty_at(span)?;
        let item = self.cx.seqs.elem(list).unwrap_or(Ty::None);
        let mut pending = Vec::new();
        let mut start = None;
        for a in args {
            if name == "enumerate" && (a.name.is_some() || !pending.is_empty()) {
                let op = self.expr(&a.value)?;
                let n = self.as_int(op, a.value.span);
                start = Some(self.temp(Ty::Int, Rvalue::Use(n), a.value.span));
                continue;
            }
            pending.push(self.pending(&a.value)?);
        }
        // As CPython's, every argument is evaluated before any is iterated.
        let fields: Vec<Ty> = self
            .cx
            .seqs
            .fields(item)
            .map(<[Ty]>::to_vec)
            .unwrap_or_default();
        let skip = usize::from(name == "enumerate");
        let mut sources = Vec::new();
        for (k, p) in pending.into_iter().enumerate() {
            let want = fields.get(k + skip).copied().unwrap_or(Ty::None);
            sources.push(self.source(p, want)?);
        }
        if name == "enumerate" {
            start = start.or(Some(Operand::Int(0)));
        }
        Ok(Zip {
            sources,
            start,
            item,
            span,
        })
    }

    /// An argument of `enumerate` or `zip`, evaluated: a nested one, or
    /// the value, or for a dict view passed directly, its dict.
    fn pending(&mut self, e: &Expr) -> Result<Pending> {
        if let Some(z) = self.zip_of(e)? {
            return Ok(Pending::Zip(z));
        }
        if let Some((d, view)) = self.dict_view(e) {
            let d = self.expr(d)?;
            let part = match view {
                View::Keys => Part::Key,
                View::Values => Part::Value,
                View::Items => {
                    let (k, v) = self.key_value(self.ty(d), e.span)?;
                    Part::Item(self.cx.seqs.tuple(vec![k, v]))
                }
            };
            return Ok(Pending::Value(d, Some(part), e.span));
        }
        let v = self.expr(e)?;
        self.reads_results(e)?;
        Ok(Pending::Value(v, None, e.span))
    }

    /// The argument `p` as it is stepped, with the loop holding its own
    /// reference: a table's size is taken now, once all are evaluated.
    /// `want` is the type of its items, which an empty tuple has none of.
    fn source(&mut self, p: Pending, want: Ty) -> Result<Source> {
        let (v, part, span) = match p {
            Pending::Zip(z) => return Ok(Source::Zip(Box::new(z))),
            Pending::Value(v, part, span) => (v, part, span),
        };
        let s = self.own(v, span);
        Ok(match self.ty(s) {
            Ty::Results => Source::Results {
                rs: s,
                item: self.result_ty(span)?,
                span,
            },
            Ty::Dict(_) | Ty::Set(_) => {
                let size = self.new_local(Ty::Int);
                self.assign(size, Rvalue::Len(s), span);
                Source::Table {
                    t: s,
                    size: Operand::Local(size),
                    part: part.unwrap_or(Part::Key),
                    span,
                }
            }
            ty if self.cx.seqs.fields(ty).is_some_and(<[Ty]>::is_empty) => Source::Seq {
                s,
                item: want,
                read: Rvalue::Index,
                fixed: Some(Operand::Int(0)),
                span,
            },
            ty => {
                let (item, read) = self.items_of(s, span)?;
                let fixed = (ty == Ty::Range).then(|| self.steps(s, span));
                Source::Seq {
                    s,
                    item,
                    read,
                    fixed,
                    span,
                }
            }
        })
    }

    /// A builtin call that steps an `enumerate(...)` or `zip(...)`: the
    /// call itself, whose items a builtin reading them all collects
    /// first, or `any` of one, which only its first decides, a tuple
    /// being true; `None` for any other call.
    pub(super) fn zip_builtin(
        &mut self,
        name: &str,
        args: &[Arg],
        span: Span,
    ) -> Result<Option<Operand>> {
        if self.is_zip(name) {
            let z = self.zip(name, args, span)?;
            return self.collect_zip(&z, false).map(Some);
        }
        let [arg] = args else {
            return Ok(None);
        };
        let reads = matches!(name, "any" | "all" | "min" | "max" | "set");
        if !reads || arg.name.is_some() {
            return Ok(None);
        }
        let Some(z) = self.zip_of(&arg.value)? else {
            return Ok(None);
        };
        if name == "set" {
            // Each item goes straight into the set, which keeps only
            // distinct ones.
            let ty = self.ty_at(span)?;
            return self.collect_zip_into(&z, ty, false).map(Some);
        }
        if name != "any" {
            return self.fold_zip(name, &z, span).map(Some);
        }
        let first = self.collect_zip(&z, true)?;
        let n = self.temp(Ty::Int, Rvalue::Len(first), span);
        Ok(Some(self.temp(
            Ty::Bool,
            Rvalue::Cmp(CmpOp::Gt, n, Operand::Int(0)),
            span,
        )))
    }

    /// The loop `l` over `z`: each step reads one item of every source,
    /// or leaves for `else` at the first that has none.
    pub(super) fn for_zip(&mut self, l: &Looped, z: &Zip) -> Result<()> {
        let counter = self.new_local(Ty::Int);
        self.assign(counter, Rvalue::Use(Operand::Int(0)), z.span);
        let cur = Operand::Local(counter);
        let (enter, latch) = (self.new_block(), self.new_block());
        let (other, end) = (self.new_block(), self.new_block());
        self.terminate(Term::Goto(enter));
        self.switch_to(enter);
        self.iteration(l, |b| b.zip_step(z, cur, other), latch, end)?;
        self.switch_to(latch);
        let next = Rvalue::Int(IntOp::Add, cur, Operand::Int(1));
        self.assign(counter, next, z.span);
        self.terminate(Term::Goto(enter));
        self.finish_loop(l, other, end)
    }

    /// The items of `z` collected into a new list, for a builtin that
    /// reads them all; only the first where `first`, for `any`.
    pub(super) fn collect_zip(&mut self, z: &Zip, first: bool) -> Result<Operand> {
        let ty = self.cx.seqs.list(z.item);
        self.collect_zip_into(z, ty, first)
    }

    /// As [`FnBuilder::collect_zip`], into a new list or set of type `ty`.
    fn collect_zip_into(&mut self, z: &Zip, ty: Ty, first: bool) -> Result<Operand> {
        let out = self.pooled(ty);
        self.assign(out, Rvalue::NewSeq(Vec::new()), z.span);
        let collect = Looped::Collect {
            out,
            first,
            span: z.span,
        };
        self.for_zip(&collect, z)?;
        Ok(Operand::Local(out))
    }

    /// `all`, `min` or `max` (`name`) of `z`, stepped without collecting
    /// its items: they are tuples, all true, so `all` is true once every
    /// one is read, and an empty one raises CPython's `ValueError` for
    /// `min` and `max`.
    fn fold_zip(&mut self, name: &str, z: &Zip, span: Span) -> Result<Operand> {
        let op = match name {
            "min" => Some(CmpOp::Lt),
            "max" => Some(CmpOp::Gt),
            _ => None,
        };
        let best = self.pooled(z.item);
        let seen = self.new_local(Ty::Bool);
        self.assign(seen, Rvalue::Use(Operand::Bool(false)), span);
        let fold = Looped::Fold {
            op,
            best,
            seen,
            span,
        };
        self.for_zip(&fold, z)?;
        if op.is_none() {
            return Ok(Operand::Bool(true));
        }
        let message = format!("{name}() iterable argument is empty");
        let empty = Rvalue::Cmp(CmpOp::Eq, Operand::Local(seen), Operand::Bool(false));
        self.raise_if(empty, ExcKind::Value, &message, span)?;
        Ok(Operand::Local(best))
    }

    /// The next item of `z` at step `cur`, as a new tuple; at the first
    /// source with nothing left, the statement's temporaries are released
    /// and the loop leaves for `other`.
    fn zip_step(&mut self, z: &Zip, cur: Operand, other: BlockId) -> Result<Operand> {
        if z.sources.is_empty() {
            self.leave_step(other);
        }
        let mut items = Vec::new();
        for source in &z.sources {
            items.push(self.source_next(source, cur, other)?);
        }
        if let Some(start) = z.start {
            // As CPython's, the index comes after the item it numbers.
            let index = self.temp(Ty::Int, Rvalue::Int(IntOp::Add, start, cur), z.span);
            items.insert(0, index);
        }
        let fields: Vec<Ty> = self
            .cx
            .seqs
            .fields(z.item)
            .map(<[Ty]>::to_vec)
            .unwrap_or_default();
        let mut fitted = Vec::new();
        for (x, field) in items.into_iter().zip(fields) {
            fitted.push(self.expect(x, field, z.span)?);
        }
        Ok(self.temp(z.item, Rvalue::NewSeq(fitted), z.span))
    }

    /// The item of `source` at step `cur`, or the way out to `other`.
    fn source_next(&mut self, source: &Source, cur: Operand, other: BlockId) -> Result<Operand> {
        match source {
            Source::Seq {
                s,
                item,
                read,
                fixed,
                span,
            } => {
                let n = fixed.unwrap_or_else(|| self.temp(Ty::Int, Rvalue::Len(*s), *span));
                self.step_if(Rvalue::Cmp(CmpOp::Lt, cur, n), other, *span);
                Ok(self.temp(*item, read(*s, cur), *span))
            }
            Source::Table {
                t,
                size,
                part,
                span,
            } => {
                let span = *span;
                let n = self.temp(Ty::Int, Rvalue::Len(*t), span);
                let changed = changed_size(self.ty(*t));
                self.fail_if(Rvalue::Cmp(CmpOp::Ne, n, *size), span, changed);
                self.step_if(Rvalue::Cmp(CmpOp::Lt, cur, *size), other, span);
                self.entry(*t, cur, *part, span)
            }
            Source::Results { rs, item, span } => {
                let n = self.temp(Ty::Int, Rvalue::Len(*rs), *span);
                self.step_if(Rvalue::Cmp(CmpOp::Gt, n, Operand::Int(0)), other, *span);
                self.next_result(*rs, *item, *span)
            }
            Source::Zip(z) => self.zip_step(z, cur, other),
        }
    }

    /// Continues where `test` holds; else leaves the step for `other`.
    fn step_if(&mut self, test: Rvalue, other: BlockId, span: Span) {
        let going = self.temp(Ty::Bool, test, span);
        let (go, stop) = (self.new_block(), self.new_block());
        self.terminate(Term::Branch {
            cond: going,
            then: go,
            orelse: stop,
        });
        self.switch_to(stop);
        self.leave_step(other);
        self.switch_to(go);
    }

    /// Releases the step's temporaries so far and jumps to `other`.
    fn leave_step(&mut self, other: BlockId) {
        let pool: Vec<LocalId> = self.pools.last().cloned().unwrap_or_default();
        pool.into_iter().for_each(|t| self.emit(Inst::Release(t)));
        self.jump_away(Term::Goto(other));
    }

    /// How many items the range `r` gives, as a step count: one past
    /// `i64::MAX` cannot be reached, so a longer range counts as that.
    fn steps(&mut self, r: Operand, span: Span) -> Operand {
        let n = self.new_local(Ty::Int);
        self.assign(n, Rvalue::Len(r), span);
        let (wide, done) = (self.new_block(), self.new_block());
        // The runtime gives -1 for a length past `i64::MAX`.
        let too_long = Rvalue::Cmp(CmpOp::Lt, Operand::Local(n), Operand::Int(0));
        let cond = self.temp(Ty::Bool, too_long, span);
        self.terminate(Term::Branch {
            cond,
            then: wide,
            orelse: done,
        });
        self.switch_to(wide);
        self.assign(n, Rvalue::Use(Operand::Int(i64::MAX)), span);
        self.terminate(Term::Goto(done));
        self.switch_to(done);
        Operand::Local(n)
    }
}
