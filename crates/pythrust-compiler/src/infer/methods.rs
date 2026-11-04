//! Method calls, looked up by the receiver's type: a type's own method
//! comes first, so two types can share a method name.

use crate::ast::{Arg, Expr, Ident};
use crate::diag::Result;
use crate::span::Span;

use super::solve::Need;
use super::ty::{Type, View};
use super::{error, Checker};

/// A type that has methods. Executors' `.submit` and `.map` are checked
/// with the tasks (`Checker::task`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Owner {
    List,
    Set,
    Dict,
    File,
    Future,
}

impl Owner {
    /// The owner of a resolved type, if it has methods.
    fn of(ty: &Type) -> Option<Owner> {
        match ty {
            Type::List(_) => Some(Owner::List),
            Type::Set(_) => Some(Owner::Set),
            Type::Dict(..) => Some(Owner::Dict),
            Type::File => Some(Owner::File),
            Type::Future(_) => Some(Owner::Future),
            _ => None,
        }
    }
}

/// One method: its owner, its name, and how many arguments it takes.
#[derive(Debug, Clone, Copy)]
struct Method {
    owner: Owner,
    name: &'static str,
    min: usize,
    max: usize,
}

const fn m(owner: Owner, name: &'static str, min: usize, max: usize) -> Method {
    Method {
        owner,
        name,
        min,
        max,
    }
}

/// Every method but an executor's.
const METHODS: [Method; 9] = [
    m(Owner::List, "append", 1, 1),
    m(Owner::Set, "add", 1, 1),
    m(Owner::Dict, "get", 1, 2),
    m(Owner::Dict, "keys", 0, 0),
    m(Owner::Dict, "values", 0, 0),
    m(Owner::Dict, "items", 0, 0),
    m(Owner::File, "read", 0, 0),
    m(Owner::File, "close", 0, 0),
    m(Owner::Future, "result", 0, 0),
];

/// The method `name` of a receiver owned by `here`: `here`'s own, else
/// the first type's that has it, whose check then names the receiver
/// it needs. `None` when no type has it.
fn lookup(table: &[Method], name: &str, here: Option<Owner>) -> Option<Method> {
    let mut found = table.iter().filter(|m| m.name == name);
    let first = *found.next()?;
    let own = std::iter::once(first)
        .chain(found.copied())
        .find(|m| Some(m.owner) == here);
    Some(own.unwrap_or(first))
}

impl Checker {
    /// A method call: `list.append`, `set.add`, `dict.get`, `.keys`,
    /// `.values`, `.items`, a file's `.read` and `.close`, an executor's
    /// `.submit` and `.map`, and a future's `.result`. The receiver's
    /// type picks the method; while it is unknown, the one type that has
    /// the name fixes it.
    pub(super) fn method(&mut self, value: &Expr, attr: &Ident, args: &[Arg]) -> Result<Type> {
        let name = attr.name.as_str();
        if matches!(name, "submit" | "map") {
            return self.task(value, attr, args);
        }
        let Some(first) = lookup(&METHODS, name, None) else {
            let msg = format!(
                "unknown method `.{name}`; the methods are `list.append`, `set.add`, \
                 `dict.get`, `.keys`, `.values` and `.items`, a file's `.read` \
                 and `.close`, an executor's `.submit` and `.map`, and a future's \
                 `.result`"
            );
            return Err(error(attr.span, msg));
        };
        let receiver = self.expr(value)?;
        let here = Owner::of(&self.subst.resolve(&receiver));
        let found = lookup(&METHODS, name, here).unwrap_or(first);
        method_arity(name, args, found.min, found.max, attr.span)?;
        match found.owner {
            Owner::List | Owner::Set => self.insert(name, value.span, &receiver, &args[0]),
            Owner::Dict if name == "get" => self.get(value.span, &receiver, args, attr.span),
            Owner::Dict => self.view(name, value.span, &receiver),
            Owner::File => self.file(name, value.span, &receiver),
            Owner::Future => self.result(value.span, &receiver),
        }
    }

    /// `f.read()` is `bytes`; `f.close()` is `None`.
    fn file(&mut self, name: &str, at: Span, receiver: &Type) -> Result<Type> {
        let needs = format!("`{name}` needs a file from `open`");
        self.expect(at, &Type::File, receiver, &needs)?;
        Ok(if name == "read" {
            Type::Bytes
        } else {
            Type::None
        })
    }

    /// `xs.append(v)` or `s.add(v)`: `v` flows into the element type.
    fn insert(&mut self, name: &str, at: Span, receiver: &Type, arg: &Arg) -> Result<Type> {
        let elem = self.subst.fresh();
        let (container, needs) = match name {
            "append" => (Type::List(Box::new(elem.clone())), "`append` needs a list"),
            _ => (Type::Set(Box::new(elem.clone())), "`add` needs a set"),
        };
        self.expect(at, &container, receiver, needs)?;
        let item = self.checked(&arg.value, &elem)?;
        if name == "add" {
            let need = Need::Key {
                key: elem,
                index: item,
            };
            self.need(need, arg.value.span);
            return Ok(Type::None);
        }
        let what = "appended value does not match the list";
        self.expect(arg.value.span, &elem, &item, what)?;
        Ok(Type::None)
    }

    /// `d.get(k)` is `V | None`; `d.get(k, default)` is `V`.
    fn get(&mut self, at: Span, receiver: &Type, args: &[Arg], span: Span) -> Result<Type> {
        let (key, value) = (self.subst.fresh(), self.subst.fresh());
        let dict = Type::Dict(Box::new(key.clone()), Box::new(value.clone()));
        self.expect(at, &dict, receiver, "`get` needs a dict")?;
        let k = self.checked(&args[0].value, &key)?;
        self.need(Need::Key { key, index: k }, args[0].value.span);
        if let Some(default) = args.get(1) {
            let d = self.checked(&default.value, &value)?;
            let what = "the default of `get` must match the dict's values";
            self.expect(default.value.span, &value, &d, what)?;
            return Ok(value);
        }
        let out = self.subst.fresh();
        let need = Need::OrNone {
            value,
            out: out.clone(),
        };
        self.need(need, span);
        Ok(out)
    }

    /// `d.keys()`, `d.values()` or `d.items()`: a view of the keys,
    /// values or `(key, value)` pairs, in insertion order.
    fn view(&mut self, name: &str, at: Span, receiver: &Type) -> Result<Type> {
        let (key, value) = (self.subst.fresh(), self.subst.fresh());
        let dict = Type::Dict(Box::new(key.clone()), Box::new(value.clone()));
        let needs = format!("`{name}` needs a dict");
        self.expect(at, &dict, receiver, &needs)?;
        let (view, item) = match name {
            "keys" => (View::Keys, key),
            "values" => (View::Values, value),
            _ => (View::Items, Type::Tuple(vec![key, value])),
        };
        Ok(Type::View(view, Box::new(item)))
    }
}

/// Checks a method call's argument count; methods take no keywords.
fn method_arity(name: &str, args: &[Arg], min: usize, max: usize, span: Span) -> Result<()> {
    if let Some(arg) = args.iter().find(|a| a.name.is_some()) {
        let msg = format!("`{name}` takes no keyword arguments");
        return Err(error(arg.span, msg));
    }
    if (min..=max).contains(&args.len()) {
        return Ok(());
    }
    let expected = match (min, max) {
        (0, 0) => "no arguments".to_string(),
        (1, 1) => "1 argument".to_string(),
        _ => format!("{min} or {max} arguments"),
    };
    let msg = format!("`{name}` takes {expected} but {} were given", args.len());
    Err(error(span, msg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_method_name_has_one_owner() {
        // `lookup` gives a receiver still unknown the first owner. A shared
        // name instead waits for the receiver's type (D52): add that path,
        // with its tests, along with the first shared name.
        for (i, a) in METHODS.iter().enumerate() {
            let shared = METHODS[i + 1..].iter().find(|b| b.name == a.name);
            assert!(shared.is_none(), "`.{}` has two owners", a.name);
        }
    }

    #[test]
    fn the_receiver_picks_its_own_method() {
        let table = [m(Owner::List, "pop", 0, 1), m(Owner::Dict, "pop", 1, 2)];
        let got = lookup(&table, "pop", Some(Owner::Dict)).map(|m| m.owner);
        assert_eq!(got, Some(Owner::Dict));
        let got = lookup(&table, "pop", Some(Owner::List)).map(|m| m.owner);
        assert_eq!(got, Some(Owner::List));
    }

    #[test]
    fn another_receiver_gets_the_first_owner_to_name_in_its_error() {
        let got = lookup(&METHODS, "append", Some(Owner::Dict)).map(|m| m.owner);
        assert_eq!(got, Some(Owner::List));
        let got = lookup(&METHODS, "append", None).map(|m| m.owner);
        assert_eq!(got, Some(Owner::List));
    }

    #[test]
    fn a_name_no_type_has_is_not_found() {
        assert!(lookup(&METHODS, "split", None).is_none());
        assert!(lookup(&METHODS, "split", Some(Owner::List)).is_none());
    }
}
