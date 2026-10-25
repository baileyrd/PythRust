//! Deferred constraints: operations whose typing depends on an operand's
//! head type (`+` on ints versus strs, `len`, iteration, indexing, ...).
//!
//! A constraint waits until the heads it needs are known. One still
//! waiting when its function is generalized becomes part of the function's
//! scheme and is re-checked, with fresh variables, at every call.

use std::collections::{BTreeSet, HashMap};

use crate::ast::BinOp;
use crate::diag::{DiagKind, Diagnostic, Result};
use crate::span::Span;

use super::subst::{Clash, Subst};
use super::ty::{Type, TypeVar};

/// What a constraint requires.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Need {
    Binary(Binary),
    /// Unary `-x` or `+x` has type `out`.
    Negate {
        operand: Type,
        out: Type,
    },
    /// `==` / `!=`: the sides can be compared.
    Equal {
        left: Type,
        right: Type,
    },
    /// `<` `<=` `>` `>=`: the sides can be ordered.
    Order {
        left: Type,
        right: Type,
    },
    /// `len(x)`.
    Len(Type),
    /// Iterating `iterable` yields `elem` (`for`, `in`).
    Iter {
        iterable: Type,
        elem: Type,
    },
    /// `value[index]` is `out` (read) or receives `out` (write).
    /// `literal` is the index when it is an integer literal (required for
    /// tuples).
    Index {
        value: Type,
        index: Type,
        literal: Option<i64>,
        out: Type,
        access: Access,
    },
    /// Unpacking `value` into one target per `elems` entry.
    Unpack {
        value: Type,
        elems: Vec<Type>,
    },
    /// `int(x)` / `float(x)`, named by `func`.
    ToNumber {
        func: &'static str,
        arg: Type,
    },
    /// `value` flows into `target` (see `Subst::assign`), waiting while
    /// the value is unknown under a `T | None` target. `what` heads the
    /// error.
    Assign {
        target: Type,
        value: Type,
        what: String,
    },
}

/// How a subscript is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Access {
    Read,
    /// `xs[i] = v`: `v` flows into the element.
    Write,
    /// `xs[i] += v`: read, then written back with the same type.
    ReadWrite,
}

/// `left op right` has type `out`.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Binary {
    pub op: BinOp,
    pub left: Type,
    pub right: Type,
    pub out: Type,
    /// The right operand when it is an integer literal: a negative one
    /// under `**` with an int base is rejected at compile time.
    pub right_literal: Option<i64>,
    /// The variable an augmented assignment writes the result back to.
    pub assigns: Option<String>,
}

/// A requirement at a source span. `via` names the function whose scheme
/// it was instantiated from, for diagnostics at the call site.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Constraint {
    pub need: Need,
    pub span: Span,
    pub via: Option<String>,
}

enum Step {
    /// Solved, possibly producing smaller constraints.
    Done(Vec<Need>),
    Waiting,
}

/// Solves until no constraint makes progress. Returns those still waiting.
pub(super) fn solve(s: &mut Subst, mut pending: Vec<Constraint>) -> Result<Vec<Constraint>> {
    loop {
        let mut waiting = Vec::new();
        let mut progressed = false;
        for c in pending {
            match step(s, &c)? {
                Step::Waiting => waiting.push(c),
                Step::Done(more) => {
                    progressed = true;
                    waiting.extend(
                        more.into_iter()
                            .map(|need| Constraint { need, ..c.clone() }),
                    );
                }
            }
        }
        if !progressed {
            return Ok(waiting);
        }
        pending = waiting;
    }
}

/// Unbound variables a constraint mentions.
pub(super) fn constraint_vars(s: &Subst, c: &Constraint) -> BTreeSet<TypeVar> {
    let mut out = BTreeSet::new();
    for ty in need_types(&c.need) {
        s.free_vars(ty, &mut out);
    }
    out
}

/// Copies a constraint, replacing the variables in `map`.
pub(super) fn rename(s: &Subst, need: &Need, map: &HashMap<TypeVar, Type>) -> Need {
    let r = |t: &Type| s.rename(t, map);
    match need {
        Need::Binary(b) => Need::Binary(Binary {
            left: r(&b.left),
            right: r(&b.right),
            out: r(&b.out),
            ..b.clone()
        }),
        Need::Negate { operand, out } => Need::Negate {
            operand: r(operand),
            out: r(out),
        },
        Need::Equal { left, right } => Need::Equal {
            left: r(left),
            right: r(right),
        },
        Need::Order { left, right } => Need::Order {
            left: r(left),
            right: r(right),
        },
        Need::Len(t) => Need::Len(r(t)),
        Need::Iter { iterable, elem } => Need::Iter {
            iterable: r(iterable),
            elem: r(elem),
        },
        Need::Index {
            value,
            index,
            literal,
            out,
            access,
        } => Need::Index {
            value: r(value),
            index: r(index),
            literal: *literal,
            out: r(out),
            access: *access,
        },
        Need::Unpack { value, elems } => Need::Unpack {
            value: r(value),
            elems: elems.iter().map(r).collect(),
        },
        Need::ToNumber { func, arg } => Need::ToNumber { func, arg: r(arg) },
        Need::Assign {
            target,
            value,
            what,
        } => Need::Assign {
            target: r(target),
            value: r(value),
            what: what.clone(),
        },
    }
}

fn need_types(need: &Need) -> Vec<&Type> {
    match need {
        Need::Binary(b) => vec![&b.left, &b.right, &b.out],
        Need::Negate { operand, out } => vec![operand, out],
        Need::Equal { left, right } | Need::Order { left, right } => vec![left, right],
        Need::Len(t) | Need::ToNumber { arg: t, .. } => vec![t],
        Need::Iter { iterable, elem } => vec![iterable, elem],
        Need::Assign { target, value, .. } => vec![target, value],
        Need::Index {
            value, index, out, ..
        } => vec![value, index, out],
        Need::Unpack { value, elems } => std::iter::once(value).chain(elems).collect(),
    }
}

fn step(s: &mut Subst, c: &Constraint) -> Result<Step> {
    match &c.need {
        Need::Binary(b) => binary(s, c, b),
        Need::Negate { operand, out } => negate(s, c, operand, out),
        Need::Equal { left, right } => equal(s, c, left, right),
        Need::Order { left, right } => order(s, c, left, right),
        Need::Len(t) => len(s, c, t),
        Need::Iter { iterable, elem } => iter(s, c, iterable, elem),
        Need::Index {
            value,
            index,
            literal,
            out,
            access,
        } => index_of(s, c, value, index, *literal, out, *access),
        Need::Unpack { value, elems } => unpack(s, c, value, elems),
        Need::ToNumber { func, arg } => to_number(s, c, func, arg),
        Need::Assign {
            target,
            value,
            what,
        } => assign(s, c, target, value, what),
    }
}

fn binary(s: &mut Subst, c: &Constraint, b: &Binary) -> Result<Step> {
    let (Some(l), Some(r)) = (known(s, &b.left), known(s, &b.right)) else {
        return Ok(Step::Waiting);
    };
    let op = b.op;
    let negative_power = op == BinOp::Pow && b.right_literal.is_some_and(|n| n < 0);
    if negative_power && l.is_intlike() {
        let msg =
            "negative exponent on an int base gives a float; use a float base, as in `2.0 ** -1`";
        return Err(error(c, msg.to_string()));
    }
    let result = match (&l, &r) {
        _ if l.is_numeric() && r.is_numeric() => numeric_result(op, &l, &r),
        (Type::Str, Type::Str) if op == BinOp::Add => Type::Str,
        (Type::List(a), Type::List(b)) if op == BinOp::Add => {
            expect(s, c, a, b)?;
            l.clone()
        }
        (Type::Tuple(a), Type::Tuple(b)) if op == BinOp::Add => {
            Type::Tuple(a.iter().chain(b).cloned().collect())
        }
        (Type::Str | Type::List(_), n) | (n, Type::Str | Type::List(_))
            if op == BinOp::Mul && n.is_intlike() =>
        {
            if l.is_intlike() {
                r.clone()
            } else {
                l.clone()
            }
        }
        _ => {
            let msg = format!(
                "unsupported operand types for {}: {} and {}",
                op_symbol(op),
                s.zonk(&l),
                s.zonk(&r)
            );
            return Err(error(c, msg));
        }
    };
    if s.unify(&b.out, &result).is_err() {
        let msg = result_mismatch(s, b, &result);
        return Err(error(c, msg));
    }
    Ok(Step::Done(vec![]))
}

fn result_mismatch(s: &Subst, b: &Binary, result: &Type) -> String {
    let (expected, found) = (s.zonk(&b.out), s.zonk(result));
    let symbol = op_symbol(b.op);
    let Some(name) = &b.assigns else {
        return format!("`{symbol}` gives {found} here, but {expected} is expected");
    };
    let mut msg = format!("`{name}` is {expected} here, but `{symbol}=` makes it {found}");
    if expected == Type::Int && found == Type::Float {
        msg.push_str(&format!("; start `{name}` as a float, such as `0.0`"));
    }
    msg
}

/// Python's numeric tower, with `bool` acting as `int`. `int ** int` is
/// `int`; a negative exponent is a compile error when it is a literal and
/// a runtime error otherwise (a documented deviation).
fn numeric_result(op: BinOp, l: &Type, r: &Type) -> Type {
    let float = *l == Type::Float || *r == Type::Float;
    if op == BinOp::Div || float {
        return Type::Float;
    }
    Type::Int
}

fn negate(s: &mut Subst, c: &Constraint, operand: &Type, out: &Type) -> Result<Step> {
    let Some(t) = known(s, operand) else {
        return Ok(Step::Waiting);
    };
    let result = match t {
        Type::Int | Type::Bool => Type::Int,
        Type::Float => Type::Float,
        other => {
            let msg = format!("bad operand type for unary -: {}", s.zonk(&other));
            return Err(error(c, msg));
        }
    };
    expect(s, c, out, &result)?;
    Ok(Step::Done(vec![]))
}

fn equal(s: &mut Subst, c: &Constraint, l: &Type, r: &Type) -> Result<Step> {
    let (Some(l), Some(r)) = (known(s, l), known(s, r)) else {
        return Ok(Step::Waiting);
    };
    if l.is_numeric() && r.is_numeric() {
        return Ok(Step::Done(vec![]));
    }
    match (&l, &r) {
        (Type::Optional(_), Type::None) | (Type::None, Type::Optional(_)) => {
            return Ok(Step::Done(vec![]))
        }
        (Type::Optional(t), other) | (other, Type::Optional(t)) => {
            let inner = Need::Equal {
                left: (**t).clone(),
                right: other.clone(),
            };
            return Ok(Step::Done(vec![inner]));
        }
        _ => {}
    }
    if s.unify(&l, &r).is_err() {
        let msg = format!(
            "cannot compare {} with {}: they are never equal",
            s.zonk(&l),
            s.zonk(&r)
        );
        return Err(error(c, msg));
    }
    Ok(Step::Done(vec![]))
}

fn order(s: &mut Subst, c: &Constraint, l: &Type, r: &Type) -> Result<Step> {
    let (Some(l), Some(r)) = (known(s, l), known(s, r)) else {
        return Ok(Step::Waiting);
    };
    let pair = |left: &Type, right: &Type| Need::Order {
        left: left.clone(),
        right: right.clone(),
    };
    match (&l, &r) {
        _ if l.is_numeric() && r.is_numeric() => Ok(Step::Done(vec![])),
        (Type::Str, Type::Str) => Ok(Step::Done(vec![])),
        (Type::List(a), Type::List(b)) => {
            expect(s, c, a, b)?;
            Ok(Step::Done(vec![pair(a, b)]))
        }
        (Type::Tuple(a), Type::Tuple(b)) => Ok(Step::Done(
            a.iter().zip(b).map(|(x, y)| pair(x, y)).collect(),
        )),
        _ => {
            let msg = format!("cannot order {} against {}", s.zonk(&l), s.zonk(&r));
            Err(error(c, msg))
        }
    }
}

fn len(s: &mut Subst, c: &Constraint, t: &Type) -> Result<Step> {
    let Some(t) = known(s, t) else {
        return Ok(Step::Waiting);
    };
    if matches!(t, Type::Str | Type::List(_) | Type::Tuple(_) | Type::Range) {
        return Ok(Step::Done(vec![]));
    }
    let msg = format!("object of type {} has no len()", s.zonk(&t));
    Err(error(c, msg))
}

fn iter(s: &mut Subst, c: &Constraint, iterable: &Type, elem: &Type) -> Result<Step> {
    let Some(t) = known(s, iterable) else {
        return Ok(Step::Waiting);
    };
    match &t {
        Type::List(item) => expect(s, c, elem, item)?,
        Type::Str => expect(s, c, elem, &Type::Str)?,
        Type::Range => expect(s, c, elem, &Type::Int)?,
        Type::Tuple(items) => items.iter().try_for_each(|item| expect(s, c, elem, item))?,
        other => {
            let msg = format!("{} is not iterable", s.zonk(other));
            return Err(error(c, msg));
        }
    }
    Ok(Step::Done(vec![]))
}

fn index_of(
    s: &mut Subst,
    c: &Constraint,
    value: &Type,
    index: &Type,
    literal: Option<i64>,
    out: &Type,
    access: Access,
) -> Result<Step> {
    let Some(t) = known(s, value) else {
        return Ok(Step::Waiting);
    };
    let immutable = matches!(t, Type::Str | Type::Range | Type::Tuple(_));
    if access != Access::Read && immutable {
        let kind = match &t {
            Type::Tuple(_) => "tuple".to_string(),
            other => s.zonk(other).to_string(),
        };
        let msg = format!("{kind} does not support item assignment");
        return Err(error(c, msg));
    }
    if let Type::Tuple(items) = &t {
        return tuple_index(s, c, items, literal, out);
    }
    let item = match &t {
        Type::List(item) => (**item).clone(),
        Type::Str => Type::Str,
        Type::Range => Type::Int,
        other => {
            let msg = format!("{} is not subscriptable", s.zonk(other));
            return Err(error(c, msg));
        }
    };
    if !s.resolve(index).is_intlike() {
        expect(s, c, &Type::Int, index)?;
    }
    if access != Access::Write {
        expect(s, c, out, &item)?;
        return Ok(Step::Done(vec![]));
    }
    let what = format!("cannot assign into {}", s.zonk(&t));
    match s.assign(&item, out) {
        Ok(deferred) => Ok(Step::Done(assigns(deferred, &what))),
        Err(_) => {
            let msg = format!("cannot assign {} into {}", s.zonk(out), s.zonk(&t));
            Err(error(c, msg))
        }
    }
}

fn tuple_index(
    s: &mut Subst,
    c: &Constraint,
    items: &[Type],
    literal: Option<i64>,
    out: &Type,
) -> Result<Step> {
    let Some(n) = literal else {
        return Err(error(
            c,
            "a tuple index must be an integer literal".to_string(),
        ));
    };
    let len = i64::try_from(items.len()).unwrap_or(i64::MAX);
    let at = if n < 0 { n.checked_add(len) } else { Some(n) };
    let item = at
        .and_then(|i| usize::try_from(i).ok())
        .and_then(|i| items.get(i));
    let Some(item) = item else {
        let msg = format!("tuple index {n} out of range for {} elements", items.len());
        return Err(error(c, msg));
    };
    expect(s, c, out, item)?;
    Ok(Step::Done(vec![]))
}

fn unpack(s: &mut Subst, c: &Constraint, value: &Type, elems: &[Type]) -> Result<Step> {
    let Some(t) = known(s, value) else {
        return Ok(Step::Waiting);
    };
    let item = match &t {
        Type::Tuple(items) if items.len() != elems.len() => {
            let msg = format!(
                "cannot unpack {} values into {} targets",
                items.len(),
                elems.len()
            );
            return Err(error(c, msg));
        }
        Type::Tuple(items) => {
            for (target, item) in elems.iter().zip(items) {
                expect(s, c, target, item)?;
            }
            return Ok(Step::Done(vec![]));
        }
        Type::List(item) => (**item).clone(),
        Type::Str => Type::Str,
        Type::Range => Type::Int,
        other => {
            let msg = format!("cannot unpack {}", s.zonk(other));
            return Err(error(c, msg));
        }
    };
    elems
        .iter()
        .try_for_each(|target| expect(s, c, target, &item))?;
    Ok(Step::Done(vec![]))
}

fn to_number(s: &mut Subst, c: &Constraint, func: &str, arg: &Type) -> Result<Step> {
    let Some(t) = known(s, arg) else {
        return Ok(Step::Waiting);
    };
    if t.is_numeric() || t == Type::Str {
        return Ok(Step::Done(vec![]));
    }
    let msg = format!(
        "{func}() argument must be int, float, str or bool, not {}",
        s.zonk(&t)
    );
    Err(error(c, msg))
}

fn assign(s: &mut Subst, c: &Constraint, target: &Type, value: &Type, what: &str) -> Result<Step> {
    if let (Type::Optional(_), Type::Var(_)) = (s.resolve(target), s.resolve(value)) {
        return Ok(Step::Waiting);
    }
    match s.assign(target, value) {
        Ok(deferred) => Ok(Step::Done(assigns(deferred, what))),
        Err(clash) => {
            let msg = format!("{what}: {}", mismatch(s, clash, target, value));
            Err(error(c, msg))
        }
    }
}

/// Assignments `Subst::assign` could not decide yet, as constraints.
pub(super) fn assigns(deferred: Vec<(Type, Type)>, what: &str) -> Vec<Need> {
    deferred
        .into_iter()
        .map(|(target, value)| Need::Assign {
            target,
            value,
            what: what.to_string(),
        })
        .collect()
}

/// Settles the assignments still waiting at module end: a value nothing
/// else constrains takes its target's type (`x: int | None = xs[0]` with
/// `xs` never filled makes `xs` a `list[int | None]`). The rest stays
/// waiting, as in `solve`.
pub(super) fn default_assigns(s: &mut Subst, pending: Vec<Constraint>) -> Result<Vec<Constraint>> {
    let mut waiting = solve(s, pending)?;
    while let Some(i) = waiting
        .iter()
        .position(|c| matches!(c.need, Need::Assign { .. }))
    {
        let c = waiting.swap_remove(i);
        if let Need::Assign { target, value, .. } = &c.need {
            expect(s, &c, target, value)?;
        }
        waiting = solve(s, waiting)?;
    }
    Ok(waiting)
}

/// The head-resolved type, or `None` while it is still a variable.
fn known(s: &Subst, ty: &Type) -> Option<Type> {
    match s.resolve(ty) {
        Type::Var(_) => None,
        t => Some(t),
    }
}

/// Unifies, reporting `expected` versus `found` on failure.
fn expect(s: &mut Subst, c: &Constraint, expected: &Type, found: &Type) -> Result<()> {
    s.unify(expected, found)
        .map_err(|clash| error(c, mismatch(s, clash, expected, found)))
}

/// The message for a failed unification.
pub(super) fn mismatch(s: &Subst, clash: Clash, expected: &Type, found: &Type) -> String {
    let (e, f) = (s.zonk(expected), s.zonk(found));
    match clash {
        Clash::Mismatch => format!("expected {e}, found {f}"),
        Clash::Infinite => format!("{f} would have to contain itself ({e})"),
    }
}

fn error(c: &Constraint, message: String) -> Diagnostic {
    let message = match &c.via {
        Some(func) => format!("in call to `{func}`: {message}"),
        None => message,
    };
    Diagnostic {
        span: c.span,
        message,
        kind: DiagKind::Type,
    }
}

pub(super) fn op_symbol(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::FloorDiv => "//",
        BinOp::Mod => "%",
        BinOp::Pow => "**",
    }
}
