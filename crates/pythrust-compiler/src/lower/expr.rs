//! Expressions: each lowers to an operand, emitting what it needs first.

use crate::ast::{self, Arg, BinOp, Expr, ExprKind, LogicalOp, UnaryOp};
use crate::diag::Result;
use crate::ir::{CmpOp, Inst, IntOp, Operand, Rvalue, Term, Ty};
use crate::span::Span;

use super::vars::runtime;
use super::{unsupported, FnBuilder};

impl FnBuilder<'_, '_> {
    pub(super) fn expr(&mut self, e: &Expr) -> Result<Operand> {
        match &e.kind {
            ExprKind::Int(n) => Ok(Operand::Int(*n)),
            ExprKind::Bool(b) => Ok(Operand::Bool(*b)),
            ExprKind::None => Ok(Operand::None),
            ExprKind::Name(name) => self.read(name, e.span),
            ExprKind::Unary { op, operand } => self.unary(*op, operand, e.span),
            ExprKind::Binary { op, left, right } => self.binary(*op, left, right, e.span),
            ExprKind::Logical { op, left, right } => self.logical(*op, left, right, e.span),
            ExprKind::Compare { left, rest } => self.compare(left, rest, e.span),
            ExprKind::IfExp { test, body, orelse } => self.if_exp(test, body, orelse, e.span),
            ExprKind::Call { func, args } => self.call(func, args, e.span),
            ExprKind::Float(_) => Err(unsupported(e.span, "floats")),
            ExprKind::Str(_) => Err(unsupported(e.span, "strings")),
            ExprKind::Tuple(_) => Err(unsupported(e.span, "tuples")),
            ExprKind::List(_) => Err(unsupported(e.span, "lists")),
            ExprKind::Subscript { .. } => Err(unsupported(e.span, "indexing")),
            ExprKind::Attr { .. } => Err(unsupported(e.span, "methods")),
        }
    }

    fn unary(&mut self, op: UnaryOp, operand: &Expr, span: Span) -> Result<Operand> {
        let v = self.expr(operand)?;
        match (op, self.ty(v)) {
            (UnaryOp::Not, _) => {
                let t = self.truthy(v, span);
                Ok(self.temp(
                    Ty::Bool,
                    Rvalue::Cmp(CmpOp::Eq, t, Operand::Bool(false)),
                    span,
                ))
            }
            (UnaryOp::Neg, Ty::Int) => Ok(self.temp(Ty::Int, Rvalue::Neg(v), span)),
            (UnaryOp::Pos, Ty::Int) => Ok(v),
            _ => Err(unsupported(span, "unary `-` or `+` on a non-int")),
        }
    }

    fn binary(&mut self, op: BinOp, left: &Expr, right: &Expr, span: Span) -> Result<Operand> {
        let l = self.expr(left)?;
        let r = self.expr(right)?;
        self.int_op(op, l, r, span)
    }

    /// `l op r` on two ints; shared with augmented assignment.
    pub(super) fn int_op(
        &mut self,
        op: BinOp,
        l: Operand,
        r: Operand,
        span: Span,
    ) -> Result<Operand> {
        if (self.ty(l), self.ty(r)) != (Ty::Int, Ty::Int) {
            return Err(unsupported(span, "arithmetic on a non-int"));
        }
        let op = match op {
            BinOp::Add => IntOp::Add,
            BinOp::Sub => IntOp::Sub,
            BinOp::Mul => IntOp::Mul,
            BinOp::FloorDiv => IntOp::FloorDiv,
            BinOp::Mod => IntOp::Mod,
            BinOp::Div => return Err(unsupported(span, "`/`, which gives a float,")),
            BinOp::Pow => return Err(unsupported(span, "`**`")),
        };
        Ok(self.temp(Ty::Int, Rvalue::Int(op, l, r), span))
    }

    /// `and` / `or`: the result is one of the operands, so all three
    /// types must agree.
    fn logical(&mut self, op: LogicalOp, left: &Expr, right: &Expr, span: Span) -> Result<Operand> {
        let ty = self.ty_at(span)?;
        let l = self.expr(left)?;
        if self.ty(l) != ty {
            return Err(unsupported(
                span,
                "`and` / `or` on operands of different types",
            ));
        }
        let dest = self.new_local(ty);
        self.assign(dest, Rvalue::Use(l), span);
        let test = self.truthy(l, span);
        let (rhs, end) = (self.new_block(), self.new_block());
        let (then, orelse) = match op {
            LogicalOp::And => (rhs, end),
            LogicalOp::Or => (end, rhs),
        };
        self.terminate(Term::Branch {
            cond: test,
            then,
            orelse,
        });
        self.switch_to(rhs);
        let r = self.expr(right)?;
        if self.ty(r) != ty {
            return Err(unsupported(
                span,
                "`and` / `or` on operands of different types",
            ));
        }
        self.assign(dest, Rvalue::Use(r), span);
        self.terminate(Term::Goto(end));
        self.switch_to(end);
        Ok(Operand::Local(dest))
    }

    /// `a < b < c`: each operand evaluated once, stopping at the first
    /// false comparison.
    fn compare(&mut self, left: &Expr, rest: &[(ast::CmpOp, Expr)], span: Span) -> Result<Operand> {
        let dest = self.new_local(Ty::Bool);
        let end = self.new_block();
        let mut l = self.expr(left)?;
        for (i, (op, right)) in rest.iter().enumerate() {
            let r = self.expr(right)?;
            let c = self.cmp(*op, l, r, span)?;
            self.assign(dest, Rvalue::Use(c), span);
            if i + 1 == rest.len() {
                break;
            }
            let next = self.new_block();
            self.terminate(Term::Branch {
                cond: c,
                then: next,
                orelse: end,
            });
            self.switch_to(next);
            l = r;
        }
        self.terminate(Term::Goto(end));
        self.switch_to(end);
        Ok(Operand::Local(dest))
    }

    /// One comparison. `is` only meets `None` (inference ensures it), and
    /// no value in this subset is optional, so it folds to a constant.
    fn cmp(&mut self, op: ast::CmpOp, l: Operand, r: Operand, span: Span) -> Result<Operand> {
        let (lt, rt) = (self.ty(l), self.ty(r));
        let same = lt == rt;
        let op = match op {
            ast::CmpOp::Is => return Ok(Operand::Bool(same)),
            ast::CmpOp::IsNot => return Ok(Operand::Bool(!same)),
            ast::CmpOp::In | ast::CmpOp::NotIn => return Err(unsupported(span, "`in`")),
            ast::CmpOp::Eq => CmpOp::Eq,
            ast::CmpOp::NotEq => CmpOp::Ne,
            ast::CmpOp::Lt => CmpOp::Lt,
            ast::CmpOp::LtEq => CmpOp::Le,
            ast::CmpOp::Gt => CmpOp::Gt,
            ast::CmpOp::GtEq => CmpOp::Ge,
        };
        match (lt, rt) {
            (Ty::None, Ty::None) => Ok(Operand::Bool(matches!(
                op,
                CmpOp::Eq | CmpOp::Le | CmpOp::Ge
            ))),
            _ if !same => Err(unsupported(span, "comparing values of different types")),
            _ => Ok(self.temp(Ty::Bool, Rvalue::Cmp(op, l, r), span)),
        }
    }

    fn if_exp(&mut self, test: &Expr, body: &Expr, orelse: &Expr, span: Span) -> Result<Operand> {
        let ty = self.ty_at(span)?;
        let dest = self.new_local(ty);
        let (then, other, end) = (self.new_block(), self.new_block(), self.new_block());
        let t = self.expr(test)?;
        let cond = self.truthy(t, span);
        self.terminate(Term::Branch {
            cond,
            then,
            orelse: other,
        });
        for (block, e) in [(then, body), (other, orelse)] {
            self.switch_to(block);
            let v = self.expr(e)?;
            if self.ty(v) != ty {
                return Err(unsupported(
                    span,
                    "a conditional expression whose branches differ in type",
                ));
            }
            self.assign(dest, Rvalue::Use(v), span);
            self.terminate(Term::Goto(end));
        }
        self.switch_to(end);
        Ok(Operand::Local(dest))
    }

    /// Python truthiness as a `Bool` operand.
    pub(super) fn truthy(&mut self, v: Operand, span: Span) -> Operand {
        match self.ty(v) {
            Ty::Bool => v,
            Ty::Int => self.temp(Ty::Bool, Rvalue::Cmp(CmpOp::Ne, v, Operand::Int(0)), span),
            Ty::None => Operand::Bool(false),
        }
    }

    fn call(&mut self, func: &Expr, args: &[Arg], span: Span) -> Result<Operand> {
        let ExprKind::Name(name) = &func.kind else {
            return Err(unsupported(span, "calling this kind of expression"));
        };
        if args.iter().any(|a| a.name.is_some()) {
            return Err(unsupported(span, "keyword arguments"));
        }
        let def = self.cx.defs.get(name.as_str()).copied();
        if def.is_some() && !self.callable(name, func.span)? {
            // Python looks the callee up before evaluating arguments.
            return Ok(Operand::Local(self.new_local(self.ty_at(span)?)));
        }
        let mut ops = Vec::new();
        for a in args {
            ops.push(self.expr(&a.value)?);
        }
        let Some(def) = def else {
            return self.builtin(name, ops, span);
        };
        if def.params.len() != ops.len() {
            return Err(unsupported(span, "default arguments"));
        }
        let tys = ops.iter().map(|op| self.ty(*op)).collect();
        let ret = self.ty_at(span)?;
        let id = self.cx.instance(def, tys, ret, span)?;
        Ok(self.temp(ret, Rvalue::Call(id, ops), span))
    }

    /// Whether a call from here to top-level `name` finds it defined. In
    /// module code a call before its `def` raises `NameError`, emitted
    /// here (false). A call that can reach a function whose `def` has not
    /// run yet would fail midway, which is not supported. Function bodies
    /// only run once a module-level call reached them, which checked.
    fn callable(&mut self, name: &str, span: Span) -> Result<bool> {
        let Some(defined) = &self.defined else {
            return Ok(true);
        };
        if !defined.contains(name) {
            let diag = runtime(span, &format!("NameError: name '{name}' is not defined"));
            self.jump_away(Term::Fail(diag));
            return Ok(false);
        }
        let reach = super::reach(&self.cx.defs, name);
        match reach.into_iter().find(|f| !defined.contains(*f)) {
            Some(late) => Err(unsupported(
                span,
                &format!("calling `{name}` while `{late}`, which it may call, is not yet defined,"),
            )),
            None => Ok(true),
        }
    }

    fn builtin(&mut self, name: &str, args: Vec<Operand>, span: Span) -> Result<Operand> {
        match name {
            "print" => {
                self.emit(Inst::Print(args));
                Ok(Operand::None)
            }
            other => Err(unsupported(span, &format!("`{other}`"))),
        }
    }
}
