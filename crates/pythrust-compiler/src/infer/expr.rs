//! Expressions, calls and annotations.

use crate::ast::{Arg, CmpOp, Expr, ExprKind, Ident, LogicalOp, TypeExpr, UnaryOp};
use crate::diag::Result;
use crate::span::Span;

use super::scope;
use super::solve::{Access, Binary, Need};
use super::ty::{FuncType, Type, View};
use super::{builtins, error, Checker, Func, TruthUse};

/// Why `errs = (ValueError, IndexError)` and `except errs:` are errors.
pub(super) const CLASS_NOT_A_VALUE: &str = "exception classes can't be held in variables; \
     name them where they are used, as in `except (ValueError, IndexError):`";

/// How `checked` matches a part of a display against its slot: a dict
/// key or set element as a key (a `bool` fits an `int` key), anything
/// else as a value.
#[derive(Clone, Copy)]
enum Slot {
    Key,
    Element,
}

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
            ExprKind::Dict(items) => self.dict(items, e.span),
            ExprKind::Set(items) => self.set(items, e.span),
            ExprKind::Unary { op, operand } => self.unary(*op, operand, e.span),
            ExprKind::Binary { op, left, right } => {
                let (l, r) = (self.expr(left)?, self.expr(right)?);
                let out = self.subst.fresh();
                let need = Binary {
                    op: *op,
                    left: l,
                    right: r,
                    out: out.clone(),
                    right_literal: int_literal(right),
                    assigns: None,
                };
                self.truth_use(left, e.span, |t| {
                    Need::Binary(Binary {
                        left: t,
                        ..need.clone()
                    })
                });
                self.truth_use(right, e.span, |t| {
                    Need::Binary(Binary {
                        right: t,
                        ..need.clone()
                    })
                });
                self.need(Need::Binary(need), e.span);
                Ok(out)
            }
            ExprKind::Logical { op, left, right } => self.logical(*op, left, right),
            ExprKind::Compare { left, rest } => self.compare(left, rest),
            ExprKind::IfExp { test, body, orelse } => {
                self.expr(test)?;
                let b = self.narrowed(&scope::not_none(test, true), &[], |c| c.expr(body))?;
                let o = self.narrowed(&scope::not_none(test, false), &[], |c| c.expr(orelse))?;
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

    /// The type `e` is already known to have, without inferring it: a
    /// variable, or an element of one (`xss[0]`). Gives assignment
    /// targets their expected type.
    pub(super) fn known_type(&self, e: &Expr) -> Option<Type> {
        match &e.kind {
            ExprKind::Name(name) => self.lookup(name),
            ExprKind::Subscript { value, .. } => self.element_of(value),
            _ => None,
        }
    }

    /// The element type of `container` when it is a known list, or the
    /// value type when it is a known dict.
    pub(super) fn element_of(&self, container: &Expr) -> Option<Type> {
        match self.subst.resolve(&self.known_type(container)?) {
            Type::List(elem) | Type::Dict(_, elem) => Some(*elem),
            _ => None,
        }
    }

    /// The type of variable `name` here, if it has one.
    pub(super) fn lookup(&self, name: &str) -> Option<Type> {
        self.frame
            .narrowed
            .get(name)
            .or_else(|| self.frame.locals.get(name))
            .or_else(|| self.globals.get(name))
            .cloned()
    }

    /// The type of variable `name`, recorded at `span`. A builtin
    /// exception class is not a value: it is called, raised or caught.
    pub(super) fn variable(&mut self, name: &str, span: Span) -> Result<Type> {
        if let Some(t) = self.lookup(name) {
            self.nodes.insert(span, t.clone());
            return Ok(t);
        }
        if self.is_exception_name(name) {
            return Err(error(span, CLASS_NOT_A_VALUE.to_string()));
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

    /// `{k: v, ...}`: keys share one hashable type, values another.
    fn dict(&mut self, items: &[(Expr, Expr)], span: Span) -> Result<Type> {
        let (key, value) = (self.subst.fresh(), self.subst.fresh());
        for (k, v) in items {
            let t = self.expr(k)?;
            self.expect(k.span, &key, &t, "dict keys must share one type")?;
            let t = self.expr(v)?;
            self.expect(v.span, &value, &t, "dict values must share one type")?;
        }
        self.need(Need::Hashable(key.clone()), span);
        Ok(Type::Dict(Box::new(key), Box::new(value)))
    }

    /// `{a, ...}`: elements share one hashable type.
    fn set(&mut self, items: &[Expr], span: Span) -> Result<Type> {
        let elem = self.subst.fresh();
        for x in items {
            let t = self.expr(x)?;
            self.expect(x.span, &elem, &t, "set elements must share one type")?;
        }
        self.need(Need::Hashable(elem.clone()), span);
        Ok(Type::Set(Box::new(elem)))
    }

    fn unary(&mut self, op: UnaryOp, operand: &Expr, span: Span) -> Result<Type> {
        let t = self.expr(operand)?;
        if op == UnaryOp::Not {
            return Ok(Type::Bool);
        }
        let out = self.subst.fresh();
        let negate = |operand: Type| Need::Negate {
            operand,
            out: out.clone(),
        };
        self.truth_use(operand, span, negate);
        self.need(negate(t), span);
        Ok(out)
    }

    /// Records the constraint `narrow` builds at `span` when `operand`
    /// reads a name tested only for truth that is still `T | None`, with
    /// the present type in its place: if that solves, the error at `span`
    /// can suggest `x is not None`.
    fn truth_use(&mut self, operand: &Expr, span: Span, narrow: impl FnOnce(Type) -> Need) {
        let ExprKind::Name(name) = &operand.kind else {
            return;
        };
        if !self.frame.truthy.contains(name) {
            return;
        }
        let Some(Type::Optional(present)) = self.lookup(name).map(|t| self.subst.resolve(&t))
        else {
            return;
        };
        self.frame.truth_uses.push(TruthUse {
            span,
            name: name.clone(),
            narrowed: narrow(*present),
        });
    }

    /// `and`/`or` return an operand, so both sides share one type. The
    /// right side runs only when the left holds (`and`) or fails (`or`),
    /// so it sees what that proves (`x is None or k > x`).
    fn logical(&mut self, op: LogicalOp, left: &Expr, right: &Expr) -> Result<Type> {
        let l = self.expr(left)?;
        let (word, holds) = match op {
            LogicalOp::And => ("and", true),
            LogicalOp::Or => ("or", false),
        };
        let r = self.narrowed(&scope::not_none(left, holds), &[], |c| c.expr(right))?;
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
                prev = (t, next);
                continue;
            }
            if matches!(op, CmpOp::Lt | CmpOp::LtEq | CmpOp::Gt | CmpOp::GtEq) {
                let right = t.clone();
                self.truth_use(prev.1, span, |left| Need::Order { left, right });
                let left = prev.0.clone();
                self.truth_use(next, span, |right| Need::Order { left, right });
            }
            self.comparison(*op, &prev.0, &t, span);
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
                    iterable: right.clone(),
                    elem: elem.clone(),
                };
                self.need(iter, span);
                let probe = Need::Probe {
                    container: right,
                    item: left.clone(),
                };
                self.need(probe, span);
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
        if let Some(class) = self.stub_redefined.get(name) {
            let msg = format!(
                "`{name}` raises the builtin `{class}`, which this program redefines, \
                 so it can be neither declared nor caught; rename the program's `{class}`"
            );
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
    pub(super) fn bind_args(
        &mut self,
        name: &str,
        sig: &FuncType,
        args: &[Arg],
        span: Span,
    ) -> Result<()> {
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

    /// A method call: `list.append`, `set.add`, `dict.get`, `.keys`,
    /// `.values`, `.items`, and a file's `.read` and `.close`. Each name
    /// belongs to one type, so the name alone fixes the receiver's.
    fn method(&mut self, value: &Expr, attr: &Ident, args: &[Arg]) -> Result<Type> {
        let name = attr.name.as_str();
        let (min, max) = match name {
            "append" | "add" => (1, 1),
            "get" => (1, 2),
            "keys" | "values" | "items" | "read" | "close" | "result" => (0, 0),
            "submit" | "map" => return self.task(value, attr, args),
            _ => {
                let msg = format!(
                    "unknown method `.{name}`; the methods are `list.append`, `set.add`, \
                     `dict.get`, `.keys`, `.values` and `.items`, a file's `.read` \
                     and `.close`, an executor's `.submit` and `.map`, and a future's \
                     `.result`"
                );
                return Err(error(attr.span, msg));
            }
        };
        let receiver = self.expr(value)?;
        method_arity(name, args, min, max, attr.span)?;
        match name {
            "append" | "add" => self.insert(name, value.span, &receiver, &args[0]),
            "get" => self.get(value.span, &receiver, args, attr.span),
            "read" | "close" => {
                let needs = format!("`{name}` needs a file from `open`");
                self.expect(value.span, &Type::File, &receiver, &needs)?;
                Ok(if name == "read" {
                    Type::Bytes
                } else {
                    Type::None
                })
            }
            "result" => self.result(value.span, &receiver),
            _ => self.view(name, value.span, &receiver),
        }
    }

    /// `xs.append(v)` or `s.add(v)`: `v` flows into the element type.
    fn insert(&mut self, name: &str, at: Span, receiver: &Type, arg: &Arg) -> Result<Type> {
        let elem = self.subst.fresh();
        let (container, needs) = match name {
            "append" => (Type::List(Box::new(elem.clone())), "`append` needs a list"),
            _ => (Type::Set(Box::new(elem.clone())), "`add` needs a set"),
        };
        self.expect(at, &container, receiver, needs)?;
        let item = self.checked(&arg.value, &elem)?;
        if name == "add" {
            let need = Need::Key {
                key: elem,
                index: item,
            };
            self.need(need, arg.value.span);
            return Ok(Type::None);
        }
        let what = "appended value does not match the list";
        self.expect(arg.value.span, &elem, &item, what)?;
        Ok(Type::None)
    }

    /// `d.get(k)` is `V | None`; `d.get(k, default)` is `V`.
    fn get(&mut self, at: Span, receiver: &Type, args: &[Arg], span: Span) -> Result<Type> {
        let (key, value) = (self.subst.fresh(), self.subst.fresh());
        let dict = Type::Dict(Box::new(key.clone()), Box::new(value.clone()));
        self.expect(at, &dict, receiver, "`get` needs a dict")?;
        let k = self.checked(&args[0].value, &key)?;
        self.need(Need::Key { key, index: k }, args[0].value.span);
        if let Some(default) = args.get(1) {
            let d = self.checked(&default.value, &value)?;
            let what = "the default of `get` must match the dict's values";
            self.expect(default.value.span, &value, &d, what)?;
            return Ok(value);
        }
        let out = self.subst.fresh();
        let need = Need::OrNone {
            value,
            out: out.clone(),
        };
        self.need(need, span);
        Ok(out)
    }

    /// `d.keys()`, `d.values()` or `d.items()`: a view of the keys,
    /// values or `(key, value)` pairs, in insertion order.
    fn view(&mut self, name: &str, at: Span, receiver: &Type) -> Result<Type> {
        let (key, value) = (self.subst.fresh(), self.subst.fresh());
        let dict = Type::Dict(Box::new(key.clone()), Box::new(value.clone()));
        let needs = format!("`{name}` needs a dict");
        self.expect(at, &dict, receiver, &needs)?;
        let (view, item) = match name {
            "keys" => (View::Keys, key),
            "values" => (View::Values, value),
            _ => (View::Items, Type::Tuple(vec![key, value])),
        };
        Ok(Type::View(view, Box::new(item)))
    }

    /// Infers `e` against a known `expected` type, so list and tuple
    /// literals take their element types from it: with
    /// `xs: list[int | None] = [None, 1]` the elements need not agree.
    /// Under `T | None` a literal is checked against `T`, and keeps that
    /// type (it is not `None`).
    pub(super) fn checked(&mut self, e: &Expr, expected: &Type) -> Result<Type> {
        let resolved = self.subst.resolve(expected);
        let slots: Vec<(&Expr, Type, Slot)> = match (&e.kind, &resolved) {
            (ExprKind::List(_) | ExprKind::Tuple(_), Type::Optional(inner))
            | (ExprKind::Dict(_) | ExprKind::Set(_), Type::Optional(inner)) => {
                return self.checked(e, inner)
            }
            (ExprKind::List(items), Type::List(elem)) => items
                .iter()
                .map(|x| (x, (**elem).clone(), Slot::Element))
                .collect(),
            (ExprKind::Set(items), Type::Set(elem)) => items
                .iter()
                .map(|x| (x, (**elem).clone(), Slot::Key))
                .collect(),
            (ExprKind::Dict(items), Type::Dict(key, value)) => items
                .iter()
                .flat_map(|(k, v)| {
                    [
                        (k, (**key).clone(), Slot::Key),
                        (v, (**value).clone(), Slot::Element),
                    ]
                })
                .collect(),
            (ExprKind::Tuple(items), Type::Tuple(elems)) if elems.len() == items.len() => items
                .iter()
                .zip(elems.iter().cloned())
                .map(|(x, t)| (x, t, Slot::Element))
                .collect(),
            _ => return self.expr(e),
        };
        let mut found = Vec::new();
        for (x, slot, kind) in slots {
            let t = self.checked(x, &slot)?;
            match kind {
                Slot::Key => self.need(
                    Need::Key {
                        key: slot,
                        index: t.clone(),
                    },
                    x.span,
                ),
                Slot::Element => {
                    self.expect(x.span, &slot, &t, "element does not match the annotation")?
                }
            }
            found.push(t);
        }
        let t = match (&e.kind, resolved) {
            (ExprKind::Tuple(_), _) => Type::Tuple(found),
            (ExprKind::Dict(_), Type::Dict(key, _)) | (ExprKind::Set(_), Type::Set(key)) => {
                self.need(Need::Hashable(*key), e.span);
                expected.clone()
            }
            _ => expected.clone(),
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
            "Exception" => Type::Exception,
            "bytes" => Type::Bytes,
            "list" => return self.list_annotation(args, t.span),
            "dict" | "set" => return self.container_annotation(name, args, t.span),
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

    /// `dict[K, V]` or `set[T]`, or bare `dict` / `set` with fresh
    /// arguments. The key or element must be hashable.
    fn container_annotation(&mut self, name: &str, args: Vec<Type>, span: Span) -> Result<Type> {
        let arity = if name == "dict" { 2 } else { 1 };
        let mut args = match args.len() {
            0 => (0..arity).map(|_| self.subst.fresh()).collect(),
            n if n == arity => args,
            _ => {
                let msg = match name {
                    "dict" => "`dict` takes two type arguments, as in `dict[str, int]`",
                    _ => "`set` takes one type argument",
                };
                return Err(error(span, msg.to_string()));
            }
        };
        let key = args.remove(0);
        self.need(Need::Hashable(key.clone()), span);
        Ok(match args.pop() {
            Some(value) => Type::Dict(Box::new(key), Box::new(value)),
            None => Type::Set(Box::new(key)),
        })
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

/// Checks a method call's argument count; methods take no keywords.
fn method_arity(name: &str, args: &[Arg], min: usize, max: usize, span: Span) -> Result<()> {
    if let Some(arg) = args.iter().find(|a| a.name.is_some()) {
        let msg = format!("`{name}` takes no keyword arguments");
        return Err(error(arg.span, msg));
    }
    if (min..=max).contains(&args.len()) {
        return Ok(());
    }
    let expected = match (min, max) {
        (0, 0) => "no arguments".to_string(),
        (1, 1) => "1 argument".to_string(),
        _ => format!("{min} or {max} arguments"),
    };
    let msg = format!("`{name}` takes {expected} but {} were given", args.len());
    Err(error(span, msg))
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
