//! Expressions, calls and annotations.

use std::collections::BTreeSet;

use crate::ast::{
    source_name, Arg, CmpOp, CompFor, Expr, ExprKind, Gather, Ident, LogicalOp, TypeExpr, UnaryOp,
};
use crate::diag::Result;
use crate::span::Span;

use super::scope;
use super::solve::{Access, Binary, Need};
use super::ty::{FuncType, Type};
use super::{builtins, error, Checker, Func, TruthUse};

/// Why `errs = (ValueError, IndexError)` and `except errs:` are errors.
pub(super) const CLASS_NOT_A_VALUE: &str = "exception classes can't be held in variables; \
     name them where they are used, as in `except (ValueError, IndexError):`";

/// D121: a generator expression anywhere but a reducer's argument.
const NOT_VALUE: &str =
    "a generator expression is not a value yet; use a list comprehension `[...]`";

/// How `checked` matches a part of a display against its slot: a dict
/// key or set element as a key (a `bool` fits an `int` key), anything
/// else as a value.
#[derive(Clone, Copy)]
enum Slot {
    Key,
    Element,
}

/// What a comprehension gathers, the type expected where it goes (if
/// any, D117), and its span: threaded through its clauses unchanged.
#[derive(Clone, Copy)]
struct Gathering<'a> {
    gather: &'a Gather,
    expected: Option<&'a Type>,
    span: Span,
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
            ExprKind::Slice {
                value,
                start,
                stop,
                step,
            } => {
                let receiver = self.expr(value)?;
                for x in [start, stop, step].into_iter().flatten() {
                    let t = self.expr(x)?;
                    self.need(Need::SliceBound(t), x.span);
                }
                let out = self.subst.fresh();
                let bounds = [start, stop, step].map(|x| x.as_deref().map(slice_literal));
                self.need(
                    Need::Slice {
                        value: receiver,
                        bounds,
                        out: out.clone(),
                    },
                    e.span,
                );
                Ok(out)
            }
            ExprKind::Attr { value, attr } => self.field(value, attr),
            ExprKind::Comp {
                gather: Gather::Gen(_),
                ..
            } if self.reduced != Some(e.span) => Err(error(e.span, NOT_VALUE.to_string())),
            ExprKind::Comp { gather, clauses } => {
                let g = Gathering {
                    gather,
                    expected: None,
                    span: e.span,
                };
                self.comp_clauses(g, clauses)
            }
        }
    }

    /// The type of the comprehension at `span` (D85, D116), typed as the
    /// loop it stands for: each clause binds its target as `for` does.
    /// Rebinding a name ends its narrowing.
    fn comp_clauses(&mut self, g: Gathering, clauses: &[CompFor]) -> Result<Type> {
        let Some((c, rest)) = clauses.split_first() else {
            return self.gathered(g);
        };
        let iterable = self.consumed(&c.iter)?;
        let elem = self.subst.fresh();
        let need = Need::Iter {
            iterable,
            elem: elem.clone(),
        };
        self.need(need, c.iter.span);
        self.bind(&c.target, &elem)?;
        let mut bound = BTreeSet::new();
        scope::target_names(&c.target, &mut bound);
        let saved = self.frame.narrowed.clone();
        self.frame.narrowed.retain(|n, _| !bound.contains(n));
        let out = self.comp_filters(&c.ifs, g, rest);
        self.frame.narrowed = saved;
        out
    }

    /// What `g` makes, as a display of its one item would: the expected
    /// type when there is one of its shape (D117), each item checked
    /// against its slot as `checked` checks a display's.
    fn gathered(&mut self, g: Gathering) -> Result<Type> {
        let Some(expected) = g.expected else {
            return self.inferred(g.gather, g.span);
        };
        match (g.gather, expected) {
            (Gather::List(e), Type::List(elem)) => {
                self.slot(e, elem, Slot::Element)?;
            }
            (Gather::Set(e), Type::Set(elem)) => {
                self.slot(e, elem, Slot::Key)?;
                self.need(Need::Hashable((**elem).clone()), g.span);
            }
            (Gather::Dict(k, v), Type::Dict(key, value)) => {
                self.slot(k, key, Slot::Key)?;
                self.slot(v, value, Slot::Element)?;
                self.need(Need::Hashable((**key).clone()), g.span);
            }
            _ => return self.inferred(g.gather, g.span),
        }
        Ok(expected.clone())
    }

    /// What `gather` makes with nothing expected: a list, a set of one
    /// hashable type, or a dict with hashable keys.
    /// A generator expression is typed as the list comprehension in its
    /// slot (D122).
    fn inferred(&mut self, gather: &Gather, span: Span) -> Result<Type> {
        match gather {
            Gather::List(e) | Gather::Gen(e) => Ok(Type::List(Box::new(self.expr(e)?))),
            Gather::Set(e) => self.set(std::slice::from_ref(e), span),
            Gather::Dict(k, v) => {
                let key = self.expr(k)?;
                let value = self.expr(v)?;
                self.need(Need::Hashable(key.clone()), span);
                Ok(Type::Dict(Box::new(key), Box::new(value)))
            }
        }
    }

    /// A comprehension's filters, each narrowing what follows it as an
    /// `if` test narrows its block, then its later clauses.
    fn comp_filters(&mut self, ifs: &[Expr], g: Gathering, rest: &[CompFor]) -> Result<Type> {
        let Some((test, more)) = ifs.split_first() else {
            return self.comp_clauses(g, rest);
        };
        self.expr(test)?;
        let saved = (self.frame.truthy.clone(), self.frame.truth_uses.len());
        self.frame.truthy.extend(scope::truth_tested(test));
        let inside = scope::not_none(test, true);
        // Settled here, as a block settles each statement, so an error
        // on a name tested only for truth gets its hint.
        let out = self
            .narrowed(&inside, &[], |c| {
                let t = c.comp_filters(more, g, rest)?;
                c.settle()?;
                Ok(t)
            })
            .map_err(|e| self.truth_hint(e));
        self.frame.truthy = saved.0;
        self.frame.truth_uses.truncate(saved.1);
        out
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
        if self.is_record(name) {
            let msg =
                format!("`{name}` is a record, not a value; construct one with `{name}(...)`");
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
            ExprKind::Attr { value, attr } => self.method(value, attr, args, span),
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
            let name = source_name(name);
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
        if self.is_record(name) {
            return self.construct(name, args, span);
        }
        if self.classes.user(name).is_some() {
            self.positional(source_name(name), args, 0, 1, span)?;
            return Ok(Type::Exception);
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

    /// Infers `e` against a known `expected` type, so list and tuple
    /// literals take their element types from it: with
    /// `xs: list[int | None] = [None, 1]` the elements need not agree.
    /// Under `T | None` a literal is checked against `T`, and keeps that
    /// type (it is not `None`).
    pub(super) fn checked(&mut self, e: &Expr, expected: &Type) -> Result<Type> {
        let resolved = self.subst.resolve(expected);
        let slots: Vec<(&Expr, Type, Slot)> = match (&e.kind, &resolved) {
            (ExprKind::List(_) | ExprKind::Tuple(_), Type::Optional(inner))
            | (ExprKind::Dict(_) | ExprKind::Set(_), Type::Optional(inner))
            | (ExprKind::Comp { .. }, Type::Optional(inner)) => return self.checked(e, inner),
            (ExprKind::Comp { gather, clauses }, _) if !matches!(gather, Gather::Gen(_)) => {
                let g = Gathering {
                    gather,
                    expected: Some(&resolved),
                    span: e.span,
                };
                let t = self.comp_clauses(g, clauses)?;
                self.nodes.insert(e.span, t.clone());
                return Ok(t);
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
            found.push(self.slot(x, &slot, kind)?);
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

    /// `x` checked against the type its slot expects: a key needs only
    /// to fit it (a `bool` fits `int`, D20), an element must match it.
    fn slot(&mut self, x: &Expr, slot: &Type, kind: Slot) -> Result<Type> {
        let t = self.checked(x, slot)?;
        match kind {
            Slot::Key => self.need(
                Need::Key {
                    key: slot.clone(),
                    index: t.clone(),
                },
                x.span,
            ),
            Slot::Element => {
                self.expect(x.span, slot, &t, "element does not match the annotation")?
            }
        }
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
            _ if self.is_record(name) => Type::Record(name.to_string()),
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

/// Signed literal syntax for fixed positional tuple slices.
pub(super) fn slice_literal(e: &Expr) -> Option<i64> {
    if let ExprKind::Unary {
        op: UnaryOp::Pos,
        operand,
    } = &e.kind
    {
        return match operand.kind {
            ExprKind::Int(n) => Some(n),
            _ => None,
        };
    }
    int_literal(e)
}
