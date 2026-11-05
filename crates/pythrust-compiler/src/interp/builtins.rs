//! The closed set of builtins: `print len list set sorted range str int
//! float bool abs ord chr sum min max any all enumerate zip`,
//! and the builtin exceptions, which build an exception value when called.

use std::io::Write;
use std::rc::Rc;

use super::ops::overflow;
use super::table::{Key, Table};
use super::value::{compare, str_repr, Exc, ExcArg, Num, Range, Value};
use super::{fsum, raised, rt_err, tasks, text, Exec};
use crate::ast::{CmpOp, Ident};
use crate::diag::Result;
use crate::exc::ExcKind;
use crate::span::Span;
use crate::stdlib::StdFn;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Builtin {
    Print,
    Len,
    List,
    Set,
    Sorted,
    Range,
    Str,
    Int,
    Float,
    Bool,
    Abs,
    Ord,
    Chr,
    Sum,
    Min,
    Max,
    Any,
    All,
    /// `enumerate` and `zip`: the interpreter steps them (`Interp::lazy`).
    Enumerate,
    Zip,
    Exception(ExcKind),
    /// A module function; the interpreter calls it (`Interp::std`).
    Std(StdFn),
}

/// Evaluated call arguments, in source order within each kind.
pub(super) struct Args<'m> {
    pub positional: Vec<Value<'m>>,
    pub keywords: Vec<(&'m Ident, Value<'m>)>,
    pub span: Span,
}

impl Builtin {
    pub fn lookup(name: &str) -> Option<Builtin> {
        Some(match name {
            "print" => Builtin::Print,
            "len" => Builtin::Len,
            "list" => Builtin::List,
            "set" => Builtin::Set,
            "sorted" => Builtin::Sorted,
            "range" => Builtin::Range,
            "str" => Builtin::Str,
            "int" => Builtin::Int,
            "float" => Builtin::Float,
            "bool" => Builtin::Bool,
            "abs" => Builtin::Abs,
            "ord" => Builtin::Ord,
            "chr" => Builtin::Chr,
            "sum" => Builtin::Sum,
            "min" => Builtin::Min,
            "max" => Builtin::Max,
            "any" => Builtin::Any,
            "all" => Builtin::All,
            "enumerate" => Builtin::Enumerate,
            "zip" => Builtin::Zip,
            _ => {
                if let Some(f) = StdFn::from_name(name) {
                    return Some(Builtin::Std(f));
                }
                return ExcKind::from_name(name).map(Builtin::Exception);
            }
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Builtin::Print => "print",
            Builtin::Len => "len",
            Builtin::List => "list",
            Builtin::Set => "set",
            Builtin::Sorted => "sorted",
            Builtin::Range => "range",
            Builtin::Str => "str",
            Builtin::Int => "int",
            Builtin::Float => "float",
            Builtin::Bool => "bool",
            Builtin::Abs => "abs",
            Builtin::Ord => "ord",
            Builtin::Chr => "chr",
            Builtin::Sum => "sum",
            Builtin::Min => "min",
            Builtin::Max => "max",
            Builtin::Any => "any",
            Builtin::All => "all",
            Builtin::Enumerate => "enumerate",
            Builtin::Zip => "zip",
            Builtin::Exception(kind) => kind.name(),
            Builtin::Std(f) => f.name(),
        }
    }

    /// The exception this builtin builds, if it is one.
    pub fn exception(self) -> Option<ExcKind> {
        match self {
            Builtin::Exception(kind) => Some(kind),
            _ => None,
        }
    }
}

/// Calls a builtin. `print` is the only one that writes to `out`; `int`
/// and `float` of a malformed string raise a catchable `ValueError`.
pub(super) fn call<'m>(b: Builtin, args: Args<'m>, out: &mut dyn Write) -> Exec<'m, Value<'m>> {
    if !matches!(b, Builtin::Print | Builtin::Sum) {
        reject_keywords(b.name(), &args)?;
    }
    let (pos, span) = (args.positional.as_slice(), args.span);
    Ok(match b {
        Builtin::Len => len(one(b, pos, span)?, span)?,
        Builtin::List => copy_list(one(b, pos, span)?, span)?,
        Builtin::Set => new_set(opt(b, pos, span)?, span)?,
        Builtin::Sorted => sorted(one(b, pos, span)?, span)?,
        Builtin::Range => range(pos, span)?,
        Builtin::Str => {
            Value::str(&opt(b, pos, span)?.map_or(Ok(String::new()), |v| v.to_str(span))?)
        }
        Builtin::Int => opt(b, pos, span)?.map_or(Ok(Value::Int(0)), |v| to_int(v, span))?,
        Builtin::Float => {
            opt(b, pos, span)?.map_or(Ok(Value::Float(0.0)), |v| to_float(v, span))?
        }
        Builtin::Bool => Value::Bool(opt(b, pos, span)?.is_some_and(Value::truthy)),
        Builtin::Print => print(&args, out)?,
        Builtin::Abs => abs(one(b, pos, span)?, span)?,
        Builtin::Ord => {
            let Value::Str(s) = one(b, pos, span)? else {
                return Err(rt_err(span, "TypeError: ord() expects a str".to_string()).into());
            };
            let code = text::ord(s).map_err(|n| rt_err(span, text::ord_message(n)))?;
            Value::Int(i64::from(code))
        }
        Builtin::Chr => {
            let n = one(b, pos, span)?.int().unwrap_or(-1);
            let c = text::chr(n).map_err(|msg| raised(ExcKind::Value, msg.to_string(), span))?;
            Value::str(&c.to_string())
        }
        Builtin::Sum => sum(&args)?,
        Builtin::Min | Builtin::Max => extreme(b, pos, span)?,
        Builtin::Any | Builtin::All => {
            let any = b == Builtin::Any;
            let decided = match one(b, pos, span)? {
                Value::Results(r) => tasks::decide(r, any)?,
                v => {
                    let mut items = v.iter(span)?;
                    items.any(|x| x.truthy() == any) == any
                }
            };
            Value::Bool(decided)
        }
        Builtin::Exception(kind) => {
            let arg = ExcArg::of(opt(b, pos, span)?.cloned());
            Value::Exc(Rc::new(Exc { kind, arg }))
        }
        Builtin::Std(_) | Builtin::Enumerate | Builtin::Zip => {
            let msg = format!("internal: `{}` needs the interpreter", b.name());
            return Err(rt_err(span, msg).into());
        }
    })
}

/// `abs(x)` of an int, float or bool; `abs(i64::MIN)` overflows, as
/// `-i64::MIN` does.
fn abs<'m>(v: &Value<'m>, span: Span) -> Result<Value<'m>> {
    match v.num() {
        Some(Num::Int(i)) => i
            .checked_abs()
            .map(Value::Int)
            .ok_or_else(|| overflow(span)),
        Some(Num::Float(f)) => Ok(Value::Float(f.abs())),
        None => Err(rt_err(
            span,
            "TypeError: bad operand type for abs()".to_string(),
        )),
    }
}

/// `sum(xs, start=0)`: ints and bools total as a checked int, floats as
/// CPython's compensated sum (D57).
fn sum<'m>(args: &Args<'m>) -> Result<Value<'m>> {
    let span = args.span;
    if let Some((name, _)) = args.keywords.iter().find(|(n, _)| n.name != "start") {
        let msg = format!(
            "TypeError: sum() got an unexpected keyword argument '{}'",
            name.name
        );
        return Err(rt_err(name.span, msg));
    }
    let start = args.keywords.first().map(|(_, v)| v);
    let (xs, start) = match (args.positional.as_slice(), start) {
        ([xs], start) | ([xs, _], start @ None) => (xs, start.or(args.positional.get(1))),
        _ => return Err(arity_error(Builtin::Sum, 1, 2, args.positional.len(), span)),
    };
    // Items are read one at a time, so a large range needs no memory. As
    // CPython's, an int total turns float at the first float item.
    let mut items = xs.iter(span)?;
    let mut total = match start {
        Some(Value::Float(f)) => return Ok(Value::Float(float_sum(*f, items))),
        Some(v) => v.int().unwrap_or(0),
        None => 0,
    };
    while let Some(x) = items.next() {
        if let Value::Float(f) = x {
            let rest = std::iter::once(Value::Float(f)).chain(items);
            return Ok(Value::Float(float_sum(total as f64, rest)));
        }
        let x = x
            .int()
            .ok_or_else(|| rt_err(span, "TypeError: sum() of a non-number".to_string()))?;
        total = total.checked_add(x).ok_or_else(|| overflow(span))?;
    }
    Ok(Value::Int(total))
}

/// CPython's compensated float sum of `xs` from `start`.
fn float_sum<'m>(start: f64, xs: impl Iterator<Item = Value<'m>>) -> f64 {
    fsum::fsum(start, xs.filter_map(|x| x.num()).map(Num::float))
}

/// `min` or `max` of an iterable, or of two or more arguments: the
/// first item no later one beats, as CPython keeps (D57). An empty
/// iterable raises a catchable `ValueError`.
fn extreme<'m>(b: Builtin, args: &[Value<'m>], span: Span) -> Exec<'m, Value<'m>> {
    // An iterable is read one item at a time, so a large range needs no
    // memory.
    let mut items: Box<dyn Iterator<Item = Value<'m>> + '_> = match args {
        [xs] => Box::new(xs.iter(span)?),
        _ => Box::new(args.iter().cloned()),
    };
    extreme_of(b, || Ok(items.next()), span)
}

/// `min` or `max` (`b`) of the items `next` gives one at a time.
pub(super) fn extreme_of<'m>(
    b: Builtin,
    mut next: impl FnMut() -> Exec<'m, Option<Value<'m>>>,
    span: Span,
) -> Exec<'m, Value<'m>> {
    let op = if b == Builtin::Min {
        CmpOp::Lt
    } else {
        CmpOp::Gt
    };
    let Some(mut best) = next()? else {
        let msg = format!("{}() iterable argument is empty", b.name());
        return Err(raised(ExcKind::Value, msg, span));
    };
    while let Some(x) = next()? {
        if compare(op, &x, &best, span)? {
            best = x;
        }
    }
    Ok(best)
}

fn reject_keywords(func: &str, args: &Args) -> Result<()> {
    match args.keywords.first() {
        Some((name, _)) => Err(rt_err(
            name.span,
            format!("TypeError: {func}() takes no keyword arguments"),
        )),
        None => Ok(()),
    }
}

/// `print(*values, sep=' ', end='\n')`.
fn print<'m>(args: &Args<'m>, out: &mut dyn Write) -> Result<Value<'m>> {
    let (mut sep, mut end) = (" ".to_string(), "\n".to_string());
    for (name, v) in &args.keywords {
        let slot = match name.name.as_str() {
            "sep" => &mut sep,
            "end" => &mut end,
            other => {
                return Err(rt_err(
                    name.span,
                    format!("TypeError: '{other}' is an invalid keyword argument for print()"),
                ))
            }
        };
        if let Some(s) = print_option(name, v)? {
            *slot = s;
        }
    }
    let text = args
        .positional
        .iter()
        .map(|v| v.to_str(args.span))
        .collect::<Result<Vec<String>>>()?;
    let line = text.join(&sep) + &end;
    out.write_all(line.as_bytes())
        .map_err(|e| rt_err(args.span, format!("OSError: cannot write output: {e}")))?;
    Ok(Value::None)
}

/// `sep` and `end` take a string, or `None` for the default.
fn print_option(name: &Ident, v: &Value) -> Result<Option<String>> {
    match v {
        Value::None => Ok(None),
        Value::Str(s) => Ok(Some(s.to_string())),
        other => Err(rt_err(
            name.span,
            format!(
                "TypeError: {} must be None or a string, not {}",
                name.name,
                other.type_name()
            ),
        )),
    }
}

fn len<'m>(v: &Value<'m>, span: Span) -> Result<Value<'m>> {
    let n = match v {
        Value::Str(s) => s.chars().count() as i128,
        Value::List(l) => l.borrow().len() as i128,
        Value::Dict(t) | Value::Set(t) => t.borrow().len() as i128,
        Value::Tuple(t) | Value::View(_, t) => t.len() as i128,
        Value::Range(r) => r.len(),
        Value::Bytes(b) => b.len() as i128,
        other => {
            return Err(rt_err(
                span,
                format!(
                    "TypeError: object of type '{}' has no len()",
                    other.type_name()
                ),
            ))
        }
    };
    i64::try_from(n).map(Value::Int).map_err(|_| {
        rt_err(
            span,
            "OverflowError: length does not fit in an int".to_string(),
        )
    })
}

/// `list(xs)`: a shallow copy.
fn copy_list<'m>(v: &Value<'m>, span: Span) -> Result<Value<'m>> {
    match v {
        Value::List(l) => Ok(Value::list(l.borrow().clone())),
        Value::Dict(_) | Value::Set(_) | Value::View(..) => {
            Ok(Value::list(v.iter(span)?.collect()))
        }
        other => Err(rt_err(
            span,
            format!("TypeError: '{}' object is not iterable", other.type_name()),
        )),
    }
}

/// `sorted(xs)`: a new list of what `xs` iterates over, stably merge
/// sorted with `<` alone, as Python's sort compares.
fn sorted<'m>(v: &Value<'m>, span: Span) -> Result<Value<'m>> {
    let items: Vec<Value<'m>> = v.iter(span)?.collect();
    Ok(Value::list(merge_sort(items, span)?))
}

fn merge_sort<'m>(mut items: Vec<Value<'m>>, span: Span) -> Result<Vec<Value<'m>>> {
    if items.len() < 2 {
        return Ok(items);
    }
    let right = items.split_off(items.len() / 2);
    let (left, right) = (merge_sort(items, span)?, merge_sort(right, span)?);
    let mut out = Vec::with_capacity(left.len() + right.len());
    let (mut left, mut right) = (left.into_iter().peekable(), right.into_iter().peekable());
    while let (Some(l), Some(r)) = (left.peek(), right.peek()) {
        // Take from the right only when strictly less: equal items keep
        // their order.
        let next = match compare(CmpOp::Lt, r, l, span)? {
            true => right.next(),
            false => left.next(),
        };
        out.extend(next);
    }
    out.extend(left);
    out.extend(right);
    Ok(out)
}

/// `set()` or `set(xs)`: a new set of what `xs` iterates over.
fn new_set<'m>(from: Option<&Value<'m>>, span: Span) -> Result<Value<'m>> {
    let mut set = Table::default();
    for v in from
        .map(|x| x.iter(span))
        .transpose()?
        .into_iter()
        .flatten()
    {
        set.insert(Key::of(&v, span)?, v, Value::None);
    }
    Ok(Value::set(set))
}

/// `range(stop)`, `range(start, stop)`, `range(start, stop, step)`.
fn range<'m>(args: &[Value<'m>], span: Span) -> Result<Value<'m>> {
    arity(Builtin::Range, args, 1, 3, span)?;
    let ints = args
        .iter()
        .map(|v| {
            v.int().ok_or_else(|| {
                rt_err(
                    span,
                    format!(
                        "TypeError: '{}' object cannot be interpreted as an integer",
                        v.type_name()
                    ),
                )
            })
        })
        .collect::<Result<Vec<i64>>>()?;
    let (start, stop, step) = match ints.as_slice() {
        [stop] => (0, *stop, 1),
        [start, stop] => (*start, *stop, 1),
        [start, stop, step, ..] => (*start, *stop, *step),
        [] => (0, 0, 1),
    };
    if step == 0 {
        return Err(rt_err(
            span,
            "ValueError: range() arg 3 must not be zero".to_string(),
        ));
    }
    Ok(Value::Range(Range { start, stop, step }))
}

fn to_int<'m>(v: &Value<'m>, span: Span) -> Exec<'m, Value<'m>> {
    match (v, v.num()) {
        (_, Some(Num::Int(i))) => Ok(Value::Int(i)),
        (_, Some(Num::Float(f))) => Ok(float_to_int(f, span)?),
        (Value::Str(s), None) => parse_int(s, span),
        _ => Err(rt_err(
            span,
            format!(
                "TypeError: int() argument must be a string or a number, not '{}'",
                v.type_name()
            ),
        )
        .into()),
    }
}

/// Truncates toward zero, as Python's `int(float)`.
fn float_to_int<'m>(f: f64, span: Span) -> Result<Value<'m>> {
    if f.is_nan() {
        return Err(rt_err(
            span,
            "ValueError: cannot convert float NaN to integer".to_string(),
        ));
    }
    // Exactly the floats in [-2**63, 2**63) truncate into i64.
    let limit = 9_223_372_036_854_775_808.0;
    if !(-limit..limit).contains(&f) {
        return Err(rt_err(
            span,
            "OverflowError: float too large to convert to int".to_string(),
        ));
    }
    Ok(Value::Int(f.trunc() as i64))
}

/// A malformed string raises a catchable `ValueError`; one out of range
/// is an overflow fault.
fn parse_int<'m>(s: &str, span: Span) -> Exec<'m, Value<'m>> {
    let invalid = || {
        let message = format!("invalid literal for int() with base 10: {}", str_repr(s));
        raised(ExcKind::Value, message, span)
    };
    let text = strip_underscores(s.trim()).ok_or_else(invalid)?;
    let digits = text.strip_prefix(['+', '-']).unwrap_or(&text);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(invalid());
    }
    // Only digits and a sign remain, so the one failure left is range.
    let parsed = text.parse().map(Value::Int);
    Ok(parsed.map_err(|_| rt_err(span, "OverflowError: int too large for i64".to_string()))?)
}

/// A malformed string raises a catchable `ValueError`.
fn to_float<'m>(v: &Value<'m>, span: Span) -> Exec<'m, Value<'m>> {
    if let Some(n) = v.num() {
        return Ok(Value::Float(n.float()));
    }
    let Value::Str(s) = v else {
        return Err(rt_err(
            span,
            format!(
                "TypeError: float() argument must be a string or a number, not '{}'",
                v.type_name()
            ),
        )
        .into());
    };
    strip_underscores(s.trim())
        .and_then(|t| t.parse().ok())
        .map(Value::Float)
        .ok_or_else(|| {
            let message = format!("could not convert string to float: {}", str_repr(s));
            raised(ExcKind::Value, message, span)
        })
}

/// Removes `_` separators, each of which must sit between two digits, as
/// in Python's numeric strings. `None` if one does not.
fn strip_underscores(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let valid = bytes.iter().enumerate().all(|(i, b)| {
        *b != b'_'
            || (i > 0
                && bytes[i - 1].is_ascii_digit()
                && bytes.get(i + 1).is_some_and(u8::is_ascii_digit))
    });
    valid.then(|| s.replace('_', ""))
}

fn one<'a, 'm>(b: Builtin, args: &'a [Value<'m>], span: Span) -> Result<&'a Value<'m>> {
    arity(b, args, 1, 1, span)?;
    args.first().ok_or_else(|| arity_error(b, 1, 1, 0, span))
}

fn opt<'a, 'm>(b: Builtin, args: &'a [Value<'m>], span: Span) -> Result<Option<&'a Value<'m>>> {
    arity(b, args, 0, 1, span)?;
    Ok(args.first())
}

fn arity(b: Builtin, args: &[Value], min: usize, max: usize, span: Span) -> Result<()> {
    if (min..=max).contains(&args.len()) {
        return Ok(());
    }
    Err(arity_error(b, min, max, args.len(), span))
}

fn arity_error(
    b: Builtin,
    min: usize,
    max: usize,
    got: usize,
    span: Span,
) -> crate::diag::Diagnostic {
    let expected = if min == max {
        format!("exactly {min}")
    } else {
        format!("{min} to {max}")
    };
    rt_err(
        span,
        format!(
            "TypeError: {}() takes {expected} arguments ({got} given)",
            b.name()
        ),
    )
}
