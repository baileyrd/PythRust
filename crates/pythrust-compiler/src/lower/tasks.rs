//! Tasks (M6c), for analysis only: `build` refuses the `with` that
//! makes an executor until native tasks land (`docs/native.md`).
//!
//! They lower as `run` runs them: each task to completion where it is
//! submitted, a call of its function there. A future is its task's
//! result, and `map`'s results are a list of them ([`super::concrete`]),
//! so `fut.result()` reads the value. The call neither raises nor ends
//! the path: `run` keeps what escapes the task, and raises it where its
//! result is taken ([`FnBuilder::took_result`]).

use crate::ast::{Arg, Block, Expr, ExprKind, FuncDef, Ident};
use crate::diag::Result;
use crate::infer::{scope, Type};
use crate::ir::{Inst, LocalId, Operand, Rvalue, Ty};
use crate::span::Span;

use super::expr::bind_params;
use super::stmt::Looped;
use super::{internal, unsupported, FnBuilder, Reach};

impl<'a> FnBuilder<'_, 'a> {
    /// `with ThreadPoolExecutor() as ex: body`. The executor holds
    /// nothing, and making it does nothing analysis sees.
    pub(super) fn with_stmt(&mut self, binding: &Ident, body: &Block, span: Span) -> Result<()> {
        if self.cx.reach == Reach::Called {
            return Err(unsupported(span, "tasks (`concurrent.futures`)"));
        }
        self.write(&binding.name, Operand::None, binding.span)?;
        self.block(body)
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
        match (self.node_type(value.span)?, attr) {
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
        Ok(self.emit_call(def, slots, node, span)?.0)
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
        let Type::Results(t) = self.node_type(span)? else {
            return Err(internal(span, "`map` gives no results"));
        };
        let elem = super::settle(t, &self.subst, &self.dead, &mut self.cx.seqs);
        Ok(self.cx.seqs.list(elem))
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
        let list = Operand::Local(out);
        self.emit(Inst::Append { list, value, span });
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

    /// [`FnBuilder::took_result`] where `e` is `map`'s results.
    pub(super) fn reads_results(&mut self, e: &Expr) -> Result<()> {
        match self.node_type(e.span)? {
            Type::Results(_) => self.took_result(e.span),
            _ => Ok(()),
        }
    }
}
