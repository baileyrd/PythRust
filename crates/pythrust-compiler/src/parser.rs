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
    Arg, BinOp, Block, Branch, CmpOp, Expr, ExprKind, FuncDef, Handler, Ident, LogicalOp, Mode,
    Module, Param, Stmt, StmtKind, Target, TargetKind, TypeExpr, UnaryOp,
};
use crate::diag::{DiagKind, Diagnostic, Result};
use crate::span::Span;
use crate::token::{Token, TokenKind};

/// Nesting limit for blocks, parentheses and unary chains. Hostile input
/// gets a diagnostic instead of a stack overflow.
const MAX_DEPTH: usize = 100;

/// Parses a token stream that ends in exactly one `EndMarker`.
pub fn parse(tokens: &[Token]) -> Result<Module> {
    Parser::new(tokens)?.module()
}

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
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token]) -> Result<Self> {
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

    fn unexpected(&self, expected: &str) -> Diagnostic {
        let tok = self.tok(0);
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
        let outer = self.in_loop;
        self.in_loop = true;
        let out = rule(self);
        self.in_loop = outer;
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
        while !self.at(&TokenKind::EndMarker) {
            body.push(self.stmt()?);
        }
        Ok(Module {
            body,
            span: start.merge(self.eof.span),
        })
    }

    fn stmt(&mut self) -> Result<Stmt> {
        match self.peek() {
            TokenKind::Def | TokenKind::Temper => self.func_def(),
            TokenKind::If => self.if_stmt(),
            TokenKind::While => self.while_stmt(),
            TokenKind::For => self.for_stmt(),
            TokenKind::Try => self.try_stmt(),
            _ => {
                let stmt = self.simple_stmt()?;
                self.expect(&TokenKind::Newline)?;
                Ok(stmt)
            }
        }
    }

    fn block(&mut self) -> Result<Block> {
        self.expect(&TokenKind::Newline)?;
        self.expect(&TokenKind::Indent)?;
        self.nested(|p| {
            let mut body = vec![p.block_stmt()?];
            while !p.eat(&TokenKind::Dedent) {
                body.push(p.block_stmt()?);
            }
            Ok(body)
        })
    }

    fn block_stmt(&mut self) -> Result<Stmt> {
        if matches!(self.peek(), TokenKind::Def | TokenKind::Temper) {
            let span = self.tok(0).span;
            return Err(error(span, "`def` is only allowed at module top level"));
        }
        self.stmt()
    }

    fn simple_stmt(&mut self) -> Result<Stmt> {
        let start = self.tok(0).span;
        let kind = match self.peek() {
            TokenKind::Return => self.return_stmt()?,
            TokenKind::Raise => self.raise_stmt()?,
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

    fn loop_jump(&mut self, kind: StmtKind, word: &str) -> Result<StmtKind> {
        let span = self.bump().span;
        if !self.in_loop {
            return Err(error(span, format!("`{word}` outside loop")));
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
        let ExprKind::Name(name) = lhs.kind else {
            return Err(error(lhs.span, "only a single name can be annotated"));
        };
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
        let start = self.tok(0).span;
        let mode = if self.eat(&TokenKind::Temper) {
            Mode::Tempered
        } else {
            Mode::Loose
        };
        self.expect(&TokenKind::Def)?;
        let name = self.expect_name("a function name")?;
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
        self.expect(&TokenKind::Colon)?;
        self.in_func = true;
        let body = self.block();
        self.in_func = false;
        let span = self.span_from(start);
        let def = FuncDef {
            mode,
            name,
            params,
            returns,
            body: body?,
            span,
        };
        Ok(Stmt {
            kind: StmtKind::FuncDef(def),
            span,
        })
    }

    fn param(&mut self) -> Result<Param> {
        let name = self.expect_name("a parameter name")?;
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
            span: self.span_from(name.span),
            name,
            annotation,
            default,
        })
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

    /// `'try' ':' block handler+`. `else` and `finally` clauses are later.
    fn try_stmt(&mut self) -> Result<Stmt> {
        let start = self.bump().span;
        self.expect(&TokenKind::Colon)?;
        let body = self.block()?;
        let mut handlers = vec![self.handler()?];
        while self.at(&TokenKind::Except) {
            handlers.push(self.handler()?);
        }
        if self.at(&TokenKind::Else) {
            let span = self.tok(0).span;
            return Err(error(span, "`else` on `try` is not supported yet"));
        }
        Ok(Stmt {
            kind: StmtKind::Try { body, handlers },
            span: self.span_from(start),
        })
    }

    /// `'except' NAME ['as' NAME] ':' block`
    fn handler(&mut self) -> Result<Handler> {
        let start = self.expect(&TokenKind::Except)?.span;
        if self.at(&TokenKind::Colon) {
            let msg = "bare `except:` is not supported yet; name the exception, \
                       as in `except ValueError:`";
            return Err(error(self.tok(0).span, msg));
        }
        let exception = self.expect_name("an exception name, such as `ValueError`")?;
        let binding = if self.eat(&TokenKind::As) {
            Some(self.expect_name("a name to bind the exception to")?)
        } else {
            None
        };
        self.expect(&TokenKind::Colon)?;
        let body = self.block()?;
        Ok(Handler {
            exception,
            binding,
            body,
            span: self.span_from(start),
        })
    }

    /// `target_list` before `in`. Parsed at `postfix` level so the `in`
    /// is not read as a comparison.
    fn for_target(&mut self) -> Result<Target> {
        let start = self.tok(0).span;
        let (items, comma) = self.comma_list(Self::postfix)?;
        to_target(self.tuple_or_single(items, comma, start))
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
            _ => return Err(self.unexpected("a type")),
        };
        self.bump();
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
        Ok(self.tuple_or_single(items, comma, start))
    }

    fn tuple_or_single(&self, items: Vec<Expr>, comma: bool, start: Span) -> Expr {
        let items = match <[Expr; 1]>::try_from(items) {
            Ok([single]) if !comma => return single,
            Ok(one) => Vec::from(one),
            Err(many) => many,
        };
        Expr {
            kind: ExprKind::Tuple(items),
            span: self.span_from(start),
        }
    }

    fn expr(&mut self) -> Result<Expr> {
        self.nested(Self::conditional)
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
        Ok(Expr { kind, span })
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
        Ok(unary(UnaryOp::Not, start, operand))
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
        Ok(Expr { kind, span })
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
        build: fn(Op, Expr, Expr) -> Expr,
    ) -> Result<Expr> {
        let mut left = operand(self)?;
        while let Some(op) = op_of(self.peek()) {
            self.bump();
            let right = operand(self)?;
            left = build(op, left, right);
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
        Ok(unary(op, start, operand))
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
        Ok(binary(BinOp::Pow, base, exponent))
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
            self.comma_list(Self::arg)?.0
        };
        self.expect(&TokenKind::RParen)?;
        check_args(&args)?;
        Ok(Expr {
            span: self.span_from(func.span),
            kind: ExprKind::Call {
                func: Box::new(func),
                args,
            },
        })
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
        Ok(Arg {
            name,
            value,
            span: self.span_from(start),
        })
    }

    fn subscript(&mut self, value: Expr) -> Result<Expr> {
        self.bump();
        let index = self.expr()?;
        self.expect(&TokenKind::RBracket)?;
        Ok(Expr {
            span: self.span_from(value.span),
            kind: ExprKind::Subscript {
                value: Box::new(value),
                index: Box::new(index),
            },
        })
    }

    fn attr(&mut self, value: Expr) -> Result<Expr> {
        self.bump();
        let attr = self.expect_name("an attribute name")?;
        Ok(Expr {
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
            TokenKind::Name(name) => ExprKind::Name(name.clone()),
            TokenKind::Int(value) => ExprKind::Int(int_literal(*value, tok.span)?),
            TokenKind::Float(value) => ExprKind::Float(*value),
            TokenKind::Str(value) => ExprKind::Str(value.clone()),
            TokenKind::True => ExprKind::Bool(true),
            TokenKind::False => ExprKind::Bool(false),
            TokenKind::None => ExprKind::None,
            TokenKind::LParen => return self.paren(),
            TokenKind::LBracket => return self.list(),
            _ => return Err(self.unexpected("an expression")),
        };
        self.bump();
        Ok(Expr {
            kind,
            span: tok.span,
        })
    }

    /// `'(' ')'`, `'(' expr ')'` or a parenthesized tuple. The span
    /// includes the parentheses.
    fn paren(&mut self) -> Result<Expr> {
        let start = self.bump().span;
        if self.eat(&TokenKind::RParen) {
            return Ok(Expr {
                kind: ExprKind::Tuple(Vec::new()),
                span: self.span_from(start),
            });
        }
        let inner = self.expr_list()?;
        self.expect(&TokenKind::RParen)?;
        Ok(Expr {
            kind: inner.kind,
            span: self.span_from(start),
        })
    }

    fn list(&mut self) -> Result<Expr> {
        let start = self.bump().span;
        let items = if self.at(&TokenKind::RBracket) {
            Vec::new()
        } else {
            self.comma_list(Self::expr)?.0
        };
        self.expect(&TokenKind::RBracket)?;
        Ok(Expr {
            kind: ExprKind::List(items),
            span: self.span_from(start),
        })
    }
}

// Validation after parsing.

/// An expression on the left of `=`, `op=` or in a `for` header, as a
/// `Target`: a name, a subscript, or a non-empty tuple of targets.
fn to_target(expr: Expr) -> Result<Target> {
    let span = expr.span;
    let kind = match expr.kind {
        ExprKind::Name(name) => TargetKind::Name(name),
        ExprKind::Subscript { value, index } => TargetKind::Subscript { value, index },
        ExprKind::Tuple(items) if !items.is_empty() => {
            let targets = items.into_iter().map(to_target);
            TargetKind::Tuple(targets.collect::<Result<_>>()?)
        }
        ExprKind::Attr { .. } => {
            return Err(error(span, "attribute assignment is not supported yet"));
        }
        other => {
            let what = expr_noun(&other);
            return Err(error(span, format!("cannot assign to {what}")));
        }
    };
    Ok(Target { kind, span })
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

fn unary(op: UnaryOp, start: Span, operand: Expr) -> Expr {
    Expr {
        span: start.merge(operand.span),
        kind: ExprKind::Unary {
            op,
            operand: Box::new(operand),
        },
    }
}

fn binary(op: BinOp, left: Expr, right: Expr) -> Expr {
    Expr {
        span: left.span.merge(right.span),
        kind: ExprKind::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
    }
}

fn logical(op: LogicalOp, left: Expr, right: Expr) -> Expr {
    Expr {
        span: left.span.merge(right.span),
        kind: ExprKind::Logical {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
    }
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
            | TokenKind::Minus
            | TokenKind::Plus
            | TokenKind::Not
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
        ExprKind::Call { .. } => "a function call",
        ExprKind::IfExp { .. } => "a conditional expression",
        ExprKind::Compare { .. } => "a comparison",
        ExprKind::Unary { .. } | ExprKind::Binary { .. } | ExprKind::Logical { .. } => {
            "an operator expression"
        }
        ExprKind::Name(_) | ExprKind::Subscript { .. } | ExprKind::Attr { .. } => "this expression",
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
        TokenKind::Raise => "raise",
        TokenKind::As => "as",
        TokenKind::Pipe => "|",
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
        TokenKind::Comma => ",",
        TokenKind::Colon => ":",
        TokenKind::Dot => ".",
        TokenKind::Arrow => "->",
    };
    format!("`{text}`")
}

#[cfg(test)]
mod tests;
