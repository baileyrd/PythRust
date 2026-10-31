//! Rust binding stubs and the names they bring in (M6b,
//! `docs/research/rust-interop.md`).
//!
//! A stub holds only `temper def` signatures with `...` bodies, every
//! type one that crosses the boundary. `import fastfs` of a binding makes
//! `fastfs.f` one dotted name, as `os.listdir` is, which inference
//! checks against the stub.

use super::{check_params, error, Parser};
use crate::ast::{Binding, Expr, ExprKind, Ident, Param, ParamMode, StubDef, StubParam, TypeExpr};
use crate::diag::Result;
use crate::span::Span;
use crate::token::TokenKind;

/// The types a stub parameter may have; a result may also be `None`.
const PARAM_TYPES: &[&str] = &["int", "float", "bool", "str", "bytes"];

/// The classes a stub may raise: a Rust `io::Error` is an `OSError`.
const RAISES: &[&str] = &["OSError", "Exception"];

const ONLY: &str = "a stub holds only `temper def` signatures whose bodies are `...`";

impl<'a> Parser<'a> {
    /// The whole stub of binding `name`.
    pub(super) fn stub(&mut self, name: &str) -> Result<Binding> {
        let mut funcs: Vec<StubDef> = Vec::new();
        while !self.at(&TokenKind::EndMarker) {
            let f = self.stub_def(name)?;
            if funcs.iter().any(|g| g.name.name == f.name.name) {
                let msg = format!("`{}` is defined twice", f.name.name);
                return Err(error(f.name.span, msg));
            }
            funcs.push(f);
        }
        Ok(Binding {
            name: name.to_string(),
            funcs,
        })
    }

    /// `'temper' 'def' NAME '(' [params] ')' '->' type [raises] ':' '...'`,
    /// the `...` on the `def` line or the next.
    fn stub_def(&mut self, binding: &str) -> Result<StubDef> {
        let start = self.tok(0).span;
        if !self.eat(&TokenKind::Temper) {
            return Err(error(start, ONLY));
        }
        self.expect(&TokenKind::Def)?;
        let name = self.expect_name("a function name")?;
        self.expect(&TokenKind::LParen)?;
        let params = match self.at(&TokenKind::RParen) {
            true => Vec::new(),
            false => self.comma_list(Self::param)?.0,
        };
        self.expect(&TokenKind::RParen)?;
        check_params(&params)?;
        let params = params.into_iter().map(stub_param).collect::<Result<_>>()?;
        if !self.eat(&TokenKind::Arrow) {
            let msg = format!("stub `{}` must annotate its return type", name.name);
            return Err(error(name.span, msg));
        }
        let returns = self.type_expr()?;
        crossing(&returns, "a stub result", true)?;
        let raises = self.raises()?;
        stub_raises(&raises)?;
        self.expect(&TokenKind::Colon)?;
        self.stub_body()?;
        Ok(StubDef {
            name: Ident {
                name: format!("{binding}.{}", name.name),
                span: name.span,
            },
            params,
            returns,
            raises,
            span: self.span_from(start),
        })
    }

    /// `'...' NEWLINE`, or the same as an indented block.
    fn stub_body(&mut self) -> Result<()> {
        let block = self.eat(&TokenKind::Newline);
        if block {
            self.expect(&TokenKind::Indent)?;
        }
        if !self.eat(&TokenKind::Ellipsis) {
            return Err(error(self.tok(0).span, ONLY));
        }
        self.expect(&TokenKind::Newline)?;
        if block && !self.eat(&TokenKind::Dedent) {
            return Err(error(self.tok(0).span, ONLY));
        }
        Ok(())
    }

    /// Records the import of binding `name`, if one is given by that
    /// name; whether it was.
    pub(super) fn import_binding(&mut self, name: &Ident) -> bool {
        let Some(binding) = self.bindings.iter().find(|b| b.name == name.name) else {
            return false;
        };
        if self.imported_binding(&name.name).is_none() {
            self.imported_bindings.push(binding);
            self.stubs.extend(binding.funcs.iter().cloned());
        }
        true
    }

    /// The imported binding named `name`, if any.
    pub(super) fn imported_binding(&self, name: &str) -> Option<&'a Binding> {
        self.imported_bindings
            .iter()
            .copied()
            .find(|b| b.name == name)
    }

    /// After the binding name (already consumed at `first`), `.f` for one
    /// of its functions: `fastfs.f` as one name.
    pub(super) fn binding_func(&mut self, binding: &Binding, first: Span) -> Result<Expr> {
        let names = || {
            let names: Vec<&str> = binding.funcs.iter().map(|f| f.name.name.as_str()).collect();
            names.join(", ")
        };
        if !self.eat(&TokenKind::Dot) {
            let msg = format!(
                "`{}` is a Rust binding, not a value; use one of its functions: {}",
                binding.name,
                names()
            );
            return Err(error(first, msg));
        }
        let attr = self.expect_name("a function name")?;
        let path = format!("{}.{}", binding.name, attr.name);
        let span = first.merge(attr.span);
        if !binding.funcs.iter().any(|f| f.name.name == path) {
            let msg = format!(
                "the stub for `{}` has no `{}`; its functions are: {}",
                binding.name,
                attr.name,
                names()
            );
            return Err(error(span, msg));
        }
        Ok(Expr {
            kind: ExprKind::Name(path),
            span,
        })
    }
}

/// A stub parameter: annotated with a crossing type, no mode, no default.
fn stub_param(p: Param) -> Result<StubParam> {
    let name = &p.name.name;
    if p.mode != ParamMode::Read {
        let msg =
            format!("`mut` and `owned` are not supported in stubs yet; drop it from `{name}`");
        return Err(error(p.span, msg));
    }
    if let Some(default) = &p.default {
        let msg = format!("stub parameter `{name}` cannot have a default");
        return Err(error(default.span, msg));
    }
    let Some(annotation) = p.annotation else {
        let msg = format!("stub parameter `{name}` must be annotated");
        return Err(error(p.name.span, msg));
    };
    crossing(&annotation, "a stub parameter", false)?;
    Ok(StubParam {
        name: p.name,
        annotation,
    })
}

/// Checks that `t` is a type that crosses into Rust: a [`PARAM_TYPES`]
/// name, or `None` for a result.
fn crossing(t: &TypeExpr, what: &str, result: bool) -> Result<()> {
    let name = t.name.name.as_str();
    let scalar = PARAM_TYPES.contains(&name) || (result && name == "None");
    if scalar && t.args.is_empty() && !t.optional {
        return Ok(());
    }
    let none = if result { ", or None" } else { "" };
    let msg = format!(
        "{what} must be one of {}{none}; `{name}` does not cross into Rust yet",
        PARAM_TYPES.join(", ")
    );
    Err(error(t.span, msg))
}

/// A stub's `raises` lists `OSError` or `Exception`, each at most once.
fn stub_raises(raises: &[Ident]) -> Result<()> {
    for (i, r) in raises.iter().enumerate() {
        if !RAISES.contains(&r.name.as_str()) {
            let msg = format!(
                "a stub raises only `OSError` or `Exception` (a Rust `io::Error`), not `{}`",
                r.name
            );
            return Err(error(r.span, msg));
        }
        if raises[..i].iter().any(|q| q.name == r.name) {
            let msg = format!("`{}` is listed twice in `raises`", r.name);
            return Err(error(r.span, msg));
        }
    }
    Ok(())
}
