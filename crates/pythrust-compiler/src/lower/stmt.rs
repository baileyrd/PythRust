//! Statements and control flow.

use crate::ast::{
    Arg, BinOp, Block, Branch, Expr, ExprKind, FuncDef, Stmt, StmtKind, Target, TargetKind,
};
use crate::diag::Result;
use crate::ir::{BlockId, CmpOp, Inst, IntOp, LocalId, Operand, Rvalue, Term, Ty};
use crate::span::Span;

use super::{unsupported, FnBuilder, Loop};

/// A `for` loop's parts other than what it iterates.
struct Looped<'s> {
    target: &'s Target,
    body: &'s Block,
    orelse: &'s Block,
}

impl FnBuilder<'_, '_> {
    pub(super) fn block(&mut self, block: &Block) -> Result<()> {
        block
            .iter()
            .try_for_each(|s| self.pooled_stmt(|b| b.stmt(s)))
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
                let ty = self.ty(list);
                if !matches!(ty, Ty::List(_)) {
                    return Err(unsupported(
                        target.span,
                        "augmented assignment to this item",
                    ));
                }
                let elem = self.slot_ty(ty, 0, target.span)?;
                let l = self.temp(elem, Rvalue::Index(list, index), target.span);
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

    /// A loop or branch condition, as a `Bool`. Its strings are released
    /// before the branch.
    fn cond(&mut self, test: &Expr) -> Result<Operand> {
        self.pooled_stmt(|b| {
            let t = b.expr(test)?;
            Ok(b.truthy(t, test.span))
        })
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
        self.jump_away(Term::Goto(to(l)));
        Ok(())
    }

    fn if_stmt(&mut self, branch: &Branch, elifs: &[Branch], orelse: &Block) -> Result<()> {
        let end = self.new_block();
        for b in std::iter::once(branch).chain(elifs) {
            let (then, next) = (self.new_block(), self.new_block());
            let cond = self.cond(&b.test)?;
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
        let cond = self.cond(&branch.test)?;
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
    fn loop_body(&mut self, body: &Block, next: BlockId, exit: BlockId) -> Result<()> {
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
        let looped = Looped {
            target,
            body,
            orelse,
        };
        if let Some(args) = self.range_args(iter) {
            return self.for_range(&looped, args, iter.span);
        }
        let s = self.expr(iter)?;
        let item = match self.ty(s) {
            Ty::Str => Ty::Str,
            ty @ Ty::List(_) => self.slot_ty(ty, 0, iter.span)?,
            _ => {
                return Err(unsupported(
                    iter.span,
                    "`for` over anything but `range(...)`, a string or a list",
                ))
            }
        };
        self.for_seq(&looped, s, item, iter.span)
    }

    /// `for x in s` over a string or list: by index, re-reading the
    /// length each time as Python does, so appending in the body extends
    /// the loop. The loop holds its own reference to `s`.
    fn for_seq(&mut self, l: &Looped, s: Operand, item: Ty, span: Span) -> Result<()> {
        let s = self.own(s, span);
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
        self.bind_iteration(l.target, |b| Ok(b.temp(item, Rvalue::Index(s, cur), span)))?;
        self.loop_body(l.body, latch, end)?;
        self.switch_to(latch);
        let next = Rvalue::Int(IntOp::Add, cur, Operand::Int(1));
        self.assign(counter, next, span);
        self.terminate(Term::Goto(head));
        self.finish_loop(l.orelse, other, end)
    }

    /// Binds a loop target to the value `next` gives, as a statement of
    /// its own: the item and whatever binding it creates (the fields an
    /// unpacking reads, the list an indexed target evaluates) are
    /// released each iteration.
    fn bind_iteration(
        &mut self,
        target: &Target,
        next: impl FnOnce(&mut Self) -> Result<Operand>,
    ) -> Result<()> {
        self.pooled_stmt(|b| {
            let x = next(b)?;
            b.bind(target, x)
        })
    }

    /// Lowers the `else` of a loop in `other`, then continues at `end`.
    fn finish_loop(&mut self, orelse: &Block, other: BlockId, end: BlockId) -> Result<()> {
        self.switch_to(other);
        self.block(orelse)?;
        self.terminate(Term::Goto(end));
        self.switch_to(end);
        Ok(())
    }

    /// `for x in range(...)`. The bounds are evaluated once. The counter
    /// steps until it passes `stop`; a step that would overflow `i64` has
    /// passed it too, since `stop` fits in one.
    fn for_range(&mut self, l: &Looped, args: &[Arg], iter_span: Span) -> Result<()> {
        let (start, stop, step) = self.range_bounds(args, iter_span)?;
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
        self.bind_iteration(l.target, |_| Ok(cur))?;
        self.loop_body(l.body, latch, end)?;
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
        self.finish_loop(l.orelse, other, end)
    }

    /// The arguments of `iter` if it is a call to the builtin `range`.
    fn range_args<'e>(&self, iter: &'e Expr) -> Option<&'e [Arg]> {
        let ExprKind::Call { func, args } = &iter.kind else {
            return None;
        };
        let is_range = matches!(&func.kind, ExprKind::Name(n) if n == "range" && !self.cx.defs.contains_key("range"));
        is_range.then_some(args.as_slice())
    }

    /// `(start, stop, step)` snapshotted into temporaries. A zero step
    /// fails here, as Python's `range` does.
    fn range_bounds(&mut self, args: &[Arg], span: Span) -> Result<(Operand, Operand, Operand)> {
        let mut ops = Vec::new();
        for a in args {
            if a.name.is_some() {
                return Err(unsupported(a.span, "keyword arguments"));
            }
            let op = self.expr(&a.value)?;
            if self.ty(op) != Ty::Int {
                return Err(unsupported(a.span, "`range` over a non-int"));
            }
            ops.push(self.temp(Ty::Int, Rvalue::Use(op), a.span));
        }
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
