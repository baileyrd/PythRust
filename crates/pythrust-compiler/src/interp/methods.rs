//! Methods: `list.append`, `set.add`, and `dict.get`, `.keys`,
//! `.values`, `.items`. `x.name` binds the method to `x`; calling the
//! bound method runs it.

use std::rc::Rc;

use super::builtins::Args;
use super::table::{Key, TableRef};
use super::value::{ListRef, Value};
use super::{rt_err, Exec};
use crate::ast::Ident;
use crate::diag::Result;
use crate::infer::View;

/// A method's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Name {
    Append,
    Add,
    Get,
    Keys,
    Values,
    Items,
}

impl Name {
    pub fn as_str(self) -> &'static str {
        match self {
            Name::Append => "append",
            Name::Add => "add",
            Name::Get => "get",
            Name::Keys => "keys",
            Name::Values => "values",
            Name::Items => "items",
        }
    }
}

/// A method bound to its receiver, which `on` holds.
#[derive(Debug, Clone)]
pub(super) enum Method<'m> {
    List(Name, ListRef<'m>),
    /// A dict's or a set's method; which one the receiver tells.
    Table(Name, Value<'m>, TableRef<'m>),
}

impl<'m> Method<'m> {
    pub fn name(&self) -> Name {
        match self {
            Method::List(name, _) | Method::Table(name, _, _) => *name,
        }
    }

    /// The receiver's type name, for `repr`.
    pub fn owner(&self) -> &'static str {
        match self {
            Method::List(..) => "list",
            Method::Table(_, on, _) => on.type_name(),
        }
    }

    /// Whether both are the same method of the same object.
    pub fn same(&self, other: &Method<'m>) -> bool {
        match (self, other) {
            (Method::List(a, x), Method::List(b, y)) => a == b && Rc::ptr_eq(x, y),
            (Method::Table(a, _, x), Method::Table(b, _, y)) => a == b && Rc::ptr_eq(x, y),
            _ => false,
        }
    }
}

/// `v.attr`: the method bound to `v`, or an `AttributeError` fault.
pub(super) fn bind<'m>(v: Value<'m>, attr: &Ident) -> Result<Value<'m>> {
    let name = match attr.name.as_str() {
        "append" => Some(Name::Append),
        "add" => Some(Name::Add),
        "get" => Some(Name::Get),
        "keys" => Some(Name::Keys),
        "values" => Some(Name::Values),
        "items" => Some(Name::Items),
        _ => None,
    };
    let method = match (name, &v) {
        (Some(Name::Append), Value::List(l)) => Some(Method::List(Name::Append, Rc::clone(l))),
        (Some(Name::Add), Value::Set(t)) => Some(Method::Table(Name::Add, v.clone(), Rc::clone(t))),
        (Some(n @ (Name::Get | Name::Keys | Name::Values | Name::Items)), Value::Dict(t)) => {
            Some(Method::Table(n, v.clone(), Rc::clone(t)))
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
        Name::Append | Name::Add => (1, 1),
        Name::Get => (1, 2),
        Name::Keys | Name::Values | Name::Items => (0, 0),
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
