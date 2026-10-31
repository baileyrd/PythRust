//! `import` statements and module names (M6a, [`crate::stdlib`]).
//!
//! Imports come first in a module, so every later use of a module name
//! is known while parsing: `sys.argv` becomes the name `sys.argv`.

use super::{error, Parser};
use crate::ast::{Expr, ExprKind, Ident, Stmt, StmtKind, Target, TargetKind};
use crate::diag::Result;
use crate::span::Span;
use crate::stdlib::{self, ModuleDef, StdFn};
use crate::token::TokenKind;

impl Parser<'_> {
    /// `'import' NAME (',' NAME)* NEWLINE`: an assignment of each
    /// variable of each module from its initializer.
    pub(super) fn import_stmt(&mut self) -> Result<Vec<Stmt>> {
        let start = self.bump().span;
        let mut stmts = Vec::new();
        loop {
            let name = self.expect_name("a module name")?;
            let module = self.import(&name)?;
            stmts.extend(
                module
                    .vars
                    .iter()
                    .map(|(var, init)| bind(var, *init, name.span)),
            );
            if !self.eat(&TokenKind::Comma) {
                break;
            }
        }
        match self.peek() {
            TokenKind::Dot => {
                let msg = "import the top-level module, as in `import os`, \
                           and write `os.path.join`";
                return Err(error(self.tok(0).span, msg));
            }
            TokenKind::As => {
                return Err(error(
                    self.tok(0).span,
                    "`import ... as` is not supported yet",
                ));
            }
            _ => {}
        }
        self.expect(&TokenKind::Newline)?;
        let span = self.span_from(start);
        Ok(stmts.into_iter().map(|s| Stmt { span, ..s }).collect())
    }

    /// Records the import of the module `name`.
    fn import(&mut self, name: &Ident) -> Result<&'static ModuleDef> {
        let Some(module) = stdlib::module(&name.name) else {
            let msg = format!(
                "no module named `{}`; the modules are: {}",
                name.name,
                stdlib::module_names()
            );
            return Err(error(name.span, msg));
        };
        if !self.modules.iter().any(|m| m.name == module.name) {
            self.modules.push(module);
        }
        Ok(module)
    }

    /// The imported module named `name`, if any.
    pub(super) fn imported(&self, name: &str) -> Option<&'static ModuleDef> {
        self.modules.iter().copied().find(|m| m.name == name)
    }

    /// After the module name `first` (already consumed), its attributes
    /// up to an exposed name: `sys.argv` as one name.
    pub(super) fn module_name(&mut self, module: &ModuleDef, first: Span) -> Result<Expr> {
        let mut path = module.name.to_string();
        let mut span = first;
        loop {
            if !self.eat(&TokenKind::Dot) {
                let msg = format!(
                    "`{path}` is a module, not a value; use one of its names: {}",
                    module.names().join(", ")
                );
                return Err(error(span, msg));
            }
            let attr = self.expect_name("an attribute name")?;
            path = format!("{path}.{}", attr.name);
            span = span.merge(attr.span);
            if module.exposes(&path) {
                return match StdFn::from_name(&path).and_then(StdFn::chain) {
                    Some(method) => self.chained(&path, span, method),
                    None => Ok(name_expr(&path, span)),
                };
            }
            if !module.continues(&path) {
                let msg = format!(
                    "`{path}` is not supported; the `{}` names are: {}",
                    module.name,
                    module.names().join(", ")
                );
                return Err(error(span, msg));
            }
        }
    }

    /// `f(args).method()`, which the call to `f` alone stands for: the
    /// object `f` returns is not a value yet.
    fn chained(&mut self, name: &str, span: Span, method: &str) -> Result<Expr> {
        let missing = || {
            format!(
                "`{name}(...)` is supported only as `{name}(...).{method}()` yet; \
                 its object is not a value"
            )
        };
        if !self.at(&TokenKind::LParen) {
            return Err(error(span, missing()));
        }
        let call = self.call(name_expr(name, span))?;
        let follows = self.eat(&TokenKind::Dot)
            && matches!(self.peek(), TokenKind::Name(n) if n == method)
            && self.tok(1).kind == TokenKind::LParen
            && self.tok(2).kind == TokenKind::RParen;
        if !follows {
            return Err(error(call.span, missing()));
        }
        for _ in 0..3 {
            self.bump();
        }
        Ok(Expr {
            span: self.span_from(call.span),
            ..call
        })
    }

    /// A name a statement binds: not an imported module's.
    pub(super) fn binding_name(&mut self, what: &str) -> Result<Ident> {
        let name = self.expect_name(what)?;
        if self.imported(&name.name).is_some() {
            let msg = format!(
                "`{}` names an imported module; choose another name",
                name.name
            );
            return Err(error(name.span, msg));
        }
        Ok(name)
    }
}

/// `var = init()`, at `span`.
fn bind(var: &str, init: StdFn, span: Span) -> Stmt {
    let call = Expr {
        kind: ExprKind::Call {
            func: Box::new(name_expr(init.name(), span)),
            args: Vec::new(),
        },
        span,
    };
    let target = Target {
        kind: TargetKind::Name(var.to_string()),
        span,
    };
    Stmt {
        kind: StmtKind::Assign {
            target,
            value: call,
        },
        span,
    }
}

fn name_expr(name: &str, span: Span) -> Expr {
    Expr {
        kind: ExprKind::Name(name.to_string()),
        span,
    }
}
