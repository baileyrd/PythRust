//! Runtime values and the operations every other part of the interpreter
//! shares: truthiness, equality, ordering, membership, display.

use std::cell::RefCell;
use std::cmp::Ordering;
use std::collections::HashSet;
use std::rc::Rc;

use super::builtins::Builtin;
use super::rt_err;
use crate::ast::{CmpOp, FuncDef};
use crate::diag::Result;
use crate::span::Span;

/// A shared, mutable list. Python lists alias, so lists are reference counted.
pub(super) type ListRef<'m> = Rc<RefCell<Vec<Value<'m>>>>;

/// A runtime value. Cheap to clone: compound values are reference counted.
/// `'m` is the lifetime of the module whose functions the values refer to.
#[derive(Debug, Clone)]
pub(super) enum Value<'m> {
    None,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Rc<str>),
    List(ListRef<'m>),
    Tuple(Rc<[Value<'m>]>),
    Range(Range),
    Func(Rc<Func<'m>>),
    Builtin(Builtin),
    /// `xs.append`, bound to its list.
    Append(ListRef<'m>),
}

/// `range(start, stop, step)`. Invariant: `step != 0`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Range {
    pub start: i64,
    pub stop: i64,
    pub step: i64,
}

/// A user function: its definition, defaults evaluated when `def` ran, and
/// the names it assigns (Python's rule: those are local everywhere in it).
#[derive(Debug)]
pub(super) struct Func<'m> {
    pub def: &'m FuncDef,
    pub defaults: Vec<Option<Value<'m>>>,
    pub locals: HashSet<&'m str>,
}

/// An `int` or `float` operand. `bool` counts as `int`, as in Python.
#[derive(Debug, Clone, Copy)]
pub(super) enum Num {
    Int(i64),
    Float(f64),
}

impl<'m> Value<'m> {
    pub fn str(s: &str) -> Self {
        Value::Str(Rc::from(s))
    }

    pub fn list(items: Vec<Value<'m>>) -> Self {
        Value::List(Rc::new(RefCell::new(items)))
    }

    pub fn num(&self) -> Option<Num> {
        match self {
            Value::Bool(b) => Some(Num::Int(i64::from(*b))),
            Value::Int(i) => Some(Num::Int(*i)),
            Value::Float(f) => Some(Num::Float(*f)),
            _ => None,
        }
    }

    /// The value as an integer, if it is an `int` or `bool`.
    pub fn int(&self) -> Option<i64> {
        match self.num()? {
            Num::Int(i) => Some(i),
            Num::Float(_) => None,
        }
    }

    pub fn truthy(&self) -> bool {
        match self {
            Value::None => false,
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            Value::Str(s) => !s.is_empty(),
            Value::List(l) => !l.borrow().is_empty(),
            Value::Tuple(t) => !t.is_empty(),
            Value::Range(r) => r.len() != 0,
            Value::Func(_) | Value::Builtin(_) | Value::Append(_) => true,
        }
    }

    /// Python's type name, for error messages.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::None => "NoneType",
            Value::Bool(_) => "bool",
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "str",
            Value::List(_) => "list",
            Value::Tuple(_) => "tuple",
            Value::Range(_) => "range",
            Value::Func(_) => "function",
            Value::Builtin(_) | Value::Append(_) => "builtin_function_or_method",
        }
    }

    /// Python's `str(x)`.
    pub fn to_str(&self) -> String {
        match self {
            Value::Str(s) => s.to_string(),
            other => other.repr(),
        }
    }

    /// Python's `repr(x)`.
    pub fn repr(&self) -> String {
        match self {
            Value::None => "None".to_string(),
            Value::Bool(true) => "True".to_string(),
            Value::Bool(false) => "False".to_string(),
            Value::Int(i) => i.to_string(),
            Value::Float(f) => float_repr(*f),
            Value::Str(s) => str_repr(s),
            Value::List(l) => format!("[{}]", join_repr(&l.borrow())),
            Value::Tuple(t) if t.len() == 1 => format!("({},)", t[0].repr()),
            Value::Tuple(t) => format!("({})", join_repr(t)),
            Value::Range(r) if r.step == 1 => format!("range({}, {})", r.start, r.stop),
            Value::Range(r) => format!("range({}, {}, {})", r.start, r.stop, r.step),
            Value::Func(f) => format!("<function {}>", f.def.name.name),
            Value::Builtin(b) => format!("<built-in function {}>", b.name()),
            Value::Append(_) => "<built-in method append of list object>".to_string(),
        }
    }

    /// Iterates a value as `for` and unpacking do. A list is read by index
    /// on every step, so appending while iterating behaves as in Python.
    pub fn iter(&self, span: Span) -> Result<Iter<'m>> {
        match self {
            Value::List(l) => Ok(Iter::List(Rc::clone(l), 0)),
            Value::Tuple(t) => Ok(Iter::Tuple(Rc::clone(t), 0)),
            Value::Str(s) => Ok(Iter::Chars(s.chars().collect(), 0)),
            Value::Range(r) => Ok(Iter::Range(*r, Some(r.start))),
            other => Err(rt_err(
                span,
                format!("TypeError: '{}' object is not iterable", other.type_name()),
            )),
        }
    }
}

impl Range {
    /// Number of elements, as Python computes it. Wide math: a range over
    /// all of `i64` has more elements than `i64::MAX`.
    pub fn len(&self) -> i128 {
        let (start, stop, step) = (
            i128::from(self.start),
            i128::from(self.stop),
            i128::from(self.step),
        );
        let n = if step > 0 {
            (stop - start + step - 1) / step
        } else {
            (start - stop - step - 1) / -step
        };
        n.max(0)
    }

    fn in_bounds(&self, x: i64) -> bool {
        if self.step > 0 {
            self.start <= x && x < self.stop
        } else {
            self.stop < x && x <= self.start
        }
    }

    fn contains(&self, x: i64) -> bool {
        self.in_bounds(x) && (i128::from(x) - i128::from(self.start)) % i128::from(self.step) == 0
    }

    /// Ranges are equal when they produce the same sequence.
    fn same_sequence(&self, other: &Range) -> bool {
        let n = self.len();
        n == other.len()
            && (n == 0 || self.start == other.start && (n == 1 || self.step == other.step))
    }
}

/// The iteration state of a `for` loop or an unpacking.
pub(super) enum Iter<'m> {
    List(ListRef<'m>, usize),
    Tuple(Rc<[Value<'m>]>, usize),
    Chars(Vec<char>, usize),
    /// The next value, or `None` once the range is exhausted.
    Range(Range, Option<i64>),
}

impl<'m> Iterator for Iter<'m> {
    type Item = Value<'m>;

    fn next(&mut self) -> Option<Value<'m>> {
        match self {
            Iter::List(l, i) => {
                let v = l.borrow().get(*i).cloned()?;
                *i += 1;
                Some(v)
            }
            Iter::Tuple(t, i) => {
                let v = t.get(*i).cloned()?;
                *i += 1;
                Some(v)
            }
            Iter::Chars(cs, i) => {
                let c = *cs.get(*i)?;
                *i += 1;
                Some(Value::str(c.encode_utf8(&mut [0; 4])))
            }
            Iter::Range(r, next) => {
                let v = (*next).filter(|v| r.in_bounds(*v))?;
                *next = v.checked_add(r.step);
                Some(Value::Int(v))
            }
        }
    }
}

/// Python `==`. Numbers compare across `bool`, `int` and `float`.
pub(super) fn eq<'m>(a: &Value<'m>, b: &Value<'m>) -> bool {
    if let (Some(x), Some(y)) = (a.num(), b.num()) {
        return num_cmp(x, y) == Some(Ordering::Equal);
    }
    match (a, b) {
        (Value::None, Value::None) => true,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::List(x), Value::List(y)) => seq_eq(&x.borrow(), &y.borrow()),
        (Value::Tuple(x), Value::Tuple(y)) => seq_eq(x, y),
        (Value::Range(x), Value::Range(y)) => x.same_sequence(y),
        (Value::Func(x), Value::Func(y)) => Rc::ptr_eq(x, y),
        (Value::Builtin(x), Value::Builtin(y)) => x == y,
        (Value::Append(x), Value::Append(y)) => Rc::ptr_eq(x, y),
        _ => false,
    }
}

/// Evaluates one link of a comparison chain.
pub(super) fn compare<'m>(op: CmpOp, a: &Value<'m>, b: &Value<'m>, span: Span) -> Result<bool> {
    let ord = match op {
        CmpOp::Eq => return Ok(eq(a, b)),
        CmpOp::NotEq => return Ok(!eq(a, b)),
        CmpOp::In => return contains(b, a, span),
        CmpOp::NotIn => return contains(b, a, span).map(|found| !found),
        CmpOp::Lt | CmpOp::LtEq | CmpOp::Gt | CmpOp::GtEq => order(op, a, b, span)?,
    };
    Ok(match op {
        CmpOp::Lt => ord == Some(Ordering::Less),
        CmpOp::LtEq => matches!(ord, Some(Ordering::Less | Ordering::Equal)),
        CmpOp::Gt => ord == Some(Ordering::Greater),
        _ => matches!(ord, Some(Ordering::Greater | Ordering::Equal)),
    })
}

/// Orders two values for `op`. `Ok(None)` means unordered (a NaN).
fn order<'m>(op: CmpOp, a: &Value<'m>, b: &Value<'m>, span: Span) -> Result<Option<Ordering>> {
    if let (Some(x), Some(y)) = (a.num(), b.num()) {
        return Ok(num_cmp(x, y));
    }
    match (a, b) {
        (Value::Str(x), Value::Str(y)) => Ok(Some(x.cmp(y))),
        (Value::List(x), Value::List(y)) => seq_order(op, &x.borrow(), &y.borrow(), span),
        (Value::Tuple(x), Value::Tuple(y)) => seq_order(op, x, y, span),
        _ => Err(rt_err(
            span,
            format!(
                "TypeError: '{}' not supported between instances of '{}' and '{}'",
                cmp_symbol(op),
                a.type_name(),
                b.type_name()
            ),
        )),
    }
}

/// Lexicographic, as Python: the first unequal pair decides, else length.
fn seq_order<'m>(
    op: CmpOp,
    a: &[Value<'m>],
    b: &[Value<'m>],
    span: Span,
) -> Result<Option<Ordering>> {
    match a.iter().zip(b).find(|(x, y)| !eq(x, y)) {
        Some((x, y)) => order(op, x, y, span),
        None => Ok(Some(a.len().cmp(&b.len()))),
    }
}

fn seq_eq<'m>(a: &[Value<'m>], b: &[Value<'m>]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| eq(x, y))
}

/// Compares numbers. Mixed `int`/`float` compares as `float`, which is
/// exact below 2**53 in magnitude.
pub(super) fn num_cmp(a: Num, b: Num) -> Option<Ordering> {
    match (a, b) {
        (Num::Int(x), Num::Int(y)) => Some(x.cmp(&y)),
        (x, y) => x.float().partial_cmp(&y.float()),
    }
}

impl Num {
    pub fn float(self) -> f64 {
        match self {
            Num::Int(i) => i as f64,
            Num::Float(f) => f,
        }
    }
}

/// Python `item in container`.
fn contains<'m>(container: &Value<'m>, item: &Value<'m>, span: Span) -> Result<bool> {
    match (container, item) {
        (Value::List(l), _) => Ok(l.borrow().iter().any(|v| eq(v, item))),
        (Value::Tuple(t), _) => Ok(t.iter().any(|v| eq(v, item))),
        (Value::Str(s), Value::Str(sub)) => Ok(s.contains(&**sub)),
        (Value::Str(_), _) => Err(rt_err(
            span,
            format!(
                "TypeError: 'in <string>' requires string as left operand, not {}",
                item.type_name()
            ),
        )),
        (Value::Range(r), _) => Ok(range_contains(r, item)),
        _ => Err(rt_err(
            span,
            format!(
                "TypeError: argument of type '{}' is not iterable",
                container.type_name()
            ),
        )),
    }
}

fn range_contains(r: &Range, item: &Value) -> bool {
    match item.num() {
        Some(Num::Int(i)) => r.contains(i),
        Some(Num::Float(f)) => {
            let i = f as i64;
            f.fract() == 0.0 && i as f64 == f && r.contains(i)
        }
        None => false,
    }
}

fn cmp_symbol(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Eq => "==",
        CmpOp::NotEq => "!=",
        CmpOp::Lt => "<",
        CmpOp::LtEq => "<=",
        CmpOp::Gt => ">",
        CmpOp::GtEq => ">=",
        CmpOp::In => "in",
        CmpOp::NotIn => "not in",
    }
}

fn join_repr(items: &[Value]) -> String {
    items.iter().map(Value::repr).collect::<Vec<_>>().join(", ")
}

/// Python's `repr` of a string: single quotes unless the text holds a
/// single quote and no double quote.
fn str_repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if c.is_control() => out.push_str(&control_escape(c)),
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

fn control_escape(c: char) -> String {
    let n = u32::from(c);
    if n <= 0xff {
        format!("\\x{n:02x}")
    } else {
        format!("\\u{n:04x}")
    }
}

/// Python's `repr` of a float: shortest round-trip digits, positional
/// for exponents in `-4..16`, scientific otherwise.
pub(super) fn float_repr(f: f64) -> String {
    if f.is_nan() {
        return "nan".to_string();
    }
    if f.is_infinite() {
        return if f > 0.0 { "inf" } else { "-inf" }.to_string();
    }
    // `{:e}` always yields shortest round-trip digits and an exponent,
    // e.g. "-1.25e-7", so the fallbacks below are unreachable.
    let sci = format!("{f:e}");
    let (mantissa, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let (sign, mantissa) = match mantissa.strip_prefix('-') {
        Some(m) => ("-", m),
        None => ("", mantissa),
    };
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    if (-4..16).contains(&exp) {
        return format!("{sign}{}", positional(&digits, exp));
    }
    let frac = &digits[1..];
    let dot = if frac.is_empty() { "" } else { "." };
    let exp_sign = if exp < 0 { '-' } else { '+' };
    format!(
        "{sign}{}{dot}{frac}e{exp_sign}{:02}",
        &digits[..1],
        exp.abs()
    )
}

/// Places the decimal point in `digits` (d.ddd times 10**exp).
fn positional(digits: &str, exp: i32) -> String {
    if exp < 0 {
        let zeros = "0".repeat((-exp - 1) as usize);
        return format!("0.{zeros}{digits}");
    }
    let point = exp as usize + 1;
    if digits.len() <= point {
        let zeros = "0".repeat(point - digits.len());
        return format!("{digits}{zeros}.0");
    }
    format!("{}.{}", &digits[..point], &digits[point..])
}
