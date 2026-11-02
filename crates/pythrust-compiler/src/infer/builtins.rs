//! The closed builtin functions: `print len list set sorted range str int
//! float bool open`,
//! and the builtin exception classes (D6, B4), which build an `Exception`
//! when called.

use crate::ast::{Arg, ExprKind};
use crate::diag::Result;
use crate::exc::ExcKind;
use crate::span::Span;
use crate::stdlib::StdFn;

use super::solve::Need;
use super::ty::Type;
use super::{error, Checker};

/// The builtin exception classes, sorted, for messages that list them.
pub(super) fn exception_names() -> String {
    let mut names: Vec<&str> = ExcKind::ALL.into_iter().map(ExcKind::name).collect();
    names.sort_unstable();
    names.join(", ")
}

/// Whether `name` is a builtin function, exception class or module
/// function, when nothing shadows it.
pub fn is_builtin(name: &str) -> bool {
    let function = matches!(
        name,
        "print" | "len" | "list" | "set" | "sorted" | "range" | "str" | "int" | "float" | "bool"
    );
    function || is_exception(name) || StdFn::from_name(name).is_some()
}

pub(super) fn is_exception(name: &str) -> bool {
    ExcKind::from_name(name).is_some()
}

impl Checker {
    /// A call to builtin `name`, which no variable or `def` shadows.
    pub(super) fn builtin(&mut self, name: &str, args: &[Arg], span: Span) -> Result<Type> {
        match name {
            "print" => self.print(args),
            "len" => {
                for (t, _) in self.positional(name, args, 1, 1, span)? {
                    self.need(Need::Len(t), span);
                }
                Ok(Type::Int)
            }
            "list" => self.copy_list(args, span),
            "set" => self.new_set(args, span),
            "sorted" => self.sorted(args, span),
            "range" => self.range(args, span),
            "str" | "bool" => {
                self.positional(name, args, 0, 1, span)?;
                Ok(if name == "str" { Type::Str } else { Type::Bool })
            }
            "int" => self.number("int", Type::Int, args, span),
            "float" => self.number("float", Type::Float, args, span),
            // Any one value as the message, as `str(x)` takes.
            _ if StdFn::from_name(name).is_some() => self.std(name, args, span),
            _ if is_exception(name) => {
                self.positional(name, args, 0, 1, span)?;
                Ok(Type::Exception)
            }
            _ => Err(error(span, format!("internal: `{name}` is not a builtin"))),
        }
    }

    /// A call to a [`StdFn`] by its builtin name. Every argument is a
    /// `str` (a path), but `open`'s mode.
    fn std(&mut self, name: &str, args: &[Arg], span: Span) -> Result<Type> {
        let Some(f) = StdFn::from_name(name) else {
            return Err(error(
                span,
                format!("internal: `{name}` is not a module function"),
            ));
        };
        let strs = || Type::List(Box::new(Type::Str));
        let (min, max, result) = match f {
            StdFn::Argv => (0, 0, strs()),
            StdFn::Listdir => (0, 1, strs()),
            StdFn::Join => (1, usize::MAX, Type::Str),
            StdFn::Isdir | StdFn::Isfile | StdFn::Islink => (1, 1, Type::Bool),
            StdFn::Getsize => (1, 1, Type::Int),
            StdFn::Executor => (0, 0, Type::Executor),
            StdFn::Open => return self.open(args, span),
            StdFn::Sha256 => return self.sha256(args, span),
            StdFn::Read | StdFn::Close => {
                let msg = format!("internal: `{name}` is a method");
                return Err(error(span, msg));
            }
        };
        let what = format!("argument of `{name}`");
        let args = by_position(f, args)?;
        for (t, at) in self.positional(name, &args, min, max, span)? {
            self.expect(at, &Type::Str, &t, &what)?;
        }
        Ok(result)
    }

    /// `open(path, "rb")`: the one mode supported, written as a literal
    /// so the file's type is known.
    fn open(&mut self, args: &[Arg], span: Span) -> Result<Type> {
        let mut found = self.positional("open", args, 1, 2, span)?.into_iter();
        if let Some((t, at)) = found.next() {
            self.expect(at, &Type::Str, &t, "argument of `open`")?;
        }
        let rb = args
            .get(1)
            .is_some_and(|a| matches!(&a.value.kind, ExprKind::Str(m) if m == "rb"));
        if !rb {
            let at = args.get(1).map_or(span, |a| a.value.span);
            let msg = "only binary reading is supported yet: write `open(path, \"rb\")`";
            return Err(error(at, msg.to_string()));
        }
        Ok(Type::File)
    }

    /// `hashlib.sha256(data).hexdigest()`, which the parser made one
    /// call: the hex digest of `bytes`.
    fn sha256(&mut self, args: &[Arg], span: Span) -> Result<Type> {
        for (t, at) in self.positional("hashlib.sha256", args, 1, 1, span)? {
            self.expect(at, &Type::Bytes, &t, "argument of `hashlib.sha256`")?;
        }
        Ok(Type::Str)
    }

    /// Any positional values; `sep` and `end` must be `str`.
    fn print(&mut self, args: &[Arg]) -> Result<Type> {
        for arg in args {
            let t = self.expr(&arg.value)?;
            let Some(kw) = &arg.name else {
                continue;
            };
            if kw.name != "sep" && kw.name != "end" {
                let msg = format!("`print` got an unexpected keyword argument `{}`", kw.name);
                return Err(error(kw.span, msg));
            }
            let what = format!("`{}` of `print`", kw.name);
            self.expect(arg.value.span, &Type::Str, &t, &what)?;
        }
        Ok(Type::None)
    }

    /// `list(x)`: a new list of a list's elements (a shallow copy), a
    /// dict's keys, a set's elements or a dict view's items.
    fn copy_list(&mut self, args: &[Arg], span: Span) -> Result<Type> {
        let elem = self.subst.fresh();
        for (arg, _) in self.positional("list", args, 1, 1, span)? {
            let need = Need::ToList {
                arg,
                elem: elem.clone(),
            };
            self.need(need, span);
        }
        Ok(Type::List(Box::new(elem)))
    }

    /// `sorted(xs)`: a new list of what `xs` iterates over, in `<` order.
    fn sorted(&mut self, args: &[Arg], span: Span) -> Result<Type> {
        let elem = self.subst.fresh();
        for (iterable, _) in self.positional("sorted", args, 1, 1, span)? {
            let need = Need::Iter {
                iterable,
                elem: elem.clone(),
            };
            self.need(need, span);
        }
        let order = Need::Order {
            left: elem.clone(),
            right: elem.clone(),
        };
        self.need(order, span);
        Ok(Type::List(Box::new(elem)))
    }

    /// `set()` or `set(xs)`: a new set of what `xs` iterates over.
    fn new_set(&mut self, args: &[Arg], span: Span) -> Result<Type> {
        let elem = self.subst.fresh();
        for (iterable, _) in self.positional("set", args, 0, 1, span)? {
            let need = Need::Iter {
                iterable,
                elem: elem.clone(),
            };
            self.need(need, span);
        }
        self.need(Need::Hashable(elem.clone()), span);
        Ok(Type::Set(Box::new(elem)))
    }

    fn range(&mut self, args: &[Arg], span: Span) -> Result<Type> {
        for (t, at) in self.positional("range", args, 1, 3, span)? {
            if !self.subst.resolve(&t).is_intlike() {
                self.expect(at, &Type::Int, &t, "`range` takes ints")?;
            }
        }
        Ok(Type::Range)
    }

    /// `int(x)` / `float(x)`: `x` must be a number, bool or str.
    fn number(&mut self, func: &'static str, out: Type, args: &[Arg], span: Span) -> Result<Type> {
        for (arg, _) in self.positional(func, args, 0, 1, span)? {
            self.need(Need::ToNumber { func, arg }, span);
        }
        Ok(out)
    }

    /// Infers `min..=max` positional arguments, rejecting keywords.
    fn positional(
        &mut self,
        name: &str,
        args: &[Arg],
        min: usize,
        max: usize,
        span: Span,
    ) -> Result<Vec<(Type, Span)>> {
        if let Some(kw) = args.iter().find_map(|a| a.name.as_ref()) {
            return Err(error(
                kw.span,
                format!("`{name}` takes no keyword arguments"),
            ));
        }
        if args.len() < min || args.len() > max {
            let expected = if min == max {
                min.to_string()
            } else {
                format!("{min} to {max}")
            };
            let msg = format!(
                "`{name}` takes {expected} arguments but {} were given",
                args.len()
            );
            return Err(error(span, msg));
        }
        args.iter()
            .map(|a| Ok((self.expr(&a.value)?, a.value.span)))
            .collect()
    }
}

/// `args` with a keyword argument naming `f`'s [`StdFn::keyword`]
/// parameter made positional; any other keyword is an error. Since that
/// parameter is a function's only one, a second argument is an arity
/// error after this.
fn by_position(f: StdFn, args: &[Arg]) -> Result<Vec<Arg>> {
    args.iter()
        .map(|a| match &a.name {
            None => Ok(a.clone()),
            Some(kw) if Some(kw.name.as_str()) == f.keyword() => Ok(Arg {
                name: None,
                ..a.clone()
            }),
            Some(kw) => {
                let msg = format!(
                    "`{}` got an unexpected keyword argument `{}`",
                    f.name(),
                    kw.name
                );
                Err(error(kw.span, msg))
            }
        })
        .collect()
}
