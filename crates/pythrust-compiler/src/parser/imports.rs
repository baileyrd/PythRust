//! `import` and `from` statements and module names (M6a, step 3:
//! D75-D84; [`crate::stdlib`]).
//!
//! Imports come first in a module, so every later use of a name an
//! import binds is known while parsing: `sys.argv` becomes the one name
//! `sys.argv`, `a.f` the name `a.f`, and after `from os.path import join`
//! the name `join` is `os.path.join`.

use super::{error, Parser};
use crate::ast::{Binding, Expr, ExprKind, Ident, LocalModule, Stmt, StmtKind, Target, TargetKind};
use crate::diag::{Diagnostic, Result};
use crate::span::Span;
use crate::stdlib::{self, ModuleDef, StdFn};
use crate::token::TokenKind;

/// The standard module whose `dataclass` makes records (D132).
const DATACLASSES: &str = "dataclasses";

/// The module whose `annotations` feature makes annotations lazy (D154).
const FUTURE: &str = "__future__";

/// A `from __future__` import after other statements (D164).
const LATE_FUTURE: &str = "`from __future__` imports must occur at the beginning of the file";

/// CPython's future features other than `annotations` (D164).
const FEATURES: &[&str] = &[
    "nested_scopes",
    "generators",
    "division",
    "absolute_import",
    "with_statement",
    "print_function",
    "unicode_literals",
    "barry_as_FLUFL",
    "generator_stop",
];

/// Whether `kind` is the name `__future__`.
fn is_future(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::Name(n) if n == FUTURE)
}

/// `name` is `annotations`, the one future feature recognized (D164).
fn future_feature(name: &Ident) -> Result<()> {
    let n = name.name.as_str();
    if n == "annotations" {
        return Ok(());
    }
    let msg = match FEATURES.contains(&n) {
        true => format!("`from __future__ import {n}` is not supported yet"),
        false => format!("future feature `{n}` is not defined"),
    };
    Err(error(name.span, msg))
}

/// A module an import names.
#[derive(Clone)]
pub(super) enum Module<'a> {
    /// A standard module, or a part of one, by its dotted path: `os`,
    /// `os.path`.
    Std(&'static ModuleDef, String),
    Local(&'a LocalModule),
    /// A directory of local modules, by its dotted path: the `a` of
    /// `import a.b` (D78).
    Package(String),
    Binding(&'a Binding),
}

/// What a name an import binds stands for. None is a value (D83).
#[derive(Clone)]
pub(super) enum Imported<'a> {
    Module(Module<'a>),
    /// A function `from` binds: its qualified name, `a.f` or
    /// `os.path.join`.
    Func(String),
}

/// A dotted module path as written, `a.b.c`.
struct Path {
    parts: Vec<Ident>,
}

impl Path {
    fn text(&self) -> String {
        let parts: Vec<&str> = self.parts.iter().map(|p| p.name.as_str()).collect();
        parts.join(".")
    }

    fn span(&self) -> Span {
        let first = self.parts[0].span;
        self.parts.iter().fold(first, |s, p| s.merge(p.span))
    }
}

impl<'a> Parser<'a> {
    /// `'import' path ['as' NAME] (',' ...)* NEWLINE`, or
    /// `'from' path 'import' names NEWLINE`: the statements that bind
    /// what it imports, each at the statement's span.
    pub(super) fn import_stmt(&mut self) -> Result<Vec<Stmt>> {
        let start = self.bump();
        let stmts = match start.kind {
            TokenKind::From => self.import_from()?,
            _ => self.plain_import()?,
        };
        self.expect(&TokenKind::Newline)?;
        let span = self.span_from(start.span);
        Ok(stmts.into_iter().map(|s| Stmt { span, ..s }).collect())
    }

    /// The `from __future__` imports that start a module (D154, D164):
    /// whether its annotations are lazy, as CPython makes them, so they
    /// may name a class defined later. `annotations` is the one feature
    /// recognized, so any such import makes them lazy.
    pub(super) fn future_imports(&mut self) -> Result<bool> {
        let mut lazy = false;
        while self.at(&TokenKind::From) && is_future(&self.tok(1).kind) {
            self.future_import()?;
            lazy = true;
        }
        Ok(lazy)
    }

    /// The error for an import after other statements, CPython's own
    /// for a future import (D164).
    pub(super) fn misplaced_import(&self) -> Diagnostic {
        let span = self.tok(0).span;
        match self.at(&TokenKind::From) && is_future(&self.tok(1).kind) {
            true => error(span, LATE_FUTURE),
            false => error(
                span,
                "`import` must come at module top level, before other statements",
            ),
        }
    }

    /// `'from' '__future__' 'import' names NEWLINE`, the names in
    /// parentheses or not.
    fn future_import(&mut self) -> Result<()> {
        self.bump();
        self.bump();
        self.expect(&TokenKind::Import)?;
        if self.at(&TokenKind::Star) {
            let msg = "future feature `*` is not defined";
            return Err(error(self.tok(0).span, msg));
        }
        let paren = self.eat(&TokenKind::LParen);
        loop {
            future_feature(&self.expect_name("a future feature")?)?;
            if self.at(&TokenKind::As) {
                let msg = "`as` on a `__future__` import is not supported yet";
                return Err(error(self.tok(0).span, msg));
            }
            let more = self.eat(&TokenKind::Comma);
            if !more || (paren && self.at(&TokenKind::RParen)) {
                break;
            }
        }
        if paren {
            self.expect(&TokenKind::RParen)?;
        }
        self.expect(&TokenKind::Newline)?;
        Ok(())
    }

    /// The modules after `import`, each bound by its first name, or by
    /// its `as` name to the whole path.
    fn plain_import(&mut self) -> Result<Vec<Stmt>> {
        let mut stmts = Vec::new();
        loop {
            let path = self.path()?;
            if path.text() == DATACLASSES {
                let msg = "`import dataclasses` is not supported yet; \
                           write `from dataclasses import dataclass`";
                return Err(error(path.span(), msg));
            }
            let module = self.import_module(&path, &mut stmts)?;
            if self.eat(&TokenKind::As) {
                let alias = self.binding_name("a name for the module")?;
                self.bind_name(&alias.name, Imported::Module(module));
            } else {
                let top = top(&path, module)?;
                self.bind_name(&path.parts[0].name, Imported::Module(top));
            }
            if !self.eat(&TokenKind::Comma) {
                return Ok(stmts);
            }
        }
    }

    /// `from path import name [as name], ...`, the names in parentheses
    /// or not.
    fn import_from(&mut self) -> Result<Vec<Stmt>> {
        if self.at(&TokenKind::Dot) {
            let msg = "relative imports are not supported yet";
            return Err(error(self.tok(0).span, msg));
        }
        let path = self.path()?;
        if path.text() == FUTURE {
            return Err(error(path.span(), LATE_FUTURE));
        }
        self.expect(&TokenKind::Import)?;
        if path.text() == DATACLASSES {
            return self.import_dataclass();
        }
        if self.at(&TokenKind::Star) {
            let msg = "`import *` is not supported; import the names you use";
            return Err(error(self.tok(0).span, msg));
        }
        let mut stmts = Vec::new();
        let module = self.import_module(&path, &mut stmts)?;
        let paren = self.eat(&TokenKind::LParen);
        loop {
            let name = self.expect_name("a name to import")?;
            let alias = match self.eat(&TokenKind::As) {
                true => self.binding_name("a name for it")?,
                false => self.check_bindable(name.clone())?,
            };
            stmts.extend(self.import_name(&module, &name, &alias)?);
            let more = self.eat(&TokenKind::Comma);
            if !more || (paren && self.at(&TokenKind::RParen)) {
                break;
            }
        }
        if paren {
            self.expect(&TokenKind::RParen)?;
        }
        Ok(stmts)
    }

    /// `from dataclasses import dataclass`, the one form (D132): it
    /// binds nothing a value can name, only the decorator.
    fn import_dataclass(&mut self) -> Result<Vec<Stmt>> {
        let name = self.expect_name("`dataclass`")?;
        if name.name != "dataclass" {
            let msg = format!(
                "`dataclasses.{}` is not supported yet; only `dataclass` is",
                name.name
            );
            return Err(error(name.span, msg));
        }
        if self.at(&TokenKind::As) || self.at(&TokenKind::Comma) {
            let msg = "only `from dataclasses import dataclass` is supported yet";
            return Err(error(self.tok(0).span, msg));
        }
        self.dataclass = true;
        Ok(Vec::new())
    }

    /// `NAME ('.' NAME)*`.
    fn path(&mut self) -> Result<Path> {
        let mut parts = vec![self.expect_name("a module name")?];
        while self.eat(&TokenKind::Dot) {
            parts.push(self.expect_name("a module name")?);
        }
        Ok(Path { parts })
    }

    /// The module `path` names, recorded as imported; a standard one's
    /// variables are bound into `stmts` from their initializers.
    fn import_module(&mut self, path: &Path, stmts: &mut Vec<Stmt>) -> Result<Module<'a>> {
        let text = path.text();
        let first = &path.parts[0];
        if let [one] = path.parts.as_slice() {
            if let Some(binding) = self.import_binding(one)? {
                return Ok(Module::Binding(binding));
            }
        }
        if let Some(local) = self.local(&text) {
            return Ok(Module::Local(local));
        }
        if self.is_package(&text) {
            return Ok(Module::Package(text));
        }
        let Some(module) = stdlib::module(&first.name) else {
            return Err(error(first.span, stdlib::not_found(&first.name)));
        };
        if text != module.name && !module.continues(&text) {
            let msg = format!(
                "`{text}` is not a module; the `{}` names are: {}",
                module.name,
                module.names().join(", ")
            );
            return Err(error(path.span(), msg));
        }
        stmts.extend(self.initialize(module, path.span()));
        Ok(Module::Std(module, text))
    }

    /// The assignments that initialize `module`'s variables, unless an
    /// import of it has already: every importer shares them (D83).
    fn initialize(&mut self, module: &'static ModuleDef, span: Span) -> Vec<Stmt> {
        if self.ready.contains(&module.name) {
            return Vec::new();
        }
        self.ready.push(module.name);
        let vars = module.vars.iter();
        vars.map(|(var, init)| bind(var, *init, span)).collect()
    }

    /// The local module `path`, recorded as imported here, if the
    /// program has one.
    fn local(&mut self, path: &str) -> Option<&'a LocalModule> {
        let local = self.locals.iter().find(|m| m.name == path)?;
        if !self.imported_locals.iter().any(|m| m.name == path) {
            self.imported_locals.push(local);
        }
        Some(local)
    }

    /// Whether `path` is a directory of local modules: one imported as
    /// a package, or holding some module of the program.
    fn is_package(&self, path: &str) -> bool {
        let prefix = format!("{path}.");
        let within = |name: &str| name.starts_with(&prefix);
        let imported = self.packages.iter().any(|p| p == path || within(p));
        imported || self.locals.iter().any(|m| within(&m.name))
    }

    /// `name as alias` from `module`: a submodule or function is bound
    /// as a name; a variable is copied into a global (D83).
    fn import_name(
        &mut self,
        module: &Module<'a>,
        name: &Ident,
        alias: &Ident,
    ) -> Result<Vec<Stmt>> {
        let path = format!("{}.{}", module_path(module), name.name);
        let found = match module {
            Module::Std(m, _) => from_std(m, &path, name)?,
            Module::Local(m) => from_local(m, &path, name)?,
            Module::Package(p) => Some(self.package_member(p, &path, name)?),
            Module::Binding(b) => Some(from_binding(b, &path, name)?),
        };
        let Some(imported) = found else {
            return Ok(vec![assign(
                &alias.name,
                name_expr(&path, name.span),
                name.span,
            )]);
        };
        self.bind_name(&alias.name, imported);
        Ok(Vec::new())
    }

    /// `path`, a module or package in the package `p`.
    fn package_member(&mut self, p: &str, path: &str, name: &Ident) -> Result<Imported<'a>> {
        if let Some(local) = self.local(path) {
            return Ok(Imported::Module(Module::Local(local)));
        }
        if self.is_package(path) {
            return Ok(Imported::Module(Module::Package(path.to_string())));
        }
        let msg = format!("package `{p}` has no module `{}`", name.name);
        Err(error(name.span, msg))
    }

    fn bind_name(&mut self, name: &str, imported: Imported<'a>) {
        self.imports.retain(|(n, _)| n != name);
        self.imports.push((name.to_string(), imported));
    }

    /// What the imported name `name` stands for, if an import bound it.
    pub(super) fn imported(&self, name: &str) -> Option<&Imported<'a>> {
        self.imports.iter().find(|(n, _)| n == name).map(|(_, i)| i)
    }

    /// The use of an imported name (already consumed at `first`): a
    /// module's name as one dotted name, or a function's.
    pub(super) fn imported_expr(&mut self, imported: Imported<'a>, first: Span) -> Result<Expr> {
        let module = match imported {
            Imported::Func(path) => return self.func_name(&path, first),
            Imported::Module(module) => module,
        };
        match module {
            Module::Std(module, path) => self.module_name(module, path, first),
            Module::Local(local) => self.local_name(local, first),
            Module::Package(path) => self.package_name(path, first),
            Module::Binding(binding) => self.binding_func(binding, first),
        }
    }

    /// A function's qualified name; a hash function with its required
    /// method (M6a).
    fn func_name(&mut self, path: &str, span: Span) -> Result<Expr> {
        match StdFn::from_name(path).and_then(StdFn::chain) {
            Some(method) => self.chained(path, span, method),
            None => Ok(name_expr(path, span)),
        }
    }

    /// After the local module's name (already consumed at `first`), `.x`
    /// for one of its names: `a.x` as one name. A module is not a value
    /// (D83).
    fn local_name(&mut self, local: &LocalModule, first: Span) -> Result<Expr> {
        if !self.eat(&TokenKind::Dot) {
            let msg = format!(
                "`{}` is a module, not a value; use one of its names",
                local.name
            );
            return Err(error(first, msg));
        }
        let attr = self.expect_name("an attribute name")?;
        let span = first.merge(attr.span);
        if !local.names.contains(&attr.name) {
            let msg = format!("module `{}` has no `{}`", local.name, attr.name);
            return Err(error(span, msg));
        }
        Ok(name_expr(&format!("{}.{}", local.name, attr.name), span))
    }

    /// After a package's name (already consumed at `first`), the path to
    /// a module imported here, then one of its names: `a.b.f`.
    fn package_name(&mut self, mut path: String, first: Span) -> Result<Expr> {
        let mut span = first;
        loop {
            if !self.eat(&TokenKind::Dot) {
                let msg = format!("`{path}` is a package, not a value; use one of its modules");
                return Err(error(span, msg));
            }
            let attr = self.expect_name("a module name")?;
            path = format!("{path}.{}", attr.name);
            span = span.merge(attr.span);
            let local = self
                .imported_locals
                .iter()
                .copied()
                .find(|m| m.name == path);
            if let Some(local) = local {
                return self.local_name(local, span);
            }
            if !self.is_package(&path) {
                let msg = format!("module `{path}` is not imported here; write `import {path}`");
                return Err(error(span, msg));
            }
        }
    }

    /// After the standard module's name (already consumed at `first`),
    /// standing for `path`, its attributes up to an exposed name:
    /// `sys.argv` as one name.
    fn module_name(&mut self, module: &ModuleDef, mut path: String, first: Span) -> Result<Expr> {
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
                return self.func_name(&path, span);
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

    /// The type `name` (already consumed at `first`) names: a record
    /// `from` imported, or a module's `a.P`, by its qualified name
    /// (D136); any other name as written.
    pub(super) fn imported_type(&mut self, name: String, first: Span) -> Result<String> {
        let Some(imported) = self.imported(&name).cloned() else {
            return Ok(name);
        };
        let expr = self.imported_expr(imported, first)?;
        match expr.kind {
            ExprKind::Name(path) => Ok(path),
            _ => Ok(name),
        }
    }

    /// A name a statement binds: not one an import bound.
    pub(super) fn binding_name(&mut self, what: &str) -> Result<Ident> {
        let name = self.expect_name(what)?;
        self.check_bindable(name)
    }

    /// `name`, unless an import bound it: a module, package, binding or
    /// function `from` bound is not a value to rebind.
    fn check_bindable(&self, name: Ident) -> Result<Ident> {
        let kind = match self.imported(&name.name) {
            None => return Ok(name),
            Some(Imported::Func(_)) => "function",
            Some(Imported::Module(Module::Package(_))) => "package",
            Some(Imported::Module(Module::Binding(_))) => "Rust binding",
            Some(Imported::Module(_)) => "module",
        };
        let msg = format!(
            "`{}` names an imported {kind}; choose another name",
            name.name
        );
        Err(error(name.span, msg))
    }
}

/// What `import path` binds its first name to: the top module or
/// package. `concurrent` alone has no names, so it is imported as
/// `concurrent.futures` (M6c).
fn top<'a>(path: &Path, module: Module<'a>) -> Result<Module<'a>> {
    let first = &path.parts[0];
    let dotted = path.parts.len() > 1;
    match module {
        Module::Std(m, _) => {
            let (top, sub) = stdlib::DOTTED_IMPORT;
            if m.name == top && !dotted {
                let msg = format!("write `import {top}.{sub}`: `{top}` alone has no names");
                return Err(error(first.span, msg));
            }
            Ok(Module::Std(m, m.name.to_string()))
        }
        Module::Local(_) | Module::Package(_) if dotted => Ok(Module::Package(first.name.clone())),
        other => Ok(other),
    }
}

/// The dotted path of `module`.
fn module_path(module: &Module<'_>) -> String {
    match module {
        Module::Std(_, p) | Module::Package(p) => p.clone(),
        Module::Local(m) => m.name.clone(),
        Module::Binding(b) => b.name.clone(),
    }
}

/// `path`, a name of the standard module `m`: `None` for a variable.
fn from_std<'a>(m: &'static ModuleDef, path: &str, name: &Ident) -> Result<Option<Imported<'a>>> {
    if m.vars.iter().any(|(v, _)| *v == path) {
        return Ok(None);
    }
    if m.exposes(path) {
        return Ok(Some(Imported::Func(path.to_string())));
    }
    if m.continues(path) {
        return Ok(Some(Imported::Module(Module::Std(m, path.to_string()))));
    }
    let msg = format!(
        "`{path}` is not supported; the `{}` names are: {}",
        m.name,
        m.names().join(", ")
    );
    Err(error(name.span, msg))
}

/// `path`, a name of the local module `m`: `None` for a variable.
fn from_local<'a>(m: &LocalModule, path: &str, name: &Ident) -> Result<Option<Imported<'a>>> {
    if m.defs.contains(&name.name) {
        return Ok(Some(Imported::Func(path.to_string())));
    }
    if m.names.contains(&name.name) {
        return Ok(None);
    }
    let msg = format!("module `{}` has no `{}`", m.name, name.name);
    Err(error(name.span, msg))
}

/// `path`, a function of the binding `b` (M6b).
fn from_binding<'a>(b: &Binding, path: &str, name: &Ident) -> Result<Imported<'a>> {
    if b.funcs.iter().any(|f| f.name.name == path) {
        return Ok(Imported::Func(path.to_string()));
    }
    let msg = format!("the stub for `{}` has no `{}`", b.name, name.name);
    Err(error(name.span, msg))
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
    assign(var, call, span)
}

/// `var = value`: a module variable's initializer, or a variable `from`
/// copies into a global, holding the same object (D83).
fn assign(var: &str, value: Expr, span: Span) -> Stmt {
    let target = Target {
        kind: TargetKind::Name(var.to_string()),
        span,
    };
    Stmt {
        kind: StmtKind::Assign { target, value },
        span,
    }
}

fn name_expr(name: &str, span: Span) -> Expr {
    Expr {
        kind: ExprKind::Name(name.to_string()),
        span,
    }
}
