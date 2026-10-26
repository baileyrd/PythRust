//! Tree-walking interpreter: `ast::Module` to program output. Thread D.
//!
//! The semantic reference, not the fast path. Runs untyped AST, so every
//! type error the checker would catch is also a runtime error here.
//! Runtime errors are uncatchable in phase 2: the first one stops the
//! program (see `docs/grammar.md`, Runtime errors).

mod builtins;
mod ops;
mod value;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::iter;
use std::rc::Rc;

use crate::ast::{
    Arg, BinOp, Block, CmpOp, Expr, ExprKind, FuncDef, Ident, LogicalOp, Module, Stmt, StmtKind,
    Target, TargetKind,
};
use crate::diag::{DiagKind, Diagnostic, Result};
use crate::span::Span;
use builtins::{Args, Builtin};
use value::{Func, Value};

/// Python's default recursion limit.
pub(crate) const MAX_DEPTH: usize = 1000;

/// Stack for the interpreter thread. A Python call costs several Rust
/// frames (up to ~16 KiB in debug builds), so `MAX_DEPTH` calls need far
/// more than a default thread's 2-8 MiB. The pages are reserved, not
/// committed, so the unused part costs nothing.
const STACK_BYTES: usize = 256 << 20;

/// Runs `module`, writing everything it prints to `out`.
///
/// Output written before a runtime error stays written, as in CPython.
/// The error is returned as a `DiagKind::Runtime` diagnostic; the caller
/// reports it and exits with status 1. Runs on a dedicated thread with a
/// large stack, so hitting the recursion limit is an error, not a crash.
pub fn run(module: &Module, out: &mut (dyn Write + Send)) -> Result<()> {
    std::thread::scope(|scope| {
        let worker = std::thread::Builder::new()
            .name("pythrust-interp".to_string())
            .stack_size(STACK_BYTES)
            .spawn_scoped(scope, || run_here(module, out))
            .map_err(|e| {
                rt_err(
                    module.span,
                    format!("cannot start the interpreter thread: {e}"),
                )
            })?;
        // A panic is an interpreter bug: re-raise it rather than disguise it.
        worker
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    })
}

fn run_here(module: &Module, out: &mut dyn Write) -> Result<()> {
    let mut interp = Interp {
        globals: HashMap::new(),
        frames: Vec::new(),
        out,
    };
    let flow = interp.exec_block(&module.body)?;
    match unlooped(flow)? {
        Some((_, span)) => Err(rt_err(
            span,
            "SyntaxError: 'return' outside function".to_string(),
        )),
        None => Ok(()),
    }
}

/// Builds a runtime diagnostic.
fn rt_err(span: Span, message: String) -> Diagnostic {
    Diagnostic {
        span,
        message,
        kind: DiagKind::Runtime,
    }
}

struct Interp<'m, 'o> {
    globals: HashMap<&'m str, Value<'m>>,
    /// One per active call; empty at module level.
    frames: Vec<Frame<'m>>,
    out: &'o mut dyn Write,
}

struct Frame<'m> {
    func: Rc<Func<'m>>,
    vars: HashMap<&'m str, Value<'m>>,
}

/// How a statement finished.
enum Flow<'m> {
    Next,
    Break(Span),
    Continue(Span),
    Return(Value<'m>, Span),
}

/// What a loop does with its body's flow: `Some` exits the loop with it.
fn loop_exit(flow: Flow) -> Option<Flow> {
    match flow {
        Flow::Break(_) => Some(Flow::Next),
        Flow::Return(..) => Some(flow),
        Flow::Next | Flow::Continue(_) => None,
    }
}

/// The flow left once no loop remains to catch it: a stray `break` or
/// `continue` is an error; a `return` yields its value and span.
fn unlooped(flow: Flow) -> Result<Option<(Value, Span)>> {
    match flow {
        Flow::Next => Ok(None),
        Flow::Return(v, span) => Ok(Some((v, span))),
        Flow::Break(span) => Err(rt_err(
            span,
            "SyntaxError: 'break' outside loop".to_string(),
        )),
        Flow::Continue(span) => Err(rt_err(
            span,
            "SyntaxError: 'continue' not properly in loop".to_string(),
        )),
    }
}

impl<'m> Interp<'m, '_> {
    fn exec_block(&mut self, block: &'m [Stmt]) -> Result<Flow<'m>> {
        for stmt in block {
            let flow = self.exec(stmt)?;
            if !matches!(flow, Flow::Next) {
                return Ok(flow);
            }
        }
        Ok(Flow::Next)
    }

    fn exec(&mut self, stmt: &'m Stmt) -> Result<Flow<'m>> {
        match &stmt.kind {
            StmtKind::Assign { target, value } => {
                let v = self.eval(value)?;
                self.assign(target, v)?;
            }
            StmtKind::AnnAssign { target, value, .. } => {
                let v = self.eval(value)?;
                self.store(&target.name, v);
            }
            StmtKind::AugAssign { target, op, value } => {
                self.aug_assign(target, *op, value, stmt.span)?
            }
            StmtKind::Return(value) => {
                let v = value.as_ref().map_or(Ok(Value::None), |e| self.eval(e))?;
                return Ok(Flow::Return(v, stmt.span));
            }
            StmtKind::Break => return Ok(Flow::Break(stmt.span)),
            StmtKind::Continue => return Ok(Flow::Continue(stmt.span)),
            StmtKind::Pass => {}
            StmtKind::Expr(e) => {
                self.eval(e)?;
            }
            StmtKind::FuncDef(def) => self.define(def)?,
            StmtKind::If {
                branch,
                elifs,
                orelse,
            } => {
                for b in iter::once(branch).chain(elifs) {
                    if self.eval(&b.test)?.truthy() {
                        return self.exec_block(&b.body);
                    }
                }
                return self.exec_block(orelse);
            }
            StmtKind::While { branch, orelse } => {
                return self.exec_while(&branch.test, &branch.body, orelse)
            }
            StmtKind::For {
                target,
                iter,
                body,
                orelse,
            } => return self.exec_for(target, iter, body, orelse),
        }
        Ok(Flow::Next)
    }

    fn exec_while(
        &mut self,
        test: &'m Expr,
        body: &'m Block,
        orelse: &'m Block,
    ) -> Result<Flow<'m>> {
        while self.eval(test)?.truthy() {
            if let Some(flow) = loop_exit(self.exec_block(body)?) {
                return Ok(flow);
            }
        }
        self.exec_block(orelse)
    }

    fn exec_for(
        &mut self,
        target: &'m Target,
        iter: &'m Expr,
        body: &'m Block,
        orelse: &'m Block,
    ) -> Result<Flow<'m>> {
        for v in self.eval(iter)?.iter(iter.span)? {
            self.assign(target, v)?;
            if let Some(flow) = loop_exit(self.exec_block(body)?) {
                return Ok(flow);
            }
        }
        self.exec_block(orelse)
    }

    /// `def`: evaluates defaults now, as Python does, and binds the name.
    fn define(&mut self, def: &'m FuncDef) -> Result<()> {
        if !self.frames.is_empty() {
            return Err(rt_err(
                def.span,
                "SyntaxError: 'def' is only allowed at module top level".to_string(),
            ));
        }
        let defaults = def
            .params
            .iter()
            .map(|p| p.default.as_ref().map(|e| self.eval(e)).transpose())
            .collect::<Result<_>>()?;
        let func = Func {
            def,
            defaults,
            locals: locals_of(def),
        };
        self.store(&def.name.name, Value::Func(Rc::new(func)));
        Ok(())
    }

    fn assign(&mut self, target: &'m Target, v: Value<'m>) -> Result<()> {
        match &target.kind {
            TargetKind::Name(name) => {
                self.store(name, v);
                Ok(())
            }
            TargetKind::Subscript { value, index } => {
                let container = self.eval(value)?;
                let index = self.eval(index)?;
                set_item(&container, &index, v, target.span)
            }
            TargetKind::Tuple(targets) => self.unpack(targets, v, target.span),
        }
    }

    fn unpack(&mut self, targets: &'m [Target], v: Value<'m>, span: Span) -> Result<()> {
        // Take one extra to detect "too many" without draining a long range.
        let values: Vec<Value> = v.iter(span)?.take(targets.len() + 1).collect();
        if values.len() != targets.len() {
            let (want, got) = (targets.len(), values.len());
            let message = if got > want {
                format!("ValueError: too many values to unpack (expected {want})")
            } else {
                format!("ValueError: not enough values to unpack (expected {want}, got {got})")
            };
            return Err(rt_err(span, message));
        }
        targets
            .iter()
            .zip(values)
            .try_for_each(|(t, v)| self.assign(t, v))
    }

    /// `target op= value`. Evaluation order follows Python: the target's
    /// container and index once, then the value. `list +=` extends in
    /// place, so aliases see the change.
    fn aug_assign(
        &mut self,
        target: &'m Target,
        op: BinOp,
        value: &'m Expr,
        span: Span,
    ) -> Result<()> {
        match &target.kind {
            TargetKind::Name(name) => {
                let current = self.lookup(name, target.span)?;
                let rhs = self.eval(value)?;
                let new = in_place(op, current, &rhs, span)?;
                self.store(name, new);
                Ok(())
            }
            TargetKind::Subscript {
                value: container,
                index,
            } => {
                let container = self.eval(container)?;
                let index = self.eval(index)?;
                let current = get_item(&container, &index, target.span)?;
                let rhs = self.eval(value)?;
                let new = in_place(op, current, &rhs, span)?;
                set_item(&container, &index, new, target.span)
            }
            TargetKind::Tuple(_) => Err(rt_err(
                target.span,
                "SyntaxError: illegal expression for augmented assignment".to_string(),
            )),
        }
    }

    fn store(&mut self, name: &'m str, v: Value<'m>) {
        match self.frames.last_mut() {
            Some(frame) => frame.vars.insert(name, v),
            None => self.globals.insert(name, v),
        };
    }

    /// Python's scoping without `global`: a name a function assigns is
    /// local throughout it; any other name is global, then builtin.
    fn lookup(&self, name: &str, span: Span) -> Result<Value<'m>> {
        if let Some(frame) = self.frames.last().filter(|f| f.func.locals.contains(name)) {
            return frame.vars.get(name).cloned().ok_or_else(|| {
                rt_err(
                    span,
                    format!("UnboundLocalError: cannot access local variable '{name}' where it is not associated with a value"),
                )
            });
        }
        if let Some(v) = self.globals.get(name) {
            return Ok(v.clone());
        }
        Builtin::lookup(name)
            .map(Value::Builtin)
            .ok_or_else(|| rt_err(span, format!("NameError: name '{name}' is not defined")))
    }

    fn eval(&mut self, e: &'m Expr) -> Result<Value<'m>> {
        match &e.kind {
            ExprKind::Name(name) => self.lookup(name, e.span),
            ExprKind::Int(i) => Ok(Value::Int(*i)),
            ExprKind::Float(f) => Ok(Value::Float(*f)),
            ExprKind::Str(s) => Ok(Value::str(s)),
            ExprKind::Bool(b) => Ok(Value::Bool(*b)),
            ExprKind::None => Ok(Value::None),
            ExprKind::Tuple(items) => Ok(Value::Tuple(self.eval_all(items)?.into())),
            ExprKind::List(items) => Ok(Value::list(self.eval_all(items)?)),
            ExprKind::Unary { op, operand } => ops::unary(*op, &self.eval(operand)?, e.span),
            ExprKind::Binary { op, left, right } => {
                let l = self.eval(left)?;
                let r = self.eval(right)?;
                ops::binary(*op, &l, &r, e.span)
            }
            ExprKind::Logical { op, left, right } => {
                let l = self.eval(left)?;
                let short = match op {
                    LogicalOp::And => !l.truthy(),
                    LogicalOp::Or => l.truthy(),
                };
                if short {
                    return Ok(l);
                }
                self.eval(right)
            }
            ExprKind::Compare { left, rest } => self.eval_compare(left, rest),
            ExprKind::IfExp { test, body, orelse } => {
                let branch = if self.eval(test)?.truthy() {
                    body
                } else {
                    orelse
                };
                self.eval(branch)
            }
            ExprKind::Call { func, args } => self.eval_call(func, args, e.span),
            ExprKind::Subscript { value, index } => {
                let container = self.eval(value)?;
                let index = self.eval(index)?;
                get_item(&container, &index, e.span)
            }
            ExprKind::Attr { value, attr } => get_attr(self.eval(value)?, attr),
        }
    }

    fn eval_all(&mut self, items: &'m [Expr]) -> Result<Vec<Value<'m>>> {
        items.iter().map(|e| self.eval(e)).collect()
    }

    /// `a < b < c`: each operand is evaluated once, and evaluation stops at
    /// the first false link. A failing link's span covers its two operands.
    fn eval_compare(&mut self, left: &'m Expr, rest: &'m [(CmpOp, Expr)]) -> Result<Value<'m>> {
        let mut l = self.eval(left)?;
        let mut l_span = left.span;
        for (op, right) in rest {
            let r = self.eval(right)?;
            if !value::compare(*op, &l, &r, l_span.merge(right.span))? {
                return Ok(Value::Bool(false));
            }
            (l, l_span) = (r, right.span);
        }
        Ok(Value::Bool(true))
    }

    fn eval_call(&mut self, func: &'m Expr, args: &'m [Arg], span: Span) -> Result<Value<'m>> {
        let callee = self.eval(func)?;
        let mut evaluated = Args {
            positional: Vec::new(),
            keywords: Vec::new(),
            span,
        };
        for arg in args {
            let v = self.eval(&arg.value)?;
            match &arg.name {
                Some(name) => evaluated.keywords.push((name, v)),
                None => evaluated.positional.push(v),
            }
        }
        match callee {
            Value::Func(f) => self.call_func(f, evaluated),
            Value::Builtin(b) => builtins::call(b, evaluated, self.out),
            Value::Append(list) => builtins::append(&list, evaluated),
            other => Err(rt_err(
                span,
                format!("TypeError: '{}' object is not callable", other.type_name()),
            )),
        }
    }

    fn call_func(&mut self, func: Rc<Func<'m>>, args: Args<'m>) -> Result<Value<'m>> {
        if self.frames.len() >= MAX_DEPTH {
            return Err(rt_err(
                args.span,
                "RecursionError: maximum recursion depth exceeded".to_string(),
            ));
        }
        let vars = bind(&func, args)?;
        let body = &func.def.body;
        self.frames.push(Frame {
            func: Rc::clone(&func),
            vars,
        });
        let flow = self.exec_block(body);
        self.frames.pop();
        Ok(unlooped(flow?)?.map_or(Value::None, |(v, _)| v))
    }
}

/// Binds call arguments to parameters, as Python does: positional first,
/// then keywords by name, then defaults.
fn bind<'m>(func: &Func<'m>, args: Args<'m>) -> Result<HashMap<&'m str, Value<'m>>> {
    let (params, fname) = (&func.def.params, &func.def.name.name);
    if args.positional.len() > params.len() {
        return Err(rt_err(
            args.span,
            format!(
                "TypeError: {fname}() takes {} positional arguments but {} were given",
                params.len(),
                args.positional.len()
            ),
        ));
    }
    let mut slots: Vec<Option<Value>> = vec![None; params.len()];
    for (slot, v) in slots.iter_mut().zip(args.positional) {
        *slot = Some(v);
    }
    for (name, v) in args.keywords {
        bind_keyword(func, &mut slots, name, v)?;
    }
    let mut vars = HashMap::with_capacity(params.len());
    for ((param, slot), default) in params.iter().zip(slots).zip(&func.defaults) {
        let v = slot.or_else(|| default.clone()).ok_or_else(|| {
            rt_err(
                args.span,
                format!(
                    "TypeError: {fname}() missing required argument: '{}'",
                    param.name.name
                ),
            )
        })?;
        vars.insert(param.name.name.as_str(), v);
    }
    Ok(vars)
}

fn bind_keyword<'m>(
    func: &Func<'m>,
    slots: &mut [Option<Value<'m>>],
    name: &Ident,
    v: Value<'m>,
) -> Result<()> {
    let fname = &func.def.name.name;
    let slot = func
        .def
        .params
        .iter()
        .position(|p| p.name.name == name.name)
        .and_then(|i| slots.get_mut(i))
        .ok_or_else(|| {
            rt_err(
                name.span,
                format!(
                    "TypeError: {fname}() got an unexpected keyword argument '{}'",
                    name.name
                ),
            )
        })?;
    if slot.is_some() {
        return Err(rt_err(
            name.span,
            format!(
                "TypeError: {fname}() got multiple values for argument '{}'",
                name.name
            ),
        ));
    }
    *slot = Some(v);
    Ok(())
}

/// Names a function binds: its parameters and every assignment target.
fn locals_of(def: &FuncDef) -> HashSet<&str> {
    let mut names: HashSet<&str> = def.params.iter().map(|p| p.name.name.as_str()).collect();
    collect_block(&def.body, &mut names);
    names
}

fn collect_block<'m>(block: &'m [Stmt], names: &mut HashSet<&'m str>) {
    for stmt in block {
        match &stmt.kind {
            StmtKind::Assign { target, .. } | StmtKind::AugAssign { target, .. } => {
                collect_target(target, names)
            }
            StmtKind::AnnAssign { target, .. } => {
                names.insert(&target.name);
            }
            StmtKind::For {
                target,
                body,
                orelse,
                ..
            } => {
                collect_target(target, names);
                collect_block(body, names);
                collect_block(orelse, names);
            }
            StmtKind::If {
                branch,
                elifs,
                orelse,
            } => {
                iter::once(branch)
                    .chain(elifs)
                    .for_each(|b| collect_block(&b.body, names));
                collect_block(orelse, names);
            }
            StmtKind::While { branch, orelse } => {
                collect_block(&branch.body, names);
                collect_block(orelse, names);
            }
            _ => {}
        }
    }
}

fn collect_target<'m>(target: &'m Target, names: &mut HashSet<&'m str>) {
    match &target.kind {
        TargetKind::Name(name) => {
            names.insert(name);
        }
        TargetKind::Tuple(targets) => targets.iter().for_each(|t| collect_target(t, names)),
        TargetKind::Subscript { .. } => {}
    }
}

/// `op=` semantics: `list +=` and `list *=` change the list itself, so
/// aliases see it; everything else is the plain binary operator.
fn in_place<'m>(op: BinOp, current: Value<'m>, rhs: &Value<'m>, span: Span) -> Result<Value<'m>> {
    let Value::List(list) = &current else {
        return ops::binary(op, &current, rhs, span);
    };
    match op {
        BinOp::Add => {
            // Collect first: `xs += xs` reads the list it extends.
            let items: Vec<Value> = rhs.iter(span)?.collect();
            list.borrow_mut().extend(items);
        }
        BinOp::Mul => {
            if let Value::List(repeated) = ops::binary(op, &current, rhs, span)? {
                list.swap(&repeated);
            }
        }
        _ => return ops::binary(op, &current, rhs, span),
    }
    Ok(current)
}

fn get_attr<'m>(v: Value<'m>, attr: &Ident) -> Result<Value<'m>> {
    match v {
        Value::List(list) if attr.name == "append" => Ok(Value::Append(list)),
        other => Err(rt_err(
            attr.span,
            format!(
                "AttributeError: '{}' object has no attribute '{}'",
                other.type_name(),
                attr.name
            ),
        )),
    }
}

fn get_item<'m>(container: &Value<'m>, index: &Value<'m>, span: Span) -> Result<Value<'m>> {
    match container {
        Value::List(list) => {
            let list = list.borrow();
            Ok(list[position(index, list.len(), "list", span)?].clone())
        }
        Value::Tuple(items) => Ok(items[position(index, items.len(), "tuple", span)?].clone()),
        Value::Str(s) => {
            let i = position(index, s.chars().count(), "string", span)?;
            let c: String = s.chars().skip(i).take(1).collect();
            Ok(Value::str(&c))
        }
        Value::Range(r) => {
            let len = usize::try_from(r.len()).unwrap_or(usize::MAX);
            let i = position(index, len, "range object", span)?;
            let v =
                i128::from(r.start) + i128::try_from(i).unwrap_or(i128::MAX) * i128::from(r.step);
            // `i < len`, so `v` lies between start and stop and fits in i64.
            i64::try_from(v)
                .map(Value::Int)
                .map_err(|_| ops::overflow(span))
        }
        other => Err(rt_err(
            span,
            format!(
                "TypeError: '{}' object is not subscriptable",
                other.type_name()
            ),
        )),
    }
}

fn set_item<'m>(container: &Value<'m>, index: &Value<'m>, v: Value<'m>, span: Span) -> Result<()> {
    let Value::List(list) = container else {
        return Err(rt_err(
            span,
            format!(
                "TypeError: '{}' object does not support item assignment",
                container.type_name()
            ),
        ));
    };
    let i = position(index, list.borrow().len(), "list", span)?;
    list.borrow_mut()[i] = v;
    Ok(())
}

/// Resolves a Python index (negative counts from the end) into `0..len`.
fn position(index: &Value, len: usize, kind: &str, span: Span) -> Result<usize> {
    let i = index.int().ok_or_else(|| {
        rt_err(
            span,
            format!(
                "TypeError: {kind} indices must be integers, not {}",
                index.type_name()
            ),
        )
    })?;
    let len_i = i128::try_from(len).unwrap_or(i128::MAX);
    let resolved = if i < 0 {
        i128::from(i) + len_i
    } else {
        i128::from(i)
    };
    usize::try_from(resolved)
        .ok()
        .filter(|&r| r < len)
        .ok_or_else(|| rt_err(span, format!("IndexError: {kind} index out of range")))
}
