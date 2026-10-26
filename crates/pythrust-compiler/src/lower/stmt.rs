//! Statements and control flow.

use crate::ast::{Arg, BinOp, Block, Branch, Expr, ExprKind, Stmt, StmtKind, Target, TargetKind};
use crate::diag::{DiagKind, Diagnostic, Result};
use crate::ir::{BlockId, CmpOp, IntOp, Operand, Rvalue, Term, Ty};
use crate::span::Span;

use super::{unsupported, FnBuilder, Loop};

impl FnBuilder<'_, '_> {
    pub(super) fn block(&mut self, block: &Block) -> Result<()> {
        block.iter().try_for_each(|s| self.stmt(s))
    }

    fn stmt(&mut self, s: &Stmt) -> Result<()> {
        match &s.kind {
            StmtKind::Expr(e) => self.expr(e).map(drop),
            StmtKind::Assign { target, value } => self.assign_stmt(target, value),
            StmtKind::AnnAssign { target, value, .. } => {
                let v = self.expr(value)?;
                let dest = self.var(&target.name, target.span)?;
                self.assign(dest, Rvalue::Use(v), s.span);
                Ok(())
            }
            StmtKind::AugAssign { target, op, value } => {
                self.aug_assign(target, *op, value, s.span)
            }
            StmtKind::Return(value) => {
                let v = match value {
                    Some(e) => self.expr(e)?,
                    None => Operand::None,
                };
                self.jump_away(Term::Return(v));
                Ok(())
            }
            StmtKind::Break => self.loop_jump(s.span, |l| l.exit),
            StmtKind::Continue => self.loop_jump(s.span, |l| l.next),
            // Top-level `def`s are lowered as instances; inference
            // rejects nested ones.
            StmtKind::Pass | StmtKind::FuncDef(_) => Ok(()),
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

    /// `x = v`, or `a, b = v1, v2` from a tuple display: every value is
    /// evaluated, and snapshotted, before any name is bound.
    fn assign_stmt(&mut self, target: &Target, value: &Expr) -> Result<()> {
        if let TargetKind::Name(name) = &target.kind {
            let v = self.expr(value)?;
            let dest = self.var(name, target.span)?;
            self.assign(dest, Rvalue::Use(v), target.span);
            return Ok(());
        }
        let (TargetKind::Tuple(targets), ExprKind::Tuple(values)) = (&target.kind, &value.kind)
        else {
            return Err(unsupported(target.span, "this assignment target"));
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
            ops.push(self.temp(self.ty(op), Rvalue::Use(op), v.span));
        }
        for (t, op) in targets.iter().zip(ops) {
            let TargetKind::Name(name) = &t.kind else {
                return Err(unsupported(t.span, "nested or indexed unpacking"));
            };
            let dest = self.var(name, t.span)?;
            self.assign(dest, Rvalue::Use(op), t.span);
        }
        Ok(())
    }

    fn aug_assign(&mut self, target: &Target, op: BinOp, value: &Expr, span: Span) -> Result<()> {
        let TargetKind::Name(name) = &target.kind else {
            return Err(unsupported(target.span, "augmented assignment to an index"));
        };
        let dest = self.var(name, target.span)?;
        let r = self.expr(value)?;
        let v = self.int_op(op, Operand::Local(dest), r, span)?;
        self.assign(dest, Rvalue::Use(v), span);
        Ok(())
    }

    fn loop_jump(&mut self, span: Span, to: fn(Loop) -> BlockId) -> Result<()> {
        let Some(l) = self.loops.last().copied() else {
            return Err(super::internal(
                span,
                "`break` or `continue` outside a loop",
            ));
        };
        self.jump_away(Term::Goto(to(l)));
        Ok(())
    }

    fn if_stmt(&mut self, branch: &Branch, elifs: &[Branch], orelse: &Block) -> Result<()> {
        let end = self.new_block();
        for b in std::iter::once(branch).chain(elifs) {
            let (then, next) = (self.new_block(), self.new_block());
            let t = self.expr(&b.test)?;
            let cond = self.truthy(t, b.test.span);
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
        let t = self.expr(&branch.test)?;
        let cond = self.truthy(t, branch.test.span);
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
        self.loops.push(Loop { next, exit });
        let lowered = self.block(body);
        self.loops.pop();
        lowered?;
        self.terminate(Term::Goto(next));
        Ok(())
    }

    /// `for x in range(...)`. The bounds are evaluated once. The counter
    /// steps until it passes `stop`; a step that would overflow `i64` has
    /// passed it too, since `stop` fits in one.
    fn for_stmt(
        &mut self,
        target: &Target,
        iter: &Expr,
        body: &Block,
        orelse: &Block,
    ) -> Result<()> {
        let TargetKind::Name(name) = &target.kind else {
            return Err(unsupported(target.span, "unpacking in a `for` target"));
        };
        let Some(args) = self.range_args(iter) else {
            return Err(unsupported(
                iter.span,
                "`for` over anything but `range(...)`",
            ));
        };
        let (start, stop, step) = self.range_bounds(args, iter.span)?;
        let counter = self.new_local(Ty::Int);
        self.assign(counter, Rvalue::Use(start), iter.span);
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
        let going = self.range_going(cur, stop, step, iter.span);
        self.terminate(Term::Branch {
            cond: going,
            then: enter,
            orelse: other,
        });
        self.switch_to(enter);
        let dest = self.var(name, target.span)?;
        self.assign(dest, Rvalue::Use(cur), target.span);
        self.loop_body(body, latch, end)?;
        self.switch_to(latch);
        let over = self.temp(Ty::Bool, Rvalue::AddOverflows(cur, step), iter.span);
        self.terminate(Term::Branch {
            cond: over,
            then: other,
            orelse: step_on,
        });
        self.switch_to(step_on);
        self.assign(counter, Rvalue::Int(IntOp::Add, cur, step), iter.span);
        self.terminate(Term::Goto(head));
        self.switch_to(other);
        self.block(orelse)?;
        self.terminate(Term::Goto(end));
        self.switch_to(end);
        Ok(())
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

    /// Fails with `message` at `span` when the `Bool` `test` holds.
    fn fail_if(&mut self, test: Rvalue, span: Span, message: &str) {
        let cond = self.temp(Ty::Bool, test, span);
        let (fail, ok) = (self.new_block(), self.new_block());
        self.terminate(Term::Branch {
            cond,
            then: fail,
            orelse: ok,
        });
        self.switch_to(fail);
        self.terminate(Term::Fail(Diagnostic {
            span,
            message: message.to_string(),
            kind: DiagKind::Runtime,
        }));
        self.switch_to(ok);
    }
}
