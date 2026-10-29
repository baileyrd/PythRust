//! The closed set of builtins: `print len list range str int float bool`,
//! and the builtin exceptions, which build an exception value when called.

use std::io::Write;
use std::rc::Rc;

use super::value::{str_repr, Exc, Num, Range, Value};
use super::{rt_err, value_error, Exec};
use crate::ast::Ident;
use crate::diag::Result;
use crate::exc::ExcKind;
use crate::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Builtin {
    Print,
    Len,
    List,
    Range,
    Str,
    Int,
    Float,
    Bool,
    Exception(ExcKind),
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
            "range" => Builtin::Range,
            "str" => Builtin::Str,
            "int" => Builtin::Int,
            "float" => Builtin::Float,
            "bool" => Builtin::Bool,
            _ => return ExcKind::from_name(name).map(Builtin::Exception),
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Builtin::Print => "print",
            Builtin::Len => "len",
            Builtin::List => "list",
            Builtin::Range => "range",
            Builtin::Str => "str",
            Builtin::Int => "int",
            Builtin::Float => "float",
            Builtin::Bool => "bool",
            Builtin::Exception(kind) => kind.name(),
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
    if b != Builtin::Print {
        reject_keywords(b.name(), &args)?;
    }
    let (pos, span) = (args.positional.as_slice(), args.span);
    Ok(match b {
        Builtin::Len => len(one(b, pos, span)?, span)?,
        Builtin::List => copy_list(one(b, pos, span)?, span)?,
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
        Builtin::Exception(kind) => {
            let arg = opt(b, pos, span)?.cloned();
            Value::Exc(Rc::new(Exc { kind, arg }))
        }
    })
}

/// `xs.append(v)`.
pub(super) fn append<'m>(list: &super::value::ListRef<'m>, args: Args<'m>) -> Result<Value<'m>> {
    reject_keywords("append", &args)?;
    let [v] = <[Value; 1]>::try_from(args.positional).map_err(|vs| {
        rt_err(
            args.span,
            format!(
                "TypeError: append() takes exactly one argument ({} given)",
                vs.len()
            ),
        )
    })?;
    list.borrow_mut().push(v);
    Ok(Value::None)
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
        Value::Tuple(t) => t.len() as i128,
        Value::Range(r) => r.len(),
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
        other => Err(rt_err(
            span,
            format!("TypeError: '{}' object is not iterable", other.type_name()),
        )),
    }
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
/// is an overflow fault (D7).
fn parse_int<'m>(s: &str, span: Span) -> Exec<'m, Value<'m>> {
    let invalid = || {
        let message = format!("invalid literal for int() with base 10: {}", str_repr(s));
        value_error(message, span)
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
            value_error(message, span)
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
