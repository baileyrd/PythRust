//! Frozen records (step 4, D129-D136): `@dataclass(frozen=True)` then
//! `class NAME:` with annotated fields and methods.
//!
//! `@dataclass` is a recognized form, not a general decorator (D132).
//! A method `m` of `P` becomes the top-level `def` named `P.m` (see
//! [`method_name`]) right after the record, its `self` annotated `P`, so
//! every later stage sees an ordinary function.

use super::{error, Parser};
use crate::ast::{
    method_name, Expr, ExprKind, Field, FuncDef, Ident, ParamMode, RecordDef, Stmt, StmtKind,
    TypeExpr,
};
use crate::diag::Result;
use crate::infer::scope;
use crate::span::Span;
use crate::token::TokenKind;

/// D38's message for `@dataclass` without `frozen=True`.
const NOT_FROZEN: &str =
    "`@dataclass` without `frozen=True` is not supported yet; use `@dataclass(frozen=True)`";

/// What a record's body holds, gathered member by member.
#[derive(Default)]
struct Body {
    fields: Vec<Field>,
    methods: Vec<FuncDef>,
}

impl Body {
    /// Whether `name` is a field or method already.
    fn has(&self, name: &str) -> bool {
        let field = self.fields.iter().any(|f| f.name.name == name);
        let method = self.methods.iter().any(|m| m.name.name == name);
        field || method
    }
}

impl Parser<'_> {
    /// `'@' 'dataclass' '(' 'frozen' '=' 'True' ')' NEWLINE 'class' NAME
    /// ':' body`: the record, then its methods.
    pub(super) fn record(&mut self) -> Result<Vec<Stmt>> {
        let start = self.tok(0).span;
        self.decorator()?;
        if !self.at(&TokenKind::Class) {
            let msg = "decorators are not supported yet, except `@dataclass(frozen=True)` \
                       on a class";
            return Err(error(start, msg));
        }
        self.bump();
        let name = self.binding_name("a class name")?;
        self.no_bases(&name)?;
        self.expect(&TokenKind::Colon)?;
        let body = self.record_body(&name)?;
        let span = self.span_from(start);
        let record = RecordDef {
            name: name.clone(),
            fields: body.fields,
            methods: body.methods.iter().map(|m| m.name.clone()).collect(),
            span,
        };
        let methods = body.methods.into_iter().map(|m| method(&name, m));
        let first = Stmt {
            kind: StmtKind::Record(record),
            span,
        };
        Ok(std::iter::once(first).chain(methods).collect())
    }

    /// The decorator line before `class`, which must be exactly
    /// `@dataclass(frozen=True)`; a `class` with none is refused.
    fn decorator(&mut self) -> Result<()> {
        let at = self.tok(0).span;
        if !self.eat(&TokenKind::At) {
            let msg = "classes without `@dataclass(frozen=True)` are not supported yet; \
                       use `@dataclass(frozen=True)`";
            return Err(error(at, msg));
        }
        let name = self.expect_name("a decorator name")?;
        if name.name != "dataclass" {
            let msg = "decorators are not supported yet, except `@dataclass(frozen=True)` \
                       on a class";
            return Err(error(at.merge(name.span), msg));
        }
        if !self.dataclass {
            let msg = "name `dataclass` is not defined; add `from dataclasses import dataclass`";
            return Err(error(name.span, msg));
        }
        if !self.at(&TokenKind::LParen) {
            return Err(error(at.merge(name.span), NOT_FROZEN));
        }
        let call = self.call(Expr {
            kind: ExprKind::Name(name.name.clone()),
            span: name.span,
        })?;
        frozen_only(&call)?;
        self.expect(&TokenKind::Newline)?;
        Ok(())
    }

    /// No base classes and no type parameters (D132, D135); `()` is
    /// none.
    fn no_bases(&mut self, name: &Ident) -> Result<()> {
        if self.at(&TokenKind::LBracket) {
            return Err(error(
                self.tok(0).span,
                "generic records are not supported yet",
            ));
        }
        if !self.eat(&TokenKind::LParen) {
            return Ok(());
        }
        if self.eat(&TokenKind::RParen) {
            return Ok(());
        }
        let generic = matches!(self.peek(), TokenKind::Name(n) if n == "Generic");
        let msg = match generic {
            true => "generic records are not supported yet".to_string(),
            false => format!(
                "base classes are not supported yet; `{}` stands alone",
                name.name
            ),
        };
        Err(error(self.tok(0).span, msg))
    }

    /// `NEWLINE INDENT member+ DEDENT`: a leading docstring, `pass`,
    /// fields and methods.
    fn record_body(&mut self, record: &Ident) -> Result<Body> {
        self.ellipsis_stmt(" on its own indented line")?;
        self.expect(&TokenKind::Newline)?;
        self.expect(&TokenKind::Indent)?;
        let mut body = Body::default();
        if matches!(self.peek(), TokenKind::Str(_)) && self.tok(1).kind == TokenKind::Newline {
            self.bump();
            self.bump();
        }
        while !self.eat(&TokenKind::Dedent) {
            self.member(record, &mut body)?;
        }
        Ok(body)
    }

    /// One member of a record's body.
    fn member(&mut self, record: &Ident, body: &mut Body) -> Result<()> {
        let tok = self.tok(0);
        match &tok.kind {
            TokenKind::Pass => {
                self.bump();
                self.expect(&TokenKind::Newline)?;
                Ok(())
            }
            TokenKind::Def | TokenKind::Temper => {
                let def = self.nested(|p| p.func_head(|p| p.expect_name("a method name")))?;
                check_method(record, &def, body)?;
                body.methods.push(def);
                Ok(())
            }
            TokenKind::At => {
                let msg = "decorators on methods are not supported yet";
                Err(error(tok.span, msg))
            }
            TokenKind::Name(_) if self.tok(1).kind == TokenKind::Colon => {
                let field = self.field()?;
                check_member(&field.name, body)?;
                body.fields.push(field);
                Ok(())
            }
            _ => {
                let msg = "class attributes are not supported yet; a record's body holds \
                           annotated fields (`x: int`) and methods";
                Err(error(tok.span, msg))
            }
        }
    }

    /// `NAME ':' type NEWLINE`; a default is not supported yet (D132).
    fn field(&mut self) -> Result<Field> {
        let name = self.expect_name("a field name")?;
        self.expect(&TokenKind::Colon)?;
        let annotation = self.type_expr()?;
        if annotation.name.name == "ClassVar" {
            let msg = "`ClassVar` is not supported yet";
            return Err(error(annotation.span, msg));
        }
        if self.at(&TokenKind::Assign) {
            let msg = "field defaults are not supported yet; pass every field when \
                       constructing the record";
            return Err(error(self.tok(0).span, msg));
        }
        self.expect(&TokenKind::Newline)?;
        Ok(Field { name, annotation })
    }
}

/// The decorator's arguments: `frozen=True` alone (D38, D132).
fn frozen_only(call: &Expr) -> Result<()> {
    let ExprKind::Call { args, .. } = &call.kind else {
        return Err(error(call.span, NOT_FROZEN));
    };
    let mut frozen = false;
    for arg in args {
        let Some(keyword) = &arg.name else {
            let msg = "`@dataclass` takes only `frozen=True`";
            return Err(error(arg.span, msg));
        };
        if keyword.name != "frozen" {
            let msg = format!(
                "`@dataclass({}=...)` is not supported yet; only `frozen=True` is",
                keyword.name
            );
            return Err(error(arg.span, msg));
        }
        frozen = matches!(arg.value.kind, ExprKind::Bool(true));
    }
    match frozen {
        true => Ok(()),
        false => Err(error(call.span, NOT_FROZEN)),
    }
}

/// A member's name is new to the record, and not a dunder (D132).
fn check_member(name: &Ident, body: &Body) -> Result<()> {
    let n = &name.name;
    if n.starts_with("__") && n.ends_with("__") {
        let msg = format!("`{n}` in a record is not supported yet");
        return Err(error(name.span, msg));
    }
    if body.has(n) {
        let msg = format!("`{n}` is declared twice in this record; give each member its own name");
        return Err(error(name.span, msg));
    }
    Ok(())
}

/// A method's name is new, and it takes an unannotated, read `self`
/// first (D131).
fn check_method(record: &Ident, def: &FuncDef, body: &Body) -> Result<()> {
    check_member(&def.name, body)?;
    let Some(first) = def.params.first() else {
        let msg = format!(
            "method `{}` needs `self` as its first parameter",
            def.name.name
        );
        return Err(error(def.name.span, msg));
    };
    if first.name.name != "self" {
        let msg = format!(
            "method `{}` needs `self` as its first parameter, not `{}`",
            def.name.name, first.name.name
        );
        return Err(error(first.name.span, msg));
    }
    if first.mode != ParamMode::Read {
        let msg = "a method that changes or keeps `self` is not supported yet";
        return Err(error(first.span, msg));
    }
    if first.annotation.is_some() || first.default.is_some() {
        let msg = format!(
            "`self` is the record `{}`; drop its annotation",
            record.name
        );
        return Err(error(first.span, msg));
    }
    Ok(())
}

/// Method `def` of `record` as the top-level `def` named `P.m`, `self`
/// annotated with the record.
fn method(record: &Ident, mut def: FuncDef) -> Stmt {
    def.name.name = method_name(&record.name, &def.name.name);
    if let Some(slf) = def.params.first_mut() {
        slf.annotation = Some(record_type(record, slf.name.span));
    }
    let span = def.span;
    Stmt {
        kind: StmtKind::FuncDef(def),
        span,
    }
}

/// The annotation naming `record`, at `span`.
fn record_type(record: &Ident, span: Span) -> TypeExpr {
    TypeExpr {
        name: Ident {
            name: record.name.clone(),
            span,
        },
        args: Vec::new(),
        optional: false,
        span,
    }
}

/// The module-wide rules of a file's records (D129, D136): a record's
/// name is no other binding's, and an annotation evaluated when its
/// statement runs (a field's, a `def`'s parameters and result) names
/// only records defined before it.
pub(super) fn check_module(body: &[Stmt]) -> Result<()> {
    let records: Vec<(usize, &RecordDef)> = body
        .iter()
        .enumerate()
        .filter_map(|(i, s)| match &s.kind {
            StmtKind::Record(r) => Some((i, r)),
            _ => None,
        })
        .collect();
    if records.is_empty() {
        return Ok(());
    }
    for &(_, r) in &records {
        check_unbound(r, body)?;
    }
    for (at, s) in body.iter().enumerate() {
        let at = runs_at(s, at, &records);
        for t in evaluated(s) {
            check_defined(t, at, &records)?;
        }
    }
    Ok(())
}

/// Where statement `s`, at `at` in the body, evaluates its annotations:
/// a method's run in its record's body, before the record exists.
fn runs_at(s: &Stmt, at: usize, records: &[(usize, &RecordDef)]) -> usize {
    let StmtKind::FuncDef(f) = &s.kind else {
        return at;
    };
    let Some((record, _)) = f.name.name.split_once('.') else {
        return at;
    };
    let own = records.iter().find(|(_, r)| r.name.name == record);
    own.map_or(at, |(i, _)| *i)
}

/// No variable, parameter, function or other record takes `r`'s name.
fn check_unbound(r: &RecordDef, body: &[Stmt]) -> Result<()> {
    let name = r.name.name.as_str();
    let mut bound = scope::assigned(body).into_iter().map(|n| source(&n));
    let mut clash = bound.any(|n| n == name);
    for s in body {
        match &s.kind {
            StmtKind::FuncDef(f) => {
                clash |= f.name.name == name;
                clash |= f.params.iter().any(|p| p.name.name == name);
                clash |= scope::assigned(&f.body).iter().any(|n| source(n) == name);
            }
            StmtKind::Record(other) => clash |= other.span != r.span && other.name.name == name,
            _ => {}
        }
    }
    match clash {
        false => Ok(()),
        true => {
            let msg = format!("`{name}` names a record; choose another name");
            Err(error(r.name.span, msg))
        }
    }
}

fn source(n: &str) -> String {
    crate::ast::source_name(n).to_string()
}

/// The annotations statement `s` evaluates when it runs.
fn evaluated(s: &Stmt) -> Vec<&TypeExpr> {
    match &s.kind {
        StmtKind::Record(r) => r.fields.iter().map(|f| &f.annotation).collect(),
        StmtKind::FuncDef(f) => {
            // A method's `self` is annotated by the parser, not the source.
            let skip = usize::from(f.name.name.contains('.'));
            let params = f.params.iter().skip(skip);
            let params = params.filter_map(|p| p.annotation.as_ref());
            params.chain(f.returns.as_ref()).collect()
        }
        _ => Vec::new(),
    }
}

/// `t` names no record that the statement at `at` runs before.
fn check_defined(t: &TypeExpr, at: usize, records: &[(usize, &RecordDef)]) -> Result<()> {
    let later = records
        .iter()
        .any(|(i, r)| *i >= at && r.name.name == t.name.name);
    if later {
        let msg = format!(
            "`{}` is not defined yet: a record is defined when its `class` runs",
            t.name.name
        );
        return Err(error(t.name.span, msg));
    }
    t.args
        .iter()
        .try_for_each(|a| check_defined(a, at, records))
}
