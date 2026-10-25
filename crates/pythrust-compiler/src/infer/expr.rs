//! Expressions, calls and annotations.

use crate::ast::{Arg, CmpOp, Expr, ExprKind, Ident, LogicalOp, TypeExpr, UnaryOp};
use crate::diag::Result;
use crate::span::Span;

use super::solve::{Access, Binary, Need};
use super::ty::{FuncType, Type};
use super::{builtins, error, Checker, Func};

impl Checker {
    /// Infers `e` and records its type at its span.
    pub(super) fn expr(&mut self, e: &Expr) -> Result<Type> {
        let t = self.expr_kind(e)?;
        self.nodes.insert(e.span, t.clone());
        Ok(t)
    }

    fn expr_kind(&mut self, e: &Expr) -> Result<Type> {
        match &e.kind {
            ExprKind::Name(name) => self.variable(name, e.span),
            ExprKind::Int(_) => Ok(Type::Int),
            ExprKind::Float(_) => Ok(Type::Float),
            ExprKind::Str(_) => Ok(Type::Str),
            ExprKind::Bool(_) => Ok(Type::Bool),
            ExprKind::None => Ok(Type::None),
            ExprKind::Tuple(items) => {
                let types = items.iter().map(|x| self.expr(x)).collect::<Result<_>>()?;
                Ok(Type::Tuple(types))
            }
            ExprKind::List(items) => self.list(items),
            ExprKind::Unary { op, operand } => self.unary(*op, operand, e.span),
            ExprKind::Binary { op, left, right } => {
                let (l, r) = (self.expr(left)?, self.expr(right)?);
                let out = self.subst.fresh();
                let need = Need::Binary(Binary {
                    op: *op,
                    left: l,
                    right: r,
                    out: out.clone(),
                    right_literal: int_literal(right),
                    assigns: None,
                });
                self.need(need, e.span);
                Ok(out)
            }
            ExprKind::Logical { op, left, right } => self.logical(*op, left, right),
            ExprKind::Compare { left, rest } => self.compare(left, rest),
            ExprKind::IfExp { test, body, orelse } => {
                self.expr(test)?;
                let (b, o) = (self.expr(body)?, self.expr(orelse)?);
                let what = "both branches of a conditional must share one type";
                self.expect(orelse.span, &b, &o, what)?;
                Ok(b)
            }
            ExprKind::Call { func, args } => self.call(func, args, e.span),
            ExprKind::Subscript { value, index } => {
                let out = self.subst.fresh();
                self.subscript(value, index, Access::Read, out)
            }
            ExprKind::Attr { attr, .. } => {
                let msg = format!("methods can only be called: `.{}(...)`", attr.name);
                Err(error(attr.span, msg))
            }
        }
    }

    /// The type of variable `name`, recorded at `span`.
    pub(super) fn variable(&mut self, name: &str, span: Span) -> Result<Type> {
        let found = self
            .frame
            .narrowed
            .get(name)
            .or_else(|| self.frame.locals.get(name))
            .or_else(|| self.globals.get(name))
            .cloned();
        if let Some(t) = found {
            self.nodes.insert(span, t.clone());
            return Ok(t);
        }
        if self.funcs.contains_key(name) || builtins::is_builtin(name) {
            let msg = format!("`{name}` is a function; functions are not values yet");
            return Err(error(span, msg));
        }
        Err(error(span, format!("name `{name}` is not defined")))
    }

    fn list(&mut self, items: &[Expr]) -> Result<Type> {
        let elem = self.subst.fresh();
        for x in items {
            let t = self.expr(x)?;
            self.expect(x.span, &elem, &t, "list elements must share one type")?;
        }
        Ok(Type::List(Box::new(elem)))
    }

    fn unary(&mut self, op: UnaryOp, operand: &Expr, span: Span) -> Result<Type> {
        let t = self.expr(operand)?;
        if op == UnaryOp::Not {
            return Ok(Type::Bool);
        }
        let out = self.subst.fresh();
        let need = Need::Negate {
            operand: t,
            out: out.clone(),
        };
        self.need(need, span);
        Ok(out)
    }

    /// `and`/`or` return an operand, so both sides share one type.
    fn logical(&mut self, op: LogicalOp, left: &Expr, right: &Expr) -> Result<Type> {
        let (l, r) = (self.expr(left)?, self.expr(right)?);
        let word = match op {
            LogicalOp::And => "and",
            LogicalOp::Or => "or",
        };
        let what = format!("both sides of `{word}` must share one type");
        self.expect(right.span, &l, &r, &what)?;
        Ok(l)
    }

    /// Each operand is inferred once; each adjacent pair is checked.
    fn compare(&mut self, left: &Expr, rest: &[(CmpOp, Expr)]) -> Result<Type> {
        let mut prev = (self.expr(left)?, left);
        for (op, next) in rest {
            let t = self.expr(next)?;
            let span = prev.1.span.merge(next.span);
            if matches!(op, CmpOp::Is | CmpOp::IsNot) {
                is_none_operand(prev.1, next, span)?;
            } else {
                self.comparison(*op, &prev.0, &t, span);
            }
            prev = (t, next);
        }
        Ok(Type::Bool)
    }

    /// `==`, `<`, `in` and their kin. `is` is checked syntactically.
    fn comparison(&mut self, op: CmpOp, left: &Type, right: &Type, span: Span) {
        let (left, right) = (left.clone(), right.clone());
        let need = match op {
            CmpOp::Is | CmpOp::IsNot => return,
            CmpOp::Eq | CmpOp::NotEq => Need::Equal { left, right },
            CmpOp::Lt | CmpOp::LtEq | CmpOp::Gt | CmpOp::GtEq => Need::Order { left, right },
            CmpOp::In | CmpOp::NotIn => {
                let elem = self.subst.fresh();
                let iter = Need::Iter {
                    iterable: right,
                    elem: elem.clone(),
                };
                self.need(iter, span);
                Need::Equal { left, right: elem }
            }
        };
        self.need(need, span);
    }

    /// `value[index]` used as `access`; `out` is the element read, or
    /// the value written. Returns `out`.
    pub(super) fn subscript(
        &mut self,
        value: &Expr,
        index: &Expr,
        access: Access,
        out: Type,
    ) -> Result<Type> {
        let (v, i) = (self.expr(value)?, self.expr(index)?);
        let need = Need::Index {
            value: v,
            index: i,
            literal: int_literal(index),
            out: out.clone(),
            access,
        };
        self.need(need, index.span);
        Ok(out)
    }

    fn call(&mut self, func: &Expr, args: &[Arg], span: Span) -> Result<Type> {
        match &func.kind {
            ExprKind::Name(name) => self.call_name(name, func.span, args, span),
            ExprKind::Attr { value, attr } => self.method(value, attr, args),
            _ => Err(error(
                func.span,
                "only named functions can be called".to_string(),
            )),
        }
    }

    fn call_name(&mut self, name: &str, at: Span, args: &[Arg], span: Span) -> Result<Type> {
        let local = self
            .frame
            .locals
            .get(name)
            .or_else(|| self.globals.get(name));
        if let Some(t) = local {
            let msg = format!("`{name}` is {}, not a function", self.subst.zonk(t));
            return Err(error(at, msg));
        }
        let sig = match self.funcs.get(name).cloned() {
            Some(Func::Mono(sig)) => sig,
            Some(Func::Poly(scheme)) => self.instantiate(name, &scheme, span),
            None if builtins::is_builtin(name) => return self.builtin(name, args, span),
            None => return Err(error(at, format!("name `{name}` is not defined"))),
        };
        self.bind_args(name, &sig, args, span)?;
        Ok(sig.ret)
    }

    /// Python binding: positional arguments first, then keywords by name.
    fn bind_args(&mut self, name: &str, sig: &FuncType, args: &[Arg], span: Span) -> Result<()> {
        let given = args.iter().filter(|a| a.name.is_none()).count();
        let mut bound = vec![false; sig.params.len()];
        let mut next = Some(0);
        for arg in args {
            let slot = match &arg.name {
                Some(kw) => keyword_slot(name, sig, kw)?,
                None => positional_slot(name, sig, arg, next, given)?,
            };
            next = next.filter(|_| arg.name.is_none()).map(|n| n + 1);
            if bound[slot] {
                let param = &sig.params[slot].name;
                let msg = format!("`{name}` got multiple values for argument `{param}`");
                return Err(error(arg.span, msg));
            }
            bound[slot] = true;
            let param = &sig.params[slot];
            let t = self.checked(&arg.value, &param.ty)?;
            let what = format!("argument `{}` of `{name}`", param.name);
            self.expect(arg.value.span, &param.ty, &t, &what)?;
        }
        let missing = sig
            .params
            .iter()
            .zip(&bound)
            .find(|(p, b)| !**b && !p.has_default);
        match missing {
            Some((p, _)) => Err(error(
                span,
                format!("`{name}` is missing argument `{}`", p.name),
            )),
            None => Ok(()),
        }
    }

    /// `xs.append(v)`, the one phase-2 method.
    fn method(&mut self, value: &Expr, attr: &Ident, args: &[Arg]) -> Result<Type> {
        if attr.name != "append" {
            let msg = format!(
                "unknown method `.{}`; phase 2 has only `list.append`",
                attr.name
            );
            return Err(error(attr.span, msg));
        }
        let receiver = self.expr(value)?;
        let [arg] = args else {
            let msg = format!("`append` takes 1 argument but {} were given", args.len());
            return Err(error(attr.span, msg));
        };
        if arg.name.is_some() {
            return Err(error(
                arg.span,
                "`append` takes no keyword arguments".to_string(),
            ));
        }
        let item = self.expr(&arg.value)?;
        let elem = self.subst.fresh();
        let list = Type::List(Box::new(elem.clone()));
        self.expect(value.span, &list, &receiver, "`append` needs a list")?;
        let what = "appended value does not match the list";
        self.expect(arg.value.span, &elem, &item, what)?;
        Ok(Type::None)
    }

    /// Infers `e` against a known `expected` type, so list and tuple
    /// literals take their element types from it: with
    /// `xs: list[int | None] = [None, 1]` the elements need not agree.
    pub(super) fn checked(&mut self, e: &Expr, expected: &Type) -> Result<Type> {
        let items = match &e.kind {
            ExprKind::List(items) | ExprKind::Tuple(items) => items,
            _ => return self.expr(e),
        };
        let slots: Vec<Type> = match (&e.kind, self.subst.resolve(expected)) {
            (ExprKind::List(_), Type::List(elem)) => vec![*elem; items.len()],
            (ExprKind::Tuple(_), Type::Tuple(elems)) if elems.len() == items.len() => elems,
            _ => return self.expr(e),
        };
        let mut found = Vec::new();
        for (x, slot) in items.iter().zip(&slots) {
            let t = self.checked(x, slot)?;
            self.expect(x.span, slot, &t, "element does not match the annotation")?;
            found.push(t);
        }
        let t = match &e.kind {
            ExprKind::List(_) => expected.clone(),
            _ => Type::Tuple(found),
        };
        self.nodes.insert(e.span, t.clone());
        Ok(t)
    }

    /// The type an annotation spells, with its `| None` suffix.
    pub(super) fn annotation(&mut self, t: &TypeExpr) -> Result<Type> {
        let atom = self.annotation_atom(t)?;
        if !t.optional {
            return Ok(atom);
        }
        Ok(match atom {
            Type::None | Type::Optional(_) => atom,
            other => Type::Optional(Box::new(other)),
        })
    }

    fn annotation_atom(&mut self, t: &TypeExpr) -> Result<Type> {
        let args = t
            .args
            .iter()
            .map(|a| self.annotation(a))
            .collect::<Result<Vec<_>>>()?;
        let name = t.name.name.as_str();
        let scalar = match name {
            "int" => Type::Int,
            "float" => Type::Float,
            "str" => Type::Str,
            "bool" => Type::Bool,
            "None" => Type::None,
            "list" => return self.list_annotation(args, t.span),
            "tuple" if args.is_empty() => {
                let msg = "`tuple` needs its element types, as in `tuple[int, str]`";
                return Err(error(t.span, msg.to_string()));
            }
            "tuple" => return Ok(Type::Tuple(args)),
            _ => return Err(error(t.name.span, format!("unknown type `{name}`"))),
        };
        if !args.is_empty() {
            return Err(error(t.span, format!("`{name}` takes no type arguments")));
        }
        Ok(scalar)
    }

    fn list_annotation(&mut self, args: Vec<Type>, span: Span) -> Result<Type> {
        let mut args = args.into_iter();
        match (args.next(), args.next()) {
            (None, _) => Ok(Type::List(Box::new(self.subst.fresh()))),
            (Some(elem), None) => Ok(Type::List(Box::new(elem))),
            (Some(_), Some(_)) => Err(error(span, "`list` takes one type argument".to_string())),
        }
    }
}

/// The parameter a positional argument binds to. `next` is `None` once a
/// keyword argument has been seen.
fn positional_slot(
    name: &str,
    sig: &FuncType,
    arg: &Arg,
    next: Option<usize>,
    given: usize,
) -> Result<usize> {
    let Some(next) = next else {
        let msg = "positional argument follows keyword argument".to_string();
        return Err(error(arg.span, msg));
    };
    if next >= sig.params.len() {
        let n = sig.params.len();
        let msg = format!("`{name}` takes {n} arguments but {given} were given");
        return Err(error(arg.span, msg));
    }
    Ok(next)
}

fn keyword_slot(name: &str, sig: &FuncType, kw: &Ident) -> Result<usize> {
    match sig.params.iter().position(|p| p.name == kw.name) {
        Some(slot) => Ok(slot),
        None => {
            let msg = format!("`{name}` got an unexpected keyword argument `{}`", kw.name);
            Err(error(kw.span, msg))
        }
    }
}

/// Phase 2 allows `is` only with a `None` literal on one side.
fn is_none_operand(left: &Expr, right: &Expr, span: Span) -> Result<()> {
    if matches!(left.kind, ExprKind::None) || matches!(right.kind, ExprKind::None) {
        return Ok(());
    }
    let msg = "`is` compares only with None in phase 2; use `==` for values";
    Err(error(span, msg.to_string()))
}

/// An index usable for a tuple: `2` or `-1`.
pub(super) fn int_literal(e: &Expr) -> Option<i64> {
    match &e.kind {
        ExprKind::Int(n) => Some(*n),
        ExprKind::Unary {
            op: UnaryOp::Neg,
            operand,
        } => match operand.kind {
            ExprKind::Int(n) => n.checked_neg(),
            _ => None,
        },
        _ => None,
    }
}
