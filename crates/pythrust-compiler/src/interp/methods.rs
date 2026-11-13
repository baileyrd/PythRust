//! Methods: `str.split`, `.join`, `.strip`, `.lstrip`, `.rstrip`,
//! `.replace`, `.startswith`, `.endswith`, `.find`, `.rfind`, `.index`,
//! `.rindex`, `.count`, `.upper` and `.lower`; `list.append`, `.pop`,
//! `.extend`, `.insert`, `.remove`, `.index`, `.count` and `.sort`
//! (D70); `set.add`; `dict.get`, `.pop`, `.setdefault`, `.update`,
//! `.keys`, `.values` and `.items`; and a file's `.read` and `.close`. `x.name` binds the
//! method to `x`; calling the bound method runs it.

use std::rc::Rc;

use super::builtins::{merge_sort, Args};
use super::table::{Key, TableRef};
use super::value::{eq, File, ListRef, Value};
use super::{raised, rt_err, stdlib, text, writable, Exec};
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
    Pop,
    Extend,
    Insert,
    Remove,
    Sort,
    Add,
    Get,
    Setdefault,
    Update,
    Keys,
    Values,
    Items,
    Read,
    Readline,
    Write,
    Flush,
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
            Name::Pop => "pop",
            Name::Extend => "extend",
            Name::Insert => "insert",
            Name::Remove => "remove",
            Name::Sort => "sort",
            Name::Add => "add",
            Name::Get => "get",
            Name::Setdefault => "setdefault",
            Name::Update => "update",
            Name::Keys => "keys",
            Name::Values => "values",
            Name::Items => "items",
            Name::Read => "read",
            Name::Readline => "readline",
            Name::Write => "write",
            Name::Flush => "flush",
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
                | Name::Pop
                | Name::Extend
                | Name::Insert
                | Name::Remove
                | Name::Sort
                | Name::Add
                | Name::Get
                | Name::Setdefault
                | Name::Update
                | Name::Keys
                | Name::Values
                | Name::Items
        ) && !self.on_file()
    }

    /// Whether a file has it (D91).
    fn on_file(self) -> bool {
        matches!(
            self,
            Name::Read | Name::Readline | Name::Write | Name::Flush | Name::Close
        )
    }
}

impl Name {
    /// Whether a list has it.
    fn on_list(self) -> bool {
        matches!(
            self,
            Name::Append
                | Name::Pop
                | Name::Extend
                | Name::Insert
                | Name::Remove
                | Name::Index
                | Name::Count
                | Name::Sort
        )
    }

    /// Whether a dict has it.
    fn on_dict(self) -> bool {
        matches!(
            self,
            Name::Get
                | Name::Pop
                | Name::Setdefault
                | Name::Update
                | Name::Keys
                | Name::Values
                | Name::Items
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
            Method::File(_, f) => f.file.borrow().kind().qualified(),
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
        "pop" => Some(Name::Pop),
        "extend" => Some(Name::Extend),
        "insert" => Some(Name::Insert),
        "remove" => Some(Name::Remove),
        "sort" => Some(Name::Sort),
        "add" => Some(Name::Add),
        "get" => Some(Name::Get),
        "setdefault" => Some(Name::Setdefault),
        "update" => Some(Name::Update),
        "keys" => Some(Name::Keys),
        "values" => Some(Name::Values),
        "items" => Some(Name::Items),
        "read" => Some(Name::Read),
        "readline" => Some(Name::Readline),
        "write" => Some(Name::Write),
        "flush" => Some(Name::Flush),
        "close" => Some(Name::Close),
        _ => None,
    };
    let method = match (name, &v) {
        (Some(n), Value::Str(s)) if n.on_str() => Some(Method::Str(n, Rc::clone(s))),
        (Some(n), Value::List(l)) if n.on_list() => Some(Method::List(n, Rc::clone(l))),
        (Some(Name::Add), Value::Set(t)) => Some(Method::Table(Name::Add, v.clone(), Rc::clone(t))),
        (Some(n), Value::Dict(t)) if n.on_dict() => Some(Method::Table(n, v.clone(), Rc::clone(t))),
        (Some(n), Value::File(f)) if n.on_file() => Some(Method::File(n, Rc::clone(f))),
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

/// Calls a bound method. Only `list.sort` takes a keyword, `reverse`.
pub(super) fn call<'m>(m: &Method<'m>, args: Args<'m>) -> Exec<'m, Value<'m>> {
    let name = m.name().as_str();
    let mut reverse = false;
    for (kw, v) in &args.keywords {
        if m.name() != Name::Sort || kw.name != "reverse" {
            let msg = format!("TypeError: {name}() takes no keyword arguments");
            return Err(rt_err(kw.span, msg).into());
        }
        reverse = v.truthy();
    }
    let (pos, span) = (args.positional, args.span);
    let (min, max) = arity(m);
    if pos.len() < min || pos.len() > max {
        return Err(rt_err(span, arity_message(name, min, max, pos.len())).into());
    }
    match m {
        Method::Str(name, s) => str_call(*name, s, pos, span),
        Method::File(name, f) => stdlib::file_call(*name, f, pos, span),
        Method::List(name, list) => list_call(*name, list, pos, reverse, span),
        Method::Table(name, _, table) => table_call(*name, table, pos, span),
    }
}

/// How many positional arguments `m` takes, as CPython counts them.
fn arity(m: &Method) -> (usize, usize) {
    match (m.name(), m) {
        (Name::Pop, Method::List(..)) => (0, 1),
        (Name::Pop | Name::Get | Name::Setdefault, _) => (1, 2),
        (Name::Split | Name::Strip | Name::Lstrip | Name::Rstrip, _) => (0, 1),
        (Name::Replace | Name::Insert, _) => (2, 2),
        (
            Name::Keys
            | Name::Values
            | Name::Items
            | Name::Read
            | Name::Readline
            | Name::Flush
            | Name::Close
            | Name::Upper
            | Name::Lower
            | Name::Sort,
            _,
        ) => (0, 0),
        _ => (1, 1),
    }
}

/// CPython's `TypeError` for a call to `name` with `got` arguments.
fn arity_message(name: &str, min: usize, max: usize, got: usize) -> String {
    match (min, max) {
        (1, 1) => format!("TypeError: {name}() takes exactly one argument ({got} given)"),
        (0, 0) => format!("TypeError: {name}() takes no arguments ({got} given)"),
        _ if got < min => format!("TypeError: {name} expected at least {min} argument, got {got}"),
        _ => format!("TypeError: {name} expected at most {max} arguments, got {got}"),
    }
}

/// A list method on `list` (D70). Inference typed the arguments.
fn list_call<'m>(
    name: Name,
    list: &ListRef<'m>,
    args: Vec<Value<'m>>,
    reverse: bool,
    span: Span,
) -> Exec<'m, Value<'m>> {
    let arg = args.first().cloned().unwrap_or(Value::None);
    match name {
        Name::Append => writable(list, span)?.push(arg),
        Name::Pop => return pop(list, args.first(), span),
        Name::Extend => {
            // Read in full first, so `xs.extend(xs)` doubles `xs`.
            let items: Vec<Value<'m>> = match &arg {
                Value::List(other) => other.borrow().clone(),
                other => other.iter(span)?.collect(),
            };
            writable(list, span)?.extend(items);
        }
        Name::Insert => {
            let len = list.borrow().len();
            let at = clamped(&arg, len, span)?;
            let v = args.get(1).cloned().unwrap_or(Value::None);
            writable(list, span)?.insert(at, v);
        }
        Name::Remove => {
            let Some(i) = find(list, &arg, span)? else {
                let msg = "list.remove(x): x not in list".to_string();
                return Err(raised(ExcKind::Value, msg, span));
            };
            let removed = writable(list, span)?.remove(i);
            drop(removed);
        }
        Name::Index => {
            let Some(i) = find(list, &arg, span)? else {
                let msg = format!("{} is not in list", arg.repr(span)?);
                return Err(raised(ExcKind::Value, msg, span));
            };
            return int(i, span);
        }
        Name::Count => {
            let items = list.borrow().clone();
            let mut n = 0;
            for x in &items {
                n += usize::from(eq(x, &arg, span)?);
            }
            return int(n, span);
        }
        Name::Sort => {
            // The list is empty while it sorts, as in CPython.
            let items = std::mem::take(&mut *writable(list, span)?);
            let sorted = merge_sort(items, reverse, span)?;
            *writable(list, span)? = sorted;
        }
        _ => return Err(rt_err(span, format!("`{}` is not a list method", name.as_str())).into()),
    }
    Ok(Value::None)
}

/// `xs.pop()` or `xs.pop(i)`: removes and gives the item, or raises
/// CPython's `IndexError` (D71).
fn pop<'m>(list: &ListRef<'m>, index: Option<&Value<'m>>, span: Span) -> Exec<'m, Value<'m>> {
    let len = list.borrow().len();
    if len == 0 {
        return Err(raised(
            ExcKind::Index,
            "pop from empty list".to_string(),
            span,
        ));
    }
    let at = match index {
        Some(i) => super::resolve(i, len, "list", span)?,
        None => Some(len - 1),
    };
    let Some(at) = at else {
        return Err(raised(
            ExcKind::Index,
            "pop index out of range".to_string(),
            span,
        ));
    };
    Ok(writable(list, span)?.remove(at))
}

/// `insert`'s index in `0..=len`: negative counts from the end, and one
/// past either end clamps, as in CPython.
fn clamped(index: &Value, len: usize, span: Span) -> Result<usize> {
    let i = index.int().ok_or_else(|| {
        let msg = format!(
            "TypeError: '{}' object cannot be interpreted as an integer",
            index.type_name()
        );
        rt_err(span, msg)
    })?;
    let len_i = i128::try_from(len).unwrap_or(i128::MAX);
    let at = if i < 0 {
        i128::from(i) + len_i
    } else {
        i128::from(i)
    };
    Ok(usize::try_from(at.clamp(0, len_i)).unwrap_or(len))
}

/// The first position of an item `==` to `x`, comparing a snapshot so a
/// comparison never meets the list borrowed.
fn find<'m>(list: &ListRef<'m>, x: &Value<'m>, span: Span) -> Result<Option<usize>> {
    let items = list.borrow().clone();
    for (i, item) in items.iter().enumerate() {
        if eq(item, x, span)? {
            return Ok(Some(i));
        }
    }
    Ok(None)
}

/// A dict's or set's method (D70): `set.add`, `dict.get`, `.pop`,
/// `.setdefault`, `.update` and the views.
fn table_call<'m>(
    name: Name,
    table: &TableRef<'m>,
    args: Vec<Value<'m>>,
    span: Span,
) -> Exec<'m, Value<'m>> {
    let arg = args.first().cloned().unwrap_or(Value::None);
    let default = args.get(1).cloned();
    match name {
        Name::Add => {
            let key = Key::of(&arg, span)?;
            writable(table, span)?.insert(key, arg, Value::None);
            Ok(Value::None)
        }
        Name::Get => {
            let key = Key::of(&arg, span)?;
            Ok(table
                .borrow()
                .get(&key)
                .cloned()
                .or(default)
                .unwrap_or(Value::None))
        }
        Name::Pop => {
            let key = Key::of(&arg, span)?;
            let removed = writable(table, span)?.remove(&key);
            match (removed, default) {
                (Some((_, v)), _) => Ok(v),
                (None, Some(d)) => Ok(d),
                (None, None) => Err(super::missing_key(&arg, span)),
            }
        }
        Name::Setdefault => {
            let key = Key::of(&arg, span)?;
            if let Some(v) = table.borrow().get(&key) {
                return Ok(v.clone());
            }
            let v = default.unwrap_or(Value::None);
            writable(table, span)?.insert(key, arg, v.clone());
            Ok(v)
        }
        Name::Update => {
            let Value::Dict(other) = &arg else {
                return Err(rt_err(span, "`update` needs a dict".to_string()).into());
            };
            let entries = other.borrow().entries().to_vec();
            for (k, v) in entries {
                let key = Key::of(&k, span)?;
                writable(table, span)?.insert(key, k, v);
            }
            Ok(Value::None)
        }
        _ => Ok(view(name, table)),
    }
}

/// `d.keys()`, `d.values()` or `d.items()`: a snapshot of the entries.
fn view<'m>(name: Name, dict: &TableRef<'m>) -> Value<'m> {
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
    Value::View(view, items.collect())
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
