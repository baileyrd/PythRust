//! Statements and assignment targets.

use crate::ast::{BinOp, Block, Branch, Expr, Stmt, StmtKind, Target, TargetKind};
use crate::diag::Result;
use crate::span::Span;

use super::expr::int_literal;
use super::solve::{self, Binary, Need};
use super::ty::Type;
use super::{error, Checker};

impl Checker {
    pub(super) fn block(&mut self, block: &Block) -> Result<()> {
        for s in block {
            self.stmt(s)?;
            self.settle()?;
        }
        Ok(())
    }

    pub(super) fn stmt(&mut self, stmt: &Stmt) -> Result<()> {
        match &stmt.kind {
            StmtKind::Assign { target, value } => {
                let t = self.expr(value)?;
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
            } => {
                std::iter::once(branch)
                    .chain(elifs)
                    .try_for_each(|b| self.branch(b))?;
                self.block(orelse)
            }
            StmtKind::While { branch, orelse } => {
                self.branch(branch)?;
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

    /// A test of any type (every value has a truth value) and its block.
    fn branch(&mut self, b: &Branch) -> Result<()> {
        self.expr(&b.test)?;
        self.block(&b.body)
    }

    fn aug_assign(&mut self, target: &Target, op: BinOp, value: &Expr, span: Span) -> Result<()> {
        let current = match &target.kind {
            TargetKind::Name(name) => self.variable(name, target.span)?,
            TargetKind::Subscript { value, index } => self.subscript(value, index, true)?,
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
            Some(e) => self.expr(e)?,
            None => Type::None,
        };
        self.expect(span, &expected, &found, "return type mismatch")
    }

    /// Assigns a value of type `ty` to `target`.
    pub(super) fn bind(&mut self, target: &Target, ty: &Type) -> Result<()> {
        match &target.kind {
            TargetKind::Name(name) => self.bind_name(name, target.span, ty),
            TargetKind::Subscript { value, index } => {
                let slot = self.subscript(value, index, true)?;
                self.expect(target.span, &slot, ty, "assigned value")
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

    fn bind_name(&mut self, name: &str, span: Span, ty: &Type) -> Result<()> {
        let var = self.variable(name, span)?;
        self.subst.assign(&var, ty).map_err(|clash| {
            let detail = solve::mismatch(&self.subst, clash, &var, ty);
            let mut msg = format!("cannot change the type of `{name}`: {detail}");
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
        })
    }
}
