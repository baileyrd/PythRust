//! Statements and control flow.

use crate::ast::{
    Arg, BinOp, Block, Branch, Expr, ExprKind, FuncDef, Stmt, StmtKind, Target, TargetKind,
};
use crate::diag::Result;
use crate::infer::Type;
use crate::ir::{BlockId, CmpOp, Inst, IntOp, LocalId, Operand, Rvalue, Term, Ty, View};
use crate::span::Span;

use super::table::Part;
use super::{unsupported, FnBuilder, Loop};

/// What a loop does with each item.
pub(super) enum Looped<'s> {
    /// A `for` statement's target, body and `else`; `takes` where it
    /// reads `map`'s results, each step taking a task's result.
    For {
        target: &'s Target,
        body: &'s Block,
        orelse: &'s Block,
        takes: Option<Span>,
    },
    /// `ex.map(f, xs)` (M6c): runs the task `f` on the item and appends
    /// its result to `out` ([`FnBuilder::map_item`]), or in `build`
    /// submits it to `exec` and appends its future.
    Map {
        f: &'s str,
        out: LocalId,
        exec: Option<Operand>,
        span: Span,
    },
    /// An `enumerate(...)` or `zip(...)` that a builtin reads: appends
    /// each item to `out`, or only the first where `first`
    /// ([`FnBuilder::collect_zip`]).
    Collect {
        out: LocalId,
        first: bool,
        span: Span,
    },
}

/// A generic function's result type that no `return` decides first.
pub(super) const OPEN_RESULT: &str =
    "a generic function whose open result type no `return` decides";

/// A range's `(start, stop, step)`.
pub(super) type Bounds = (Operand, Operand, Operand);

impl FnBuilder<'_, '_> {
    /// Lowers `block`'s statements. In an instance whose result type is
    /// still undecided, a statement stuck on a recursive call is skipped
    /// (the instance is lowered again once a `return` decides it). A
    /// statement that calls an instance that never returns ends there.
    pub(super) fn block(&mut self, block: &Block) -> Result<()> {
        for (i, s) in block.iter().enumerate() {
            let (pools, loops) = (self.pools.len(), self.loops.len());
            let (tries, unbinds) = (self.tries.len(), self.unbinds.len());
            match self.pooled_stmt(|b| b.stmt(s)) {
                Err(_) if self.stuck || self.diverged => {
                    let diverged = self.diverged;
                    self.skipped |= self.stuck;
                    (self.stuck, self.diverged) = (false, false);
                    self.pools.truncate(pools);
                    self.loops.truncate(loops);
                    self.tries.truncate(tries);
                    self.unbinds.truncate(unbinds);
                    self.jump_away(Term::Unreachable);
                    // What follows in this block is never reached.
                    if diverged {
                        self.dead_assigns(&block[i..]);
                        return Ok(());
                    }
                }
                r => r?,
            }
        }
        Ok(())
    }

    fn stmt(&mut self, s: &Stmt) -> Result<()> {
        match &s.kind {
            StmtKind::Expr(e) => self.expr(e).map(drop),
            StmtKind::Assign { target, value } => self.assign_stmt(target, value),
            StmtKind::AnnAssign { target, value, .. } => {
                let v = self.expr(value)?;
                self.write(&target.name, v, target.span)
            }
            StmtKind::AugAssign { target, op, value } => {
                self.aug_assign(target, *op, value, s.span)
            }
            StmtKind::Return(value) => self.return_stmt(value.as_ref(), s.span),
            StmtKind::Break => self.loop_jump(s.span, |l| l.exit),
            StmtKind::Continue => self.loop_jump(s.span, |l| l.next),
            StmtKind::Pass => Ok(()),
            StmtKind::FuncDef(f) => self.def_stmt(f),
            StmtKind::If {
                branch,
                elifs,
                orelse,
            } => self.if_stmt(branch, elifs, orelse),
            StmtKind::While { branch, orelse } => self.while_stmt(branch, orelse),
            StmtKind::For {
                target,
                iter,
                body,
                orelse,
            } => self.for_stmt(target, iter, body, orelse),
            StmtKind::Raise(value) => self.raise_stmt(value, s.span),
            StmtKind::Try { body, handlers } => self.try_stmt(body, handlers),
            StmtKind::With { binding, body, .. } => self.with_stmt(binding, body, s.span),
        }
    }

    /// `target = value`. Unpacking a tuple display evaluates, and
    /// snapshots, every value before binding any target.
    fn assign_stmt(&mut self, target: &Target, value: &Expr) -> Result<()> {
        let (TargetKind::Tuple(targets), ExprKind::Tuple(values)) = (&target.kind, &value.kind)
        else {
            let v = self.expr(value)?;
            return self.bind(target, v);
        };
        if targets.len() != values.len() {
            return Err(unsupported(
                target.span,
                "unpacking a tuple of another length",
            ));
        }
        let mut ops = Vec::new();
        for v in values {
            let op = self.expr(v)?;
            let snapshot = match self.ty(op) {
                ty if ty.is_rc() => self.own(op, v.span),
                ty => self.temp(ty, Rvalue::Use(op), v.span),
            };
            ops.push(snapshot);
        }
        targets
            .iter()
            .zip(ops)
            .try_for_each(|(t, op)| self.bind(t, op))
    }

    /// `x op= v`, or `list[i] op= v` with the list and index evaluated
    /// once.
    fn aug_assign(&mut self, target: &Target, op: BinOp, value: &Expr, span: Span) -> Result<()> {
        match &target.kind {
            TargetKind::Name(name) => {
                let l = self.read(name, target.span)?;
                let r = self.expr(value)?;
                let v = self.arith(op, l, r, span)?;
                self.write(name, v, target.span)
            }
            TargetKind::Subscript { value: list, index } => {
                let list = self.expr(list)?;
                let index = self.expr(index)?;
                if let Ty::Dict(_) = self.ty(list) {
                    let l = self.dict_read(list, index, target.span)?;
                    let r = self.expr(value)?;
                    let v = self.arith(op, l, r, span)?;
                    return self.store_item(list, index, v, target.span);
                }
                let index = self.as_int(index, target.span);
                let ty = self.ty(list);
                if !matches!(ty, Ty::List(_)) || self.ty(index) != Ty::Int {
                    return Err(unsupported(
                        target.span,
                        "augmented assignment to this item",
                    ));
                }
                let elem = self.slot_ty(ty, 0, target.span)?;
                let l = self.temp(elem, Rvalue::Index(list, index), target.span);
                self.check_raised(target.span)?;
                let r = self.expr(value)?;
                let v = self.arith(op, l, r, span)?;
                self.store_item(list, index, v, target.span)
            }
            TargetKind::Tuple(_) => Err(unsupported(target.span, "augmented unpacking")),
        }
    }

    /// Leaves the value in the return slot and goes to the exit block,
    /// which releases the function's strings.
    fn return_stmt(&mut self, value: Option<&Expr>, span: Span) -> Result<()> {
        let v = match value {
            Some(e) => self.expr(e)?,
            None => Operand::None,
        };
        if let Some(declared) = self.open_ret.clone() {
            self.decide_ret(&declared, v, span)?;
        }
        let v = self.fit(v, self.ret, span);
        if self.ty(v) != self.ret {
            return Err(unsupported(
                span,
                "returning a value of another type than inferred",
            ));
        }
        let slot = self.return_slot();
        self.move_into(slot, v, span);
        self.jump_away(Term::Goto(self.exit));
        Ok(())
    }

    /// Decides an open result type from the first `return`'s value `v`:
    /// its type in the declared shape, so `[x]` returned from a
    /// `-> list | None` function makes it `list[int] | None`. A `None`
    /// whose optional's present type is still open decides nothing: the
    /// statement is skipped for the second pass ([`FnBuilder::block`]).
    fn decide_ret(&mut self, declared: &Type, v: Operand, span: Span) -> Result<()> {
        let have = self.ty(v);
        let open_optional =
            matches!(declared, Type::Optional(_)) && !super::is_closed(declared, &self.subst);
        if have == Ty::None && open_optional {
            (self.stuck, self.may_return) = (true, true);
            return Err(unsupported(span, OPEN_RESULT));
        }
        let ret = self.shape(declared, have, span)?;
        if !super::bind(declared, ret, &self.cx.seqs, &mut self.subst) {
            return Err(super::internal(
                span,
                "a result of another shape than declared",
            ));
        }
        (self.ret, self.open_ret) = (ret, None);
        Ok(())
    }

    /// A loop or branch condition, as a `Bool`. Its strings are released
    /// before the branch. One that calls an instance that never returns
    /// ends its path there; the statement's other paths (an earlier
    /// branch to the join) carry on. `branch`: evaluated only when an
    /// earlier test failed (`elif`), so stuck there, the statement may
    /// still return another way.
    fn cond(&mut self, test: &Expr, branch: bool) -> Result<Operand> {
        let cond = self.pooled_stmt(|b| {
            let t = b.expr(test)?;
            Ok(b.truthy(t, test.span))
        });
        match cond {
            Err(_) if self.diverged => {
                self.diverged = false;
                self.jump_away(Term::Unreachable);
                Ok(Operand::Bool(false))
            }
            Err(e) if self.stuck && branch => {
                self.may_return = true;
                Err(e)
            }
            r => r,
        }
    }

    /// A top-level `def` (inference rejects nested ones). Its body is
    /// lowered per instance; here its defaults are evaluated, then it
    /// becomes callable.
    fn def_stmt(&mut self, f: &FuncDef) -> Result<()> {
        self.store_defaults(f)?;
        if let Some(defined) = &mut self.defined {
            defined.insert(f.name.name.clone());
        }
        Ok(())
    }

    fn loop_jump(&mut self, span: Span, to: fn(Loop) -> BlockId) -> Result<()> {
        let Some(l) = self.loops.last().copied() else {
            return Err(super::internal(
                span,
                "`break` or `continue` outside a loop",
            ));
        };
        // The statements being left end here: release their temporaries.
        // From a nested loop's `else`, that includes its snapshot.
        let left: Vec<LocalId> = self.pools[l.pools..].concat();
        left.into_iter().for_each(|t| self.emit(Inst::Release(t)));
        let depth = self.loops.len();
        self.unbind_left(|u| u.loops >= depth)?;
        self.jump_away(Term::Goto(to(l)));
        Ok(())
    }

    fn if_stmt(&mut self, branch: &Branch, elifs: &[Branch], orelse: &Block) -> Result<()> {
        let end = self.new_block();
        for (i, b) in std::iter::once(branch).chain(elifs).enumerate() {
            let (then, next) = (self.new_block(), self.new_block());
            let cond = self.cond(&b.test, i > 0)?;
            self.terminate(Term::Branch {
                cond,
                then,
                orelse: next,
            });
            self.switch_to(then);
            self.block(&b.body)?;
            self.terminate(Term::Goto(end));
            self.switch_to(next);
        }
        self.block(orelse)?;
        self.terminate(Term::Goto(end));
        self.switch_to(end);
        Ok(())
    }

    fn while_stmt(&mut self, branch: &Branch, orelse: &Block) -> Result<()> {
        let (head, body, other, end) = (
            self.new_block(),
            self.new_block(),
            self.new_block(),
            self.new_block(),
        );
        self.terminate(Term::Goto(head));
        self.switch_to(head);
        let cond = self.cond(&branch.test, false)?;
        self.terminate(Term::Branch {
            cond,
            then: body,
            orelse: other,
        });
        self.switch_to(body);
        self.loop_body(&branch.body, head, end)?;
        self.switch_to(other);
        self.block(orelse)?;
        self.terminate(Term::Goto(end));
        self.switch_to(end);
        Ok(())
    }

    /// Lowers `body` with `break` to `exit` and `continue` to `next`, then
    /// jumps to `next`.
    pub(super) fn loop_body(&mut self, body: &Block, next: BlockId, exit: BlockId) -> Result<()> {
        let pools = self.pools.len();
        self.loops.push(Loop { next, exit, pools });
        let lowered = self.block(body);
        self.loops.pop();
        lowered?;
        self.terminate(Term::Goto(next));
        Ok(())
    }

    fn for_stmt(
        &mut self,
        target: &Target,
        iter: &Expr,
        body: &Block,
        orelse: &Block,
    ) -> Result<()> {
        // In `build` each step takes its result itself ([`FnBuilder::for_results`]).
        let results = matches!(self.node_type(iter.span)?, Type::Results(_));
        let takes = (results && !self.native_tasks()).then_some(iter.span);
        let looped = Looped::For {
            target,
            body,
            orelse,
            takes,
        };
        if let Some(z) = self.zip_of(iter)? {
            return self.for_zip(&looped, &z);
        }
        if let Some(args) = self.range_args(iter) {
            let mut ops = Vec::new();
            for a in args {
                ops.push(self.expr(&a.value)?);
            }
            let bounds = self.range_bounds(args, &ops, iter.span)?;
            return self.for_range(&looped, bounds, iter.span);
        }
        // `for x in d.keys()` (or values, or items) reads `d` itself.
        if let Some((d, view)) = self.dict_view(iter) {
            let d = self.expr(d)?;
            let part = match view {
                View::Keys => Part::Key,
                View::Values => Part::Value,
                View::Items => {
                    let (k, v) = self.key_value(self.ty(d), iter.span)?;
                    Part::Item(self.cx.seqs.tuple(vec![k, v]))
                }
            };
            return self.for_table(&looped, d, part, iter.span);
        }
        let s = self.expr(iter)?;
        if self.ty(s) == Ty::Results {
            let item = self.result_ty(iter.span)?;
            return self.for_results(&looped, s, item, iter.span);
        }
        self.for_value(&looped, s, iter.span)
    }

    /// `for x in rs` over `map`'s results in `build`: takes the next
    /// while any are left, so a loop left early leaves the rest, and one
    /// that raised has ended them. The loop holds its own reference.
    fn for_results(&mut self, l: &Looped, s: Operand, item: Ty, span: Span) -> Result<()> {
        let s = self.own(s, span);
        let (head, enter, latch) = (self.new_block(), self.new_block(), self.new_block());
        let (other, end) = (self.new_block(), self.new_block());
        self.terminate(Term::Goto(head));
        self.switch_to(head);
        let n = self.temp(Ty::Int, Rvalue::Len(s), span);
        let going = self.temp(Ty::Bool, Rvalue::Cmp(CmpOp::Gt, n, Operand::Int(0)), span);
        self.terminate(Term::Branch {
            cond: going,
            then: enter,
            orelse: other,
        });
        self.switch_to(enter);
        self.iteration(l, |b| b.next_result(s, item, span), latch, end)?;
        self.switch_to(latch);
        self.terminate(Term::Goto(head));
        self.finish_loop(l, other, end)
    }

    /// A loop over the value `s`, already evaluated.
    pub(super) fn for_value(&mut self, l: &Looped, s: Operand, span: Span) -> Result<()> {
        if let Ty::Dict(_) | Ty::Set(_) = self.ty(s) {
            return self.for_table(l, s, Part::Key, span);
        }
        if self.ty(s) == Ty::Range {
            let bounds = [0, 1, 2].map(|k| self.temp(Ty::Int, Rvalue::Field(s, k), span));
            return self.for_range(l, bounds.into(), span);
        }
        if self
            .cx
            .seqs
            .fields(self.ty(s))
            .is_some_and(<[Ty]>::is_empty)
        {
            // `for x in ()`: the body never runs, so its target is never
            // bound and keeps whatever it held; only `else` runs.
            return match l {
                Looped::For { orelse, .. } => self.block(orelse),
                Looped::Map { .. } | Looped::Collect { .. } => Ok(()),
            };
        }
        let item = match self.ty(s) {
            Ty::Str => Ty::Str,
            ty @ Ty::List(_) => self.slot_ty(ty, 0, span)?,
            ty @ Ty::View(_) => self.cx.seqs.elem(ty).unwrap_or(Ty::None),
            ty @ Ty::Tuple(_) => self.tuple_item(ty, span)?,
            _ => {
                return Err(unsupported(
                    span,
                    "`for` over anything but `range(...)`, a string, a list or a tuple",
                ))
            }
        };
        self.for_seq(l, s, item, span)
    }

    /// The one field type of a non-empty tuple `for` iterates: inference
    /// requires every field to have it.
    pub(super) fn tuple_item(&self, ty: Ty, span: Span) -> Result<Ty> {
        let fields = self.cx.seqs.fields(ty).unwrap_or_default();
        match fields.split_first() {
            Some((first, rest)) if rest.iter().all(|f| f == first) => Ok(*first),
            _ => Err(unsupported(span, "`for` over a tuple of mixed types")),
        }
    }

    /// `for x in s` over a string, list or tuple: by index, re-reading the
    /// length each time as Python does, so appending in the body extends
    /// the loop. The loop holds its own reference to `s`, and for a list
    /// its body cannot change, a read borrow of it too ([`super::hold`]).
    fn for_seq(&mut self, l: &Looped, s: Operand, item: Ty, span: Span) -> Result<()> {
        let s = self.own(s, span);
        let (setup, first) = (self.cur, self.blocks.len());
        let counter = self.new_local(Ty::Int);
        self.assign(counter, Rvalue::Use(Operand::Int(0)), span);
        let cur = Operand::Local(counter);
        let (head, enter, latch) = (self.new_block(), self.new_block(), self.new_block());
        let (other, end) = (self.new_block(), self.new_block());
        self.terminate(Term::Goto(head));
        self.switch_to(head);
        let n = self.temp(Ty::Int, Rvalue::Len(s), span);
        let going = self.temp(Ty::Bool, Rvalue::Cmp(CmpOp::Lt, cur, n), span);
        self.terminate(Term::Branch {
            cond: going,
            then: enter,
            orelse: other,
        });
        self.switch_to(enter);
        self.iteration(
            l,
            |b| Ok(b.temp(item, Rvalue::Index(s, cur), span)),
            latch,
            end,
        )?;
        self.switch_to(latch);
        let next = Rvalue::Int(IntOp::Add, cur, Operand::Int(1));
        self.assign(counter, next, span);
        self.terminate(Term::Goto(head));
        self.finish_loop(l, other, end)?;
        self.hold_if_unchanged(s, setup, counter, first, span);
        Ok(())
    }

    /// Binds a loop target to the value `next` gives, as a statement of
    /// its own: the item and whatever binding it creates (the fields an
    /// unpacking reads, the list an indexed target evaluates) are
    /// released each iteration.
    pub(super) fn bind_iteration(
        &mut self,
        target: &Target,
        next: impl FnOnce(&mut Self) -> Result<Operand>,
    ) -> Result<()> {
        self.pooled_stmt(|b| {
            let x = next(b)?;
            b.bind(target, x)
        })
    }

    /// One iteration of `l`: binds the item `next` gives and runs the
    /// body, which continues at `latch` or leaves for `end`.
    pub(super) fn iteration(
        &mut self,
        l: &Looped,
        next: impl FnOnce(&mut Self) -> Result<Operand>,
        latch: BlockId,
        end: BlockId,
    ) -> Result<()> {
        match l {
            Looped::For {
                target,
                body,
                takes,
                ..
            } => {
                if let Some(at) = takes {
                    self.took_result(*at)?;
                }
                self.bind_iteration(target, next)?;
                self.loop_body(body, latch, end)
            }
            Looped::Map { f, out, exec, span } => {
                self.pooled_stmt(|b| {
                    let x = next(b)?;
                    match exec {
                        Some(exec) => b.native_map_item(f, *out, *exec, x, *span),
                        None => b.map_item(f, *out, x, *span),
                    }
                })?;
                self.terminate(Term::Goto(latch));
                Ok(())
            }
            Looped::Collect { out, first, span } => {
                self.pooled_stmt(|b| {
                    let value = next(b)?;
                    let list = Operand::Local(*out);
                    let span = *span;
                    b.emit(Inst::Append { list, value, span });
                    Ok(())
                })?;
                self.terminate(Term::Goto(if *first { end } else { latch }));
                Ok(())
            }
        }
    }

    /// Lowers the `else` of a loop in `other`, then continues at `end`.
    pub(super) fn finish_loop(&mut self, l: &Looped, other: BlockId, end: BlockId) -> Result<()> {
        self.switch_to(other);
        if let Looped::For { orelse, .. } = l {
            self.block(orelse)?;
        }
        self.terminate(Term::Goto(end));
        self.switch_to(end);
        Ok(())
    }

    /// `for x` over the range `(start, stop, step)`, its bounds evaluated
    /// once. The counter steps until it passes `stop`; a step that would
    /// overflow `i64` has passed it too, since `stop` fits in one.
    fn for_range(&mut self, l: &Looped, bounds: Bounds, iter_span: Span) -> Result<()> {
        let (start, stop, step) = bounds;
        let counter = self.new_local(Ty::Int);
        self.assign(counter, Rvalue::Use(start), iter_span);
        let cur = Operand::Local(counter);
        let (head, enter, latch, step_on) = (
            self.new_block(),
            self.new_block(),
            self.new_block(),
            self.new_block(),
        );
        let (other, end) = (self.new_block(), self.new_block());
        self.terminate(Term::Goto(head));
        self.switch_to(head);
        let going = self.range_going(cur, stop, step, iter_span);
        self.terminate(Term::Branch {
            cond: going,
            then: enter,
            orelse: other,
        });
        self.switch_to(enter);
        self.iteration(l, |_| Ok(cur), latch, end)?;
        self.switch_to(latch);
        let over = self.temp(Ty::Bool, Rvalue::AddOverflows(cur, step), iter_span);
        self.terminate(Term::Branch {
            cond: over,
            then: other,
            orelse: step_on,
        });
        self.switch_to(step_on);
        self.assign(counter, Rvalue::Int(IntOp::Add, cur, step), iter_span);
        self.terminate(Term::Goto(head));
        self.finish_loop(l, other, end)
    }

    /// The arguments of `iter` if it is a call to the builtin `range`.
    fn range_args<'e>(&self, iter: &'e Expr) -> Option<&'e [Arg]> {
        let ExprKind::Call { func, args } = &iter.kind else {
            return None;
        };
        let is_range = matches!(&func.kind, ExprKind::Name(n) if n == "range" && !self.cx.defs.contains_key("range"));
        is_range.then_some(args.as_slice())
    }

    /// `(start, stop, step)` of `range(args)`, the arguments evaluated
    /// into `ops`, snapshotted into temporaries. A zero step fails here,
    /// as Python's `range` does.
    pub(super) fn range_bounds(
        &mut self,
        args: &[Arg],
        ops: &[Operand],
        span: Span,
    ) -> Result<Bounds> {
        let mut snap = Vec::new();
        for (a, op) in args.iter().zip(ops) {
            if a.name.is_some() {
                return Err(unsupported(a.span, "keyword arguments"));
            }
            let op = self.as_int(*op, a.span);
            if self.ty(op) != Ty::Int {
                return Err(unsupported(a.span, "`range` over a non-int"));
            }
            snap.push(self.temp(Ty::Int, Rvalue::Use(op), a.span));
        }
        let ops = snap;
        let (start, stop, step) = match ops[..] {
            [stop] => (Operand::Int(0), stop, Operand::Int(1)),
            [start, stop] => (start, stop, Operand::Int(1)),
            [start, stop, step] => (start, stop, step),
            _ => return Err(super::internal(span, "`range` arity")),
        };
        if args.len() == 3 {
            self.fail_if(
                Rvalue::Cmp(CmpOp::Eq, step, Operand::Int(0)),
                span,
                "ValueError: range() arg 3 must not be zero",
            );
        }
        Ok((start, stop, step))
    }

    /// Whether the counter has not yet reached `stop`, in `step`'s
    /// direction. Only a given step is unknown here; the default is 1.
    fn range_going(&mut self, cur: Operand, stop: Operand, step: Operand, span: Span) -> Operand {
        let up = self.temp(Ty::Bool, Rvalue::Cmp(CmpOp::Lt, cur, stop), span);
        if step == Operand::Int(1) {
            return up;
        }
        let down = self.temp(Ty::Bool, Rvalue::Cmp(CmpOp::Gt, cur, stop), span);
        let positive = self.temp(
            Ty::Bool,
            Rvalue::Cmp(CmpOp::Gt, step, Operand::Int(0)),
            span,
        );
        let dest = self.new_local(Ty::Bool);
        let (pos, neg, end) = (self.new_block(), self.new_block(), self.new_block());
        self.terminate(Term::Branch {
            cond: positive,
            then: pos,
            orelse: neg,
        });
        for (block, v) in [(pos, up), (neg, down)] {
            self.switch_to(block);
            self.assign(dest, Rvalue::Use(v), span);
            self.terminate(Term::Goto(end));
        }
        self.switch_to(end);
        Operand::Local(dest)
    }
}
