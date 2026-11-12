//! Parser: `[Token]` to `ast::Module`. Thread B.
//!
//! Recursive descent, one function per rule in `docs/grammar.md`;
//! operator precedence follows the grammar's rule chain. Assignment
//! targets are parsed as expressions and validated afterwards.
//!
//! Also rejected here, as CPython rejects them before running anything:
//! `def` below top level, `return` outside a function, `break` and
//! `continue` outside a loop, duplicate parameters, a parameter without a
//! default after one with a default, and a positional argument after a
//! keyword argument.

use crate::ast::{
    Arg, BinOp, Binding, Block, Branch, CmpOp, CompFor, Expr, ExprKind, FuncDef, Gather, Handler,
    Ident, LocalModule, LogicalOp, Mode, Module, Param, ParamMode, Pause, Stmt, StmtKind, StubDef,
    Target, TargetKind, TypeExpr, UnaryOp,
};
use crate::diag::{DiagKind, Diagnostic, Result};
use crate::span::Span;
use crate::token::{Token, TokenKind};

/// Limit both recursive grammar calls and constructed expression height.
/// Iterative folds must not bypass the bound protecting later passes and drop.
const MAX_DEPTH: usize = 100;

/// Parses a token stream that ends in exactly one `EndMarker`.
pub fn parse(tokens: &[Token]) -> Result<Module> {
    parse_with(tokens, &[])
}

/// [`parse`], where `import` may also name one of `bindings` (M6b).
pub fn parse_with(tokens: &[Token], bindings: &[Binding]) -> Result<Module> {
    parse_module(tokens, bindings, &[])
}

/// [`parse_with`], where `import` may also name one of the program's
/// `locals` modules (step 3), each already parsed.
pub fn parse_module(
    tokens: &[Token],
    bindings: &[Binding],
    locals: &[LocalModule],
) -> Result<Module> {
    parse_in(tokens, bindings, locals, &[], &mut Vec::new())
}

/// [`parse_module`], with the program's package directories, where the
/// standard modules in `ready` are already initialized by an earlier
/// module, so importing one shares its state (D83); those this module
/// initializes are added.
fn parse_in(
    tokens: &[Token],
    bindings: &[Binding],
    locals: &[LocalModule],
    packages: &[String],
    ready: &mut Vec<&'static str>,
) -> Result<Module> {
    let mut parser = Parser::new(tokens, bindings)?;
    parser.locals = locals;
    parser.packages = packages;
    parser.ready = std::mem::take(ready);
    let module = parser.module()?;
    *ready = parser.ready;
    Ok(module)
}

/// A local module: its dotted name and its tokens, their spans already
/// in the program's one span space (D82).
pub struct Unit<'t> {
    pub name: &'t str,
    pub tokens: &'t [Token],
}

/// Parses a whole program as one module (step 3, D76): each local
/// module in the order its top level runs (D83), its globals qualified,
/// then the main file, which may import any of them or of `packages`,
/// the package directories imported, by dotted name (D78).
pub fn parse_program(
    locals: &[Unit<'_>],
    packages: &[String],
    main: &[Token],
    bindings: &[Binding],
) -> Result<Module> {
    let mut done: Vec<LocalModule> = Vec::new();
    let mut parts: Vec<Module> = Vec::new();
    let mut ready = Vec::new();
    for unit in locals {
        let mut module = parse_in(unit.tokens, bindings, &done, packages, &mut ready)?;
        done.push(qualify::qualify(&mut module, unit.name));
        parts.push(module);
    }
    let mut module = parse_in(main, bindings, &done, packages, &mut ready)?;
    qualify::main_name(&mut module, &parts);
    let module = qualify::link(parts, module);
    record::check_modules(&module, &done)?;
    Ok(module)
}

/// Parses the stub of the Rust binding `name`: only `temper def`
/// signatures whose bodies are `...` (M6b).
pub fn parse_stub(tokens: &[Token], name: &str) -> Result<Binding> {
    Parser::new(tokens, &[])?.stub(name)
}

mod comp;
mod depth;
pub mod qualify;
use depth::bounded;

type Rule<'a, T> = fn(&mut Parser<'a>) -> Result<T>;

struct Parser<'a> {
    tokens: &'a [Token],
    /// The final `EndMarker`; the cursor never moves past it.
    eof: &'a Token,
    pos: usize,
    /// Span of the last consumed token that is not layout.
    last: Span,
    depth: usize,
    in_func: bool,
    in_loop: bool,
    /// Inside a `finally` block of this function (or the module's code):
    /// `return` would leave it (D89).
    in_finally: bool,
    /// Inside a `finally` block entered since the innermost loop:
    /// `break` and `continue` would leave it (D89).
    jump_leaves_finally: bool,
    /// The names imports bound so far (D84).
    imports: Vec<(String, imports::Imported<'a>)>,
    /// The standard modules whose variables are initialized, by this
    /// module or an earlier one: importing one again shares them.
    ready: Vec<&'static str>,
    /// The Rust bindings `import` may name, and those it has.
    bindings: &'a [Binding],
    imported_bindings: Vec<&'a Binding>,
    /// The program's local modules `import` may name, and those it has.
    locals: &'a [LocalModule],
    imported_locals: Vec<&'a LocalModule>,
    /// The program's package directories, by dotted name (D78).
    packages: &'a [String],
    /// The functions of the imported bindings.
    stubs: Vec<StubDef>,
    /// The `pause` markers so far (M6c).
    pauses: Vec<Pause>,
    /// File `with` items desugared so far: numbers their hidden temps.
    withs: usize,
    /// The program defines its own `open`, so `with open(...)` is an
    /// executor item (D115).
    user_open: bool,
    /// `from dataclasses import dataclass` came first (D132).
    dataclass: bool,
    /// While a plain class's `__init__` is parsed, the field annotations
    /// it wrote (`self.x: T = v`, D144).
    init: Option<Vec<(Ident, TypeExpr)>>,
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token], bindings: &'a [Binding]) -> Result<Self> {
        let first_end = tokens.iter().position(|t| t.kind == TokenKind::EndMarker);
        let eof = match (first_end, tokens.last()) {
            (Some(i), Some(eof)) if i + 1 == tokens.len() => eof,
            (_, last) => {
                let span = last.map_or(Span { start: 0, end: 0 }, |t| t.span);
                return Err(error(span, "token stream must end with one end marker"));
            }
        };
        Ok(Parser {
            tokens,
            eof,
            pos: 0,
            last: eof.span,
            depth: 0,
            in_func: false,
            in_loop: false,
            in_finally: false,
            jump_leaves_finally: false,
            imports: Vec::new(),
            ready: Vec::new(),
            bindings,
            imported_bindings: Vec::new(),
            locals: &[],
            imported_locals: Vec::new(),
            packages: &[],
            stubs: Vec::new(),
            pauses: Vec::new(),
            withs: 0,
            user_open: with::defines_open(tokens),
            dataclass: false,
            init: None,
        })
    }

    // Cursor.

    fn tok(&self, ahead: usize) -> &'a Token {
        self.tokens.get(self.pos + ahead).unwrap_or(self.eof)
    }

    fn peek(&self) -> &'a TokenKind {
        &self.tok(0).kind
    }

    fn at(&self, kind: &TokenKind) -> bool {
        self.peek() == kind
    }

    fn bump(&mut self) -> &'a Token {
        let tok = self.tok(0);
        if tok.kind != TokenKind::EndMarker {
            self.pos += 1;
        }
        if !is_layout(&tok.kind) {
            self.last = tok.span;
        }
        tok
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        let found = self.at(kind);
        if found {
            self.bump();
        }
        found
    }

    fn expect(&mut self, kind: &TokenKind) -> Result<&'a Token> {
        if self.at(kind) {
            return Ok(self.bump());
        }
        Err(self.unexpected(&describe(kind)))
    }

    fn expect_name(&mut self, what: &str) -> Result<Ident> {
        let tok = self.tok(0);
        let TokenKind::Name(name) = &tok.kind else {
            return Err(self.unexpected(what));
        };
        self.bump();
        Ok(Ident {
            name: name.clone(),
            span: tok.span,
        })
    }

    /// A statement that is only `...`, with Nano's fix-it: Python's
    /// placeholder body, which Pythrust spells `pass` outside a stub.
    /// `hint` says where `pass` goes.
    fn ellipsis_stmt(&self, hint: &str) -> Result<()> {
        let tok = self.tok(0);
        if tok.kind == TokenKind::Ellipsis && self.tok(1).kind == TokenKind::Newline {
            let msg = format!("`...` is for stub bodies only; use pass{hint}");
            return Err(error(tok.span, msg));
        }
        Ok(())
    }

    fn unexpected(&self, expected: &str) -> Diagnostic {
        let tok = self.tok(0);
        if tok.kind == TokenKind::Ellipsis {
            let msg = "`...` (Ellipsis) is not supported; it is legal only as a stub body";
            return error(tok.span, msg);
        }
        if tok.kind == TokenKind::As {
            return error(
                tok.span,
                "'as' is allowed only in `except Name as e` and `with value as name`",
            );
        }
        let found = describe(&tok.kind);
        error(tok.span, format!("expected {expected}, found {found}"))
    }

    /// From `start` through the last consumed token.
    fn span_from(&self, start: Span) -> Span {
        start.merge(self.last)
    }

    fn nested<T>(&mut self, rule: Rule<'a, T>) -> Result<T> {
        if self.depth >= MAX_DEPTH {
            return Err(error(self.tok(0).span, "too deeply nested"));
        }
        self.depth += 1;
        let out = rule(self);
        self.depth -= 1;
        out
    }

    fn looping<T>(&mut self, rule: Rule<'a, T>) -> Result<T> {
        let outer = (self.in_loop, self.jump_leaves_finally);
        (self.in_loop, self.jump_leaves_finally) = (true, false);
        let out = rule(self);
        (self.in_loop, self.jump_leaves_finally) = outer;
        out
    }

    /// A `finally` block: no `return`, `break` or `continue` may leave it
    /// (D89).
    fn finally_block(&mut self) -> Result<Block> {
        let outer = (self.in_finally, self.jump_leaves_finally);
        (self.in_finally, self.jump_leaves_finally) = (true, true);
        let out = self.block();
        (self.in_finally, self.jump_leaves_finally) = outer;
        out
    }

    /// `item (',' item)* [',']`. Returns the items and whether a comma was
    /// seen. A comma not followed by an expression start is trailing.
    fn comma_list<T>(&mut self, item: Rule<'a, T>) -> Result<(Vec<T>, bool)> {
        let mut items = vec![item(self)?];
        let mut comma = false;
        while self.eat(&TokenKind::Comma) {
            comma = true;
            if !starts_expr(self.peek()) {
                break;
            }
            items.push(item(self)?);
        }
        Ok((items, comma))
    }

    // Program and statements.

    fn module(&mut self) -> Result<Module> {
        let start = self.tok(0).span;
        let mut body = Vec::new();
        while self.at(&TokenKind::Import) || self.at(&TokenKind::From) {
            body.extend(self.import_stmt()?);
        }
        while !self.at(&TokenKind::EndMarker) {
            body.extend(self.stmts()?);
        }
        record::check_module(&body)?;
        Ok(Module {
            body,
            stubs: std::mem::take(&mut self.stubs),
            pauses: std::mem::take(&mut self.pauses),
            starts: vec![0],
            span: start.merge(self.eof.span),
        })
    }

    /// One statement, or the several a file `with` desugars to.
    fn stmts(&mut self) -> Result<Vec<Stmt>> {
        if self.at(&TokenKind::With) {
            return self.with_stmt();
        }
        if self.at_exc_class() {
            return Ok(vec![self.exc_class()?]);
        }
        if self.at(&TokenKind::Class) {
            return self.plain_class();
        }
        if self.at(&TokenKind::At) {
            return self.record();
        }
        Ok(vec![self.stmt()?])
    }

    fn stmt(&mut self) -> Result<Stmt> {
        self.ellipsis_stmt("")?;
        match self.peek() {
            TokenKind::Def | TokenKind::Temper => self.func_def(),
            TokenKind::If => self.if_stmt(),
            TokenKind::While => self.while_stmt(),
            TokenKind::For => self.for_stmt(),
            TokenKind::Try => self.try_stmt(),
            TokenKind::Import | TokenKind::From => {
                let msg = "`import` must come at module top level, before other statements";
                Err(error(self.tok(0).span, msg))
            }
            _ => {
                let stmt = self.simple_stmt()?;
                self.expect(&TokenKind::Newline)?;
                Ok(stmt)
            }
        }
    }

    fn block(&mut self) -> Result<Block> {
        // A body is never on the `:` line, so `pass` must move down too.
        self.ellipsis_stmt(" on its own indented line")?;
        self.expect(&TokenKind::Newline)?;
        self.expect(&TokenKind::Indent)?;
        self.nested(|p| {
            let mut body = p.block_stmt()?;
            while !p.eat(&TokenKind::Dedent) {
                body.extend(p.block_stmt()?);
            }
            Ok(body)
        })
    }

    fn block_stmt(&mut self) -> Result<Vec<Stmt>> {
        if matches!(self.peek(), TokenKind::Def | TokenKind::Temper) {
            let span = self.tok(0).span;
            return Err(error(span, "`def` is only allowed at module top level"));
        }
        if matches!(self.peek(), TokenKind::At | TokenKind::Class) {
            let msg = "a class must be defined at the top level of a module";
            return Err(error(self.tok(0).span, msg));
        }
        self.stmts()
    }

    fn simple_stmt(&mut self) -> Result<Stmt> {
        let start = self.tok(0).span;
        let kind = match self.peek() {
            TokenKind::Return => self.return_stmt()?,
            TokenKind::Raise => self.raise_stmt()?,
            TokenKind::Del => self.del_stmt()?,
            TokenKind::Break => self.loop_jump(StmtKind::Break, "break")?,
            TokenKind::Continue => self.loop_jump(StmtKind::Continue, "continue")?,
            TokenKind::Pass => {
                self.bump();
                StmtKind::Pass
            }
            _ => self.expr_stmt()?,
        };
        Ok(Stmt {
            kind,
            span: self.span_from(start),
        })
    }

    fn return_stmt(&mut self) -> Result<StmtKind> {
        let span = self.bump().span;
        if !self.in_func {
            return Err(error(span, "`return` outside function"));
        }
        if self.in_finally {
            return Err(error(span, leaves_finally("return")));
        }
        if !starts_expr(self.peek()) {
            return Ok(StmtKind::Return(None));
        }
        Ok(StmtKind::Return(Some(self.expr_list()?)))
    }

    /// `'raise' expr`. A bare `raise` (re-raise) is not supported yet.
    fn raise_stmt(&mut self) -> Result<StmtKind> {
        let span = self.bump().span;
        if !starts_expr(self.peek()) {
            let msg = "bare `raise` is not supported yet; catch with `except Name as e` \
                       and write `raise e`";
            return Err(error(span, msg));
        }
        Ok(StmtKind::Raise(self.expr()?))
    }

    /// `'del' target`: only one subscript, `xs[i]` or `d[k]`, for now.
    fn del_stmt(&mut self) -> Result<StmtKind> {
        self.bump();
        let target = self.expr_list()?;
        let what = match target.kind {
            ExprKind::Subscript { value, index } => {
                return Ok(StmtKind::Del {
                    value: *value,
                    index: *index,
                })
            }
            ExprKind::Slice { .. } => "`del` of a slice is not supported yet",
            ExprKind::Name(_) => "`del` of a name is not supported yet",
            ExprKind::Tuple(_) => "`del` of several targets is not supported yet",
            ExprKind::Attr { .. } => "`del` of an attribute is not supported yet",
            _ => "cannot delete this; `del` takes an item, as in `del xs[i]` or `del d[k]`",
        };
        Err(error(target.span, what))
    }

    fn loop_jump(&mut self, kind: StmtKind, word: &str) -> Result<StmtKind> {
        let span = self.bump().span;
        if !self.in_loop {
            return Err(error(span, format!("`{word}` outside loop")));
        }
        if self.jump_leaves_finally {
            return Err(error(span, leaves_finally(word)));
        }
        Ok(kind)
    }

    /// `assign | ann_assign | aug_assign | expr_list`, told apart by the
    /// token after the first `expr_list`.
    fn expr_stmt(&mut self) -> Result<StmtKind> {
        let lhs = self.expr_list()?;
        if self.eat(&TokenKind::Assign) {
            return self.assign(lhs);
        }
        if self.eat(&TokenKind::Colon) {
            return self.ann_assign(lhs);
        }
        let Some(op) = aug_op(self.peek()) else {
            return Ok(StmtKind::Expr(lhs));
        };
        self.bump();
        self.aug_assign(lhs, op)
    }

    fn assign(&mut self, lhs: Expr) -> Result<StmtKind> {
        let target = to_target(lhs)?;
        let value = self.expr_list()?;
        if self.at(&TokenKind::Assign) {
            let span = self.tok(0).span;
            return Err(error(span, "chained assignment is not supported"));
        }
        Ok(StmtKind::Assign { target, value })
    }

    fn ann_assign(&mut self, lhs: Expr) -> Result<StmtKind> {
        if let Some(field) = self_field(&lhs).filter(|_| self.init.is_some()) {
            return self.field_init(lhs, field);
        }
        let ExprKind::Name(name) = lhs.kind else {
            return Err(error(lhs.span, "only a single name can be annotated"));
        };
        check_assignable(&name, lhs.span)?;
        let annotation = self.type_expr()?;
        self.expect(&TokenKind::Assign)?;
        let value = self.expr()?;
        Ok(StmtKind::AnnAssign {
            target: Ident {
                name,
                span: lhs.span,
            },
            annotation,
            value,
        })
    }

    /// `self.x: T = v` in a plain class's `__init__` (D144): a store
    /// to field `x`, which `T` annotates.
    fn field_init(&mut self, lhs: Expr, field: Ident) -> Result<StmtKind> {
        let annotation = self.type_expr()?;
        self.expect(&TokenKind::Assign)?;
        let value = self.expr()?;
        if let Some(init) = self.init.as_mut() {
            init.push((field, annotation));
        }
        let target = to_target(lhs)?;
        Ok(StmtKind::Assign { target, value })
    }

    fn aug_assign(&mut self, lhs: Expr, op: BinOp) -> Result<StmtKind> {
        let target = to_target(lhs)?;
        if matches!(target.kind, TargetKind::Tuple(_)) {
            let msg = "augmented assignment needs a single target, not a tuple";
            return Err(error(target.span, msg));
        }
        let value = self.expr()?;
        Ok(StmtKind::AugAssign { target, op, value })
    }

    fn func_def(&mut self) -> Result<Stmt> {
        let def = self.func_head(|p| p.binding_name("a function name"))?;
        let span = def.span;
        Ok(Stmt {
            kind: StmtKind::FuncDef(def),
            span,
        })
    }

    /// `['temper'] 'def' NAME '(' ...` through its body, the name read
    /// by `name`.
    fn func_head(&mut self, name: Rule<'a, Ident>) -> Result<FuncDef> {
        let start = self.tok(0).span;
        let mode = if self.eat(&TokenKind::Temper) {
            Mode::Tempered
        } else {
            Mode::Loose
        };
        self.expect(&TokenKind::Def)?;
        let name = name(self)?;
        self.expect(&TokenKind::LParen)?;
        let params = if self.at(&TokenKind::RParen) {
            Vec::new()
        } else {
            self.comma_list(Self::param)?.0
        };
        self.expect(&TokenKind::RParen)?;
        check_params(&params)?;
        let returns = if self.eat(&TokenKind::Arrow) {
            Some(self.type_expr()?)
        } else {
            None
        };
        let raises = self.raises()?;
        self.expect(&TokenKind::Colon)?;
        self.in_func = true;
        let body = self.block();
        self.in_func = false;
        Ok(FuncDef {
            mode,
            name,
            params,
            returns,
            raises,
            body: body?,
            span: self.span_from(start),
        })
    }

    /// `['raises' NAME (',' NAME)*]`. `raises` is contextual: a keyword
    /// only here, so it stays usable as a name.
    fn raises(&mut self) -> Result<Vec<Ident>> {
        if !matches!(self.peek(), TokenKind::Name(n) if n == "raises") {
            return Ok(Vec::new());
        }
        self.bump();
        let mut names = vec![self.exc_name("an exception name after `raises`")?];
        while self.eat(&TokenKind::Comma) {
            names.push(self.exc_name("an exception name after `,`")?);
        }
        Ok(names)
    }

    fn param(&mut self) -> Result<Param> {
        let start = self.tok(0).span;
        let mode = self.param_mode()?;
        let name = self.binding_name("a parameter name")?;
        let annotation = if self.eat(&TokenKind::Colon) {
            Some(self.type_expr()?)
        } else {
            None
        };
        let default = if self.eat(&TokenKind::Assign) {
            Some(self.expr()?)
        } else {
            None
        };
        Ok(Param {
            span: self.span_from(start),
            mode,
            name,
            annotation,
            default,
        })
    }

    /// `['mut' | 'owned']` before a parameter name (D2). The words are
    /// contextual, a mode only when another name follows, so a parameter
    /// may still be called `mut`. `read` is the default and never written.
    fn param_mode(&mut self) -> Result<ParamMode> {
        let (TokenKind::Name(word), TokenKind::Name(_)) = (self.peek(), &self.tok(1).kind) else {
            return Ok(ParamMode::Read);
        };
        let mode = match word.as_str() {
            "mut" => ParamMode::Mut,
            "owned" => ParamMode::Owned,
            "read" => {
                let msg = "parameters are read unless marked; drop `read`";
                return Err(error(self.tok(0).span, msg));
            }
            _ => return Ok(ParamMode::Read),
        };
        self.bump();
        Ok(mode)
    }

    fn if_stmt(&mut self) -> Result<Stmt> {
        let start = self.tok(0).span;
        let branch = self.branch()?;
        let mut elifs = Vec::new();
        while self.at(&TokenKind::Elif) {
            elifs.push(self.branch()?);
        }
        let orelse = self.else_clause()?;
        let kind = StmtKind::If {
            branch,
            elifs,
            orelse,
        };
        Ok(Stmt {
            kind,
            span: self.span_from(start),
        })
    }

    fn while_stmt(&mut self) -> Result<Stmt> {
        let start = self.tok(0).span;
        let branch = self.looping(Self::branch)?;
        let orelse = self.else_clause()?;
        Ok(Stmt {
            kind: StmtKind::While { branch, orelse },
            span: self.span_from(start),
        })
    }

    fn for_stmt(&mut self) -> Result<Stmt> {
        let start = self.bump().span;
        let target = self.for_target()?;
        self.expect(&TokenKind::In)?;
        let iter = self.expr()?;
        self.expect(&TokenKind::Colon)?;
        let body = self.looping(Self::block)?;
        let orelse = self.else_clause()?;
        let kind = StmtKind::For {
            target,
            iter,
            body,
            orelse,
        };
        Ok(Stmt {
            kind,
            span: self.span_from(start),
        })
    }

    /// `'try' ':' block (handler+ [finally] | finally)`. An `else`
    /// clause is later.
    fn try_stmt(&mut self) -> Result<Stmt> {
        let start = self.bump().span;
        self.expect(&TokenKind::Colon)?;
        let body = self.block()?;
        let mut handlers = Vec::new();
        while self.at(&TokenKind::Except) {
            handlers.push(self.handler()?);
        }
        if self.at(&TokenKind::Else) && !handlers.is_empty() {
            let span = self.tok(0).span;
            return Err(error(span, "`else` on `try` is not supported yet"));
        }
        let finally = match self.eat(&TokenKind::Finally) {
            true => {
                self.expect(&TokenKind::Colon)?;
                Some(self.finally_block()?)
            }
            false if handlers.is_empty() => {
                return Err(self.unexpected("`except` or `finally`"));
            }
            false => None,
        };
        Ok(Stmt {
            kind: StmtKind::Try {
                body,
                handlers,
                finally,
            },
            span: self.span_from(start),
        })
    }

    /// `'except' classes ['as' NAME] ':' block`
    fn handler(&mut self) -> Result<Handler> {
        let start = self.expect(&TokenKind::Except)?.span;
        if self.at(&TokenKind::Colon) {
            let msg = "bare `except:` is not supported yet; name the exception, \
                       or write `except Exception:` to catch any";
            return Err(error(self.tok(0).span, msg));
        }
        let classes = self.handler_classes()?;
        let binding = if self.eat(&TokenKind::As) {
            Some(self.binding_name("a name to bind the exception to")?)
        } else {
            None
        };
        self.expect(&TokenKind::Colon)?;
        let body = self.block()?;
        Ok(Handler {
            classes,
            binding,
            body,
            span: self.span_from(start),
        })
    }

    /// `NAME | '(' NAME (',' NAME)* [','] ')'`. Unparenthesized
    /// `A, B` is an error, as in Python before 3.14; so are `()` and a
    /// nested tuple, which Python accepts.
    fn handler_classes(&mut self) -> Result<Vec<Ident>> {
        if !self.eat(&TokenKind::LParen) {
            let class = self.handler_class()?;
            if self.at(&TokenKind::Comma) {
                let msg = "multiple exception types must be parenthesized, \
                           as in `except (ValueError, IndexError):`";
                return Err(error(self.tok(0).span, msg));
            }
            return Ok(vec![class]);
        }
        if self.at(&TokenKind::RParen) {
            let msg = "`except ()` catches nothing and is not supported; \
                       name at least one exception class";
            return Err(error(self.tok(0).span, msg));
        }
        let mut classes = vec![self.handler_class()?];
        while self.eat(&TokenKind::Comma) && !self.at(&TokenKind::RParen) {
            classes.push(self.handler_class()?);
        }
        self.expect(&TokenKind::RParen)?;
        Ok(classes)
    }

    /// One class of a handler: a name, not a nested tuple.
    fn handler_class(&mut self) -> Result<Ident> {
        if self.at(&TokenKind::LParen) {
            let msg = "nested tuples of exception classes are not supported; \
                       list every class in one tuple, as in `except (A, B, C):`";
            return Err(error(self.tok(0).span, msg));
        }
        self.exc_name("an exception name, such as `ValueError`")
    }

    /// `target_list` before `in`. Parsed at `postfix` level so the `in`
    /// is not read as a comparison.
    fn for_target(&mut self) -> Result<Target> {
        let start = self.tok(0).span;
        let (items, comma) = self.comma_list(Self::postfix)?;
        to_target(self.tuple_or_single(items, comma, start)?)
    }

    /// `('if' | 'elif' | 'while') expr ':' block`. The caller has seen the
    /// keyword.
    fn branch(&mut self) -> Result<Branch> {
        let start = self.bump().span;
        let test = self.expr()?;
        self.expect(&TokenKind::Colon)?;
        let body = self.block()?;
        Ok(Branch {
            test,
            body,
            span: self.span_from(start),
        })
    }

    /// `['else' ':' block]`; empty when absent.
    fn else_clause(&mut self) -> Result<Block> {
        if !self.eat(&TokenKind::Else) {
            return Ok(Vec::new());
        }
        self.expect(&TokenKind::Colon)?;
        self.block()
    }

    // Types.

    /// `type_atom ['|' 'None']`. `None | None` is rejected: it is `None`.
    fn type_expr(&mut self) -> Result<TypeExpr> {
        let atom = self.type_atom()?;
        if !self.eat(&TokenKind::Pipe) {
            return Ok(atom);
        }
        if !self.at(&TokenKind::None) {
            return Err(self.unexpected("`None` (only `T | None` unions exist)"));
        }
        self.bump();
        let span = self.span_from(atom.span);
        if atom.name.name == "None" {
            return Err(error(span, "`None | None` is just `None`"));
        }
        Ok(TypeExpr {
            optional: true,
            span,
            ..atom
        })
    }

    fn type_atom(&mut self) -> Result<TypeExpr> {
        let tok = self.tok(0);
        let name = match &tok.kind {
            TokenKind::Name(name) => name.clone(),
            TokenKind::None => "None".to_string(),
            TokenKind::Str(_) => {
                let msg = "string annotations are not supported yet";
                return Err(error(tok.span, msg));
            }
            _ => return Err(self.unexpected("a type")),
        };
        self.bump();
        let name = self.imported_type(name, tok.span)?;
        let mut args = Vec::new();
        if self.eat(&TokenKind::LBracket) {
            args.push(self.nested(Self::type_expr)?);
            while self.eat(&TokenKind::Comma) {
                args.push(self.nested(Self::type_expr)?);
            }
            self.expect(&TokenKind::RBracket)?;
        }
        Ok(TypeExpr {
            name: Ident {
                name,
                span: tok.span,
            },
            args,
            optional: false,
            span: self.span_from(tok.span),
        })
    }

    // Expressions, lowest precedence first.

    fn expr_list(&mut self) -> Result<Expr> {
        let start = self.tok(0).span;
        let (items, comma) = self.comma_list(Self::expr)?;
        self.tuple_or_single(items, comma, start)
    }

    fn tuple_or_single(&self, items: Vec<Expr>, comma: bool, start: Span) -> Result<Expr> {
        let items = match <[Expr; 1]>::try_from(items) {
            Ok([single]) if !comma => return Ok(single),
            Ok(one) => Vec::from(one),
            Err(many) => many,
        };
        bounded(Expr {
            kind: ExprKind::Tuple(items),
            span: self.span_from(start),
        })
    }

    fn expr(&mut self) -> Result<Expr> {
        self.nested(Self::paused)
    }

    /// `'pause' expr | conditional` (M6c): the marker covers the whole
    /// expression to its right and is recorded beside the tree.
    fn paused(&mut self) -> Result<Expr> {
        if !self.at(&TokenKind::Pause) {
            return self.conditional();
        }
        let keyword = self.bump().span;
        let covered = self.expr()?;
        self.pauses.push(Pause {
            keyword,
            covers: covered.span,
        });
        Ok(covered)
    }

    /// `or_expr ['if' or_expr 'else' expr]`
    fn conditional(&mut self) -> Result<Expr> {
        let body = self.or_expr()?;
        if !self.eat(&TokenKind::If) {
            return Ok(body);
        }
        let test = self.or_expr()?;
        self.expect(&TokenKind::Else)?;
        let orelse = self.expr()?;
        let span = body.span.merge(orelse.span);
        let kind = ExprKind::IfExp {
            test: Box::new(test),
            body: Box::new(body),
            orelse: Box::new(orelse),
        };
        bounded(Expr { kind, span })
    }

    fn or_expr(&mut self) -> Result<Expr> {
        self.left_assoc(Self::and_expr, or_op, logical)
    }

    fn and_expr(&mut self) -> Result<Expr> {
        self.left_assoc(Self::not_expr, and_op, logical)
    }

    fn not_expr(&mut self) -> Result<Expr> {
        if !self.at(&TokenKind::Not) {
            return self.comparison();
        }
        let start = self.bump().span;
        let operand = self.nested(Self::not_expr)?;
        unary(UnaryOp::Not, start, operand)
    }

    fn comparison(&mut self) -> Result<Expr> {
        let left = self.sum()?;
        let mut rest = Vec::new();
        while let Some(op) = self.comp_op() {
            rest.push((op, self.sum()?));
        }
        let Some((_, last)) = rest.last() else {
            return Ok(left);
        };
        let span = left.span.merge(last.span);
        let kind = ExprKind::Compare {
            left: Box::new(left),
            rest,
        };
        bounded(Expr { kind, span })
    }

    /// Consumes a `comp_op`, including the two-token `not in` and `is not`.
    fn comp_op(&mut self) -> Option<CmpOp> {
        if self.at(&TokenKind::Not) && self.tok(1).kind == TokenKind::In {
            self.bump();
            self.bump();
            return Some(CmpOp::NotIn);
        }
        if self.eat(&TokenKind::Is) {
            let negated = self.eat(&TokenKind::Not);
            return Some(if negated { CmpOp::IsNot } else { CmpOp::Is });
        }
        let op = cmp_op(self.peek())?;
        self.bump();
        Some(op)
    }

    fn sum(&mut self) -> Result<Expr> {
        self.left_assoc(Self::term, sum_op, binary)
    }

    fn term(&mut self) -> Result<Expr> {
        self.left_assoc(Self::factor, term_op, binary)
    }

    /// `operand (op operand)*`, folded to the left.
    fn left_assoc<Op>(
        &mut self,
        operand: Rule<'a, Expr>,
        op_of: fn(&TokenKind) -> Option<Op>,
        build: fn(Op, Expr, Expr) -> Result<Expr>,
    ) -> Result<Expr> {
        let mut left = operand(self)?;
        while let Some(op) = op_of(self.peek()) {
            self.bump();
            let right = operand(self)?;
            left = build(op, left, right)?;
        }
        Ok(left)
    }

    fn factor(&mut self) -> Result<Expr> {
        let op = match self.peek() {
            TokenKind::Minus => UnaryOp::Neg,
            TokenKind::Plus => UnaryOp::Pos,
            _ => return self.power(),
        };
        if op == UnaryOp::Neg {
            if let Some(literal) = self.negative_literal()? {
                return Ok(literal);
            }
        }
        let start = self.bump().span;
        let operand = self.nested(Self::factor)?;
        unary(op, start, operand)
    }

    /// `'-' INT` with nothing binding tighter after it, folded to one
    /// `i64` literal. The only place `i64::MAX + 1` is in range.
    fn negative_literal(&mut self) -> Result<Option<Expr>> {
        let TokenKind::Int(magnitude) = self.tok(1).kind else {
            return Ok(None);
        };
        if binds_tighter_than_unary(&self.tok(2).kind) {
            return Ok(None);
        }
        let start = self.bump().span;
        let span = start.merge(self.bump().span);
        let Some(value) = 0i64.checked_sub_unsigned(magnitude) else {
            return Err(error(span, "integer literal is too small for int (i64)"));
        };
        Ok(Some(Expr {
            kind: ExprKind::Int(value),
            span,
        }))
    }

    /// `postfix ['**' factor]`: right-associative, binds tighter than a
    /// unary operator on its left.
    fn power(&mut self) -> Result<Expr> {
        let base = self.postfix()?;
        if !self.eat(&TokenKind::DoubleStar) {
            return Ok(base);
        }
        let exponent = self.nested(Self::factor)?;
        binary(BinOp::Pow, base, exponent)
    }

    fn postfix(&mut self) -> Result<Expr> {
        let mut expr = self.atom()?;
        loop {
            expr = match self.peek() {
                TokenKind::LParen => self.call(expr)?,
                TokenKind::LBracket => self.subscript(expr)?,
                TokenKind::Dot => self.attr(expr)?,
                _ => return Ok(expr),
            };
        }
    }

    fn call(&mut self, func: Expr) -> Result<Expr> {
        self.bump();
        let args = if self.at(&TokenKind::RParen) {
            Vec::new()
        } else {
            self.call_args()?
        };
        self.expect(&TokenKind::RParen)?;
        check_args(&args)?;
        bounded(Expr {
            span: self.span_from(func.span),
            kind: ExprKind::Call {
                func: Box::new(func),
                args,
            },
        })
    }

    /// `arg (',' arg)* [',']`, or a generator expression alone (D119):
    /// beside any other argument it must be parenthesized, as in CPython.
    fn call_args(&mut self) -> Result<Vec<Arg>> {
        let first = self.arg()?;
        if self.at(&TokenKind::For) {
            let start = first.span;
            let gen = self.comp_tail(Gather::Gen(Box::new(first.value)), start)?;
            if !self.at(&TokenKind::RParen) {
                return Err(error(gen.span, PARENS));
            }
            return Ok(vec![Arg {
                name: None,
                span: gen.span,
                value: gen,
            }]);
        }
        let mut args = vec![first];
        while self.eat(&TokenKind::Comma) && starts_expr(self.peek()) {
            let arg = self.arg()?;
            if self.at(&TokenKind::For) {
                return Err(error(arg.value.span, PARENS));
            }
            args.push(arg);
        }
        Ok(args)
    }

    /// `[NAME '='] expr`
    fn arg(&mut self) -> Result<Arg> {
        let start = self.tok(0).span;
        let keyword =
            matches!(self.peek(), TokenKind::Name(_)) && self.tok(1).kind == TokenKind::Assign;
        let name = if keyword {
            let name = self.expect_name("a keyword")?;
            self.bump();
            Some(name)
        } else {
            None
        };
        let value = self.expr()?;
        if name.is_some() && self.at(&TokenKind::For) {
            return Err(error(value.span, PARENS));
        }
        Ok(Arg {
            name,
            value,
            span: self.span_from(start),
        })
    }

    fn subscript(&mut self, value: Expr) -> Result<Expr> {
        self.bump();
        let first = self.slice_component()?;
        if self.eat(&TokenKind::Colon) {
            let stop = self.slice_component()?;
            let step = if self.eat(&TokenKind::Colon) {
                self.slice_component()?
            } else {
                None
            };
            self.expect(&TokenKind::RBracket)?;
            return bounded(Expr {
                span: self.span_from(value.span),
                kind: ExprKind::Slice {
                    value: Box::new(value),
                    start: first,
                    stop,
                    step,
                },
            });
        }
        let Some(index) = first else {
            return Err(error(self.tok(0).span, "expected an expression, found `]`"));
        };
        self.expect(&TokenKind::RBracket)?;
        bounded(Expr {
            span: self.span_from(value.span),
            kind: ExprKind::Subscript {
                value: Box::new(value),
                index,
            },
        })
    }

    fn slice_component(&mut self) -> Result<Option<Box<Expr>>> {
        if self.at(&TokenKind::Colon) || self.at(&TokenKind::RBracket) {
            return Ok(None);
        }
        self.expr().map(|e| Some(Box::new(e)))
    }

    fn attr(&mut self, value: Expr) -> Result<Expr> {
        self.bump();
        let attr = self.expect_name("an attribute name")?;
        bounded(Expr {
            span: value.span.merge(attr.span),
            kind: ExprKind::Attr {
                value: Box::new(value),
                attr,
            },
        })
    }

    fn atom(&mut self) -> Result<Expr> {
        let tok = self.tok(0);
        let kind = match &tok.kind {
            TokenKind::Name(name) => {
                if let Some(imported) = self.imported(name).cloned() {
                    self.bump();
                    return self.imported_expr(imported, tok.span);
                }
                ExprKind::Name(name.clone())
            }
            TokenKind::Int(value) => ExprKind::Int(int_literal(*value, tok.span)?),
            TokenKind::Float(value) => ExprKind::Float(*value),
            TokenKind::Str(value) => ExprKind::Str(value.clone()),
            TokenKind::True => ExprKind::Bool(true),
            TokenKind::False => ExprKind::Bool(false),
            TokenKind::None => ExprKind::None,
            TokenKind::LParen => return self.paren(),
            TokenKind::LBracket => return self.list(),
            TokenKind::LBrace => return self.braces(),
            TokenKind::Pause => {
                let msg = "`pause` covers a whole expression: put it first, or wrap \
                           what it covers in parentheses: `(pause ...)`";
                return Err(error(tok.span, msg));
            }
            _ => return Err(self.unexpected("an expression")),
        };
        self.bump();
        bounded(Expr {
            kind,
            span: tok.span,
        })
    }

    /// `'(' ')'`, `'(' expr ')'` or a parenthesized tuple. The span
    /// includes the parentheses.
    fn paren(&mut self) -> Result<Expr> {
        let start = self.bump().span;
        if self.eat(&TokenKind::RParen) {
            return bounded(Expr {
                kind: ExprKind::Tuple(Vec::new()),
                span: self.span_from(start),
            });
        }
        let (items, comma) = self.comma_list(Self::expr)?;
        if self.at(&TokenKind::For) {
            return self.paren_gen(items, comma, start);
        }
        let inner = self.tuple_or_single(items, comma, start)?;
        self.expect(&TokenKind::RParen)?;
        let span = self.span_from(start);
        self.widen_pauses(inner.span, span);
        bounded(Expr {
            kind: inner.kind,
            span,
        })
    }

    /// A parenthesized expression takes the parentheses' span, so a
    /// marker that covered it covers them too (M6c).
    fn widen_pauses(&mut self, inner: Span, outer: Span) {
        let covering = self.pauses.iter_mut().filter(|p| p.covers == inner);
        covering.for_each(|p| p.covers = outer);
    }

    /// A list display, or a comprehension when `for` follows the first
    /// item.
    fn list(&mut self) -> Result<Expr> {
        let start = self.bump().span;
        let items = if self.at(&TokenKind::RBracket) {
            Vec::new()
        } else {
            let first = self.expr()?;
            if self.at(&TokenKind::For) {
                return self.comp(Gather::List(Box::new(first)), &TokenKind::RBracket, start);
            }
            self.rest_of_list(first)?
        };
        self.expect(&TokenKind::RBracket)?;
        bounded(Expr {
            kind: ExprKind::List(items),
            span: self.span_from(start),
        })
    }

    /// `comp_for+ close`, after the opening bracket and what `gather`
    /// holds.
    fn comp(&mut self, gather: Gather, close: &TokenKind, start: Span) -> Result<Expr> {
        self.comp_closed(gather, Some(close), start)
    }

    /// A call's bare generator expression (D119): `comp_for+`, the `)`
    /// left to the call.
    fn comp_tail(&mut self, gather: Gather, start: Span) -> Result<Expr> {
        self.comp_closed(gather, None, start)
    }

    /// `( expr comp_for+ )`, after its first item: one item, with no
    /// comma, as CPython requires.
    fn paren_gen(&mut self, items: Vec<Expr>, comma: bool, start: Span) -> Result<Expr> {
        let Ok([item]) = <[Expr; 1]>::try_from(items) else {
            return Err(error(self.span_from(start), PARENS));
        };
        if comma {
            return Err(error(self.span_from(start), PARENS));
        }
        self.comp(Gather::Gen(Box::new(item)), &TokenKind::RParen, start)
    }

    fn comp_closed(
        &mut self,
        mut gather: Gather,
        close: Option<&TokenKind>,
        start: Span,
    ) -> Result<Expr> {
        let mut clauses = Vec::new();
        while self.at(&TokenKind::For) {
            clauses.push(self.comp_for()?);
        }
        if let Some(close) = close {
            self.expect(close)?;
        }
        comp::own_names(&mut gather, &mut clauses, start.start)?;
        bounded(Expr {
            kind: ExprKind::Comp { gather, clauses },
            span: self.span_from(start),
        })
    }

    /// `'for' target_list 'in' or_expr ('if' or_expr)*`
    fn comp_for(&mut self) -> Result<CompFor> {
        let start = self.bump().span;
        let target = self.for_target()?;
        self.expect(&TokenKind::In)?;
        let iter = self.nested(Self::or_expr)?;
        let mut ifs = Vec::new();
        while self.eat(&TokenKind::If) {
            ifs.push(self.nested(Self::or_expr)?);
        }
        Ok(CompFor {
            target,
            iter,
            ifs,
            span: self.span_from(start),
        })
    }

    /// `'{' '}'` (an empty dict), a dict display `{k: v, ...}`, a set
    /// display `{a, ...}`, or a dict or set comprehension (D116); the
    /// first item decides which, as in Python.
    fn braces(&mut self) -> Result<Expr> {
        let start = self.bump().span;
        let close = &TokenKind::RBrace;
        let kind = if self.at(close) {
            ExprKind::Dict(Vec::new())
        } else {
            let first = self.expr()?;
            if self.at(&TokenKind::For) {
                return self.comp(Gather::Set(Box::new(first)), close, start);
            }
            if !self.at(&TokenKind::Colon) {
                ExprKind::Set(self.rest_of_list(first)?)
            } else {
                self.bump();
                let value = self.expr()?;
                if self.at(&TokenKind::For) {
                    let gather = Gather::Dict(Box::new(first), Box::new(value));
                    return self.comp(gather, close, start);
                }
                ExprKind::Dict(self.dict_items(first, value)?)
            }
        };
        self.expect(&TokenKind::RBrace)?;
        bounded(Expr {
            kind,
            span: self.span_from(start),
        })
    }

    /// `(',' expr ':' expr)* [',']`, after the first `key: value`.
    fn dict_items(&mut self, key: Expr, value: Expr) -> Result<Vec<(Expr, Expr)>> {
        let mut items = vec![(key, value)];
        while self.eat(&TokenKind::Comma) && starts_expr(self.peek()) {
            let key = self.expr()?;
            self.expect(&TokenKind::Colon)?;
            items.push((key, self.expr()?));
        }
        Ok(items)
    }

    /// `(',' expr)* [',']`, after the first item.
    fn rest_of_list(&mut self, first: Expr) -> Result<Vec<Expr>> {
        let mut items = vec![first];
        while self.eat(&TokenKind::Comma) && starts_expr(self.peek()) {
            items.push(self.expr()?);
        }
        Ok(items)
    }
}

// Validation after parsing.

/// `self.x`'s field name `x`.
fn self_field(e: &Expr) -> Option<Ident> {
    let ExprKind::Attr { value, attr } = &e.kind else {
        return None;
    };
    matches!(&value.kind, ExprKind::Name(n) if n == "self").then(|| attr.clone())
}

/// An expression on the left of `=`, `op=` or in a `for` header, as a
/// `Target`: a name, a subscript, or a non-empty tuple of targets.
fn to_target(expr: Expr) -> Result<Target> {
    let span = expr.span;
    let kind = match expr.kind {
        ExprKind::Name(name) => {
            check_assignable(&name, span)?;
            TargetKind::Name(name)
        }
        ExprKind::Subscript { value, index } => TargetKind::Subscript { value, index },
        ExprKind::Slice { .. } => return Err(error(span, "slice assignment is not supported yet")),
        ExprKind::Tuple(items) if !items.is_empty() => {
            let targets = items.into_iter().map(to_target);
            TargetKind::Tuple(targets.collect::<Result<_>>()?)
        }
        ExprKind::Attr { value, attr } => TargetKind::Attr { value, attr },
        other => {
            let what = expr_noun(&other);
            return Err(error(span, format!("cannot assign to {what}")));
        }
    };
    Ok(Target { kind, span })
}

/// A name an assignment may bind: not a module's (`sys.argv`), which
/// the parser resolved to one dotted name.
fn check_assignable(name: &str, span: Span) -> Result<()> {
    if !name.contains('.') {
        return Ok(());
    }
    let msg = format!("assigning to a module's name (`{name}`) is not supported yet");
    Err(error(span, msg))
}

fn check_params(params: &[Param]) -> Result<()> {
    let mut seen_default = false;
    for (i, param) in params.iter().enumerate() {
        let name = &param.name.name;
        if params.iter().take(i).any(|p| &p.name.name == name) {
            let msg = format!("duplicate parameter `{name}`");
            return Err(error(param.name.span, msg));
        }
        if seen_default && param.default.is_none() {
            let msg = format!("parameter `{name}` without a default follows one with a default");
            return Err(error(param.span, msg));
        }
        seen_default |= param.default.is_some();
    }
    Ok(())
}

/// CPython's error for a generator expression beside another argument.
const PARENS: &str = "generator expression must be parenthesized";

fn check_args(args: &[Arg]) -> Result<()> {
    let mut after_keyword = args.iter().skip_while(|a| a.name.is_none());
    if let Some(arg) = after_keyword.find(|a| a.name.is_none()) {
        let msg = "positional argument follows keyword argument";
        return Err(error(arg.span, msg));
    }
    let keywords: Vec<&Ident> = args.iter().filter_map(|a| a.name.as_ref()).collect();
    for (i, keyword) in keywords.iter().enumerate() {
        if keywords.iter().take(i).any(|k| k.name == keyword.name) {
            let msg = format!("keyword argument `{}` repeated", keyword.name);
            return Err(error(keyword.span, msg));
        }
    }
    Ok(())
}

fn int_literal(value: u64, span: Span) -> Result<i64> {
    i64::try_from(value).map_err(|_| error(span, "integer literal is too large for int (i64)"))
}

// Node builders.

fn unary(op: UnaryOp, start: Span, operand: Expr) -> Result<Expr> {
    bounded(Expr {
        span: start.merge(operand.span),
        kind: ExprKind::Unary {
            op,
            operand: Box::new(operand),
        },
    })
}

fn binary(op: BinOp, left: Expr, right: Expr) -> Result<Expr> {
    bounded(Expr {
        span: left.span.merge(right.span),
        kind: ExprKind::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
    })
}

fn logical(op: LogicalOp, left: Expr, right: Expr) -> Result<Expr> {
    bounded(Expr {
        span: left.span.merge(right.span),
        kind: ExprKind::Logical {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
    })
}

fn error(span: Span, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        span,
        message: message.into(),
        kind: DiagKind::Parse,
    }
}

// Token classes.

fn or_op(kind: &TokenKind) -> Option<LogicalOp> {
    (kind == &TokenKind::Or).then_some(LogicalOp::Or)
}

fn and_op(kind: &TokenKind) -> Option<LogicalOp> {
    (kind == &TokenKind::And).then_some(LogicalOp::And)
}

fn sum_op(kind: &TokenKind) -> Option<BinOp> {
    match kind {
        TokenKind::Plus => Some(BinOp::Add),
        TokenKind::Minus => Some(BinOp::Sub),
        _ => None,
    }
}

fn term_op(kind: &TokenKind) -> Option<BinOp> {
    match kind {
        TokenKind::Star => Some(BinOp::Mul),
        TokenKind::Slash => Some(BinOp::Div),
        TokenKind::DoubleSlash => Some(BinOp::FloorDiv),
        TokenKind::Percent => Some(BinOp::Mod),
        _ => None,
    }
}

/// Single-token comparison operators; `not in` and `is` are handled by the
/// caller.
fn cmp_op(kind: &TokenKind) -> Option<CmpOp> {
    match kind {
        TokenKind::EqEq => Some(CmpOp::Eq),
        TokenKind::NotEq => Some(CmpOp::NotEq),
        TokenKind::Lt => Some(CmpOp::Lt),
        TokenKind::LtEq => Some(CmpOp::LtEq),
        TokenKind::Gt => Some(CmpOp::Gt),
        TokenKind::GtEq => Some(CmpOp::GtEq),
        TokenKind::In => Some(CmpOp::In),
        _ => None,
    }
}

fn aug_op(kind: &TokenKind) -> Option<BinOp> {
    match kind {
        TokenKind::PlusEq => Some(BinOp::Add),
        TokenKind::MinusEq => Some(BinOp::Sub),
        TokenKind::StarEq => Some(BinOp::Mul),
        TokenKind::SlashEq => Some(BinOp::Div),
        TokenKind::DoubleSlashEq => Some(BinOp::FloorDiv),
        TokenKind::PercentEq => Some(BinOp::Mod),
        TokenKind::DoubleStarEq => Some(BinOp::Pow),
        _ => None,
    }
}

fn starts_expr(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Name(_)
            | TokenKind::Int(_)
            | TokenKind::Float(_)
            | TokenKind::Str(_)
            | TokenKind::True
            | TokenKind::False
            | TokenKind::None
            | TokenKind::LParen
            | TokenKind::LBracket
            | TokenKind::LBrace
            | TokenKind::Minus
            | TokenKind::Plus
            | TokenKind::Not
            | TokenKind::Pause
    )
}

/// Tokens after `- INT` that bind to the literal before the minus does.
fn binds_tighter_than_unary(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::DoubleStar | TokenKind::LParen | TokenKind::LBracket | TokenKind::Dot
    )
}

fn is_layout(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent | TokenKind::EndMarker
    )
}

/// What an invalid assignment target is, for diagnostics.
fn expr_noun(kind: &ExprKind) -> &'static str {
    match kind {
        ExprKind::Int(_)
        | ExprKind::Float(_)
        | ExprKind::Str(_)
        | ExprKind::Bool(_)
        | ExprKind::None => "a literal",
        ExprKind::Tuple(_) => "an empty tuple",
        ExprKind::List(_) => "a list",
        ExprKind::Comp { gather, .. } => match gather {
            Gather::List(_) => "a list comprehension",
            Gather::Set(_) => "a set comprehension",
            Gather::Dict(..) => "a dict comprehension",
            Gather::Gen(_) => "a generator expression",
        },
        ExprKind::Dict(_) => "a dict display",
        ExprKind::Set(_) => "a set display",
        ExprKind::Call { .. } => "a function call",
        ExprKind::IfExp { .. } => "a conditional expression",
        ExprKind::Compare { .. } => "a comparison",
        ExprKind::Unary { .. } | ExprKind::Binary { .. } | ExprKind::Logical { .. } => {
            "an operator expression"
        }
        ExprKind::Name(_)
        | ExprKind::Subscript { .. }
        | ExprKind::Slice { .. }
        | ExprKind::Attr { .. } => "this expression",
    }
}

/// A token as a diagnostic names it.
fn describe(kind: &TokenKind) -> String {
    let text = match kind {
        TokenKind::Name(name) => return format!("name `{name}`"),
        TokenKind::Int(value) => return format!("integer `{value}`"),
        TokenKind::Float(value) => return format!("float `{value}`"),
        TokenKind::Str(_) => return "a string".to_string(),
        TokenKind::Newline => return "newline".to_string(),
        TokenKind::Indent => return "indent".to_string(),
        TokenKind::Dedent => return "dedent".to_string(),
        TokenKind::EndMarker => return "end of input".to_string(),
        TokenKind::Def => "def",
        TokenKind::Return => "return",
        TokenKind::If => "if",
        TokenKind::Elif => "elif",
        TokenKind::Else => "else",
        TokenKind::While => "while",
        TokenKind::For => "for",
        TokenKind::In => "in",
        TokenKind::Break => "break",
        TokenKind::Continue => "continue",
        TokenKind::Pass => "pass",
        TokenKind::And => "and",
        TokenKind::Or => "or",
        TokenKind::Not => "not",
        TokenKind::True => "True",
        TokenKind::False => "False",
        TokenKind::None => "None",
        TokenKind::Temper => "temper",
        TokenKind::Is => "is",
        TokenKind::Try => "try",
        TokenKind::Except => "except",
        TokenKind::Finally => "finally",
        TokenKind::Raise => "raise",
        TokenKind::Del => "del",
        TokenKind::Import => "import",
        TokenKind::From => "from",
        TokenKind::As => "as",
        TokenKind::With => "with",
        TokenKind::Pause => "pause",
        TokenKind::Class => "class",
        TokenKind::Pipe => "|",
        TokenKind::At => "@",
        TokenKind::Plus => "+",
        TokenKind::Minus => "-",
        TokenKind::Star => "*",
        TokenKind::Slash => "/",
        TokenKind::DoubleSlash => "//",
        TokenKind::Percent => "%",
        TokenKind::DoubleStar => "**",
        TokenKind::EqEq => "==",
        TokenKind::NotEq => "!=",
        TokenKind::Lt => "<",
        TokenKind::LtEq => "<=",
        TokenKind::Gt => ">",
        TokenKind::GtEq => ">=",
        TokenKind::Assign => "=",
        TokenKind::PlusEq => "+=",
        TokenKind::MinusEq => "-=",
        TokenKind::StarEq => "*=",
        TokenKind::SlashEq => "/=",
        TokenKind::DoubleSlashEq => "//=",
        TokenKind::PercentEq => "%=",
        TokenKind::DoubleStarEq => "**=",
        TokenKind::LParen => "(",
        TokenKind::RParen => ")",
        TokenKind::LBracket => "[",
        TokenKind::RBracket => "]",
        TokenKind::LBrace => "{",
        TokenKind::RBrace => "}",
        TokenKind::Comma => ",",
        TokenKind::Colon => ":",
        TokenKind::Dot => ".",
        TokenKind::Arrow => "->",
        TokenKind::Ellipsis => "...",
    };
    format!("`{text}`")
}

mod class;
mod exc_class;
mod imports;
mod init;
mod record;
mod stubs;
mod with;

/// The binding `name` from stub source `src`, its spans moved past any
/// test program's, as `pythrust` places each stub's.
#[cfg(test)]
pub(crate) fn test_binding(name: &str, src: &str) -> Binding {
    const BASE: usize = 1 << 20;
    let mut tokens = crate::lexer::lex(src).unwrap_or_else(|d| panic!("{d}"));
    for t in &mut tokens {
        t.span = Span {
            start: t.span.start + BASE,
            end: t.span.end + BASE,
        };
    }
    parse_stub(&tokens, name).unwrap_or_else(|d| panic!("{d}"))
}

#[cfg(test)]
mod tests;

/// Why `word` may not leave a `finally` block (D89; CPython 3.13 allows
/// it, swallowing a pending exception).
fn leaves_finally(word: &str) -> String {
    format!("`{word}` inside `finally` is not supported")
}
