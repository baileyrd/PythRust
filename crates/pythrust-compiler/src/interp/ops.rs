//! Arithmetic and unary operators. Integer math is checked: overflow is a
//! runtime error, never a wrap (see `docs/grammar.md`, Deviations).

use std::rc::Rc;

use super::rt_err;
use super::value::{Num, Value};
use crate::ast::{BinOp, UnaryOp};
use crate::diag::{Diagnostic, Result};
use crate::span::Span;

/// Evaluates `l op r`.
pub(super) fn binary<'m>(op: BinOp, l: &Value<'m>, r: &Value<'m>, span: Span) -> Result<Value<'m>> {
    if let (Some(a), Some(b)) = (l.num(), r.num()) {
        return match (a, b) {
            (Num::Int(x), Num::Int(y)) => int_op(op, x, y, span),
            _ => float_op(op, a.float(), b.float(), span).map(Value::Float),
        };
    }
    match (op, l, r) {
        (BinOp::Add, Value::Str(x), Value::Str(y)) => Ok(Value::str(&format!("{x}{y}"))),
        (BinOp::Add, Value::List(x), Value::List(y)) => {
            let items = [x.borrow().as_slice(), y.borrow().as_slice()].concat();
            Ok(Value::list(items))
        }
        (BinOp::Add, Value::Tuple(x), Value::Tuple(y)) => {
            Ok(Value::Tuple(Rc::from([&x[..], &y[..]].concat())))
        }
        (BinOp::Mul, _, _) => repeat(l, r, span),
        _ => Err(unsupported(op, l, r, span)),
    }
}

/// `-x`, `+x`, `not x`.
pub(super) fn unary<'m>(op: UnaryOp, v: &Value<'m>, span: Span) -> Result<Value<'m>> {
    match (op, v.num()) {
        (UnaryOp::Not, _) => Ok(Value::Bool(!v.truthy())),
        (UnaryOp::Neg, Some(Num::Int(i))) => i
            .checked_neg()
            .map(Value::Int)
            .ok_or_else(|| overflow(span)),
        (UnaryOp::Neg, Some(Num::Float(f))) => Ok(Value::Float(-f)),
        (UnaryOp::Pos, Some(Num::Int(i))) => Ok(Value::Int(i)),
        (UnaryOp::Pos, Some(Num::Float(f))) => Ok(Value::Float(f)),
        (UnaryOp::Neg | UnaryOp::Pos, None) => {
            let sym = if op == UnaryOp::Neg { '-' } else { '+' };
            Err(rt_err(
                span,
                format!(
                    "TypeError: bad operand type for unary {sym}: '{}'",
                    v.type_name()
                ),
            ))
        }
    }
}

fn int_op<'m>(op: BinOp, a: i64, b: i64, span: Span) -> Result<Value<'m>> {
    let checked = match op {
        BinOp::Add => a.checked_add(b),
        BinOp::Sub => a.checked_sub(b),
        BinOp::Mul => a.checked_mul(b),
        BinOp::Div => return float_op(op, a as f64, b as f64, span).map(Value::Float),
        BinOp::FloorDiv => floor_div(a, b, span)?,
        BinOp::Mod => Some(int_mod(a, b, span)?),
        BinOp::Pow => return int_pow(a, b, span),
    };
    checked.map(Value::Int).ok_or_else(|| overflow(span))
}

/// Floors toward negative infinity. `None` on overflow (`i64::MIN // -1`).
fn floor_div(a: i64, b: i64, span: Span) -> Result<Option<i64>> {
    if b == 0 {
        return Err(int_zero_div(span));
    }
    let Some(q) = a.checked_div(b) else {
        return Ok(None);
    };
    let inexact = q.wrapping_mul(b) != a;
    Ok(Some(if inexact && (a < 0) != (b < 0) {
        q - 1
    } else {
        q
    }))
}

/// The remainder takes the sign of the divisor, as in Python.
fn int_mod(a: i64, b: i64, span: Span) -> Result<i64> {
    if b == 0 {
        return Err(int_zero_div(span));
    }
    // `i64::MIN % -1` overflows in Rust; the true remainder is 0.
    let r = a.checked_rem(b).unwrap_or(0);
    Ok(if r != 0 && (r < 0) != (b < 0) {
        r + b
    } else {
        r
    })
}

/// A negative exponent gives a float, as in Python.
fn int_pow<'m>(a: i64, b: i64, span: Span) -> Result<Value<'m>> {
    if b < 0 {
        return float_op(BinOp::Pow, a as f64, b as f64, span).map(Value::Float);
    }
    let checked = match u32::try_from(b) {
        Ok(e) => a.checked_pow(e),
        Err(_) => huge_pow(a, b),
    };
    checked.map(Value::Int).ok_or_else(|| overflow(span))
}

/// `a ** b` for `b > u32::MAX`: only 0, 1 and -1 stay in range.
fn huge_pow(a: i64, b: i64) -> Option<i64> {
    match a {
        0 | 1 => Some(a),
        -1 => Some(if b % 2 == 0 { 1 } else { -1 }),
        _ => None,
    }
}

fn float_op(op: BinOp, a: f64, b: f64, span: Span) -> Result<f64> {
    match op {
        BinOp::Add => Ok(a + b),
        BinOp::Sub => Ok(a - b),
        BinOp::Mul => Ok(a * b),
        BinOp::Div if b == 0.0 => Err(zero_div(span, "division by zero")),
        BinOp::Div => Ok(a / b),
        BinOp::FloorDiv if b == 0.0 => Err(zero_div(span, "float floor division by zero")),
        BinOp::FloorDiv => Ok(float_divmod(a, b).0),
        BinOp::Mod if b == 0.0 => Err(zero_div(span, "float modulo")),
        BinOp::Mod => Ok(float_divmod(a, b).1),
        BinOp::Pow => float_pow(a, b, span),
    }
}

/// CPython's `float_divmod`: floor quotient and divisor-signed remainder.
/// Precondition: `b != 0`.
fn float_divmod(a: f64, b: f64) -> (f64, f64) {
    let mut m = a % b;
    let mut div = (a - m) / b;
    if m != 0.0 {
        if (b < 0.0) != (m < 0.0) {
            m += b;
            div -= 1.0;
        }
    } else {
        m = 0.0_f64.copysign(b);
    }
    if div == 0.0 {
        return (0.0_f64.copysign(a / b), m);
    }
    let floor = div.floor();
    (
        if div - floor > 0.5 {
            floor + 1.0
        } else {
            floor
        },
        m,
    )
}

/// Python's float `**`. A negative base with a fractional exponent is an
/// error here; Python returns a complex number, which Pythrust lacks.
fn float_pow(a: f64, b: f64, span: Span) -> Result<f64> {
    if a == 0.0 && b < 0.0 {
        return Err(zero_div(span, "0.0 cannot be raised to a negative power"));
    }
    if a < 0.0 && b.is_finite() && b.fract() != 0.0 {
        return Err(rt_err(
            span,
            "ValueError: negative number cannot be raised to a fractional power".to_string(),
        ));
    }
    let r = a.powf(b);
    if r.is_infinite() && a.is_finite() && b.is_finite() {
        return Err(rt_err(
            span,
            "OverflowError: numerical result out of range".to_string(),
        ));
    }
    Ok(r)
}

/// `seq * n` or `n * seq`. A count below 1 gives an empty sequence.
fn repeat<'m>(l: &Value<'m>, r: &Value<'m>, span: Span) -> Result<Value<'m>> {
    let (seq, n) = match (l.int(), r.int()) {
        (None, Some(n)) => (l, n),
        (Some(n), None) => (r, n),
        _ => return Err(unsupported(BinOp::Mul, l, r, span)),
    };
    let n = usize::try_from(n).unwrap_or(0);
    match seq {
        Value::Str(s) => {
            let mut out = String::new();
            out.try_reserve_exact(total(s.len(), n, span)?)
                .map_err(|_| memory_error(span))?;
            (0..n).for_each(|_| out.push_str(s));
            Ok(Value::str(&out))
        }
        Value::List(items) => Ok(Value::list(repeat_items(&items.borrow(), n, span)?)),
        Value::Tuple(items) => Ok(Value::Tuple(Rc::from(repeat_items(items, n, span)?))),
        _ => Err(unsupported(BinOp::Mul, l, r, span)),
    }
}

fn repeat_items<T: Clone>(items: &[T], n: usize, span: Span) -> Result<Vec<T>> {
    let mut out = Vec::new();
    out.try_reserve_exact(total(items.len(), n, span)?)
        .map_err(|_| memory_error(span))?;
    (0..n).for_each(|_| out.extend_from_slice(items));
    Ok(out)
}

/// Sizes a repeat up front, so a huge one is a runtime error rather than
/// an allocator abort.
fn total(len: usize, n: usize, span: Span) -> Result<usize> {
    len.checked_mul(n).ok_or_else(|| memory_error(span))
}

fn memory_error(span: Span) -> Diagnostic {
    rt_err(
        span,
        "MemoryError: repeated sequence is too large".to_string(),
    )
}

pub(super) fn overflow(span: Span) -> Diagnostic {
    rt_err(span, "OverflowError: integer overflow".to_string())
}

fn int_zero_div(span: Span) -> Diagnostic {
    zero_div(span, "integer division or modulo by zero")
}

fn zero_div(span: Span, what: &str) -> Diagnostic {
    rt_err(span, format!("ZeroDivisionError: {what}"))
}

fn unsupported(op: BinOp, l: &Value, r: &Value, span: Span) -> Diagnostic {
    rt_err(
        span,
        format!(
            "TypeError: unsupported operand type(s) for {}: '{}' and '{}'",
            op_symbol(op),
            l.type_name(),
            r.type_name()
        ),
    )
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
