//! Method calls, looked up by the receiver's type: a type's own method
//! comes first, so two types can share a method name.

use crate::ast::{Arg, Expr, ExprKind, Ident};
use crate::diag::Result;
use crate::span::Span;

use super::solve::Need;
use super::ty::{Type, View};
use super::{error, Checker};

/// A type that has methods. Executors' `.submit` and `.map` are checked
/// with the tasks (`Checker::task`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Owner {
    Str,
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
            Type::Str => Some(Owner::Str),
            Type::List(_) => Some(Owner::List),
            Type::Set(_) => Some(Owner::Set),
            Type::Dict(..) => Some(Owner::Dict),
            Type::File(..) => Some(Owner::File),
            Type::Future(_) => Some(Owner::Future),
            _ => None,
        }
    }
}

/// One method: its owner, its name, and how many arguments it takes
/// (CPython's count; a `str` method refuses those not supported yet).
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

/// Every method but an executor's. A name with several owners (`index`
/// and `count` on `str` and list, `pop` on list and dict) waits for the
/// receiver's type (D52).
const METHODS: [Method; 37] = [
    m(Owner::Str, "split", 0, 2),
    m(Owner::Str, "join", 1, 1),
    m(Owner::Str, "strip", 0, 1),
    m(Owner::Str, "lstrip", 0, 1),
    m(Owner::Str, "rstrip", 0, 1),
    m(Owner::Str, "replace", 2, 3),
    m(Owner::Str, "startswith", 1, 3),
    m(Owner::Str, "endswith", 1, 3),
    m(Owner::Str, "find", 1, 3),
    m(Owner::Str, "rfind", 1, 3),
    m(Owner::Str, "index", 1, 3),
    m(Owner::Str, "rindex", 1, 3),
    m(Owner::Str, "count", 1, 3),
    m(Owner::Str, "upper", 0, 0),
    m(Owner::Str, "lower", 0, 0),
    m(Owner::List, "append", 1, 1),
    m(Owner::List, "pop", 0, 1),
    m(Owner::List, "extend", 1, 1),
    m(Owner::List, "insert", 2, 2),
    m(Owner::List, "remove", 1, 1),
    m(Owner::List, "index", 1, 3),
    m(Owner::List, "count", 1, 1),
    m(Owner::List, "sort", 0, 0),
    m(Owner::Set, "add", 1, 1),
    m(Owner::Dict, "get", 1, 2),
    m(Owner::Dict, "pop", 1, 2),
    m(Owner::Dict, "setdefault", 1, 2),
    m(Owner::Dict, "update", 0, 1),
    m(Owner::Dict, "keys", 0, 0),
    m(Owner::Dict, "values", 0, 0),
    m(Owner::Dict, "items", 0, 0),
    m(Owner::File, "read", 0, 1),
    m(Owner::File, "readline", 0, 0),
    m(Owner::File, "write", 1, 1),
    m(Owner::File, "flush", 0, 0),
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

/// Whether `name` is typed by a [`Need::Method`] once the receiver's
/// type is known: the list and dict methods of D70, and a name several
/// types share.
fn deferred(name: &str) -> bool {
    let owners = METHODS.iter().filter(|m| m.name == name).count();
    owners > 1
        || matches!(
            name,
            "extend" | "insert" | "remove" | "sort" | "setdefault" | "update"
        )
}

/// A file's methods CPython has that are not supported yet (D91).
const FILE_GAPS: [&str; 6] = [
    "readlines",
    "writelines",
    "seek",
    "tell",
    "truncate",
    "fileno",
];

/// The types of a call to a file's method (D91).
pub(super) struct FileSig {
    /// The one argument's type, if it takes one.
    pub param: Option<Type>,
    pub result: Type,
    /// Whether only a text file has it: the file's data is `str`.
    pub text: bool,
}

/// What method `name` of a file, written or not, of `data` (resolved
/// or not) takes, given `got` arguments, and gives (D91), or why that
/// file has none (D92, D99).
pub(super) fn file_signature(
    name: &str,
    writes: bool,
    data: &Type,
    got: usize,
) -> std::result::Result<FileSig, String> {
    let sig = |param: Option<Type>, result: Type, text: bool| FileSig {
        param,
        result,
        text,
    };
    match name {
        "read" | "readline" if writes => Err("file opened for writing is not readable".to_string()),
        "write" if !writes => Err("file opened for reading is not writable".to_string()),
        "read" if got > 0 => Err("`read` of a size is not supported yet".to_string()),
        "readline" if *data == Type::Bytes => {
            Err("`readline` of a binary file is not supported yet".to_string())
        }
        "read" => Ok(sig(None, data.clone(), false)),
        "readline" => Ok(sig(None, Type::Str, true)),
        "write" => Ok(sig(Some(data.clone()), Type::Int, false)),
        _ => Ok(sig(None, Type::None, false)),
    }
}

/// Whether file method `name` is had only by a file that is written
/// (`Some(true)`) or read (`Some(false)`): what a call to it says of a
/// receiver whose type is not known yet.
fn file_direction(name: &str) -> Option<bool> {
    match name {
        "read" | "readline" => Some(false),
        "write" => Some(true),
        _ => None,
    }
}

/// The positional arguments `owner`'s method `name` takes, if it has
/// that method: `str`, list or dict, for [`Need::Method`].
pub(super) fn arity(owner: &Type, name: &str) -> Option<(usize, usize)> {
    let owner = Owner::of(owner)?;
    let found = METHODS
        .iter()
        .find(|m| m.owner == owner && m.name == name)?;
    Some((found.min, found.max))
}

/// The error for a call to method `name` with `got` arguments.
pub(super) fn arity_message(name: &str, min: usize, max: usize, got: usize) -> String {
    let expected = match (min, max) {
        (0, 0) => "no arguments".to_string(),
        (1, 1) => "1 argument".to_string(),
        _ if min == max => format!("{min} arguments"),
        _ if max == min + 1 => format!("{min} or {max} arguments"),
        _ => format!("{min} to {max} arguments"),
    };
    format!("`{name}` takes {expected} but {got} were given")
}

impl Checker {
    /// A method call: `list.append`, the list and dict methods of D70,
    /// `set.add`, `dict.get`, `.keys`, `.values`, `.items`, a file's
    /// `.read` and `.close`, an executor's `.submit` and `.map`, and a
    /// future's `.result`. The receiver's type picks the method; while it
    /// is unknown, the one type that has the name fixes it, and a name
    /// several types share waits for it (D52).
    pub(super) fn method(&mut self, value: &Expr, attr: &Ident, args: &[Arg]) -> Result<Type> {
        let name = attr.name.as_str();
        if matches!(name, "submit" | "map") {
            return self.task(value, attr, args);
        }
        if FILE_GAPS.contains(&name) {
            let receiver = self.expr(value)?;
            if let Type::File(..) = self.subst.resolve(&receiver) {
                let msg = format!("`{name}` of a file is not supported yet");
                return Err(error(attr.span, msg));
            }
        }
        let Some(first) = lookup(&METHODS, name, None) else {
            let msg = format!(
                "unknown method `.{name}`; the methods are `str.split`, `.join`, \
                 `.strip`, `.lstrip`, `.rstrip`, `.replace`, `.startswith`, \
                 `.endswith`, `.find`, `.rfind`, `.index`, `.rindex`, `.count`, \
                 `.upper` and `.lower`, `list.append`, `.pop`, `.extend`, \
                 `.insert`, `.remove`, `.index`, `.count` and `.sort`, `set.add`, \
                 `dict.get`, `.pop`, `.setdefault`, `.update`, `.keys`, \
                 `.values` and `.items`, a file's `.read`, `.readline`, \
                 `.write`, `.flush` and `.close`, an executor's `.submit` and `.map`, and a future's \
                 `.result`"
            );
            return Err(error(attr.span, msg));
        };
        let receiver = self.expr(value)?;
        let here = Owner::of(&self.subst.resolve(&receiver));
        // A file method fixes an unknown receiver as a file that is
        // written or read, its data still open, so a generic function's
        // call is typed by its caller; `flush` and `close` wait for it.
        if first.owner == Owner::File && here != Some(Owner::File) {
            let Some(writes) = file_direction(name) else {
                // `None` whatever the file, so known in generic code too.
                let out = self.deferred_method(first.name, receiver, args, attr.span)?;
                self.expect(attr.span, &Type::None, &out, "a file's `flush` and `close`")?;
                return Ok(Type::None);
            };
            let file = Type::File(writes, Box::new(self.subst.fresh()));
            self.expect(value.span, &file, &receiver, "a file method needs a file")?;
        } else if deferred(name) && here != Some(Owner::Str) {
            return self.deferred_method(first.name, receiver, args, attr.span);
        }
        let found = lookup(&METHODS, name, here).unwrap_or(first);
        if found.owner != Owner::Str {
            method_arity(name, args, found.min, found.max, attr.span)?;
        }
        match found.owner {
            Owner::Str => self.str_method(found, value.span, &receiver, args, attr.span),
            Owner::List | Owner::Set => self.insert(name, value.span, &receiver, &args[0]),
            Owner::Dict if name == "get" => self.get(value.span, &receiver, args, attr.span),
            Owner::Dict => self.view(name, value.span, &receiver),
            Owner::File => self.file(name, &receiver, args, attr.span),
            Owner::Future => self.result(value.span, &receiver),
        }
    }

    /// A method [`deferred`] to the solver: its positional arguments'
    /// types wait with the receiver's for [`Need::Method`]. Only
    /// `list.sort` takes a keyword, `reverse=`; `dict.update` takes none
    /// yet (D70).
    fn deferred_method(
        &mut self,
        name: &'static str,
        receiver: Type,
        args: &[Arg],
        span: Span,
    ) -> Result<Type> {
        if name == "sort" {
            self.sort_keywords("list.sort", args)?;
            if let Some(arg) = args.iter().find(|a| a.name.is_none()) {
                return Err(error(
                    arg.span,
                    "`sort` takes no positional arguments".into(),
                ));
            }
        } else if let Some(arg) = args.iter().find(|a| a.name.is_some()) {
            let msg = match name {
                "update" => "keyword arguments of `dict.update` are not supported yet".to_string(),
                _ => format!("`{name}` takes no keyword arguments"),
            };
            return Err(error(arg.span, msg));
        }
        let mut types = Vec::new();
        for arg in args.iter().filter(|a| a.name.is_none()) {
            types.push(self.expr(&arg.value)?);
        }
        let out = self.subst.fresh();
        let need = Need::Method {
            name,
            receiver,
            args: types,
            out: out.clone(),
        };
        self.need(need, span);
        Ok(out)
    }

    /// The keywords of `list.sort` and `sorted`, named `what`: `reverse=`
    /// of a bool or int; `key=` is not supported yet (D70).
    pub(super) fn sort_keywords(&mut self, what: &str, args: &[Arg]) -> Result<()> {
        for arg in args {
            let Some(kw) = &arg.name else {
                continue;
            };
            match kw.name.as_str() {
                "reverse" => {
                    let t = self.expr(&arg.value)?;
                    if self.subst.resolve(&t) != Type::Int {
                        let what = format!("`reverse=` of `{what}`");
                        self.expect(arg.value.span, &Type::Bool, &t, &what)?;
                    }
                }
                "key" => {
                    let msg = format!("`key=` of `{what}` is not supported yet");
                    return Err(error(arg.span, msg));
                }
                other => {
                    let msg = format!("`{what}` got an unexpected keyword argument `{other}`");
                    return Err(error(arg.span, msg));
                }
            }
        }
        Ok(())
    }

    /// A `str` method (D53): its arguments are `str`s, positional, and
    /// only those supported yet.
    fn str_method(
        &mut self,
        found: Method,
        at: Span,
        receiver: &Type,
        args: &[Arg],
        span: Span,
    ) -> Result<Type> {
        let name = found.name;
        if let Some(arg) = args.iter().find(|a| a.name.is_some()) {
            let kw = arg.name.as_ref().map_or("", |n| n.name.as_str());
            let msg = format!("`{kw}=` of `str.{name}` is not supported yet");
            return Err(error(arg.span, msg));
        }
        method_arity(name, args, found.min, found.max, span)?;
        if let Some((arg, msg)) = unsupported_arg(name, args) {
            return Err(error(arg.span, msg));
        }
        if let ("split", [sep]) = (name, args) {
            if matches!(&sep.value.kind, ExprKind::Str(s) if s.is_empty()) {
                let msg = "`str.split` of an empty separator raises \
                           `ValueError: empty separator`";
                return Err(error(sep.span, msg.to_string()));
            }
        }
        let needs = format!("`{name}` needs a str");
        self.expect(at, &Type::Str, receiver, &needs)?;
        if name == "join" {
            self.join_arg(&args[0])?;
            return Ok(Type::Str);
        }
        for arg in args {
            let t = self.expr(&arg.value)?;
            let affix = matches!(name, "startswith" | "endswith");
            if affix && matches!(self.subst.resolve(&t), Type::Tuple(_)) {
                let msg = format!("`str.{name}` of a tuple is not supported yet");
                return Err(error(arg.value.span, msg));
            }
            let what = format!("argument of `str.{name}`");
            self.expect(arg.value.span, &Type::Str, &t, &what)?;
        }
        Ok(str_result(name))
    }

    /// `sep.join(xs)`: `xs` is a `list[str]`. Another iterable of `str`
    /// (a dict's being its keys) is not supported yet, a listed
    /// deviation; its element type is checked first, so a wrong one is
    /// the ordinary type error.
    fn join_arg(&mut self, arg: &Arg) -> Result<()> {
        let t = self.consumed(&arg.value)?;
        // Solved first, so an `enumerate` or `zip` shows its item type.
        self.settle()?;
        let at = arg.value.span;
        let what = "the items `str.join` joins";
        let resolved = self.subst.resolve(&t);
        let elems: Vec<Type> = match &resolved {
            Type::Set(e) | Type::View(_, e) | Type::Dict(e, _) => vec![(**e).clone()],
            Type::Tuple(items) => items.clone(),
            Type::Str => Vec::new(),
            _ => {
                let list = Type::List(Box::new(Type::Str));
                return self.expect(at, &list, &t, "`str.join` needs a list of str");
            }
        };
        for e in &elems {
            self.expect(at, &Type::Str, e, what)?;
        }
        let shown = self.subst.zonk(&t);
        let msg = format!("`str.join` of {shown} is not supported yet; use `list(...)`");
        Err(error(at, msg))
    }

    /// A method of a file, written or not (D91, D92).
    fn file(&mut self, name: &str, receiver: &Type, args: &[Arg], span: Span) -> Result<Type> {
        let Type::File(writes, data) = self.subst.resolve(receiver) else {
            return Err(error(span, format!("internal: `{name}` of no file")));
        };
        let mut types = Vec::new();
        for arg in args {
            types.push((self.expr(&arg.value)?, arg.value.span));
        }
        let data = self.subst.resolve(&data);
        let sig = file_signature(name, writes, &data, args.len()).map_err(|m| error(span, m))?;
        if sig.text {
            self.expect(span, &Type::Str, &data, "`readline` needs a text file")?;
        }
        if let (Some(param), Some((t, at))) = (sig.param, types.first()) {
            let what = format!("argument of `{name}`");
            self.expect(*at, &param, t, &what)?;
        }
        Ok(sig.result)
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

/// What `str.{name}` gives: `split` a `list[str]`, the tests a `bool`,
/// the searches an `int` index or count, the rest a `str`.
fn str_result(name: &str) -> Type {
    match name {
        "split" => Type::List(Box::new(Type::Str)),
        "startswith" | "endswith" => Type::Bool,
        "find" | "rfind" | "index" | "rindex" | "count" => Type::Int,
        _ => Type::Str,
    }
}

/// The first argument of a `str` method not supported yet, with its
/// error: `maxsplit`, `replace`'s `count` or `None` (D53), a search's
/// `start` (and so `end`) or a tuple of prefixes or suffixes (D55). A
/// literal `""` separator of `split` is refused too: it can only raise.
fn unsupported_arg<'a>(name: &str, args: &'a [Arg]) -> Option<(&'a Arg, String)> {
    let (arg, what) = unsupported(name, args)?;
    let msg = match what {
        "a tuple" => format!("`str.{name}` of a tuple is not supported yet"),
        _ => format!("{what} of `str.{name}` is not supported yet"),
    };
    Some((arg, msg))
}

/// [`unsupported_arg`]'s argument and what it is.
fn unsupported<'a>(name: &str, args: &'a [Arg]) -> Option<(&'a Arg, &'static str)> {
    let search = matches!(
        name,
        "startswith" | "endswith" | "find" | "rfind" | "index" | "rindex" | "count"
    );
    match (name, args) {
        ("split", [_, extra]) => return Some((extra, "`maxsplit`")),
        ("replace", [_, _, extra]) => return Some((extra, "`count`")),
        (_, [_, start, ..]) if search => return Some((start, "`start`")),
        ("startswith" | "endswith", [affix]) if matches!(affix.value.kind, ExprKind::Tuple(_)) => {
            return Some((affix, "a tuple"));
        }
        _ => {}
    }
    args.iter()
        .find(|a| matches!(a.value.kind, ExprKind::None))
        .map(|a| (a, "a `None` argument"))
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
    Err(error(span, arity_message(name, min, max, args.len())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{infer, lexer, parser};

    /// `check`'s result for `src`: `Ok` or the first error's message.
    fn checked(src: &str) -> std::result::Result<(), String> {
        let module = lexer::lex(src)
            .and_then(|t| parser::parse(&t))
            .map_err(|d| format!("parse: {}", d.message))?;
        infer::infer(&module).map(|_| ()).map_err(|d| d.message)
    }

    fn refused(src: &str) -> String {
        match checked(src) {
            Ok(()) => panic!("accepted: {src}"),
            Err(msg) => msg,
        }
    }

    #[test]
    fn str_methods_take_and_give_strings() {
        let src = "def f(s, sep):\n    \
                   return sep.join(s.replace(\"a\", \"b\").strip().split(sep))\n\
                   print(f(\" a,b \", \",\"))\n\
                   print(\"x\".lstrip(\"y\") + \"x\".rstrip() + \"\".join([]))\n";
        assert_eq!(checked(src), Ok(()));
    }

    #[test]
    fn str_methods_need_a_str_receiver_and_str_arguments() {
        assert!(refused("x = 1\nprint(x.split())\n").contains("`split` needs a str"));
        let msg = refused("print(\"a\".replace(\"a\", 1))\n");
        assert!(msg.contains("argument of `str.replace`"), "{msg}");
        let msg = refused("print(\",\".join([1]))\n");
        assert!(msg.contains("`str.join` needs a list of str"), "{msg}");
    }

    #[test]
    fn join_of_another_iterable_names_its_type_after_its_elements() {
        let msg = refused("print(\",\".join({\"a\"}))\n");
        assert!(
            msg.contains("`str.join` of set[str] is not supported yet; use `list(...)`"),
            "{msg}"
        );
        let msg = refused("print(\",\".join(\"ab\"))\n");
        assert!(
            msg.contains("`str.join` of str is not supported yet"),
            "{msg}"
        );
        let msg = refused("print(\",\".join({1}))\n");
        assert!(msg.contains("the items `str.join` joins"), "{msg}");
        let msg = refused("print(\",\".join({\"k\": 1}))\n");
        assert!(
            msg.contains("`str.join` of dict[str, int] is not supported yet"),
            "{msg}"
        );
        let msg = refused("print(\",\".join({1: \"v\"}))\n");
        assert!(
            msg.contains("the items `str.join` joins") && msg.contains("int"),
            "{msg}"
        );
    }

    #[test]
    fn arguments_not_supported_yet_are_named() {
        let cases = [
            ("\"a\".split(\",\", 1)", "`maxsplit` of `str.split`"),
            ("\"a\".split(sep=\",\")", "`sep=` of `str.split`"),
            ("\"a\".replace(\"a\", \"b\", 1)", "`count` of `str.replace`"),
            ("\"a\".strip(chars=\"a\")", "`chars=` of `str.strip`"),
            ("\"a\".split(None)", "a `None` argument of `str.split`"),
        ];
        for (call, want) in cases {
            let msg = refused(&format!("print({call})\n"));
            assert!(
                msg.contains(want) && msg.contains("not supported yet"),
                "{call}: {msg}"
            );
        }
        let msg = refused("print(\"a\".strip(\"a\", \"b\"))\n");
        assert!(msg.contains("takes"), "{msg}");
    }

    #[test]
    fn str_b_methods_give_bools_ints_and_strings() {
        let src = "s = \"αβγ\"\n\
                   ok: bool = s.startswith(\"α\") and s.endswith(\"γ\")\n\
                   at: int = s.find(\"β\") + s.rfind(\"β\") + s.index(\"β\") + s.rindex(\"β\")\n\
                   n: int = s.count(\"β\")\n\
                   t: str = s.upper() + s.lower()\n\
                   print(ok, at, n, t)\n";
        assert_eq!(checked(src), Ok(()));
        let msg = refused("print(\"a\".find(1))\n");
        assert!(msg.contains("argument of `str.find`"), "{msg}");
        let msg = refused("print(\"a\".upper(\"b\"))\n");
        assert!(msg.contains("`upper` takes no arguments"), "{msg}");
        let msg = refused("print(\"a\".count())\n");
        assert!(msg.contains("`count` takes 1 to 3 arguments"), "{msg}");
    }

    #[test]
    fn start_end_and_prefix_tuples_are_not_supported_yet() {
        let cases = [
            ("\"a\".find(\"a\", 1)", "`start` of `str.find`"),
            ("\"a\".count(\"a\", 0, 1)", "`start` of `str.count`"),
            ("\"a\".startswith(\"a\", 1)", "`start` of `str.startswith`"),
            (
                "\"a\".startswith((\"a\", \"b\"))",
                "`str.startswith` of a tuple",
            ),
            ("\"a\".endswith((\"a\",))", "`str.endswith` of a tuple"),
        ];
        for (call, want) in cases {
            let msg = refused(&format!("print({call})\n"));
            assert!(
                msg.contains(want) && msg.contains("not supported yet"),
                "{call}: {msg}"
            );
        }
        for name in ["startswith", "endswith"] {
            let held = refused(&format!("t = (\"a\", \"b\")\nprint(\"abc\".{name}(t))\n"));
            let want = format!("`str.{name}` of a tuple is not supported yet");
            assert!(held.contains(&want), "{name}: {held}");
        }
    }

    #[test]
    fn only_a_literal_empty_separator_is_a_compile_error() {
        let msg = refused("print(\"a\".split(\"\"))\n");
        assert!(
            msg.contains("raises `ValueError: empty separator`"),
            "{msg}"
        );
        let computed = "sep = \"x\".replace(\"x\", \"\")\nprint(\"a\".split(sep))\n";
        assert_eq!(checked(computed), Ok(()));
        assert_eq!(
            checked("print(\"a\".replace(\"\", \"-\").strip(\"\"))\n"),
            Ok(())
        );
    }

    #[test]
    fn only_index_count_and_pop_have_two_owners_and_all_are_deferred() {
        // `lookup` gives a receiver still unknown the first owner; a
        // shared name instead waits for the receiver's type (D52).
        let mut shared: Vec<&str> = Vec::new();
        for (i, a) in METHODS.iter().enumerate() {
            if METHODS[i + 1..].iter().any(|b| b.name == a.name) {
                shared.push(a.name);
            }
        }
        shared.sort_unstable();
        assert_eq!(shared, ["count", "index", "pop"]);
        assert!(shared.iter().all(|n| deferred(n)));
        assert!(!deferred("append") && !deferred("get"));
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
        assert!(lookup(&METHODS, "splitlines", None).is_none());
        assert!(lookup(&METHODS, "splitlines", Some(Owner::Str)).is_none());
    }
}
