//! Frozen records (step 4, D129-D136): their declarations, construction,
//! field reads and method calls.
//!
//! A record is a nominal type ([`Type::Record`]). Its methods are the
//! top-level functions the parser named `P.m` ([`method_name`]), so a
//! call `p.m(...)` is a call of `P.m` with `p` bound to `self`. While a
//! receiver's type is unknown, the one record that has the field or
//! method fixes it, as one type's method does (D52).

use std::collections::BTreeSet;

use crate::ast::{method_name, source_name, unqualified, Arg, Expr, Ident, Module, StmtKind};
use crate::diag::Result;
use crate::span::Span;

use super::ty::{FuncType, ParamType, Type};
use super::{error, Checker, Func};

/// A record as inference sees it.
#[derive(Debug, Clone)]
pub(super) struct Record {
    /// Its fields, in declaration order, with their annotated types.
    pub fields: Vec<(String, Type)>,
    /// Its methods' own names.
    pub methods: BTreeSet<String>,
}

impl Record {
    fn field(&self, name: &str) -> Option<&Type> {
        self.fields.iter().find(|(n, _)| n == name).map(|(_, t)| t)
    }
}

impl Checker {
    /// Declares every record of `module`, in the order they run: each
    /// field's annotation names only records before it (checked by the
    /// parser, D129).
    pub(super) fn declare_records(&mut self, module: &Module) -> Result<()> {
        for s in &module.body {
            let StmtKind::Record(r) = &s.kind else {
                continue;
            };
            let mut fields = Vec::new();
            for f in &r.fields {
                fields.push((f.name.name.clone(), self.annotation(&f.annotation)?));
            }
            let methods = r.methods.iter().map(|m| m.name.clone()).collect();
            self.records
                .insert(r.name.name.clone(), Record { fields, methods });
        }
        Ok(())
    }

    /// `P(...)`: the fields bound in order, by position or keyword, as a
    /// function's parameters are (D132).
    pub(super) fn construct(&mut self, name: &str, args: &[Arg], span: Span) -> Result<Type> {
        let Some(record) = self.records.get(name) else {
            return Err(error(span, format!("name `{name}` is not defined")));
        };
        let params = record.fields.iter().map(|(n, t)| ParamType {
            name: n.clone(),
            ty: t.clone(),
            has_default: false,
        });
        let sig = FuncType {
            params: params.collect(),
            ret: Type::Record(name.to_string()),
        };
        self.bind_args(unqualified(name), &sig, args, span)?;
        Ok(sig.ret)
    }

    /// `value.attr`, read: a field of a record.
    pub(super) fn field(&mut self, value: &Expr, attr: &Ident) -> Result<Type> {
        let receiver = self.expr(value)?;
        let record = self.owner(&receiver, attr, |r| r.field(&attr.name).is_some())?;
        let found = self.records.get(&record).and_then(|r| r.field(&attr.name));
        match found {
            Some(t) => Ok(t.clone()),
            None if self.has_method(&record, &attr.name) => {
                let msg = format!("methods can only be called: `.{}(...)`", attr.name);
                Err(error(attr.span, msg))
            }
            None => Err(no_member(&record, attr)),
        }
    }

    /// Whether some record has a method `name`.
    pub(super) fn any_method(&self, name: &str) -> bool {
        self.records.values().any(|r| r.methods.contains(name))
    }

    fn has_method(&self, record: &str, name: &str) -> bool {
        self.records
            .get(record)
            .is_some_and(|r| r.methods.contains(name))
    }

    /// `value.attr(args)` where `receiver` is `value`'s type: a call of
    /// the record's method `P.attr`, `self` bound to the receiver.
    pub(super) fn record_method(
        &mut self,
        receiver: &Type,
        value: &Expr,
        attr: &Ident,
        args: &[Arg],
        span: Span,
    ) -> Result<Type> {
        let record = self.owner(receiver, attr, |r| r.methods.contains(&attr.name))?;
        if !self.has_method(&record, &attr.name) {
            return Err(no_member(&record, attr));
        }
        let name = method_name(&record, &attr.name);
        let sig = match self.funcs.get(&name).cloned() {
            Some(Func::Mono(sig)) => sig,
            Some(Func::Poly(scheme)) => self.instantiate(&name, &scheme, span),
            None => return Err(error(attr.span, format!("method `{name}` is not defined"))),
        };
        let mut params = sig.params.into_iter();
        if let Some(slf) = params.next() {
            self.expect(value.span, &slf.ty, receiver, "`self`")?;
        }
        let rest = FuncType {
            params: params.collect(),
            ret: sig.ret,
        };
        let shown = method_name(unqualified(&record), &attr.name);
        self.bind_args(&shown, &rest, args, span)?;
        Ok(rest.ret)
    }

    /// The record whose member `attr` a receiver of type `receiver`
    /// reads: its own when known, else the one record `has` holds of,
    /// which the receiver then is.
    fn owner(
        &mut self,
        receiver: &Type,
        attr: &Ident,
        has: impl Fn(&Record) -> bool,
    ) -> Result<String> {
        match self.subst.resolve(receiver) {
            Type::Record(name) => return Ok(name),
            Type::Var(_) => {}
            other => {
                let other = self.subst.zonk(&other);
                let msg = format!("{other} has no attribute `{}`", attr.name);
                return Err(error(attr.span, msg));
            }
        }
        let mut owners: Vec<&String> = self
            .records
            .iter()
            .filter(|(_, r)| has(r))
            .map(|(n, _)| n)
            .collect();
        owners.sort();
        let name = match owners.as_slice() {
            [one] => (*one).clone(),
            [] => {
                let msg = format!("no record has a field or method `{}`", attr.name);
                return Err(error(attr.span, msg));
            }
            many => {
                let names: Vec<&str> = many.iter().map(|n| unqualified(n)).collect();
                let msg = format!(
                    "`.{}` could be a member of {}; annotate the value's record",
                    attr.name,
                    names.join(" or ")
                );
                return Err(error(attr.span, msg));
            }
        };
        let ty = Type::Record(name.clone());
        self.expect(attr.span, &ty, receiver, "a record's member")?;
        Ok(name)
    }

    /// Whether `name` is a record.
    pub(super) fn is_record(&self, name: &str) -> bool {
        self.records.contains_key(name)
    }
}

fn no_member(record: &str, attr: &Ident) -> crate::diag::Diagnostic {
    let msg = format!(
        "record `{}` has no field or method `{}`",
        source_name(unqualified(record)),
        attr.name
    );
    error(attr.span, msg)
}

#[cfg(test)]
mod tests {
    use crate::{infer, lexer, parser};

    const HEAD: &str = "from dataclasses import dataclass\n\
                        @dataclass(frozen=True)\n\
                        class P:\n    x: int\n    s: str\n\
                        \x20   def m(self, k: int) -> int:\n        return self.x + k\n";

    /// `src` after [`HEAD`], checked: `Ok` or the first diagnostic.
    fn check(src: &str) -> Result<infer::Types, String> {
        let tokens = lexer::lex(&format!("{HEAD}{src}")).map_err(|d| d.message)?;
        let module = parser::parse(&tokens).map_err(|d| d.message)?;
        infer::infer(&module).map_err(|d| d.message)
    }

    fn err(src: &str) -> String {
        match check(src) {
            Ok(_) => panic!("expected an error for:\n{src}"),
            Err(msg) => msg,
        }
    }

    #[test]
    fn construction_binds_fields_by_position_and_keyword() {
        assert!(check("p = P(1, 'a')\nq = P(s='b', x=2)\nprint(p == q)\n").is_ok());
        assert_eq!(err("P(1)\n"), "`P` is missing argument `s`");
        assert!(err("P(1, 's', 2)\n").contains("takes"), "too many");
        assert!(err("P(1, 'a', y=3)\n").contains("`y`"), "unknown keyword");
        assert!(err("P('a', 1)\n").contains("argument `x` of `P`"));
    }

    #[test]
    fn fields_and_methods_are_typed() {
        let src = "def f(p: P) -> str:\n    return p.s + str(p.m(1))\nprint(f(P(1, 'é')))\n";
        assert!(check(src).is_ok());
        assert!(err("p = P(1, 'a')\ny = p.x + 'a'\n").contains("str"));
        assert_eq!(
            err("p = P(1, 'a')\nprint(p.z)\n"),
            "record `P` has no field or method `z`"
        );
        assert_eq!(
            err("p = P(1, 'a')\nprint(p.m)\n"),
            "methods can only be called: `.m(...)`"
        );
        assert!(err("print(P(1, 'a').m('k'))\n").contains("argument `k` of `P.m`"));
    }

    #[test]
    fn an_unknown_receiver_takes_the_one_record_with_the_member() {
        let src = "def g(p):\n    return p.s\nprint(g(P(1, 'a')))\n";
        assert!(check(src).is_ok());
        let two = "@dataclass(frozen=True)\nclass Q:\n    s: str\n\
                   def g(p):\n    return p.s\n";
        let msg = err(two);
        assert!(msg.contains("could be a member of P or Q"), "{msg}");
        assert_eq!(
            err("def g(p):\n    return p.nothing\n"),
            "no record has a field or method `nothing`"
        );
    }

    #[test]
    fn a_method_named_like_a_builtin_method_is_the_records() {
        let src = "@dataclass(frozen=True)\nclass C:\n    n: int\n\
                   \x20   def count(self, k: int) -> int:\n        return self.n * k\n\
                   print(C(2).count(3), [1, 1].count(1))\n";
        assert!(check(src).is_ok());
    }

    #[test]
    fn records_compare_but_do_not_order_or_hash() {
        assert!(check("print(P(1, 'a') == P(1, 'a'), P(1, 'a') != P(2, 'b'))\n").is_ok());
        assert!(err("print(P(1, 'a') == 1)\n").contains("never equal"));
        assert!(err("print(P(1, 'a') < P(2, 'b'))\n").contains("no ordering"));
        assert!(err("s = {P(1, 'a')}\n").contains("dict key or set element"));
        assert!(err("print(len(P(1, 'a')))\n").contains("no len()"));
    }

    #[test]
    fn a_record_is_not_a_value() {
        assert!(err("f = P\n").contains("`P` is a record, not a value"));
    }

    #[test]
    fn a_methods_raises_reach_its_callers() {
        let src = "@dataclass(frozen=True)\nclass R:\n    s: str\n\
                   \x20   temper def n(self) -> int raises ValueError:\n\
                   \x20       return int(self.s)\n\
                   temper def f(r: R) -> int:\n    return r.n()\n";
        let msg = err(src);
        assert!(msg.contains("`f`") && msg.contains("ValueError"), "{msg}");
        let declared = src.replace(
            "temper def f(r: R) -> int:",
            "temper def f(r: R) -> int raises ValueError:",
        );
        assert!(check(&declared).is_ok());
    }

    #[test]
    fn tempered_code_calls_only_tempered_methods() {
        let src = "temper def f(p: P) -> int:\n    return p.m(1)\n";
        assert_eq!(
            err(src),
            "tempered `f` calls loose `P.m`; temper `P.m` first"
        );
        let ok = "from dataclasses import dataclass\n\
                  @dataclass(frozen=True)\nclass T:\n    x: int\n\
                  \x20   temper def m(self) -> int:\n        return self.x\n\
                  temper def f(t: T) -> int:\n    return t.m()\n";
        let tokens = lexer::lex(ok).map_err(|d| d.message);
        let module = tokens.and_then(|t| parser::parse(&t).map_err(|d| d.message));
        assert!(module
            .and_then(|m| infer::infer(&m).map_err(|d| d.message))
            .is_ok());
    }
}
