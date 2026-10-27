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
    Exc(Rc<Exc<'m>>),
}

/// A builtin exception name (`docs/grammar.md`, D6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ExcKind {
    Value,
    Index,
    ZeroDivision,
    Overflow,
    Recursion,
}

impl ExcKind {
    pub fn lookup(name: &str) -> Option<ExcKind> {
        Some(match name {
            "ValueError" => ExcKind::Value,
            "IndexError" => ExcKind::Index,
            "ZeroDivisionError" => ExcKind::ZeroDivision,
            "OverflowError" => ExcKind::Overflow,
            "RecursionError" => ExcKind::Recursion,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            ExcKind::Value => "ValueError",
            ExcKind::Index => "IndexError",
            ExcKind::ZeroDivision => "ZeroDivisionError",
            ExcKind::Overflow => "OverflowError",
            ExcKind::Recursion => "RecursionError",
        }
    }
}

/// An exception value: its kind and the argument it was built with.
#[derive(Debug)]
pub(super) struct Exc<'m> {
    pub kind: ExcKind,
    pub arg: Option<Value<'m>>,
}

impl Exc<'_> {
    /// `str(e)`: the argument as a string, or empty without one.
    pub fn message(&self, span: Span) -> Result<String> {
        self.arg
            .as_ref()
            .map_or(Ok(String::new()), |v| v.to_str(span))
    }

    /// The last line CPython prints for an uncaught exception:
    /// `ValueError: bad`, or only the name when the message is empty.
    pub fn uncaught(&self, span: Span) -> Result<String> {
        let message = self.message(span)?;
        let name = self.kind.name();
        Ok(match message.is_empty() {
            true => name.to_string(),
            false => format!("{name}: {message}"),
        })
    }
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
            Value::Func(_) | Value::Builtin(_) | Value::Append(_) | Value::Exc(_) => true,
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
            Value::Builtin(b) if b.exception().is_some() => "type",
            Value::Builtin(_) | Value::Append(_) => "builtin_function_or_method",
            Value::Exc(e) => e.kind.name(),
        }
    }

    /// Python's `str(x)`.
    pub fn to_str(&self, span: Span) -> Result<String> {
        match self {
            Value::Str(s) => Ok(s.to_string()),
            Value::Exc(e) => e.message(span),
            other => other.repr(span),
        }
    }

    /// Python's `repr(x)`. A list that contains itself renders as `[...]`,
    /// as in Python; nesting deeper than `MAX_NESTING` is an error.
    pub fn repr(&self, span: Span) -> Result<String> {
        let mut out = String::new();
        write_repr(self, &mut Vec::new(), &mut out, span)?;
        Ok(out)
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

/// How deep containers may nest inside `repr` or a comparison before it
/// is a `RecursionError`. Bounds the Rust stack those recursions use.
const MAX_NESTING: usize = 10_000;

/// Python `==`. Numbers compare across `bool`, `int` and `float`.
pub(super) fn eq<'m>(a: &Value<'m>, b: &Value<'m>, span: Span) -> Result<bool> {
    eq_at(a, b, 0, span)
}

/// `==` at container depth `depth`. A container is equal to itself
/// without looking inside, as in Python, so `xs == xs` holds for a list
/// that contains itself.
fn eq_at<'m>(a: &Value<'m>, b: &Value<'m>, depth: usize, span: Span) -> Result<bool> {
    if let (Some(x), Some(y)) = (a.num(), b.num()) {
        return Ok(num_cmp(x, y) == Some(Ordering::Equal));
    }
    Ok(match (a, b) {
        (Value::None, Value::None) => true,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::List(x), Value::List(y)) if Rc::ptr_eq(x, y) => true,
        (Value::List(x), Value::List(y)) => {
            seq_eq(&x.borrow(), &y.borrow(), deeper(depth, span)?, span)?
        }
        (Value::Tuple(x), Value::Tuple(y)) if Rc::ptr_eq(x, y) => true,
        (Value::Tuple(x), Value::Tuple(y)) => seq_eq(x, y, deeper(depth, span)?, span)?,
        (Value::Range(x), Value::Range(y)) => x.same_sequence(y),
        (Value::Func(x), Value::Func(y)) => Rc::ptr_eq(x, y),
        (Value::Builtin(x), Value::Builtin(y)) => x == y,
        (Value::Append(x), Value::Append(y)) => Rc::ptr_eq(x, y),
        (Value::Exc(x), Value::Exc(y)) => Rc::ptr_eq(x, y),
        _ => false,
    })
}

/// One container level deeper into a comparison.
fn deeper(depth: usize, span: Span) -> Result<usize> {
    if depth >= MAX_NESTING {
        return Err(rt_err(
            span,
            "RecursionError: maximum recursion depth exceeded in comparison".to_string(),
        ));
    }
    Ok(depth + 1)
}

/// Evaluates one link of a comparison chain.
pub(super) fn compare<'m>(op: CmpOp, a: &Value<'m>, b: &Value<'m>, span: Span) -> Result<bool> {
    let ord = match op {
        CmpOp::Eq => return eq(a, b, span),
        CmpOp::NotEq => return eq(a, b, span).map(|same| !same),
        CmpOp::In => return contains(b, a, span),
        CmpOp::NotIn => return contains(b, a, span).map(|found| !found),
        CmpOp::Is => return is_none_check(a, b, span),
        CmpOp::IsNot => return is_none_check(a, b, span).map(|same| !same),
        CmpOp::Lt | CmpOp::LtEq | CmpOp::Gt | CmpOp::GtEq => order(op, a, b, 0, span)?,
    };
    Ok(match op {
        CmpOp::Lt => ord == Some(Ordering::Less),
        CmpOp::LtEq => matches!(ord, Some(Ordering::Less | Ordering::Equal)),
        CmpOp::Gt => ord == Some(Ordering::Greater),
        _ => matches!(ord, Some(Ordering::Greater | Ordering::Equal)),
    })
}

/// `a is b`, where phase 2 requires one side to be `None`.
fn is_none_check<'m>(a: &Value<'m>, b: &Value<'m>, span: Span) -> Result<bool> {
    match (a, b) {
        (Value::None, other) | (other, Value::None) => Ok(matches!(other, Value::None)),
        _ => Err(rt_err(
            span,
            "TypeError: `is` compares only with None in phase 2".to_string(),
        )),
    }
}

/// Orders two values for `op`. `Ok(None)` means unordered (a NaN).
fn order<'m>(
    op: CmpOp,
    a: &Value<'m>,
    b: &Value<'m>,
    depth: usize,
    span: Span,
) -> Result<Option<Ordering>> {
    if let (Some(x), Some(y)) = (a.num(), b.num()) {
        return Ok(num_cmp(x, y));
    }
    match (a, b) {
        (Value::Str(x), Value::Str(y)) => Ok(Some(x.cmp(y))),
        (Value::List(x), Value::List(y)) => {
            seq_order(op, &x.borrow(), &y.borrow(), deeper(depth, span)?, span)
        }
        (Value::Tuple(x), Value::Tuple(y)) => seq_order(op, x, y, deeper(depth, span)?, span),
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
    depth: usize,
    span: Span,
) -> Result<Option<Ordering>> {
    for (x, y) in a.iter().zip(b) {
        if !eq_at(x, y, depth, span)? {
            return order(op, x, y, depth, span);
        }
    }
    Ok(Some(a.len().cmp(&b.len())))
}

fn seq_eq<'m>(a: &[Value<'m>], b: &[Value<'m>], depth: usize, span: Span) -> Result<bool> {
    if a.len() != b.len() {
        return Ok(false);
    }
    for (x, y) in a.iter().zip(b) {
        if !eq_at(x, y, depth, span)? {
            return Ok(false);
        }
    }
    Ok(true)
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
        (Value::List(l), _) => any_eq(&l.borrow(), item, span),
        (Value::Tuple(t), _) => any_eq(t, item, span),
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

fn any_eq<'m>(items: &[Value<'m>], item: &Value<'m>, span: Span) -> Result<bool> {
    for v in items {
        if eq(v, item, span)? {
            return Ok(true);
        }
    }
    Ok(false)
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
        CmpOp::Is => "is",
        CmpOp::IsNot => "is not",
    }
}

/// Writes `v`'s repr. `active` holds the containers being rendered,
/// outermost first: meeting one again means a cycle, shown as `[...]`
/// or `(...)` as in Python.
fn write_repr(v: &Value, active: &mut Vec<*const ()>, out: &mut String, span: Span) -> Result<()> {
    let text = match v {
        Value::List(l) => {
            let id = Rc::as_ptr(l).cast::<()>();
            if active.contains(&id) {
                out.push_str("[...]");
                return Ok(());
            }
            return write_items(&l.borrow(), id, ("[", "]"), active, out, span);
        }
        Value::Tuple(t) => {
            let id = Rc::as_ptr(t).cast::<()>();
            if active.contains(&id) {
                out.push_str("(...)");
                return Ok(());
            }
            let close = if t.len() == 1 { ",)" } else { ")" };
            return write_items(t, id, ("(", close), active, out, span);
        }
        Value::Exc(e) => {
            out.push_str(e.kind.name());
            out.push('(');
            if let Some(arg) = &e.arg {
                write_repr(arg, active, out, span)?;
            }
            out.push(')');
            return Ok(());
        }
        Value::None => "None".to_string(),
        Value::Bool(true) => "True".to_string(),
        Value::Bool(false) => "False".to_string(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => float_repr(*f),
        Value::Str(s) => str_repr(s),
        Value::Range(r) if r.step == 1 => format!("range({}, {})", r.start, r.stop),
        Value::Range(r) => format!("range({}, {}, {})", r.start, r.stop, r.step),
        Value::Func(f) => format!("<function {}>", f.def.name.name),
        Value::Builtin(b) if b.exception().is_some() => format!("<class '{}'>", b.name()),
        Value::Builtin(b) => format!("<built-in function {}>", b.name()),
        Value::Append(_) => "<built-in method append of list object>".to_string(),
    };
    out.push_str(&text);
    Ok(())
}

fn write_items(
    items: &[Value],
    id: *const (),
    (open, close): (&str, &str),
    active: &mut Vec<*const ()>,
    out: &mut String,
    span: Span,
) -> Result<()> {
    if active.len() >= MAX_NESTING {
        return Err(rt_err(
            span,
            "RecursionError: maximum recursion depth exceeded while getting the repr of an object"
                .to_string(),
        ));
    }
    active.push(id);
    out.push_str(open);
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        write_repr(item, active, out, span)?;
    }
    out.push_str(close);
    active.pop();
    Ok(())
}

/// Python's `repr` of a string: single quotes unless the text holds a
/// single quote and no double quote.
pub(super) fn str_repr(s: &str) -> String {
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
