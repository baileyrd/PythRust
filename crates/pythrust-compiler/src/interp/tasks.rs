//! Tasks (M6c): `run` runs each task to completion at `submit`, in
//! program order, with its own call depth starting from zero
//! (`docs/research/concurrency.md#run-at-submit`). A real pool may run
//! them in that order too; what it does not reproduce is a listed
//! deviation.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::mem;
use std::rc::Rc;

use super::builtins::Args;
use super::value::{Outcome, Value};
use super::{rt_err, Exec, Interp, Unwind, MAX_DEPTH};
use crate::ast::{Arg, Block, CmpOp, Expr, ExprKind, Ident, Target};
use crate::span::Span;

/// How many calls may be active across a task and the tasks that
/// started it, since each runs on the interpreter's one stack.
const MAX_NESTED_CALLS: usize = 8 * MAX_DEPTH;

impl<'m> Interp<'m, '_> {
    /// `with value as name: body`. Every task the block starts has ended
    /// when it starts, so leaving it waits for nothing.
    pub(super) fn exec_with(
        &mut self,
        value: &'m Expr,
        binding: &'m Ident,
        body: &'m Block,
    ) -> Exec<'m, super::Flow<'m>> {
        let executor = self.eval(value)?;
        self.store(&binding.name, executor);
        self.exec_block(body)
    }

    /// `ex.submit(f, *args)`, `ex.map(f, xs)` or `fut.result()`; `None`
    /// for any other call.
    pub(super) fn task_call(
        &mut self,
        func: &'m Expr,
        args: &'m [Arg],
        span: Span,
    ) -> Option<Exec<'m, Value<'m>>> {
        let ExprKind::Attr { value, attr } = &func.kind else {
            return None;
        };
        match attr.name.as_str() {
            "submit" => Some(self.submit(value, args, span)),
            "map" => Some(self.map(value, args, span)),
            "result" => Some(self.result(value)),
            _ => None,
        }
    }

    fn submit(&mut self, ex: &'m Expr, args: &'m [Arg], span: Span) -> Exec<'m, Value<'m>> {
        self.executor(ex)?;
        let f = self.task_func(args, span)?;
        let positional = args[1..]
            .iter()
            .map(|a| self.eval(&a.value))
            .collect::<Exec<'m, Vec<Value<'m>>>>()?;
        let outcome = self.run_task(&f, positional, span)?;
        Ok(Value::Future(Rc::new(outcome)))
    }

    /// Reads `xs` once and runs a task per element, as CPython submits
    /// them all when `map` is called.
    fn map(&mut self, ex: &'m Expr, args: &'m [Arg], span: Span) -> Exec<'m, Value<'m>> {
        self.executor(ex)?;
        let f = self.task_func(args, span)?;
        let Some(xs) = args.get(1) else {
            return Err(rt_err(span, "internal: `map` without an iterable".to_string()).into());
        };
        let items: Vec<Value<'m>> = self.eval(&xs.value)?.iter(xs.value.span)?.collect();
        let mut outcomes = VecDeque::with_capacity(items.len());
        for item in items {
            outcomes.push_back(self.run_task(&f, vec![item], span)?);
        }
        Ok(Value::Results(Rc::new(RefCell::new(outcomes))))
    }

    /// `fut.result()`: the task's result, or what escaped it, raised
    /// again where it was first raised.
    fn result(&mut self, fut: &'m Expr) -> Exec<'m, Value<'m>> {
        let Value::Future(outcome) = self.eval(fut)? else {
            let msg = "internal: `result` of a non-future".to_string();
            return Err(rt_err(fut.span, msg).into());
        };
        taken(outcome.as_ref().clone())
    }

    fn executor(&mut self, ex: &'m Expr) -> Exec<'m, ()> {
        match self.eval(ex)? {
            Value::Executor => Ok(()),
            other => {
                let msg = format!("internal: a task of a {}", other.type_name());
                Err(rt_err(ex.span, msg).into())
            }
        }
    }

    /// The function a task runs: the first argument, a `def`'s name.
    fn task_func(&mut self, args: &'m [Arg], span: Span) -> Exec<'m, Value<'m>> {
        let Some(first) = args.first() else {
            return Err(rt_err(span, "internal: a task without a function".to_string()).into());
        };
        self.eval(&first.value)
    }

    /// Runs `f(args)` as a task: on the call stack of its own, so its
    /// depth counts from zero. A fault ends the program, as anywhere; a
    /// catchable exception is kept for `result()`.
    fn run_task(
        &mut self,
        f: &Value<'m>,
        args: Vec<Value<'m>>,
        span: Span,
    ) -> Exec<'m, Outcome<'m>> {
        let Value::Func(func) = f else {
            let msg = format!("internal: a task of a {}", f.type_name());
            return Err(rt_err(span, msg).into());
        };
        let outer = self.frames.len() + self.task_frames;
        if outer + MAX_DEPTH > MAX_NESTED_CALLS {
            let msg = "RecursionError: tasks nest too deeply to run at submit".to_string();
            return Err(rt_err(span, msg).into());
        }
        let saved = mem::take(&mut self.frames);
        self.task_frames += saved.len();
        let args = Args {
            positional: args,
            keywords: Vec::new(),
            span,
        };
        let ran = self.call_func(Rc::clone(func), args);
        self.task_frames -= saved.len();
        self.frames = saved;
        match ran {
            Ok(v) => Ok(Ok(v)),
            Err(Unwind::Raise(exc, at)) => Ok(Err((exc, at))),
            Err(fault) => Err(fault),
        }
    }

    /// `for target in results`: each step takes the next task's result,
    /// raising what escaped it there.
    pub(super) fn for_results(
        &mut self,
        results: &RefCell<VecDeque<Outcome<'m>>>,
        target: &'m Target,
        body: &'m Block,
        orelse: &'m Block,
    ) -> Exec<'m, super::Flow<'m>> {
        loop {
            let Some(outcome) = next(results) else {
                return self.exec_block(orelse);
            };
            self.assign(target, taken(outcome)?)?;
            if let Some(flow) = super::loop_exit(self.exec_block(body)?) {
                return Ok(flow);
            }
        }
    }

    /// `x in results` (or `not in`): takes results until one equals `x`.
    pub(super) fn results_contain(
        &mut self,
        results: &RefCell<VecDeque<Outcome<'m>>>,
        x: &Value<'m>,
        op: CmpOp,
        span: Span,
    ) -> Exec<'m, bool> {
        let found = loop {
            let Some(outcome) = next(results) else {
                break false;
            };
            if super::value::compare(CmpOp::Eq, x, &taken(outcome)?, span)? {
                break true;
            }
        };
        Ok(found == (op == CmpOp::In))
    }
}

/// Takes every remaining result, for `list`, `sorted` or `set`: the
/// first exception that escaped a task is raised.
pub(super) fn drain<'m>(results: &RefCell<VecDeque<Outcome<'m>>>) -> Exec<'m, Value<'m>> {
    let outcomes: Vec<Outcome<'m>> = results.borrow_mut().drain(..).collect();
    let values = outcomes
        .into_iter()
        .map(taken)
        .collect::<Exec<'m, Vec<Value<'m>>>>()?;
    Ok(Value::list(values))
}

/// The next result. One that raises ends the iteration, as an
/// exception ends Python's generator: nothing after it is taken.
fn next<'m>(results: &RefCell<VecDeque<Outcome<'m>>>) -> Option<Outcome<'m>> {
    let mut results = results.borrow_mut();
    let outcome = results.pop_front()?;
    if outcome.is_err() {
        results.clear();
    }
    Some(outcome)
}

fn taken<'m>(outcome: Outcome<'m>) -> Exec<'m, Value<'m>> {
    outcome.map_err(|(exc, at)| Unwind::Raise(exc, at))
}
