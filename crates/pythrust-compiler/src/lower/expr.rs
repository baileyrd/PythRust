//! Expressions: each lowers to an operand, emitting what it needs first.

use crate::ast::{self, Arg, BinOp, Expr, ExprKind, FuncDef, LogicalOp, UnaryOp};
use crate::diag::{Diagnostic, Result};
use crate::exc::ExcKind;
use crate::infer::Type;
use crate::ir::{CmpOp, FloatOp, FuncId, Inst, IntOp, LocalId, Operand, Rvalue, Term, Ty, View};
use crate::span::Span;
use crate::stdlib::StdFn;

use super::vars::runtime;
use super::{unsupported, FnBuilder};

impl<'a> FnBuilder<'_, 'a> {
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
            ExprKind::Float(f) => Ok(Operand::Float(*f)),
            ExprKind::Str(text) => Ok(self.temp(Ty::Str, Rvalue::StrLit(text.clone()), e.span)),
            ExprKind::Tuple(items) | ExprKind::List(items) => self.display(items, e.span),
            ExprKind::Dict(items) => self.dict_display(items, e.span),
            ExprKind::Set(items) => self.set_display(items, e.span),
            ExprKind::Subscript { value, index } => self.subscript(value, index, e.span),
            ExprKind::Attr { .. } => Err(unsupported(e.span, "attributes other than methods")),
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
            (UnaryOp::Neg | UnaryOp::Pos, Ty::Bool) => {
                let v = self.as_int(v, span);
                self.unary_number(op, v, span)
            }
            (_, Ty::Int | Ty::Float) => self.unary_number(op, v, span),
            _ => Err(unsupported(span, "unary `-` or `+` on a non-number")),
        }
    }

    /// `-v` or `+v` of an int or float `v`.
    fn unary_number(&mut self, op: UnaryOp, v: Operand, span: Span) -> Result<Operand> {
        match (op, v) {
            // A negative literal stays a literal, so `x / -2.0` is known
            // not to divide by zero. `i64::MIN` is no literal to negate.
            (UnaryOp::Neg, Operand::Float(f)) => Ok(Operand::Float(-f)),
            (UnaryOp::Neg, Operand::Int(n)) if n != i64::MIN => Ok(Operand::Int(-n)),
            (UnaryOp::Neg, _) => Ok(self.temp(self.ty(v), Rvalue::Neg(v), span)),
            _ => Ok(v),
        }
    }

    fn binary(&mut self, op: BinOp, left: &Expr, right: &Expr, span: Span) -> Result<Operand> {
        let l = self.expr(left)?;
        let r = self.expr(right)?;
        self.arith(op, l, r, span)
    }

    /// `l op r`; shared with augmented assignment. Python's numeric
    /// tower: a bool computes as an int, and `/`, or an int meeting a
    /// float, computes in floats.
    pub(super) fn arith(
        &mut self,
        op: BinOp,
        l: Operand,
        r: Operand,
        span: Span,
    ) -> Result<Operand> {
        let (l, r) = (self.as_int(l, span), self.as_int(r, span));
        let (ty, value) = match (op, self.ty(l), self.ty(r)) {
            (BinOp::Add, Ty::Str, Ty::Str) => (Ty::Str, Rvalue::Concat(l, r)),
            (BinOp::Mul, Ty::Str, Ty::Int) => (Ty::Str, Rvalue::Repeat(l, r)),
            (BinOp::Mul, Ty::Int, Ty::Str) => (Ty::Str, Rvalue::Repeat(r, l)),
            (BinOp::Pow, Ty::Int, Ty::Int) => (Ty::Int, Rvalue::IntPow(l, r)),
            (BinOp::Div, Ty::Int, Ty::Int) => self.float_arith(FloatOp::IntDiv, l, r, span),
            (_, Ty::Int, Ty::Int) => (Ty::Int, Rvalue::Int(int_op(op, span)?, l, r)),
            (_, Ty::Int | Ty::Float, Ty::Int | Ty::Float) => {
                self.float_arith(float_op(op), l, r, span)
            }
            _ => return Err(unsupported(span, "arithmetic on a non-number")),
        };
        let may_raise = divides(&value);
        let v = self.temp(ty, value, span);
        if may_raise {
            self.check_raised(span)?;
        }
        Ok(v)
    }

    /// `l op r` in floats, each side an int or a float.
    fn float_arith(&mut self, op: FloatOp, l: Operand, r: Operand, span: Span) -> (Ty, Rvalue) {
        let (a, b) = (self.as_float(l, span), self.as_float(r, span));
        (Ty::Float, Rvalue::Float(op, a, b))
    }

    /// `v` as an int if it is a bool (0 or 1), else as it is.
    pub(super) fn as_int(&mut self, v: Operand, span: Span) -> Operand {
        match (v, self.ty(v)) {
            (Operand::Bool(b), _) => Operand::Int(i64::from(b)),
            (_, Ty::Bool) => self.temp(Ty::Int, Rvalue::BoolToInt(v), span),
            _ => v,
        }
    }

    /// `v`, an int, bool or float, as a float.
    pub(super) fn as_float(&mut self, v: Operand, span: Span) -> Operand {
        match (v, self.ty(v)) {
            (_, Ty::Float) => v,
            (Operand::Int(n), _) => Operand::Float(n as f64),
            _ => self.temp(Ty::Float, Rvalue::ToFloat(v), span),
        }
    }

    /// `and` / `or`: the result is one of the operands, adapted to the
    /// result's type (`x or 7` on an `int | None` wraps the 7).
    fn logical(&mut self, op: LogicalOp, left: &Expr, right: &Expr, span: Span) -> Result<Operand> {
        // Until its type settles, the left operand, which always lowers
        // first, decides it.
        let open = self.settled_ty_at(span)?;
        let l = self.expr(left)?;
        let ty = match open {
            Some(ty) => ty,
            None => self.decide_open(span, l)?,
        };
        let dest = self.pooled(ty);
        let test = self.truthy(l, span);
        let (short, rhs, end) = (self.new_block(), self.new_block(), self.new_block());
        let (then, orelse) = match op {
            LogicalOp::And => (rhs, short),
            LogicalOp::Or => (short, rhs),
        };
        self.terminate(Term::Branch {
            cond: test,
            then,
            orelse,
        });
        self.switch_to(short);
        let l = self.fit(l, ty, span);
        if self.ty(l) != ty {
            return Err(unsupported(
                span,
                "`and` / `or` on operands of different types",
            ));
        }
        self.move_into(dest, l, span);
        self.terminate(Term::Goto(end));
        self.switch_to(rhs);
        let Some(r) = self.on_branch(right, true)? else {
            self.switch_to(end);
            return Ok(Operand::Local(dest));
        };
        let r = self.fit(r, ty, span);
        if self.ty(r) != ty {
            return Err(unsupported(
                span,
                "`and` / `or` on operands of different types",
            ));
        }
        self.move_into(dest, r, span);
        self.terminate(Term::Goto(end));
        self.switch_to(end);
        Ok(Operand::Local(dest))
    }

    /// `e`'s value; `None` when `e` calls an instance that never returns
    /// and `branch` (it is evaluated on one branch only): its block then
    /// ends, unreachable, and the other branches carry on. Stuck there,
    /// the statement may still return another way.
    fn on_branch(&mut self, e: &Expr, branch: bool) -> Result<Option<Operand>> {
        match self.expr(e) {
            Err(_) if self.diverged && branch => {
                self.diverged = false;
                self.terminate(Term::Unreachable);
                Ok(None)
            }
            // The other branches may still complete the statement.
            Err(e) if self.stuck && branch => {
                self.may_return = true;
                Err(e)
            }
            r => r.map(Some),
        }
    }

    /// `a < b < c`: each operand evaluated once, stopping at the first
    /// false comparison.
    fn compare(&mut self, left: &Expr, rest: &[(ast::CmpOp, Expr)], span: Span) -> Result<Operand> {
        let dest = self.new_local(Ty::Bool);
        let end = self.new_block();
        let mut l = self.expr(left)?;
        for (i, (op, right)) in rest.iter().enumerate() {
            // Past the first link, a path that never returns ends only
            // its own branch.
            // `x in d.keys()` looks `x` up in `d`, building no view.
            let right = match (op, self.dict_view(right)) {
                (ast::CmpOp::In | ast::CmpOp::NotIn, Some((d, View::Keys))) => d,
                _ => right,
            };
            let Some(r) = self.on_branch(right, i > 0)? else {
                break;
            };
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

    /// One comparison. `is` only meets `None` (inference ensures it).
    fn cmp(&mut self, op: ast::CmpOp, l: Operand, r: Operand, span: Span) -> Result<Operand> {
        let op = match op {
            ast::CmpOp::Is => return Ok(self.is(l, r, false, span)),
            ast::CmpOp::IsNot => return Ok(self.is(l, r, true, span)),
            ast::CmpOp::In | ast::CmpOp::NotIn => return self.contains(op, l, r, span),
            ast::CmpOp::Eq => CmpOp::Eq,
            ast::CmpOp::NotEq => CmpOp::Ne,
            ast::CmpOp::Lt => CmpOp::Lt,
            ast::CmpOp::LtEq => CmpOp::Le,
            ast::CmpOp::Gt => CmpOp::Gt,
            ast::CmpOp::GtEq => CmpOp::Ge,
        };
        if matches!(self.ty(l), Ty::Opt(_)) || matches!(self.ty(r), Ty::Opt(_)) {
            return self.opt_eq(op, l, r, span);
        }
        self.cmp_values(op, l, r, span)
    }

    /// `l op r` on values that are not optional.
    pub(super) fn cmp_values(
        &mut self,
        op: CmpOp,
        l: Operand,
        r: Operand,
        span: Span,
    ) -> Result<Operand> {
        let (lt, rt) = (self.ty(l), self.ty(r));
        if (lt == Ty::Bool) != (rt == Ty::Bool) {
            // A bool meets a number as the int it is.
            let (l, r) = (self.as_int(l, span), self.as_int(r, span));
            return self.cmp_values(op, l, r, span);
        }
        match (lt, rt) {
            (Ty::Int, Ty::Float) | (Ty::Float, Ty::Int) => {
                let (a, b) = (self.as_float(l, span), self.as_float(r, span));
                Ok(self.temp(Ty::Bool, Rvalue::Cmp(op, a, b), span))
            }
            (Ty::None, Ty::None) => Ok(Operand::Bool(matches!(
                op,
                CmpOp::Eq | CmpOp::Le | CmpOp::Ge
            ))),
            _ if lt != rt => Err(unsupported(span, "comparing values of different types")),
            _ => Ok(self.temp(Ty::Bool, Rvalue::Cmp(op, l, r), span)),
        }
    }

    /// `l in r` and `l not in r`: a substring of a string, or an element
    /// of a list or range.
    pub(super) fn contains(
        &mut self,
        op: ast::CmpOp,
        l: Operand,
        r: Operand,
        span: Span,
    ) -> Result<Operand> {
        let needle = match (self.ty(l), self.ty(r)) {
            (Ty::Str, Ty::Str) => l,
            // A bool meets a number element as the int it is.
            (Ty::Bool, ty @ Ty::List(_))
                if matches!(self.slot_ty(ty, 0, span)?, Ty::Int | Ty::Float) =>
            {
                let l = self.as_int(l, span);
                return self.contains(op, l, r, span);
            }
            // An int meets a float element as a float, as `==` does.
            (Ty::Int, ty @ Ty::List(_)) if self.slot_ty(ty, 0, span)? == Ty::Float => {
                self.as_float(l, span)
            }
            (_, ty @ Ty::List(_)) => {
                let elem = self.slot_ty(ty, 0, span)?;
                let needle = self.fit(l, elem, span);
                if self.ty(needle) != elem {
                    let found = self.scan(l, r, span)?;
                    return Ok(self.negate_if(op, found, span));
                }
                needle
            }
            (Ty::Int | Ty::Bool | Ty::Float, Ty::Range) => l,
            (_, Ty::Dict(_) | Ty::Set(_) | Ty::View(_)) => {
                let found = self.table_contains(l, r, span)?;
                return Ok(self.negate_if(op, found, span));
            }
            _ => {
                return Err(unsupported(
                    span,
                    "`in` on anything but strings, lists and ranges",
                ))
            }
        };
        let found = Rvalue::Contains { hay: r, needle };
        let found = self.temp(Ty::Bool, found, span);
        Ok(self.negate_if(op, found, span))
    }

    /// `found` for `in`, its negation for `not in`.
    fn negate_if(&mut self, op: ast::CmpOp, found: Operand, span: Span) -> Operand {
        if op == ast::CmpOp::In {
            return found;
        }
        let not = Rvalue::Cmp(CmpOp::Eq, found, Operand::Bool(false));
        self.temp(Ty::Bool, not, span)
    }

    /// `needle in list` for a needle of another type than the elements
    /// (`1 in [True]`, `True in [1.0, None]`): a loop comparing each
    /// element with `==`, so the needle meets each as `==` would.
    fn scan(&mut self, needle: Operand, list: Operand, span: Span) -> Result<Operand> {
        let item = self.slot_ty(self.ty(list), 0, span)?;
        let (found, counter) = (self.pooled(Ty::Bool), self.new_local(Ty::Int));
        self.assign(found, Rvalue::Use(Operand::Bool(false)), span);
        self.assign(counter, Rvalue::Use(Operand::Int(0)), span);
        let cur = Operand::Local(counter);
        let (head, body, next, end) = (
            self.new_block(),
            self.new_block(),
            self.new_block(),
            self.new_block(),
        );
        self.terminate(Term::Goto(head));
        self.switch_to(head);
        let n = self.temp(Ty::Int, Rvalue::Len(list), span);
        let going = self.temp(Ty::Bool, Rvalue::Cmp(CmpOp::Lt, cur, n), span);
        self.terminate(Term::Branch {
            cond: going,
            then: body,
            orelse: end,
        });
        self.switch_to(body);
        self.pooled_stmt(|b| {
            let x = b.temp(item, Rvalue::Index(list, cur), span);
            let eq = b.equal(needle, x, span)?;
            b.assign(found, Rvalue::Use(eq), span);
            Ok(())
        })?;
        self.terminate(Term::Branch {
            cond: Operand::Local(found),
            then: end,
            orelse: next,
        });
        self.switch_to(next);
        self.assign(counter, Rvalue::Int(IntOp::Add, cur, Operand::Int(1)), span);
        self.terminate(Term::Goto(head));
        self.switch_to(end);
        Ok(Operand::Local(found))
    }

    /// `l == r`, either side optional or not.
    fn equal(&mut self, l: Operand, r: Operand, span: Span) -> Result<Operand> {
        if matches!(self.ty(l), Ty::Opt(_)) || matches!(self.ty(r), Ty::Opt(_)) {
            return self.opt_eq(CmpOp::Eq, l, r, span);
        }
        self.cmp_values(CmpOp::Eq, l, r, span)
    }

    fn if_exp(&mut self, test: &Expr, body: &Expr, orelse: &Expr, span: Span) -> Result<Operand> {
        // Until its type settles, a branch decides it; see `if_branch`.
        let settled = self.settled_ty_at(span)?;
        let mut dest = settled.map(|ty| (self.pooled(ty), ty));
        let (then, other, end) = (self.new_block(), self.new_block(), self.new_block());
        let t = self.expr(test)?;
        let cond = self.truthy(t, span);
        self.terminate(Term::Branch {
            cond,
            then,
            orelse: other,
        });
        let (mut stuck, mut stuck_both, mut diverged) = (None, false, 0);
        for (block, e) in [(then, body), (other, orelse)] {
            self.switch_to(block);
            match self.if_branch(e, &mut dest, span) {
                Err(err) if self.stuck => {
                    stuck_both = stuck.is_some();
                    (self.stuck, stuck) = (false, Some(err));
                }
                Err(_) if self.diverged => {
                    (self.diverged, diverged) = (false, diverged + 1);
                    self.terminate(Term::Unreachable);
                }
                r => r?,
            }
            self.terminate(Term::Goto(end));
        }
        if let Some(err) = stuck {
            // The other branch may still complete the statement.
            self.may_return |= !stuck_both && diverged == 0;
            self.stuck = true;
            return Err(err);
        }
        if diverged == 2 {
            return Err(self.diverge(span));
        }
        let (dest, _) = dest.ok_or_else(|| super::internal(span, "an untyped conditional"))?;
        self.switch_to(end);
        Ok(Operand::Local(dest))
    }

    /// One branch of a conditional expression, moved into `dest`. With
    /// `dest` open (its type depends on a result still undecided), the
    /// first branch that lowers decides it, and binds the variables of
    /// its type: a branch stuck on a recursive call skips the statement,
    /// but the other may still decide the function's result type.
    fn if_branch(&mut self, e: &Expr, dest: &mut Option<(LocalId, Ty)>, span: Span) -> Result<()> {
        let v = self.expr(e)?;
        let (local, ty) = match *dest {
            Some(d) => d,
            None => {
                let ty = self.decide_open(span, v)?;
                *dest.insert((self.pooled(ty), ty))
            }
        };
        let v = self.fit(v, ty, span);
        if self.ty(v) != ty {
            return Err(unsupported(
                span,
                "a conditional expression whose branches differ in type",
            ));
        }
        self.move_into(local, v, span);
        Ok(())
    }

    /// The type of the open node at `span`, decided by `v`, one of the
    /// values it may take, in the node's shape; binds the node's variables.
    fn decide_open(&mut self, span: Span, v: Operand) -> Result<Ty> {
        let node = self.node_type(span)?;
        let have = self.ty(v);
        let ty = self.shape(node, have, span)?;
        super::bind(node, ty, &self.cx.seqs, &mut self.subst);
        Ok(ty)
    }

    /// Python truthiness as a `Bool` operand.
    pub(super) fn truthy(&mut self, v: Operand, span: Span) -> Operand {
        match self.ty(v) {
            Ty::Bool => v,
            Ty::Int => self.temp(Ty::Bool, Rvalue::Cmp(CmpOp::Ne, v, Operand::Int(0)), span),
            Ty::Float => {
                let zero = Operand::Float(0.0);
                self.temp(Ty::Bool, Rvalue::Cmp(CmpOp::Ne, v, zero), span)
            }
            Ty::None => Operand::Bool(false),
            Ty::Exc => Operand::Bool(true),
            Ty::Str
            | Ty::List(_)
            | Ty::Tuple(_)
            | Ty::Range
            | Ty::Dict(_)
            | Ty::Set(_)
            | Ty::View(_) => {
                let n = self.temp(Ty::Int, Rvalue::Len(v), span);
                self.temp(Ty::Bool, Rvalue::Cmp(CmpOp::Ne, n, Operand::Int(0)), span)
            }
            Ty::Opt(_) => self.opt_truthy(v, span),
        }
    }

    fn call(&mut self, func: &Expr, args: &[Arg], span: Span) -> Result<Operand> {
        let name = match &func.kind {
            ExprKind::Name(name) => name,
            ExprKind::Attr { value, attr } => return self.method(value, &attr.name, args, span),
            _ => return Err(unsupported(span, "calling this kind of expression")),
        };
        let mut def = self.cx.defs.get(name.as_str()).copied();
        // Before a shadowing `def` runs, the name still finds the builtin.
        let pending = def.is_some()
            && self
                .defined
                .as_ref()
                .is_some_and(|d| !d.contains(name.as_str()));
        if pending && crate::interp::is_builtin(name) {
            // Inference typed the call by the later `def`; the builtin's
            // result must agree, or the program is outside the subset.
            if name != "print" || self.ty_at(span)? != Ty::None {
                let what = format!("calling builtin `{name}` before a `def` that shadows it");
                return Err(unsupported(span, &what));
            }
            def = None;
        }
        if def.is_some() && !self.callable(name, func.span)? {
            // Python looks the callee up before evaluating arguments.
            let ty = self.ty_at(span)?;
            return Ok(Operand::Local(self.new_local(ty)));
        }
        // Python evaluates arguments as written, then binds them.
        let mut ops = Vec::new();
        for a in args {
            ops.push(self.expr(&a.value)?);
        }
        let Some(def) = def else {
            return self.builtin(name, args, ops, span);
        };
        let mut bound = Vec::new();
        for (k, op) in bind_params(def, args, ops)?.into_iter().enumerate() {
            bound.push(match op {
                Some(op) => op,
                None => self.default_arg(def, k, span)?,
            });
        }
        let sig = self
            .cx
            .types
            .func(name)
            .ok_or_else(|| super::internal(span, &format!("`{name}` has no signature")))?;
        let params: Vec<Type> = sig.params.iter().map(|p| p.ty.clone()).collect();
        let mut ops = Vec::new();
        for (op, param) in bound.into_iter().zip(&params) {
            ops.push(self.fit_param(op, param, span)?);
        }
        let tys: Vec<Ty> = ops.iter().map(|op| self.ty(*op)).collect();
        // In generic code a call's node type may hold variables only the
        // callee's body decides; its instance then does, and binds them
        // here for the nodes that share them.
        let node = self.node_type(span)?;
        let (id, ret) = match super::is_closed(node, &self.subst) {
            true => {
                let ret = self.ty_at(span)?;
                (self.cx.instance(def, tys, ret, span)?, ret)
            }
            false => self.open_call(def, tys, node, span)?,
        };
        // The callee takes its arguments' references.
        let owned: Vec<Operand> = ops.into_iter().map(|op| self.own(op, span)).collect();
        let result = self.temp(ret, Rvalue::Call(id, owned.clone()), span);
        for op in owned {
            if let (Operand::Local(l), true) = (op, self.ty(op).is_rc()) {
                self.emit(Inst::Forget(l));
            }
        }
        if self.may_raise(name) {
            self.check_raised(span)?;
        }
        if self.cx.diverges.contains(&id) {
            // Its result, of a settled type, is never there to use.
            return Err(self.diverge(span));
        }
        Ok(result)
    }

    /// Abandons the expression being lowered: the path never gets past
    /// `span`. The statement or branch it ends catches the error.
    pub(super) fn diverge(&mut self, span: Span) -> Diagnostic {
        self.diverged = true;
        super::internal(span, "code after a call that never returns")
    }

    /// The instance of `def` for `tys` whose result type, open in
    /// `node`, its body decides. A recursive call before that is
    /// unsupported, but while this function's own result is undecided
    /// it is skipped for a second pass ([`FnBuilder::block`]).
    fn open_call(
        &mut self,
        def: &'a FuncDef,
        tys: Vec<Ty>,
        node: &Type,
        span: Span,
    ) -> Result<(FuncId, Ty)> {
        let (id, ret) = match self.cx.open_instance(def, tys, span)? {
            super::Found::Known(found) => found,
            super::Found::Unknown(returns) => {
                self.stuck = self.open_ret.is_some();
                self.may_return |= returns;
                return Err(unsupported(span, super::stmt::OPEN_RESULT));
            }
        };
        // A result that never exists decides nothing for live code.
        let subst = match self.cx.diverges.contains(&id) {
            true => &mut self.dead,
            false => &mut self.subst,
        };
        super::bind(node, ret, &self.cx.seqs, subst);
        Ok((id, ret))
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

    /// A builtin call; `ops` are `args`' values. Inference allows
    /// keywords only for `print`.
    fn builtin(
        &mut self,
        name: &str,
        args: &[Arg],
        ops: Vec<Operand>,
        span: Span,
    ) -> Result<Operand> {
        if name == "print" {
            return self.print(args, ops, span);
        }
        if args.iter().any(|a| a.name.is_some()) {
            return Err(unsupported(span, "keyword arguments"));
        }
        match name {
            "len" | "list" | "str" | "int" | "float" | "bool" => {
                self.unary_builtin(name, &ops, span)
            }
            "set" => self.new_set(&ops, span),
            "sorted" => self.sorted(&ops, span),
            "range" => {
                let (start, stop, step) = self.range_bounds(args, &ops, span)?;
                let items = vec![start, stop, step];
                Ok(self.temp(Ty::Range, Rvalue::NewSeq(items), span))
            }
            other => {
                if let Some(f) = StdFn::from_name(other) {
                    let ty = self.ty_at(span)?;
                    return Ok(self.temp(ty, Rvalue::Std(f, ops), span));
                }
                match ExcKind::from_name(other) {
                    Some(kind) => self.new_exc(kind, &ops, span),
                    None => Err(unsupported(span, &format!("`{other}`"))),
                }
            }
        }
    }

    /// `print`, its `sep` and `end` keywords split from the values.
    /// Inference may have typed the call against a later `def print`, so
    /// the builtin's own keywords are checked here, failing as `run` does.
    /// A value that may hold an exception is rendered first, with
    /// `str`, which fails at `span` before anything is printed if it
    /// nests too deep, as in `run`.
    fn print(&mut self, args: &[Arg], ops: Vec<Operand>, span: Span) -> Result<Operand> {
        let (mut values, mut sep, mut end) = (Vec::new(), None, None);
        for (a, op) in args.iter().zip(ops) {
            let Some(kw) = &a.name else {
                values.push(op);
                continue;
            };
            let slot = match kw.name.as_str() {
                "sep" => &mut sep,
                "end" => &mut end,
                other => {
                    let msg =
                        format!("TypeError: '{other}' is an invalid keyword argument for print()");
                    self.jump_away(Term::Fail(runtime(kw.span, &msg)));
                    return Ok(Operand::None);
                }
            };
            match self.ty(op) {
                Ty::Str => *slot = Some(op),
                Ty::None => {}
                Ty::Opt(_) => return Err(unsupported(kw.span, "an optional `sep` or `end`")),
                Ty::Exc => return Err(unsupported(kw.span, "an exception as `sep` or `end`")),
                ty => {
                    let msg = format!(
                        "TypeError: {} must be None or a string, not {}",
                        kw.name,
                        type_name(ty)
                    );
                    self.jump_away(Term::Fail(runtime(kw.span, &msg)));
                    return Ok(Operand::None);
                }
            }
        }
        let mut rendered = Vec::new();
        for v in values {
            rendered.push(match self.holds(self.ty(v), crate::ir::is_exc) {
                true => self.temp(Ty::Str, Rvalue::ToStr(v), span),
                false => v,
            });
        }
        self.emit(Inst::Print {
            args: rendered,
            sep,
            end,
        });
        Ok(Operand::None)
    }

    /// `len`, `list`, `str`, `int`, `float` and `bool` of one argument.
    fn unary_builtin(&mut self, name: &str, args: &[Operand], span: Span) -> Result<Operand> {
        let [arg] = args else {
            return Err(unsupported(
                span,
                &format!("`{name}` with other than one argument"),
            ));
        };
        let arg = *arg;
        let (ty, value) = match (name, self.ty(arg)) {
            ("str", Ty::Str) | ("int", Ty::Int) | ("float", Ty::Float) => return Ok(arg),
            ("float", Ty::Int | Ty::Bool) => return Ok(self.as_float(arg, span)),
            ("int", Ty::Bool) => return Ok(self.as_int(arg, span)),
            ("bool", _) => return Ok(self.truthy(arg, span)),
            ("len", Ty::Range) => return Ok(self.range_len(arg, span)),
            (
                "len",
                Ty::Str | Ty::List(_) | Ty::Tuple(_) | Ty::Dict(_) | Ty::Set(_) | Ty::View(_),
            ) => (Ty::Int, Rvalue::Len(arg)),
            ("list", ty @ Ty::List(_)) => (ty, Rvalue::CopyList(arg)),
            ("list", Ty::Dict(_) | Ty::Set(_) | Ty::View(_)) => {
                (self.ty_at(span)?, Rvalue::CopyList(arg))
            }
            ("str", _) => (Ty::Str, Rvalue::ToStr(arg)),
            ("int", Ty::Float) => (Ty::Int, Rvalue::FloatToInt(arg)),
            ("int" | "float", Ty::Str) => return self.parse(name, arg, span),
            (_, ty) => {
                let what = format!("`{name}` of a {ty:?}");
                return Err(unsupported(span, &what));
            }
        };
        Ok(self.temp(ty, value, span))
    }

    /// `int(s)` or `float(s)` of a string, which may raise `ValueError`.
    fn parse(&mut self, name: &str, s: Operand, span: Span) -> Result<Operand> {
        let (ty, value) = match name {
            "int" => (Ty::Int, Rvalue::ParseInt(s)),
            _ => (Ty::Float, Rvalue::ParseFloat(s)),
        };
        let v = self.temp(ty, value, span);
        self.check_raised(span)?;
        Ok(v)
    }
}

/// Python's name for the type of a `ty` value.
fn type_name(ty: Ty) -> &'static str {
    match ty {
        Ty::Int => "int",
        Ty::Float => "float",
        Ty::Bool => "bool",
        Ty::None => "NoneType",
        Ty::Str => "str",
        Ty::List(_) => "list",
        Ty::Tuple(_) => "tuple",
        Ty::Opt(_) => "optional",
        Ty::Range => "range",
        Ty::Dict(_) => "dict",
        Ty::Set(_) => "set",
        Ty::View(_) => "dict view",
        // Exceptions print by kind, which only the run time knows.
        Ty::Exc => "exception",
    }
}

/// `ops`, the values of `args`, in `def`'s parameter order: positional
/// first, then keywords by name; `None` where the default applies.
/// Inference rejected every other shape.
fn bind_params(def: &FuncDef, args: &[Arg], ops: Vec<Operand>) -> Result<Vec<Option<Operand>>> {
    let mut slots: Vec<Option<Operand>> = vec![None; def.params.len()];
    for (next, (a, op)) in args.iter().zip(ops).enumerate() {
        let k = match &a.name {
            Some(kw) => def.params.iter().position(|p| p.name.name == kw.name),
            None => Some(next),
        };
        let slot = k.and_then(|k| slots.get_mut(k));
        let Some(slot) = slot.filter(|s| s.is_none()) else {
            return Err(super::internal(
                a.span,
                "an argument inference did not bind",
            ));
        };
        *slot = Some(op);
    }
    Ok(slots)
}

/// Whether `value` divides by what may be zero, which raises.
fn divides(value: &Rvalue) -> bool {
    let divisor = match value {
        Rvalue::Int(IntOp::FloorDiv | IntOp::Mod, _, b) => b,
        Rvalue::Float(FloatOp::Div | FloatOp::IntDiv | FloatOp::FloorDiv | FloatOp::Mod, _, b) => b,
        _ => return false,
    };
    !matches!(*divisor, Operand::Int(n) if n != 0)
        && !matches!(*divisor, Operand::Float(f) if f != 0.0)
}

/// The float operation for `op`.
fn float_op(op: BinOp) -> FloatOp {
    match op {
        BinOp::Add => FloatOp::Add,
        BinOp::Sub => FloatOp::Sub,
        BinOp::Mul => FloatOp::Mul,
        BinOp::Div => FloatOp::Div,
        BinOp::FloorDiv => FloatOp::FloorDiv,
        BinOp::Mod => FloatOp::Mod,
        BinOp::Pow => FloatOp::Pow,
    }
}

/// The int operation for `op`; `/` and `**` are not one.
fn int_op(op: BinOp, span: Span) -> Result<IntOp> {
    match op {
        BinOp::Add => Ok(IntOp::Add),
        BinOp::Sub => Ok(IntOp::Sub),
        BinOp::Mul => Ok(IntOp::Mul),
        BinOp::FloorDiv => Ok(IntOp::FloorDiv),
        BinOp::Mod => Ok(IntOp::Mod),
        BinOp::Div | BinOp::Pow => Err(super::internal(span, "`/` or `**` as an int operation")),
    }
}
