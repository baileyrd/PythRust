//! Frozen records (step 4, D129-D136): their declarations, construction,
//! field reads and method calls.
//!
//! A record is a nominal type ([`Type::Record`]). Its methods are the
//! top-level functions the parser named `P.m` ([`method_name`]), so a
//! call `p.m(...)` is a call of `P.m` with `p` bound to `self`. While a
//! receiver's type is unknown, the one record that has the field or
//! method fixes it, as one type's method does (D52).

use std::collections::BTreeSet;

use crate::ast::{
    method_name, source_name, unqualified, Arg, Expr, Ident, Module, RecordDef, RecordKind,
    StmtKind,
};
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
    /// Frozen, a dataclass or a plain class.
    pub kind: RecordKind,
}

impl Record {
    fn field(&self, name: &str) -> Option<&Type> {
        self.fields.iter().find(|(n, _)| n == name).map(|(_, t)| t)
    }
}

impl Checker {
    /// Declares every record and class of `module`. A record field's
    /// annotation names only records before it (checked by the parser,
    /// D129).
    pub(super) fn declare_records(&mut self, module: &Module) -> Result<()> {
        let defs = module.body.iter().filter_map(|s| match &s.kind {
            StmtKind::Record(r) => Some(r),
            _ => None,
        });
        let defs: Vec<&RecordDef> = defs.collect();
        // Every name first: a class's `__init__` annotations may name it
        // or a later class (D151).
        for r in &defs {
            let record = Record {
                fields: Vec::new(),
                methods: BTreeSet::new(),
                kind: r.kind,
            };
            self.records.insert(r.name.name.clone(), record);
        }
        for r in defs {
            let mut fields = Vec::new();
            for f in &r.fields {
                let ty = match &f.annotation {
                    Some(t) => self.annotation(t)?,
                    None => self.subst.fresh(),
                };
                fields.push((f.name.name.clone(), ty));
            }
            let methods = r.methods.iter().map(|m| m.name.clone()).collect();
            let record = Record {
                fields,
                methods,
                kind: r.kind,
            };
            self.records.insert(r.name.name.clone(), record);
        }
        Ok(())
    }

    /// `P(...)`: a record's fields bound in order, by position or keyword,
    /// as a function's parameters are (D132); a plain class's `__init__`
    /// parameters after `self` (D145).
    pub(super) fn construct(&mut self, name: &str, args: &[Arg], span: Span) -> Result<Type> {
        let Some(record) = self.records.get(name) else {
            return Err(error(span, format!("name `{name}` is not defined")));
        };
        if record.kind == RecordKind::Plain {
            return self.construct_plain(name, args, span);
        }
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

    /// `P(...)` of plain class `P`: a call of `P.__init__`, if any, with
    /// `self` the new object.
    fn construct_plain(&mut self, name: &str, args: &[Arg], span: Span) -> Result<Type> {
        let ty = Type::Record(name.to_string());
        let shown = unqualified(name);
        let init = method_name(name, "__init__");
        let rest = match self.method_sig(&init, span)? {
            Some(sig) => self.after_self(sig, &ty, span)?,
            None => FuncType {
                params: Vec::new(),
                ret: Type::None,
            },
        };
        self.bind_args(shown, &rest, args, span)?;
        Ok(ty)
    }

    /// Method `name`'s signature, instantiated if it is generic.
    fn method_sig(&mut self, name: &str, span: Span) -> Result<Option<FuncType>> {
        Ok(match self.funcs.get(name).cloned() {
            Some(Func::Mono(sig)) => Some(sig),
            Some(Func::Poly(scheme)) => Some(self.instantiate(name, &scheme, span)),
            None => None,
        })
    }

    /// `sig` with `self` bound to `receiver`: the parameters callers pass.
    fn after_self(&mut self, sig: FuncType, receiver: &Type, span: Span) -> Result<FuncType> {
        let mut params = sig.params.into_iter();
        if let Some(slf) = params.next() {
            self.expect(span, &slf.ty, receiver, "`self`")?;
        }
        Ok(FuncType {
            params: params.collect(),
            ret: sig.ret,
        })
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

    /// `value.attr = v`, `v` of type `ty`: a store to a field of a
    /// mutable record, of the field's one type (D3, D146, D158, D159).
    /// The current value is `ty` too, for `op=`.
    pub(super) fn store_field(&mut self, value: &Expr, attr: &Ident, ty: &Type) -> Result<()> {
        let receiver = self.expr(value)?;
        let record = self.owner(&receiver, attr, |r| r.field(&attr.name).is_some())?;
        let Some(r) = self.records.get(&record) else {
            return Err(no_member(&record, attr));
        };
        let shown = source_name(unqualified(&record));
        let Some(field) = r.field(&attr.name).cloned() else {
            let fix = match r.kind {
                RecordKind::Plain => "fields are assigned in `__init__`",
                _ => "a dataclass's fields are declared in its body",
            };
            let msg = format!("`{shown}` has no field `{}`; {fix}", attr.name);
            return Err(error(attr.span, msg));
        };
        if r.kind == RecordKind::Frozen {
            let msg = format!(
                "cannot assign to field `{}` of frozen record `{shown}`",
                attr.name
            );
            return Err(error(attr.span, msg));
        }
        let what = format!("field `{}` of `{shown}`", attr.name);
        let (was, now) = (self.subst.zonk(&field), self.subst.zonk(ty));
        self.expect(attr.span, &field, ty, &what).map_err(|mut e| {
            // D4's fix-it for a None-first field (D148).
            if let (Type::None, t) | (t, Type::None) = (&was, &now) {
                let fix = format!(
                    "; to allow None, annotate it, as in `self.{}: {t} | None = None`",
                    attr.name
                );
                e.message.push_str(&fix);
            }
            e
        })
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
        if attr.name == "__init__" {
            let msg = "calling `__init__` directly is not supported yet";
            return Err(error(attr.span, msg.to_string()));
        }
        let record = self.owner(receiver, attr, |r| r.methods.contains(&attr.name))?;
        if !self.has_method(&record, &attr.name) {
            return Err(no_member(&record, attr));
        }
        let name = method_name(&record, &attr.name);
        let Some(sig) = self.method_sig(&name, span)? else {
            return Err(error(attr.span, format!("method `{name}` is not defined")));
        };
        let rest = self.after_self(sig, receiver, value.span)?;
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
                let msg = format!("`{other}` has no field `{}`", attr.name);
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

    const DATA: &str = "@dataclass\nclass Q:\n    n: int\n    tags: list[str]\n\
                        \x20   def bump(self) -> None:\n        self.n += 1\n";

    #[test]
    fn a_dataclass_field_is_stored_with_its_one_type() {
        let ok = format!("{DATA}q = Q(1, [])\nq.n = 2\nq.n += 3\nq.tags = ['é']\nq.bump()\n");
        assert!(check(&ok).is_ok(), "{:?}", check(&ok));
        let float = err(&format!("{DATA}q = Q(1, [])\nq.n = 2.5\n"));
        assert!(float.contains("field `n` of `Q`"), "{float}");
        let none = err(&format!("{DATA}q = Q(1, [])\nq.k = 2\n"));
        assert!(
            none.contains("`Q` has no field `k`; a dataclass's fields are declared in its body"),
            "{none}"
        );
        let frozen = err("p = P(1, 'a')\np.x = 2\n");
        assert!(
            frozen.contains("cannot assign to field `x` of frozen record `P`"),
            "{frozen}"
        );
        let list = err("xs = [1]\nxs.n = 2\n");
        assert!(list.contains("has no field `n`"), "{list}");
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

    /// Plain class `Q`: an `__init__` with a default, a `__repr__`-less
    /// class `R` beside it, and `S` with only `__str__`.
    const CLASSES: &str = "class Q:\n\
                           \x20   def __init__(self, n, tag: str = 'é'):\n\
                           \x20       self.n = n\n        self.tag = tag\n\
                           \x20   def __repr__(self) -> str:\n        return self.tag\n\
                           class R:\n    pass\n\
                           class S:\n\
                           \x20   def __str__(self) -> str:\n        return 's'\n";

    fn classes(src: &str) -> Result<infer::Types, String> {
        check(&format!("{CLASSES}{src}"))
    }

    #[test]
    fn construction_binds_init_after_self() {
        assert!(classes("q = Q(1)\nr = Q(n=2, tag='ß')\nprint(q.n + r.n)\n").is_ok());
        assert_eq!(
            classes("Q()\n").err().as_deref(),
            Some("`Q` is missing argument `n`")
        );
        let fixed = classes("Q(1)\nQ('s')\n").err().unwrap_or_default();
        assert!(fixed.contains("argument `n` of `Q`"), "{fixed}");
        let none = classes("R(1)\n").err().unwrap_or_default();
        assert!(none.contains("takes"), "{none}");
        let direct = classes("q = Q(1)\nq.__init__(2)\n").err();
        assert_eq!(
            direct.as_deref(),
            Some("calling `__init__` directly is not supported yet")
        );
    }

    #[test]
    fn printing_needs_the_dunder_cpython_calls() {
        let ok = ["print(Q(1), [Q(2)], str(Q(3)))\n", "print(S(), str(S()))\n"];
        for src in ok {
            assert!(classes(src).is_ok(), "{src}");
        }
        let cases = [
            ("print(R())\n", "R"),
            ("print([S()])\n", "S"),
            ("x = str((1, R()))\n", "R"),
            ("print({'k': S()})\n", "S"),
            ("def f(x):\n    print([x])\nf(S())\n", "S"),
        ];
        for (src, class) in cases {
            let want = format!("printing `{class}` needs a `__repr__` method");
            let got = classes(src).err().unwrap_or_default();
            assert!(got.contains(&want), "{src}: {got}");
        }
    }

    #[test]
    fn a_field_has_one_type_and_d4s_fix_it() {
        let src = "class N:\n    def __init__(self):\n        self.next = None\n\
                   n = N()\nn.next = N()\n";
        let got = classes(src).err().unwrap_or_default();
        assert!(
            got.ends_with("to allow None, annotate it, as in `self.next: N | None = None`"),
            "{got}"
        );
        let typed = "class N:\n    def __init__(self):\n        self.next: N | None = None\n\
                     n = N()\nn.next = N()\n";
        assert!(classes(typed).is_ok());
        let stray = classes("q = Q(1)\nq.k = 2\n").err();
        assert_eq!(
            stray.as_deref(),
            Some("`Q` has no field `k`; fields are assigned in `__init__`")
        );
        let keyed = classes("d = {Q(1): 1}\n").err().unwrap_or_default();
        assert!(keyed.starts_with("an object as a dict key"), "{keyed}");
    }
}
