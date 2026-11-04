//! `enumerate(...)` and `zip(...)` (D51, D58), which are never values:
//! a `for` steps one in place, and a builtin that reads every item
//! collects them first. Each step pulls the sources left to right, as
//! CPython's iterators do, so `zip` stops at the first one exhausted and
//! a list that grows in the loop body is read to its new end.

use crate::ast::{Arg, Expr, ExprKind};
use crate::diag::Result;
use crate::ir::{BlockId, CmpOp, Inst, IntOp, LocalId, Operand, Rvalue, Term, Ty};
use crate::span::Span;

use super::stmt::Looped;
use super::table::{changed_size, ItemRead};
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
    /// A dict's keys or a set's elements; a size change stops the
    /// program as `for` over it does.
    Table {
        t: Operand,
        size: Operand,
        item: Ty,
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
        let mut sources = Vec::new();
        let mut start = None;
        for a in args {
            if name == "enumerate" && (a.name.is_some() || !sources.is_empty()) {
                let op = self.expr(&a.value)?;
                let n = self.as_int(op, a.value.span);
                start = Some(self.temp(Ty::Int, Rvalue::Use(n), a.value.span));
                continue;
            }
            sources.push(self.source(&a.value)?);
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

    /// An argument of `enumerate` or `zip`, evaluated, with the loop
    /// holding its own reference.
    fn source(&mut self, e: &Expr) -> Result<Source> {
        if let Some(z) = self.zip_of(e)? {
            return Ok(Source::Zip(Box::new(z)));
        }
        let v = self.expr(e)?;
        self.reads_results(e)?;
        let span = e.span;
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
                let (item, _) = self.items_of(s, span)?;
                Source::Table {
                    t: s,
                    size: Operand::Local(size),
                    item,
                    span,
                }
            }
            ty => {
                let (item, read) = self.items_of(s, span)?;
                let fixed = (ty == Ty::Range).then(|| self.count(s, span));
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
        if name != "any" || arg.name.is_some() {
            return Ok(None);
        }
        let Some(z) = self.zip_of(&arg.value)? else {
            return Ok(None);
        };
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
                item,
                span,
            } => {
                let span = *span;
                let n = self.temp(Ty::Int, Rvalue::Len(*t), span);
                let changed = changed_size(self.ty(*t));
                self.fail_if(Rvalue::Cmp(CmpOp::Ne, n, *size), span, changed);
                self.step_if(Rvalue::Cmp(CmpOp::Lt, cur, *size), other, span);
                Ok(self.temp(*item, Rvalue::Entry(*t, cur, 0), span))
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
}
