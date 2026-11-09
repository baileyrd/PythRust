//! The closed builtin functions: `print len list set sorted range str int
//! float bool open abs ord chr sum min max any all` (D56, D57),
//! and the builtin exception classes (D6, B4), which build an `Exception`
//! when called.

use crate::ast::{Arg, Expr, ExprKind};
use crate::diag::Result;
use crate::exc::ExcKind;
use crate::span::Span;
use crate::stdlib::StdFn;

use super::expr::int_literal;
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
        "print"
            | "len"
            | "list"
            | "set"
            | "sorted"
            | "range"
            | "str"
            | "int"
            | "float"
            | "bool"
            | "abs"
            | "ord"
            | "chr"
            | "sum"
            | "min"
            | "max"
            | "any"
            | "all"
            | "enumerate"
            | "zip"
    );
    function || is_exception(name) || StdFn::from_name(name).is_some()
}

/// Whether builtin `name` reads `map`'s results where it is called,
/// taking each task's result: `list`, `sorted` and `set`, and the
/// reducing builtins (D57).
pub fn reads_results(name: &str) -> bool {
    matches!(
        name,
        "list" | "sorted" | "set" | "sum" | "min" | "max" | "any" | "all" | "enumerate" | "zip"
    )
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
            "abs" => {
                let (arg, _) = self.one(name, args, span)?;
                let out = self.subst.fresh();
                let need = Need::Numeric {
                    func: "abs",
                    arg,
                    out: out.clone(),
                };
                self.need(need, span);
                Ok(out)
            }
            "ord" => self.ord(args, span),
            "chr" => self.chr(args, span),
            "sum" => self.sum(args, span),
            "min" | "max" => self.extreme(name, args, span),
            "any" | "all" => {
                let (iterable, _) = self.one(name, args, span)?;
                let elem = self.subst.fresh();
                self.need(Need::Iter { iterable, elem }, span);
                Ok(Type::Bool)
            }
            "float" => self.number("float", Type::Float, args, span),
            "enumerate" | "zip" if self.consumed != Some(span) => {
                let msg = format!("`{name}(...)` is not a value yet; use `list({name}(...))`");
                Err(error(span, msg))
            }
            "enumerate" => self.enumerate(args, span),
            "zip" => self.zip(args, span),
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
            StdFn::Read
            | StdFn::Close
            | StdFn::Split
            | StdFn::StrJoin
            | StdFn::Strip
            | StdFn::Lstrip
            | StdFn::Rstrip
            | StdFn::Replace
            | StdFn::Startswith
            | StdFn::Endswith
            | StdFn::Find
            | StdFn::Rfind
            | StdFn::Index
            | StdFn::Rindex
            | StdFn::Count
            | StdFn::Upper
            | StdFn::Lower
            | StdFn::Ord
            | StdFn::Chr
            | StdFn::FloatSum
            | StdFn::ListIndex
            | StdFn::ListFind
            | StdFn::ListCount => {
                let msg = format!("internal: `{name}` is not called by its name");
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

    /// `sorted(xs)`: a new list of what `xs` iterates over, in `<` order,
    /// or reversed with `reverse=` (D70).
    fn sorted(&mut self, args: &[Arg], span: Span) -> Result<Type> {
        let elem = self.subst.fresh();
        self.sort_keywords("sorted", args)?;
        let positional: Vec<Arg> = args.iter().filter(|a| a.name.is_none()).cloned().collect();
        for (iterable, _) in self.positional("sorted", &positional, 1, 1, span)? {
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

    /// `ord(c)`: the code point of a one-character `str` (D56). A literal
    /// of another length can only fail, so it is refused here.
    fn ord(&mut self, args: &[Arg], span: Span) -> Result<Type> {
        if let [Arg {
            name: None, value, ..
        }] = args
        {
            if let ExprKind::Str(lit) = &value.kind {
                let n = lit.chars().count();
                if n != 1 {
                    let msg = format!(
                        "`ord` of a string of length {n} raises `TypeError: ord() expected a \
                         character, but string of length {n} found`"
                    );
                    return Err(error(value.span, msg));
                }
            }
        }
        let (t, at) = self.one("ord", args, span)?;
        if self.subst.resolve(&t) == Type::Bytes {
            return Err(error(at, "`ord` of bytes is not supported yet".to_string()));
        }
        self.expect(at, &Type::Str, &t, "argument of `ord`")?;
        Ok(Type::Int)
    }

    /// `chr(n)`: the one-character `str` of code point `n`, an int or
    /// bool (D56). A literal that can only fail is refused here.
    fn chr(&mut self, args: &[Arg], span: Span) -> Result<Type> {
        if let [Arg {
            name: None, value, ..
        }] = args
        {
            if let Some(msg) = chr_literal(value).and_then(chr_refusal) {
                return Err(error(value.span, msg));
            }
        }
        let (t, at) = self.one("chr", args, span)?;
        if !self.subst.resolve(&t).is_intlike() {
            self.expect(at, &Type::Int, &t, "argument of `chr`")?;
        }
        Ok(Type::Str)
    }

    /// `sum(xs)` or `sum(xs, start)`: the int or float total of what `xs`
    /// iterates over (D57). `start`, by position or keyword, has the
    /// total's type.
    fn sum(&mut self, args: &[Arg], span: Span) -> Result<Type> {
        let args = keyword_last("sum", "start", args)?;
        let mut found = self.positional("sum", &args, 1, 2, span)?.into_iter();
        let Some((iterable, _)) = found.next() else {
            return Err(error(span, "internal: `sum` without arguments".to_string()));
        };
        let elem = self.subst.fresh();
        let need = Need::Iter {
            iterable,
            elem: elem.clone(),
        };
        self.need(need, span);
        let out = found.next().map_or_else(|| self.subst.fresh(), |(t, _)| t);
        let need = Need::Numeric {
            func: "sum",
            arg: elem,
            out: out.clone(),
        };
        self.need(need, span);
        Ok(out)
    }

    /// `min(xs)` / `max(xs)`, of what `xs` iterates over, or `min(a, b,
    /// ...)`: arguments of one ordered type (D57). An empty `xs` raises.
    fn extreme(&mut self, name: &str, args: &[Arg], span: Span) -> Result<Type> {
        if let Some(kw) = args.iter().find_map(|a| a.name.as_ref()) {
            let msg = format!("`{}=` of `{name}` is not supported yet", kw.name);
            return Err(error(kw.span, msg));
        }
        if args.is_empty() {
            let msg = format!("`{name}` takes 1 or more arguments but 0 were given");
            return Err(error(span, msg));
        }
        let found = self.positional(name, args, 1, usize::MAX, span)?;
        let (first, rest) = match found.split_first() {
            Some((first, rest)) => (first.0.clone(), rest),
            None => return Err(error(span, format!("internal: `{name}` without arguments"))),
        };
        let elem = if rest.is_empty() {
            let elem = self.subst.fresh();
            let need = Need::Iter {
                iterable: first,
                elem: elem.clone(),
            };
            self.need(need, span);
            elem
        } else {
            for (t, at) in rest {
                self.same_extreme(name, &first, t, *at)?;
            }
            first
        };
        let order = Need::Order {
            left: elem.clone(),
            right: elem.clone(),
        };
        self.need(order, span);
        Ok(elem)
    }

    /// `t`, another argument of `min` or `max`, has `first`'s type; mixed
    /// numbers are named, since one fixed result type cannot hold both.
    fn same_extreme(&mut self, name: &str, first: &Type, t: &Type, at: Span) -> Result<()> {
        let (a, b) = (self.subst.resolve(first), self.subst.resolve(t));
        if a.is_numeric() && b.is_numeric() && a != b {
            let (a, b) = (self.subst.zonk(&a), self.subst.zonk(&b));
            let msg = format!("`{name}` of {a} and {b}: convert with `float(...)`");
            return Err(error(at, msg));
        }
        self.expect(at, first, t, &format!("argument of `{name}`"))
    }

    /// The one positional argument of `name`.
    fn one(&mut self, name: &str, args: &[Arg], span: Span) -> Result<(Type, Span)> {
        let mut found = self.positional(name, args, 1, 1, span)?.into_iter();
        found
            .next()
            .ok_or_else(|| error(span, format!("internal: `{name}` without its argument")))
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
        let mut found = Vec::new();
        for (k, a) in args.iter().enumerate() {
            let t = match consumes(name, args.len(), k) {
                true => self.consumed(&a.value)?,
                false => self.expr(&a.value)?,
            };
            found.push((t, a.value.span));
        }
        Ok(found)
    }

    /// `e`, where it is consumed directly: an `enumerate(...)` or
    /// `zip(...)` there is allowed (D51).
    pub(super) fn consumed(&mut self, e: &Expr) -> Result<Type> {
        let outer = self.consumed.replace(e.span);
        let t = self.expr(e);
        self.consumed = outer;
        t
    }

    /// `enumerate(xs)` or `enumerate(xs, start)`, `start` by position or
    /// keyword an int: the `(index, item)` pairs, typed as a list of
    /// them, though they are never one (D51).
    fn enumerate(&mut self, args: &[Arg], span: Span) -> Result<Type> {
        let args = keyword_last("enumerate", "start", args)?;
        let found = self.positional("enumerate", &args, 1, 2, span)?;
        let mut found = found.into_iter();
        let Some((iterable, _)) = found.next() else {
            return Err(error(
                span,
                "internal: `enumerate` without arguments".to_string(),
            ));
        };
        if let Some((start, at)) = found.next() {
            if !self.subst.resolve(&start).is_intlike() {
                self.expect(at, &Type::Int, &start, "`start` of `enumerate`")?;
            }
        }
        let elem = self.subst.fresh();
        let need = Need::Iter {
            iterable,
            elem: elem.clone(),
        };
        self.need(need, span);
        let pair = Type::Tuple(vec![Type::Int, elem]);
        Ok(Type::List(Box::new(pair)))
    }

    /// `zip(xs, ys, ...)`: tuples of one item of each, to the shortest,
    /// typed as a list of them, though they are never one (D51).
    fn zip(&mut self, args: &[Arg], span: Span) -> Result<Type> {
        if let Some(kw) = args.iter().find_map(|a| a.name.as_ref()) {
            let msg = match kw.name.as_str() {
                "strict" => "`strict=` of `zip` is not supported yet".to_string(),
                other => format!("`zip` got an unexpected keyword argument `{other}`"),
            };
            return Err(error(kw.span, msg));
        }
        let mut items = Vec::new();
        for (iterable, _) in self.positional("zip", args, 0, usize::MAX, span)? {
            let elem = self.subst.fresh();
            let need = Need::Iter {
                iterable,
                elem: elem.clone(),
            };
            self.need(need, span);
            items.push(elem);
        }
        Ok(Type::List(Box::new(Type::Tuple(items))))
    }
}

/// Whether builtin `name` given `n` positional arguments consumes its
/// `k`th directly, so an `enumerate(...)` or `zip(...)` may be it (D51,
/// D58).
fn consumes(name: &str, n: usize, k: usize) -> bool {
    match name {
        "zip" => true,
        "sum" | "enumerate" => k == 0,
        "list" | "sorted" | "set" | "min" | "max" | "any" | "all" => n == 1,
        _ => false,
    }
}

/// `args` with keyword `kw`, the one `name` takes, as its last
/// positional argument; another keyword, or `kw` before the iterable
/// `name` takes first, is refused.
fn keyword_last(name: &str, kw: &str, args: &[Arg]) -> Result<Vec<Arg>> {
    if let Some(Arg { name: Some(k), .. }) = args.first() {
        if k.name == kw {
            let msg = format!("`{name}` needs its iterable before `{kw}=`");
            return Err(error(k.span, msg));
        }
    }
    args.iter()
        .map(|a| match &a.name {
            Some(k) if k.name == kw => Ok(Arg {
                name: None,
                ..a.clone()
            }),
            Some(k) => {
                let msg = format!("`{name}` got an unexpected keyword argument `{}`", k.name);
                Err(error(k.span, msg))
            }
            None => Ok(a.clone()),
        })
        .collect()
}

/// The code point a `chr` argument spells as a literal: an int, a
/// negated int or a bool.
pub(super) fn chr_literal(e: &crate::ast::Expr) -> Option<i64> {
    match e.kind {
        ExprKind::Bool(b) => Some(i64::from(b)),
        _ => int_literal(e),
    }
}

/// Why `chr(n)` of the literal `n` can only fail, if it does: out of
/// range, or a surrogate, which a `str` cannot hold (D56).
pub(super) fn chr_refusal(n: i64) -> Option<String> {
    if !(0..0x11_0000).contains(&n) {
        return Some(format!(
            "`chr` of {n} raises `ValueError: chr() arg not in range(0x110000)`"
        ));
    }
    (0xd800..0xe000)
        .contains(&n)
        .then(|| format!("`chr` of a surrogate (U+{n:04X}): a str cannot hold one"))
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
