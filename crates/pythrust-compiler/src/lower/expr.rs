//! Expressions: each lowers to an operand, emitting what it needs first.

use crate::ast::{self, Arg, BinOp, Expr, ExprKind, FuncDef, LogicalOp, UnaryOp};
use crate::diag::Result;
use crate::ir::{CmpOp, FloatOp, Inst, IntOp, Operand, Rvalue, Term, Ty};
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
            ExprKind::Float(f) => Ok(Operand::Float(*f)),
            ExprKind::Str(text) => Ok(self.temp(Ty::Str, Rvalue::StrLit(text.clone()), e.span)),
            ExprKind::Tuple(items) | ExprKind::List(items) => self.display(items, e.span),
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
            (UnaryOp::Neg, ty @ (Ty::Int | Ty::Float)) => Ok(self.temp(ty, Rvalue::Neg(v), span)),
            (UnaryOp::Pos, Ty::Int | Ty::Float) => Ok(v),
            _ => Err(unsupported(span, "unary `-` or `+` on a non-number")),
        }
    }

    fn binary(&mut self, op: BinOp, left: &Expr, right: &Expr, span: Span) -> Result<Operand> {
        let l = self.expr(left)?;
        let r = self.expr(right)?;
        self.arith(op, l, r, span)
    }

    /// `l op r`; shared with augmented assignment. Python's numeric
    /// tower: `/`, or an int meeting a float, computes in floats.
    pub(super) fn arith(
        &mut self,
        op: BinOp,
        l: Operand,
        r: Operand,
        span: Span,
    ) -> Result<Operand> {
        let (ty, value) = match (op, self.ty(l), self.ty(r)) {
            (BinOp::Add, Ty::Str, Ty::Str) => (Ty::Str, Rvalue::Concat(l, r)),
            (BinOp::Mul, Ty::Str, Ty::Int) => (Ty::Str, Rvalue::Repeat(l, r)),
            (BinOp::Mul, Ty::Int, Ty::Str) => (Ty::Str, Rvalue::Repeat(r, l)),
            (BinOp::Pow, Ty::Int, Ty::Int) => (Ty::Int, Rvalue::IntPow(l, r)),
            (BinOp::Div, Ty::Int, Ty::Int) => self.float_arith(op, l, r, span),
            (_, Ty::Int, Ty::Int) => (Ty::Int, Rvalue::Int(int_op(op, span)?, l, r)),
            (_, Ty::Int | Ty::Float, Ty::Int | Ty::Float) => self.float_arith(op, l, r, span),
            _ => return Err(unsupported(span, "arithmetic on a non-number")),
        };
        Ok(self.temp(ty, value, span))
    }

    /// `l op r` in floats, each side an int or a float.
    fn float_arith(&mut self, op: BinOp, l: Operand, r: Operand, span: Span) -> (Ty, Rvalue) {
        let op = match op {
            BinOp::Add => FloatOp::Add,
            BinOp::Sub => FloatOp::Sub,
            BinOp::Mul => FloatOp::Mul,
            BinOp::Div => FloatOp::Div,
            BinOp::FloorDiv => FloatOp::FloorDiv,
            BinOp::Mod => FloatOp::Mod,
            BinOp::Pow => FloatOp::Pow,
        };
        let (a, b) = (self.as_float(l, span), self.as_float(r, span));
        (Ty::Float, Rvalue::Float(op, a, b))
    }

    /// `v`, an int, bool or float, as a float.
    pub(super) fn as_float(&mut self, v: Operand, span: Span) -> Operand {
        match (v, self.ty(v)) {
            (_, Ty::Float) => v,
            (Operand::Int(n), _) => Operand::Float(n as f64),
            _ => self.temp(Ty::Float, Rvalue::ToFloat(v), span),
        }
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
        self.move_into(dest, l, span);
        self.terminate(Term::Goto(end));
        self.switch_to(rhs);
        let r = self.expr(right)?;
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
            ast::CmpOp::In | ast::CmpOp::NotIn => return self.contains(op, l, r, span),
            ast::CmpOp::Eq => CmpOp::Eq,
            ast::CmpOp::NotEq => CmpOp::Ne,
            ast::CmpOp::Lt => CmpOp::Lt,
            ast::CmpOp::LtEq => CmpOp::Le,
            ast::CmpOp::Gt => CmpOp::Gt,
            ast::CmpOp::GtEq => CmpOp::Ge,
        };
        match (lt, rt) {
            (Ty::Int, Ty::Float) | (Ty::Float, Ty::Int) => {
                let (a, b) = (self.as_float(l, span), self.as_float(r, span));
                Ok(self.temp(Ty::Bool, Rvalue::Cmp(op, a, b), span))
            }
            (Ty::None, Ty::None) => Ok(Operand::Bool(matches!(
                op,
                CmpOp::Eq | CmpOp::Le | CmpOp::Ge
            ))),
            _ if !same => Err(unsupported(span, "comparing values of different types")),
            _ => Ok(self.temp(Ty::Bool, Rvalue::Cmp(op, l, r), span)),
        }
    }

    /// `l in r` and `l not in r`: a substring of a string, or an element
    /// of a list.
    fn contains(&mut self, op: ast::CmpOp, l: Operand, r: Operand, span: Span) -> Result<Operand> {
        let needle = match (self.ty(l), self.ty(r)) {
            (Ty::Str, Ty::Str) => l,
            // An int meets a float element as a float, as `==` does.
            (Ty::Int, ty @ Ty::List(_)) if self.slot_ty(ty, 0, span)? == Ty::Float => {
                self.as_float(l, span)
            }
            (needle, ty @ Ty::List(_)) if self.slot_ty(ty, 0, span)? == needle => l,
            _ => return Err(unsupported(span, "`in` on anything but strings and lists")),
        };
        let found = Rvalue::Contains { hay: r, needle };
        let found = self.temp(Ty::Bool, found, span);
        if op == ast::CmpOp::In {
            return Ok(found);
        }
        let not = Rvalue::Cmp(CmpOp::Eq, found, Operand::Bool(false));
        Ok(self.temp(Ty::Bool, not, span))
    }

    fn if_exp(&mut self, test: &Expr, body: &Expr, orelse: &Expr, span: Span) -> Result<Operand> {
        let ty = self.ty_at(span)?;
        let dest = self.pooled(ty);
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
            self.move_into(dest, v, span);
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
            Ty::Float => {
                let zero = Operand::Float(0.0);
                self.temp(Ty::Bool, Rvalue::Cmp(CmpOp::Ne, v, zero), span)
            }
            Ty::None => Operand::Bool(false),
            Ty::Str | Ty::List(_) | Ty::Tuple(_) => {
                let n = self.temp(Ty::Int, Rvalue::Len(v), span);
                self.temp(Ty::Bool, Rvalue::Cmp(CmpOp::Ne, n, Operand::Int(0)), span)
            }
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
        let ops = bind_params(def, args, ops, span)?;
        let tys = ops.iter().map(|op| self.ty(*op)).collect();
        let ret = self.ty_at(span)?;
        let id = self.cx.instance(def, tys, ret, span)?;
        // The callee takes its arguments' references.
        let owned: Vec<Operand> = ops.into_iter().map(|op| self.own(op, span)).collect();
        let result = self.temp(ret, Rvalue::Call(id, owned.clone()), span);
        for op in owned {
            if let (Operand::Local(l), true) = (op, self.ty(op).is_rc()) {
                self.emit(Inst::Forget(l));
            }
        }
        Ok(result)
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
            return Ok(self.print(args, ops));
        }
        if args.iter().any(|a| a.name.is_some()) {
            return Err(unsupported(span, "keyword arguments"));
        }
        match name {
            "len" | "str" | "int" | "float" | "bool" => self.unary_builtin(name, &ops, span),
            other => Err(unsupported(span, &format!("`{other}`"))),
        }
    }

    /// `print`, its `sep` and `end` keywords split from the values.
    /// Inference may have typed the call against a later `def print`, so
    /// the builtin's own keywords are checked here, failing as `run` does.
    fn print(&mut self, args: &[Arg], ops: Vec<Operand>) -> Operand {
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
                    return Operand::None;
                }
            };
            match self.ty(op) {
                Ty::Str => *slot = Some(op),
                Ty::None => {}
                ty => {
                    let msg = format!(
                        "TypeError: {} must be None or a string, not {}",
                        kw.name,
                        type_name(ty)
                    );
                    self.jump_away(Term::Fail(runtime(kw.span, &msg)));
                    return Operand::None;
                }
            }
        }
        self.emit(Inst::Print {
            args: values,
            sep,
            end,
        });
        Operand::None
    }

    /// `len`, `str`, `int`, `float` and `bool` of one argument.
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
            ("bool", _) => return Ok(self.truthy(arg, span)),
            ("len", Ty::Str | Ty::List(_) | Ty::Tuple(_)) => (Ty::Int, Rvalue::Len(arg)),
            ("str", _) => (Ty::Str, Rvalue::ToStr(arg)),
            ("int", Ty::Float) => (Ty::Int, Rvalue::FloatToInt(arg)),
            ("int", Ty::Str) => (Ty::Int, Rvalue::ParseInt(arg)),
            ("float", Ty::Str) => (Ty::Float, Rvalue::ParseFloat(arg)),
            (_, ty) => {
                let what = format!("`{name}` of a {ty:?}");
                return Err(unsupported(span, &what));
            }
        };
        Ok(self.temp(ty, value, span))
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
    }
}

/// `ops`, the values of `args`, in `def`'s parameter order: positional
/// first, then keywords by name. Inference rejected every other shape.
fn bind_params(def: &FuncDef, args: &[Arg], ops: Vec<Operand>, span: Span) -> Result<Vec<Operand>> {
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
    slots
        .into_iter()
        .collect::<Option<Vec<Operand>>>()
        .ok_or_else(|| unsupported(span, "default arguments"))
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
