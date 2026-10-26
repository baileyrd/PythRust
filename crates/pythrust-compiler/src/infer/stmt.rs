//! Statements and assignment targets.

use crate::ast::{BinOp, Block, Branch, Expr, Stmt, StmtKind, Target, TargetKind};
use crate::diag::Result;
use crate::span::Span;

use super::expr::int_literal;
use super::scope;
use super::solve::{self, Access, Binary, Need};
use super::ty::Type;
use super::{error, Checker};

impl Checker {
    /// Infers `block` in order. After a guard (`if x is None: return`),
    /// the rest of the block sees `x` narrowed.
    pub(super) fn block(&mut self, block: &[Stmt]) -> Result<()> {
        for (i, s) in block.iter().enumerate() {
            self.stmt(s)?;
            self.settle()?;
            let Some(name) = scope::guard(s) else {
                continue;
            };
            let rest = &block[i + 1..];
            return self.narrowed(&[name], rest, |c| c.block(rest));
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
                self.branch(branch, &[])?;
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
        }
    }

    /// `x is None` tests narrow later clauses; `x is not None` narrows
    /// its own body.
    fn if_chain(&mut self, branch: &Branch, elifs: &[Branch], orelse: &Block) -> Result<()> {
        let mut not_none: Vec<String> = Vec::new();
        for b in std::iter::once(branch).chain(elifs) {
            self.branch(b, &not_none)?;
            if let Some((name, false)) = scope::none_test(&b.test) {
                not_none.push(name);
            }
        }
        self.narrowed(&not_none, orelse, |c| c.block(orelse))
    }

    /// A test of any type (every value has a truth value) and its block.
    /// `not_none` names are narrowed in both, as is the name the test
    /// itself proves is not `None`.
    fn branch(&mut self, b: &Branch, not_none: &[String]) -> Result<()> {
        self.narrowed(not_none, &[], |c| c.expr(&b.test).map(drop))?;
        let mut inside = not_none.to_vec();
        if let Some((name, true)) = scope::none_test(&b.test) {
            inside.push(name);
        }
        self.narrowed(&inside, &b.body, |c| c.block(&b.body))
    }

    /// Runs `f` with each of `names` narrowed from `T | None` to `T`,
    /// unless `scope` assigns it (then its type stays `T | None`).
    fn narrowed(
        &mut self,
        names: &[String],
        scope: &[Stmt],
        f: impl FnOnce(&mut Self) -> Result<()>,
    ) -> Result<()> {
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
