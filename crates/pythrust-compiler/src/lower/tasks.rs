//! Tasks (M6c), two ways.
//!
//! For analysis they lower as `run` runs them: each task to completion
//! where it is submitted, a call of its function there. A future is its
//! task's result, and `map`'s results are a list of them
//! ([`super::concrete`]), so `fut.result()` reads the value. The call
//! neither raises nor ends the path: `run` keeps what escapes the task,
//! and raises it where its result is taken ([`FnBuilder::took_result`]).
//!
//! For `build` they run on the executor's workers (`docs/native.md`):
//! the `with` statement owns an [`Ty::Exec`] temporary, released (which
//! waits for every task) however the block is left; `submit` gives a
//! [`Ty::Future`] ([`Rvalue::Submit`]) and `result()` waits for it
//! ([`Rvalue::TaskResult`]). `map` submits a task per element and
//! gives [`Ty::Results`], its futures, which each step over the results
//! takes in order ([`Rvalue::NextResult`]).

use crate::ast::{Arg, Block, Expr, ExprKind, FuncDef, Ident};
use crate::diag::Result;
use crate::infer::{scope, Type};
use crate::ir::{BlockId, CmpOp, Inst, LocalId, Operand, Rvalue, Task, Term, Ty};
use crate::span::Span;
use crate::stdlib::StdFn;

use super::expr::bind_params;
use super::stmt::Looped;
use super::{internal, unsupported, FnBuilder, Reach};

impl<'a> FnBuilder<'_, 'a> {
    /// `with ThreadPoolExecutor() as ex: body`. The variable holds
    /// nothing, and for analysis making the executor does nothing.
    pub(super) fn with_stmt(&mut self, binding: &Ident, body: &Block, span: Span) -> Result<()> {
        self.write(&binding.name, Operand::None, binding.span)?;
        if self.native_tasks() {
            return self.native_with(binding, body, span);
        }
        // The region is the blocks the body makes, from a fresh one; the
        // code after it goes on in another.
        let start = self.new_block();
        self.terminate(Term::Goto(start));
        self.switch_to(start);
        self.block(body)?;
        let region = (start.0..self.blocks.len()).map(BlockId).collect();
        self.regions.push(region);
        let after = self.new_block();
        self.terminate(Term::Goto(after));
        self.switch_to(after);
        Ok(())
    }

    /// Whether tasks run on workers, as `build` runs them, rather than
    /// where they are submitted, as analysis lowers them.
    pub(super) fn native_tasks(&self) -> bool {
        self.cx.reach == Reach::Called
    }

    /// [`FnBuilder::with_stmt`] for `build`: the executor is a temporary
    /// of the `with` statement, so its end, and a `break`, `continue`,
    /// `return` or raise leaving the block, releases it.
    fn native_with(&mut self, binding: &Ident, body: &Block, span: Span) -> Result<()> {
        let make = Rvalue::Std(StdFn::Executor, Vec::new());
        let Operand::Local(exec) = self.temp(Ty::Exec, make, span) else {
            return Err(internal(span, "an executor without a local"));
        };
        self.execs.push((binding.name.clone(), exec));
        let lowered = self.block(body);
        self.execs.pop();
        lowered
    }

    /// The executor `ex` names, in `build`.
    fn exec_named(&self, ex: &Expr) -> Result<Operand> {
        let ExprKind::Name(name) = &ex.kind else {
            return Err(internal(ex.span, "an executor that is not a name"));
        };
        self.exec_local(name)
            .ok_or_else(|| internal(ex.span, "an executor outside its `with` block"))
    }

    /// In `build`, the executor a `with` block in scope binds to `name`:
    /// its temporary, read in place of the binding, which holds `None`,
    /// so `if ex:` sees an executor, which is true.
    pub(super) fn exec_local(&self, name: &str) -> Option<Operand> {
        let found = self.execs.iter().rev().find(|(n, _)| n == name);
        found.map(|(_, l)| Operand::Local(*l))
    }

    /// `ex.submit(f, *args)`, `ex.map(f, xs)` or `fut.result()`; `None`
    /// for any other method.
    pub(super) fn task_call(
        &mut self,
        value: &Expr,
        attr: &str,
        args: &[Arg],
        span: Span,
    ) -> Result<Option<Operand>> {
        let native = self.native_tasks();
        match (self.node_type(value.span)?, attr) {
            (Type::Executor, "submit") if native => self.native_submit(value, args, span).map(Some),
            (Type::Executor, "map") if native => self.native_map(value, args, span).map(Some),
            (Type::Future(_), "result") if native => {
                let fut = self.expr(value)?;
                let ty = self.ty_at(span)?;
                let v = self.temp(ty, Rvalue::TaskResult(fut), span);
                self.took_result(span)?;
                Ok(Some(v))
            }
            (Type::Executor, "submit") => self.submit(args, span).map(Some),
            (Type::Executor, "map") => self.map(args, span).map(Some),
            (Type::Future(_), "result") => {
                let v = self.expr(value)?;
                self.took_result(span)?;
                Ok(Some(v))
            }
            _ => Ok(None),
        }
    }

    /// `ex.submit(f, *args)` in `build`: a new future of the task.
    fn native_submit(&mut self, ex: &Expr, args: &[Arg], span: Span) -> Result<Operand> {
        let def = self.task_def(args, span)?;
        let exec = self.exec_named(ex)?;
        let mut ops = Vec::new();
        for a in &args[1..] {
            ops.push(self.expr(&a.value)?);
        }
        let slots = bind_params(def, &args[1..], ops)?;
        let Type::Future(node) = self.node_type(span)? else {
            return Err(internal(span, "`submit` is not a future"));
        };
        let (args, func, _) = self.call_operands(def, slots, node, span)?;
        let submit = Rvalue::Submit { exec, func, args };
        Ok(self.temp(Ty::Future, submit, span))
    }

    /// `ex.map(f, xs)` in `build`: submits a task per element of `xs`,
    /// read once, giving their futures, reversed so the first is taken
    /// first from the end.
    fn native_map(&mut self, ex: &Expr, args: &[Arg], span: Span) -> Result<Operand> {
        let [_, xs] = args else {
            return Err(internal(span, "`map` without a function and an iterable"));
        };
        let def = self.task_def(args, span)?;
        let exec = self.exec_named(ex)?;
        let s = self.expr(&xs.value)?;
        let out = self.temp(Ty::Results, Rvalue::NewSeq(Vec::new()), span);
        let Operand::Local(out_local) = out else {
            return Err(internal(span, "`map`'s results without a local"));
        };
        let looped = Looped::Map {
            f: &def.name.name,
            out: out_local,
            exec: Some(exec),
            span,
        };
        self.for_value(&looped, s, xs.value.span)?;
        self.emit(Inst::Reverse(out));
        Ok(out)
    }

    /// `ex.submit(f, *args)`: `f(*args)`, its result the future.
    fn submit(&mut self, args: &[Arg], span: Span) -> Result<Operand> {
        let def = self.task_def(args, span)?;
        let mut ops = Vec::new();
        for a in &args[1..] {
            ops.push(self.expr(&a.value)?);
        }
        let slots = bind_params(def, &args[1..], ops)?;
        let Type::Future(node) = self.node_type(span)? else {
            return Err(internal(span, "`submit` is not a future"));
        };
        let (value, _) = self.emit_call(def, slots, node, span)?;
        self.task(value, false, span)?;
        Ok(value)
    }

    /// `ex.map(f, xs)`: a new list of `f(x)` for each `x` of `xs`, read
    /// once. The list's type is known once the loop's call has decided
    /// `f`'s result, so it is made before the loop, typed after.
    fn map(&mut self, args: &[Arg], span: Span) -> Result<Operand> {
        let [_, xs] = args else {
            return Err(internal(span, "`map` without a function and an iterable"));
        };
        let def = self.task_def(args, span)?;
        let s = self.expr(&xs.value)?;
        let before = self.cur;
        let out = self.new_local(Ty::None);
        // Released where the statement ends, or where a raise leaves it.
        if let Some(pool) = self.pools.last_mut() {
            pool.push(out);
        }
        let looped = Looped::Map {
            f: &def.name.name,
            out,
            exec: None,
            span,
        };
        self.for_value(&looped, s, xs.value.span)?;
        self.locals[out.0] = self.results_ty(span)?;
        let make = Inst::Assign {
            dest: out,
            value: Rvalue::NewSeq(Vec::new()),
            span,
        };
        self.blocks[before.0].insts.push(make);
        Ok(Operand::Local(out))
    }

    /// The type of `map`'s results at `span`. A result type no call
    /// decided (`ex.map(f, ())` of a generic `f`) holds nothing, so its
    /// open parts take any slot type, as for `not []`.
    fn results_ty(&mut self, span: Span) -> Result<Ty> {
        let elem = self.result_ty(span)?;
        Ok(self.cx.seqs.list(elem))
    }

    /// The type of each of `map`'s results at `span`.
    pub(super) fn result_ty(&mut self, span: Span) -> Result<Ty> {
        let Type::Results(t) = self.node_type(span)? else {
            return Err(internal(span, "`map` gives no results"));
        };
        Ok(super::settle(t, &self.subst, &self.dead, &mut self.cx.seqs))
    }

    /// One item `x` of `map`: `f(x)`, appended to `out`.
    pub(super) fn map_item(&mut self, f: &str, out: LocalId, x: Operand, span: Span) -> Result<()> {
        let def = self.def_named(f, span)?;
        let mut slots = vec![None; def.params.len()];
        if let Some(first) = slots.first_mut() {
            *first = Some(x);
        }
        let Type::Results(node) = self.node_type(span)? else {
            return Err(internal(span, "`map` gives no results"));
        };
        let (value, _) = self.emit_call(def, slots, node, span)?;
        self.task(value, true, span)?;
        let list = Operand::Local(out);
        self.emit(Inst::Append { list, value, span });
        Ok(())
    }

    /// One item `x` of `map` in `build`: submits `f(x)` to `exec` and
    /// appends its future to `out`.
    pub(super) fn native_map_item(
        &mut self,
        f: &str,
        out: LocalId,
        exec: Operand,
        x: Operand,
        span: Span,
    ) -> Result<()> {
        let def = self.def_named(f, span)?;
        let mut slots = vec![None; def.params.len()];
        if let Some(first) = slots.first_mut() {
            *first = Some(x);
        }
        let Type::Results(node) = self.node_type(span)? else {
            return Err(internal(span, "`map` gives no results"));
        };
        let (args, func, _) = self.call_operands(def, slots, node, span)?;
        let submit = Rvalue::Submit { exec, func, args };
        let value = self.temp(Ty::Future, submit, span);
        let list = Operand::Local(out);
        self.emit(Inst::Append { list, value, span });
        Ok(())
    }

    /// Records the call whose result is `value` as a task.
    fn task(&mut self, value: Operand, map: bool, span: Span) -> Result<()> {
        let Operand::Local(dest) = value else {
            return Err(internal(span, "a task's call has no local"));
        };
        self.tasks.push(Task { dest, map });
        Ok(())
    }

    /// The `def` a task runs: `args[0]`, which inference checked names one.
    fn task_def(&self, args: &[Arg], span: Span) -> Result<&'a FuncDef> {
        let Some(ExprKind::Name(name)) = args.first().map(|a| &a.value.kind) else {
            return Err(internal(span, "a task without a function"));
        };
        self.def_named(name, span)
    }

    /// The top-level `def` named `name`.
    fn def_named(&self, name: &str, span: Span) -> Result<&'a FuncDef> {
        let what = || format!("a task of `{name}`, which is not a top-level `def`");
        self.cx
            .defs
            .get(name)
            .copied()
            .ok_or_else(|| unsupported(span, &what()))
    }

    /// Whether a task `body` starts may raise.
    pub(super) fn raising_tasks(&self, body: &[crate::ast::Stmt]) -> bool {
        let tasks = scope::exprs(body).into_iter().filter_map(scope::task_call);
        tasks
            .map(|(name, _, _)| name)
            .any(|name| self.may_raise(name))
    }

    /// A step that takes a task's result at `span`: `fut.result()`, or
    /// reading `map`'s results. Futures and `map`'s results stay in the
    /// function that made them, so it may raise what any task the body
    /// starts may, as `raises` counts it (D8).
    pub(super) fn took_result(&mut self, span: Span) -> Result<()> {
        if !self.tasks_raise {
            return Ok(());
        }
        self.check_raised(span)
    }

    /// [`FnBuilder::took_result`] where `e` is `map`'s results, for
    /// analysis; in `build` each step that takes one checks it.
    pub(super) fn reads_results(&mut self, e: &Expr) -> Result<()> {
        if self.native_tasks() {
            return Ok(());
        }
        match self.node_type(e.span)? {
            Type::Results(_) => self.took_result(e.span),
            _ => Ok(()),
        }
    }

    /// `list(rs)` in `build`: a new list of `map`'s results left in `rs`,
    /// taking each; the argument at `span`.
    pub(super) fn drain_results(&mut self, rs: Operand, span: Span) -> Result<Operand> {
        let item = self.result_ty(span)?;
        let ty = self.cx.seqs.list(item);
        let out = self.temp(ty, Rvalue::NewSeq(Vec::new()), span);
        self.take_each(rs, item, span, |b, value| {
            b.emit(Inst::Append {
                list: out,
                value,
                span,
            });
            Ok(None)
        })?;
        Ok(out)
    }

    /// `needle in rs` in `build`: takes `map`'s results until one equals
    /// `needle`, as `in` on CPython's iterator does; `rs` at `at`.
    pub(super) fn results_contain(
        &mut self,
        needle: Operand,
        rs: Operand,
        at: Span,
        span: Span,
    ) -> Result<Operand> {
        let item = self.result_ty(at)?;
        let found = self.new_local(Ty::Bool);
        self.assign(found, Rvalue::Use(Operand::Bool(false)), span);
        self.take_each(rs, item, span, |b, x| {
            let eq = b.equal(needle, x, span)?;
            b.assign(found, Rvalue::Use(eq), span);
            Ok(Some(Operand::Local(found)))
        })?;
        Ok(Operand::Local(found))
    }

    /// A loop taking each of `map`'s results left in `rs`, of type
    /// `item`, and running `step` on it as one statement; a condition
    /// `step` gives ends the loop when true.
    pub(super) fn take_each(
        &mut self,
        rs: Operand,
        item: Ty,
        span: Span,
        step: impl FnOnce(&mut Self, Operand) -> Result<Option<Operand>>,
    ) -> Result<()> {
        let (head, body, end) = (self.new_block(), self.new_block(), self.new_block());
        self.terminate(Term::Goto(head));
        self.switch_to(head);
        let n = self.temp(Ty::Int, Rvalue::Len(rs), span);
        let going = self.temp(Ty::Bool, Rvalue::Cmp(CmpOp::Gt, n, Operand::Int(0)), span);
        self.terminate(Term::Branch {
            cond: going,
            then: body,
            orelse: end,
        });
        self.switch_to(body);
        let stop = self.pooled_stmt(|b| {
            let x = b.next_result(rs, item, span)?;
            step(b, x)
        })?;
        let term = match stop {
            Some(cond) => Term::Branch {
                cond,
                then: end,
                orelse: head,
            },
            None => Term::Goto(head),
        };
        self.terminate(term);
        self.switch_to(end);
        Ok(())
    }

    /// The next of `map`'s results left in `rs`, of type `item`, which
    /// raises what its task raised.
    pub(super) fn next_result(&mut self, rs: Operand, item: Ty, span: Span) -> Result<Operand> {
        let x = self.temp(item, Rvalue::NextResult(rs), span);
        self.took_result(span)?;
        Ok(x)
    }
}
