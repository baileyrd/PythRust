//! Methods: `str.split`, `.join`, `.strip`, `.lstrip`, `.rstrip`,
//! `.replace`, `.startswith`, `.endswith`, `.find`, `.rfind`, `.index`,
//! `.rindex`, `.count`, `.upper` and `.lower`, `list.append`, `set.add`, `dict.get`, `.keys`, `.values`,
//! `.items`, and a file's `.read` and `.close`. `x.name` binds the
//! method to `x`; calling the bound method runs it.

use std::rc::Rc;

use super::builtins::Args;
use super::table::{Key, TableRef};
use super::value::{File, ListRef, Value};
use super::{raised, rt_err, stdlib, text, Exec};
use crate::ast::Ident;
use crate::diag::Result;
use crate::exc::ExcKind;
use crate::infer::View;
use crate::span::Span;

/// A method's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Name {
    Split,
    Join,
    Strip,
    Lstrip,
    Rstrip,
    Replace,
    Startswith,
    Endswith,
    Find,
    Rfind,
    Index,
    Rindex,
    Count,
    Upper,
    Lower,
    Append,
    Add,
    Get,
    Keys,
    Values,
    Items,
    Read,
    Close,
}

impl Name {
    pub fn as_str(self) -> &'static str {
        match self {
            Name::Split => "split",
            Name::Join => "join",
            Name::Strip => "strip",
            Name::Lstrip => "lstrip",
            Name::Rstrip => "rstrip",
            Name::Replace => "replace",
            Name::Startswith => "startswith",
            Name::Endswith => "endswith",
            Name::Find => "find",
            Name::Rfind => "rfind",
            Name::Index => "index",
            Name::Rindex => "rindex",
            Name::Count => "count",
            Name::Upper => "upper",
            Name::Lower => "lower",
            Name::Append => "append",
            Name::Add => "add",
            Name::Get => "get",
            Name::Keys => "keys",
            Name::Values => "values",
            Name::Items => "items",
            Name::Read => "read",
            Name::Close => "close",
        }
    }
}

impl Name {
    /// Whether it is a `str` method.
    fn on_str(self) -> bool {
        !matches!(
            self,
            Name::Append
                | Name::Add
                | Name::Get
                | Name::Keys
                | Name::Values
                | Name::Items
                | Name::Read
                | Name::Close
        )
    }
}

/// A method bound to its receiver, which `on` holds.
#[derive(Debug, Clone)]
pub(super) enum Method<'m> {
    Str(Name, Rc<str>),
    List(Name, ListRef<'m>),
    /// A dict's or a set's method; which one the receiver tells.
    Table(Name, Value<'m>, TableRef<'m>),
    File(Name, Rc<File>),
}

impl<'m> Method<'m> {
    pub fn name(&self) -> Name {
        match self {
            Method::Str(name, _)
            | Method::List(name, _)
            | Method::Table(name, _, _)
            | Method::File(name, _) => *name,
        }
    }

    /// The receiver's type name, for `repr`.
    pub fn owner(&self) -> &'static str {
        match self {
            Method::Str(..) => "str",
            Method::List(..) => "list",
            Method::Table(_, on, _) => on.type_name(),
            Method::File(..) => "_io.BufferedReader",
        }
    }

    /// Whether both are the same method of the same object.
    pub fn same(&self, other: &Method<'m>) -> bool {
        match (self, other) {
            (Method::Str(a, x), Method::Str(b, y)) => a == b && Rc::ptr_eq(x, y),
            (Method::List(a, x), Method::List(b, y)) => a == b && Rc::ptr_eq(x, y),
            (Method::Table(a, _, x), Method::Table(b, _, y)) => a == b && Rc::ptr_eq(x, y),
            (Method::File(a, x), Method::File(b, y)) => a == b && Rc::ptr_eq(x, y),
            _ => false,
        }
    }
}

/// `v.attr`: the method bound to `v`, or an `AttributeError` fault.
pub(super) fn bind<'m>(v: Value<'m>, attr: &Ident) -> Result<Value<'m>> {
    let name = match attr.name.as_str() {
        "split" => Some(Name::Split),
        "join" => Some(Name::Join),
        "strip" => Some(Name::Strip),
        "lstrip" => Some(Name::Lstrip),
        "rstrip" => Some(Name::Rstrip),
        "replace" => Some(Name::Replace),
        "startswith" => Some(Name::Startswith),
        "endswith" => Some(Name::Endswith),
        "find" => Some(Name::Find),
        "rfind" => Some(Name::Rfind),
        "index" => Some(Name::Index),
        "rindex" => Some(Name::Rindex),
        "count" => Some(Name::Count),
        "upper" => Some(Name::Upper),
        "lower" => Some(Name::Lower),
        "append" => Some(Name::Append),
        "add" => Some(Name::Add),
        "get" => Some(Name::Get),
        "keys" => Some(Name::Keys),
        "values" => Some(Name::Values),
        "items" => Some(Name::Items),
        "read" => Some(Name::Read),
        "close" => Some(Name::Close),
        _ => None,
    };
    let method = match (name, &v) {
        (Some(n), Value::Str(s)) if n.on_str() => Some(Method::Str(n, Rc::clone(s))),
        (Some(Name::Append), Value::List(l)) => Some(Method::List(Name::Append, Rc::clone(l))),
        (Some(Name::Add), Value::Set(t)) => Some(Method::Table(Name::Add, v.clone(), Rc::clone(t))),
        (Some(n @ (Name::Get | Name::Keys | Name::Values | Name::Items)), Value::Dict(t)) => {
            Some(Method::Table(n, v.clone(), Rc::clone(t)))
        }
        (Some(n @ (Name::Read | Name::Close)), Value::File(f)) => {
            Some(Method::File(n, Rc::clone(f)))
        }
        _ => None,
    };
    match method {
        Some(m) => Ok(Value::Method(Rc::new(m))),
        None => Err(rt_err(
            attr.span,
            format!(
                "AttributeError: '{}' object has no attribute '{}'",
                v.type_name(),
                attr.name
            ),
        )),
    }
}

/// Calls a bound method.
pub(super) fn call<'m>(m: &Method<'m>, args: Args<'m>) -> Exec<'m, Value<'m>> {
    let name = m.name().as_str();
    if let Some((kw, _)) = args.keywords.first() {
        let msg = format!("TypeError: {name}() takes no keyword arguments");
        return Err(rt_err(kw.span, msg).into());
    }
    let (pos, span) = (args.positional, args.span);
    let (min, max) = match m.name() {
        Name::Split | Name::Strip | Name::Lstrip | Name::Rstrip => (0, 1),
        Name::Join | Name::Append | Name::Add => (1, 1),
        Name::Startswith
        | Name::Endswith
        | Name::Find
        | Name::Rfind
        | Name::Index
        | Name::Rindex
        | Name::Count => (1, 1),
        Name::Replace => (2, 2),
        Name::Get => (1, 2),
        Name::Keys
        | Name::Values
        | Name::Items
        | Name::Read
        | Name::Close
        | Name::Upper
        | Name::Lower => (0, 0),
    };
    if pos.len() < min || pos.len() > max {
        let msg = match (min, max) {
            (1, 1) => format!(
                "TypeError: {name}() takes exactly one argument ({} given)",
                pos.len()
            ),
            (0, 0) => format!(
                "TypeError: {name}() takes no arguments ({} given)",
                pos.len()
            ),
            _ if pos.len() < min => format!(
                "TypeError: {name} expected at least {min} argument, got {}",
                pos.len()
            ),
            _ => format!(
                "TypeError: {name} expected at most {max} arguments, got {}",
                pos.len()
            ),
        };
        return Err(rt_err(span, msg).into());
    }
    let mut pos = pos.into_iter();
    let mut arg = || pos.next().unwrap_or(Value::None);
    match m {
        Method::Str(name, s) => str_call(*name, s, pos.collect(), span),
        Method::File(Name::Read, f) => stdlib::read(f, span),
        Method::File(_, f) => {
            f.file.borrow_mut().take();
            Ok(Value::None)
        }
        Method::List(_, list) => {
            list.borrow_mut().push(arg());
            Ok(Value::None)
        }
        Method::Table(Name::Add, _, set) => {
            let v = arg();
            let key = Key::of(&v, span)?;
            set.borrow_mut().insert(key, v, Value::None);
            Ok(Value::None)
        }
        Method::Table(Name::Get, _, dict) => {
            let key = Key::of(&arg(), span)?;
            let default = arg();
            Ok(dict.borrow().get(&key).cloned().unwrap_or(default))
        }
        Method::Table(name, _, dict) => {
            let dict = dict.borrow();
            let view = match name {
                Name::Keys => View::Keys,
                Name::Values => View::Values,
                _ => View::Items,
            };
            let items = dict.entries().iter().map(|(k, v)| match view {
                View::Keys => k.clone(),
                View::Values => v.clone(),
                View::Items => Value::Tuple(Rc::from([k.clone(), v.clone()])),
            });
            Ok(Value::View(view, items.collect()))
        }
    }
}

/// A `str` method on `s`. Inference gave each its `str` arguments, and
/// `join` a list of them.
fn str_call<'m>(name: Name, s: &str, args: Vec<Value<'m>>, span: Span) -> Exec<'m, Value<'m>> {
    let strs: Vec<Rc<str>> = args
        .iter()
        .filter_map(|a| match a {
            Value::Str(t) => Some(Rc::clone(t)),
            _ => None,
        })
        .collect();
    let text = |i: usize| strs.get(i).map(|t| &**t);
    let side = match name {
        Name::Lstrip => text::Side::Left,
        Name::Rstrip => text::Side::Right,
        _ => text::Side::Both,
    };
    if let Some(found) = search(name, s, text(0), span)? {
        return Ok(found);
    }
    let out = match name {
        Name::Split => return split(s, text(0), span),
        Name::Join => return join(s, args.first(), span),
        Name::Upper | Name::Lower => text::case(s, name == Name::Upper),
        Name::Strip | Name::Lstrip | Name::Rstrip => text::strip(s, text(0), side).to_string(),
        Name::Replace => match (text(0), text(1)) {
            (Some(old), Some(new)) => text::replace(s, old, new),
            _ => return Err(rt_err(span, "`replace` needs two strings".to_string()).into()),
        },
        _ => return Err(rt_err(span, format!("`{}` is not a str method", name.as_str())).into()),
    };
    Ok(Value::str(&out))
}

/// A `str` method that looks for `sub` in `s`: a test, an index or a
/// count. `None` for any other method; `index` and `rindex` raise a
/// `ValueError` where `find` gives -1 (D50).
fn search<'m>(name: Name, s: &str, sub: Option<&str>, span: Span) -> Exec<'m, Option<Value<'m>>> {
    let Some(sub) = sub else {
        return Ok(None);
    };
    let right = matches!(name, Name::Rfind | Name::Rindex);
    let at = || text::find(s, sub, right);
    let value = match name {
        Name::Startswith => Value::Bool(s.starts_with(sub)),
        Name::Endswith => Value::Bool(s.ends_with(sub)),
        Name::Find | Name::Rfind => at().map_or(Ok(Value::Int(-1)), |i| int(i, span))?,
        Name::Index | Name::Rindex => match at() {
            Some(i) => int(i, span)?,
            None => return Err(raised(ExcKind::Value, text::NOT_FOUND.to_string(), span)),
        },
        Name::Count => int(text::count(s, sub), span)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}

/// A code-point index or count as an `int`.
fn int<'m>(n: usize, span: Span) -> Exec<'m, Value<'m>> {
    i64::try_from(n)
        .map(Value::Int)
        .map_err(|_| rt_err(span, "OverflowError: index too large".to_string()).into())
}

/// `s.split()` or `s.split(sep)`: a new list of the parts.
fn split<'m>(s: &str, sep: Option<&str>, span: Span) -> Exec<'m, Value<'m>> {
    let parts = match sep {
        None => text::split_space(s),
        Some(sep) => text::split(s, sep)
            .ok_or_else(|| raised(ExcKind::Value, text::EMPTY_SEPARATOR.to_string(), span))?,
    };
    Ok(Value::list(parts.into_iter().map(Value::str).collect()))
}

/// `sep.join(xs)` of a list of strings.
fn join<'m>(sep: &str, xs: Option<&Value<'m>>, span: Span) -> Exec<'m, Value<'m>> {
    let Some(Value::List(xs)) = xs else {
        return Err(rt_err(span, "`join` needs a list".to_string()).into());
    };
    let mut parts: Vec<Rc<str>> = Vec::new();
    for x in xs.borrow().iter() {
        let Value::Str(t) = x else {
            return Err(rt_err(span, "`join` needs strings".to_string()).into());
        };
        parts.push(Rc::clone(t));
    }
    let parts: Vec<&str> = parts.iter().map(|t| &**t).collect();
    Ok(Value::str(&parts.join(sep)))
}
