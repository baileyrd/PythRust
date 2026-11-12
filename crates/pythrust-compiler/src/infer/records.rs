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
