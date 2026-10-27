//! The closed builtin functions: `print len range str int float bool`, and
//! the builtin exceptions (D6), which build an `Exception` when called.

use crate::ast::Arg;
use crate::diag::Result;
use crate::span::Span;

use super::solve::Need;
use super::ty::Type;
use super::{error, Checker};

/// The builtin exceptions (`docs/grammar.md`, D6).
pub(super) const EXCEPTIONS: [&str; 5] = [
    "ValueError",
    "IndexError",
    "ZeroDivisionError",
    "OverflowError",
    "RecursionError",
];

pub(super) fn is_builtin(name: &str) -> bool {
    let function = matches!(
        name,
        "print" | "len" | "range" | "str" | "int" | "float" | "bool"
    );
    function || is_exception(name)
}

pub(super) fn is_exception(name: &str) -> bool {
    EXCEPTIONS.contains(&name)
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
            "range" => self.range(args, span),
            "str" | "bool" => {
                self.positional(name, args, 0, 1, span)?;
                Ok(if name == "str" { Type::Str } else { Type::Bool })
            }
            "int" => self.number("int", Type::Int, args, span),
            "float" => self.number("float", Type::Float, args, span),
            // Any one value as the message, as `str(x)` takes.
            _ if is_exception(name) => {
                self.positional(name, args, 0, 1, span)?;
                Ok(Type::Exception)
            }
            _ => Err(error(span, format!("internal: `{name}` is not a builtin"))),
        }
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
