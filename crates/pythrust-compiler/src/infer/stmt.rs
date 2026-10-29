//! Statements and assignment targets.

use std::collections::BTreeSet;

use crate::ast::{
    BinOp, Block, Branch, Expr, ExprKind, Handler, Stmt, StmtKind, Target, TargetKind,
};
use crate::diag::{Diagnostic, Result};
use crate::span::Span;

use super::expr::int_literal;
use super::solve::{self, Access, Binary, Constraint, Need};
use super::ty::Type;
use super::{builtins, scope};
use super::{error, Checker, TruthUse};

const TRUTH_HINT: &str = "a truth test does not narrow";

impl Checker {
    /// Infers `block` in order. After a guard (`if x is None: return`),
    /// the rest of the block sees `x` narrowed.
    pub(super) fn block(&mut self, block: &[Stmt]) -> Result<()> {
        for (i, s) in block.iter().enumerate() {
            self.stmt(s)?;
            self.settle()?;
            let names = scope::guard(s);
            if names.is_empty() {
                continue;
            }
            let rest = &block[i + 1..];
            return self.narrowed(&names, rest, |c| c.block(rest));
        }
        Ok(())
    }

    pub(super) fn stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match &stmt.kind {
            StmtKind::Assign { target, value } => {
                let t = match self.declared(target) {
                    Some(current) => self.checked(value, &current)?,
                    None => self.expr(value)?,
                };
                self.bind(target, &t)
            }
            StmtKind::AnnAssign {
                target,
                annotation,
                value,
            } => {
                let declared = self.annotation(annotation)?;
                let t = self.checked(value, &declared)?;
                self.expect(
                    value.span,
                    &declared,
                    &t,
                    "value does not match its annotation",
                )?;
                self.bind_name(&target.name, target.span, &declared)
            }
            StmtKind::AugAssign { target, op, value } => {
                self.aug_assign(target, *op, value, stmt.span)
            }
            StmtKind::Return(value) => self.ret(value.as_ref(), stmt.span),
            StmtKind::Break | StmtKind::Continue | StmtKind::Pass => Ok(()),
            StmtKind::Expr(e) => self.expr(e).map(drop),
            StmtKind::FuncDef(f) => Err(error(
                f.name.span,
                "`def` is only allowed at module top level".to_string(),
            )),
            StmtKind::If {
                branch,
                elifs,
                orelse,
            } => self.if_chain(branch, elifs, orelse),
            StmtKind::While { branch, orelse } => {
                self.branch(branch, &BTreeSet::new())?;
                self.block(orelse)
            }
            StmtKind::For {
                target,
                iter,
                body,
                orelse,
            } => {
                let iterable = self.expr(iter)?;
                let elem = self.subst.fresh();
                let need = Need::Iter {
                    iterable,
                    elem: elem.clone(),
                };
                self.need(need, iter.span);
                self.bind(target, &elem)?;
                self.block(body)?;
                self.block(orelse)
            }
            StmtKind::Raise(value) => self.raise(value),
            StmtKind::Try { body, handlers } => {
                self.block(body)?;
                handlers.iter().try_for_each(|h| self.handler(h))
            }
        }
    }

    /// `raise e`, or `raise ValueError` for `raise ValueError()`.
    fn raise(&mut self, value: &Expr) -> Result<()> {
        if let ExprKind::Name(name) = &value.kind {
            if self.is_exception_name(name) {
                self.nodes.insert(value.span, Type::Exception);
                return Ok(());
            }
        }
        let t = self.expr(value)?;
        let what = "`raise` needs an exception, such as `ValueError(\"...\")`";
        self.expect(value.span, &Type::Exception, &t, what)
    }

    /// `except Name [as e]:` and its block. `Name` must be a builtin
    /// exception class that nothing shadows; `e` is an `Exception`.
    fn handler(&mut self, h: &Handler) -> Result<()> {
        let name = &h.exception.name;
        if !self.is_exception_name(name) {
            return Err(error(h.exception.span, not_an_exception(name)));
        }
        if let Some(b) = &h.binding {
            self.bind_name(&b.name, b.span, &Type::Exception)?;
            self.nodes.insert(b.span, Type::Exception);
        }
        self.block(&h.body)
    }

    /// Whether `name` here is a builtin exception: one of D6's names that
    /// no variable or `def` shadows.
    pub(super) fn is_exception_name(&self, name: &str) -> bool {
        builtins::is_exception(name)
            && self.lookup(name).is_none()
            && !self.funcs.contains_key(name)
    }

    /// A clause's test narrows its body by holding and the later
    /// clauses by failing (`x is not None`, `x is None`).
    fn if_chain(&mut self, branch: &Branch, elifs: &[Branch], orelse: &Block) -> Result<()> {
        let mut not_none: BTreeSet<String> = BTreeSet::new();
        for b in std::iter::once(branch).chain(elifs) {
            self.branch(b, &not_none)?;
            not_none.extend(scope::not_none(&b.test, false));
        }
        self.narrowed(&not_none, orelse, |c| c.block(orelse))
    }

    /// A test of any type (every value has a truth value) and its block.
    /// `not_none` names are narrowed in both, as is the name the test
    /// itself proves is not `None`.
    fn branch(&mut self, b: &Branch, not_none: &BTreeSet<String>) -> Result<()> {
        self.narrowed(not_none, &[], |c| c.expr(&b.test).map(drop))?;
        let mut inside = not_none.clone();
        inside.extend(scope::not_none(&b.test, true));
        // Names the test checks only for truth, unless the body assigns
        // them: `is not None` would not narrow those either.
        let assigned = scope::assigned(&b.body);
        let tested = scope::truth_tested(&b.test);
        let saved = (self.frame.truthy.clone(), self.frame.truth_uses.len());
        let fixable = tested.into_iter().filter(|n| !assigned.contains(n));
        self.frame.truthy.extend(fixable);
        let result = self
            .narrowed(&inside, &b.body, |c| c.block(&b.body))
            .map_err(|e| self.truth_hint(e));
        self.frame.truthy = saved.0;
        self.frame.truth_uses.truncate(saved.1);
        result
    }

    /// Adds a fix-it to `err` when it comes from an operation on a name
    /// tested only for truth, and narrowing that name would fix it.
    fn truth_hint(&self, mut err: Diagnostic) -> Diagnostic {
        if err.message.contains(TRUTH_HINT) {
            return err;
        }
        let fixes = |u: &&TruthUse| {
            let narrowed = Constraint {
                need: u.narrowed.clone(),
                span: u.span,
                via: None,
            };
            let solved = solve::solve(&mut self.subst.clone(), vec![narrowed]);
            u.span == err.span && matches!(solved, Ok(rest) if rest.is_empty())
        };
        let Some(TruthUse { name, .. }) = self.frame.truth_uses.iter().find(fixes) else {
            return err;
        };
        let hint = format!("; {TRUTH_HINT} `{name}`: write `{name} is not None`");
        err.message.push_str(&hint);
        err
    }

    /// Runs `f` with each of `names` narrowed from `T | None` to `T`,
    /// unless `scope` assigns it (then its type stays `T | None`).
    pub(super) fn narrowed<T>(
        &mut self,
        names: &BTreeSet<String>,
        scope: &[Stmt],
        f: impl FnOnce(&mut Self) -> Result<T>,
    ) -> Result<T> {
        // Solve first: a name's `T | None` may still sit in a pending
        // constraint (a `for` target's element type).
        self.settle()?;
        let assigned = scope::assigned(scope);
        let saved = self.frame.narrowed.clone();
        for name in names.iter().filter(|n| !assigned.contains(*n)) {
            let current = self
                .frame
                .locals
                .get(name)
                .or_else(|| self.globals.get(name));
            if let Some(Type::Optional(inner)) = current.map(|t| self.subst.resolve(t)) {
                self.frame.narrowed.insert(name.clone(), *inner);
            }
        }
        let result = f(self);
        self.frame.narrowed = saved;
        result
    }

    fn aug_assign(&mut self, target: &Target, op: BinOp, value: &Expr, span: Span) -> Result<()> {
        let current = match &target.kind {
            TargetKind::Name(name) => self.variable(name, target.span)?,
            TargetKind::Subscript { value, index } => {
                let out = self.subst.fresh();
                self.subscript(value, index, Access::ReadWrite, out)?
            }
            TargetKind::Tuple(_) => {
                let msg = "augmented assignment needs a single target".to_string();
                return Err(error(target.span, msg));
            }
        };
        let right = self.expr(value)?;
        let assigns = match &target.kind {
            TargetKind::Name(name) => Some(name.clone()),
            _ => None,
        };
        let need = Need::Binary(Binary {
            op,
            left: current.clone(),
            right,
            out: current,
            right_literal: int_literal(value),
            assigns,
        });
        self.need(need, span);
        Ok(())
    }

    fn ret(&mut self, value: Option<&Expr>, span: Span) -> Result<()> {
        let Some(expected) = self.frame.ret.clone() else {
            return Err(error(span, "`return` outside a function".to_string()));
        };
        let found = match value {
            Some(e) => self.checked(e, &expected)?,
            None => Type::None,
        };
        self.expect(span, &expected, &found, "return type mismatch")
    }

    /// Assigns a value of type `ty` to `target`.
    pub(super) fn bind(&mut self, target: &Target, ty: &Type) -> Result<()> {
        match &target.kind {
            TargetKind::Name(name) => self.bind_name(name, target.span, ty),
            TargetKind::Subscript { value, index } => {
                self.subscript(value, index, Access::Write, ty.clone())?;
                Ok(())
            }
            TargetKind::Tuple(items) => {
                let elems: Vec<Type> = items.iter().map(|_| self.subst.fresh()).collect();
                let need = Need::Unpack {
                    value: ty.clone(),
                    elems: elems.clone(),
                };
                self.need(need, target.span);
                items
                    .iter()
                    .zip(&elems)
                    .try_for_each(|(item, elem)| self.bind(item, elem))
            }
        }
    }

    /// The type a target already has, so a literal assigned to it is
    /// checked against it (`xs = [None]` on a `list[int | None]`, or
    /// `xss[0] = [None, 1]` on a `list[list[int | None]]`).
    fn declared(&self, target: &Target) -> Option<Type> {
        match &target.kind {
            TargetKind::Name(name) => self.lookup(name),
            TargetKind::Subscript { value, .. } => self.element_of(value),
            TargetKind::Tuple(_) => None,
        }
    }

    fn bind_name(&mut self, name: &str, span: Span, ty: &Type) -> Result<()> {
        let var = self.variable(name, span)?;
        let what = format!("cannot change the type of `{name}`");
        let deferred = self.subst.assign(&var, ty).map_err(|clash| {
            let detail = solve::mismatch(&self.subst, clash, &var, ty);
            let mut msg = format!("{what}: {detail}");
            let (was, now) = (self.subst.zonk(&var), self.subst.zonk(ty));
            let other = match (&was, &now) {
                (Type::None, t) | (t, Type::None) => Some(t),
                _ => None,
            };
            if let Some(t) = other {
                msg.push_str(&format!(
                    "; to allow None, annotate it, as in `{name}: {t} | None = None`"
                ));
            }
            error(span, msg)
        })?;
        for need in solve::assigns(deferred, &what) {
            self.need(need, span);
        }
        Ok(())
    }
}

/// Why `except name` names no builtin exception: shadowed, or unknown.
fn not_an_exception(name: &str) -> String {
    if builtins::is_exception(name) {
        return format!("`{name}` is redefined here, so it no longer names the builtin exception");
    }
    let names = builtins::exception_names();
    format!("`{name}` is not an exception; the builtin exceptions are {names}")
}
