//! Tasks (M6c), for analysis only: `build` refuses the `with` that
//! makes an executor until native tasks land (`docs/native.md`).
//!
//! They lower as `run` runs them: each task to completion where it is
//! submitted, a call of its function there. A future is its task's
//! result, and `map`'s results are a list of them ([`super::concrete`]),
//! so `fut.result()` reads the value. A task's exception leaves at the
//! call, where `run` keeps it until the result is taken; the summaries
//! analysis builds from loose bodies do not depend on which, and
//! tempered code uses no executor.

use crate::ast::{Arg, Block, Expr, ExprKind, FuncDef, Ident};
use crate::diag::Result;
use crate::infer::Type;
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
            (Type::Future(_), "result") => self.expr(value).map(Some),
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
        self.call_def(def, slots, node, span)
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
        let looped = Looped::Map {
            f: &def.name.name,
            out,
            span,
        };
        self.for_value(&looped, s, xs.value.span)?;
        let ty = self.ty_at(span)?;
        self.locals[out.0] = ty;
        if let Some(pool) = self.pools.last_mut() {
            pool.push(out);
        }
        let make = Inst::Assign {
            dest: out,
            value: Rvalue::NewSeq(Vec::new()),
            span,
        };
        self.blocks[before.0].insts.push(make);
        Ok(Operand::Local(out))
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
        let value = self.call_def(def, slots, node, span)?;
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
}
