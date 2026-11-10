//! `with` statements: the scoped task executor (M6c) and `open(...)`
//! (#1374, D109-D113, D115).
//!
//! A file item is desugared here, once, so every later stage sees only
//! the try/finally of 1b-4 (D110):
//!
//! ```text
//! with#N = open(...)      # unspellable, so never shadowed or named
//! f = with#N              # when the item has `as f`
//! try:
//!     body
//! finally:
//!     with#N.close()
//! ```
//!
//! Closing the temp, not `f`, closes the file the item opened even when
//! the body rebinds `f`, as CPython's `__exit__` does.

use super::{error, Parser, MAX_DEPTH};
use crate::ast::{Block, Expr, ExprKind, Ident, Stmt, StmtKind, Target, TargetKind};
use crate::diag::Result;
use crate::span::Span;
use crate::token::{Token, TokenKind};

/// One `with` item: `value ['as' NAME]`, and the hidden temp of an
/// `open(...)` item, numbered left to right.
struct Item {
    value: Expr,
    binding: Option<Ident>,
    temp: Option<String>,
}

impl Parser<'_> {
    /// `'with' item (',' item)* ':' block`. Several items nest, entered
    /// left to right (D111).
    pub(super) fn with_stmt(&mut self) -> Result<Vec<Stmt>> {
        let start = self.bump().span;
        self.parenthesized_items()?;
        let mut items = vec![self.with_item()?];
        while self.eat(&TokenKind::Comma) {
            items.push(self.with_item()?);
            self.item_depth(&items)?;
        }
        self.expect(&TokenKind::Colon)?;
        // Each item past the first nests the body one level deeper.
        let extra = items.len() - 1;
        self.depth += extra;
        let body = self.block();
        self.depth -= extra;
        let body = body?;
        let span = self.span_from(start);
        let nested = items.into_iter().rev();
        Ok(nested.fold(body, |inner, item| with_one(item, inner, start, span)))
    }

    /// Every item nests the body one level, as a block does, so a flat
    /// line of items is bounded like nested blocks (MAX_DEPTH).
    fn item_depth(&self, items: &[Item]) -> Result<()> {
        if self.depth + items.len() <= MAX_DEPTH {
            return Ok(());
        }
        let at = items.last().map_or(self.last, |item| item.value.span);
        Err(error(at, "too deeply nested"))
    }

    /// `value ['as' NAME]`: only `open(...)` may omit the name (D112).
    fn with_item(&mut self) -> Result<Item> {
        let value = self.expr()?;
        let temp = (!self.user_open && is_open(&value)).then(|| {
            self.withs += 1;
            format!("with#{}", self.withs - 1)
        });
        let binding = match self.eat(&TokenKind::As) {
            true => Some(self.binding_name("a name to bind the value to")?),
            false => None,
        };
        if binding.is_none() && temp.is_none() {
            let msg = "`with` needs a name: write `with value as name:`";
            return Err(error(self.tok(0).span, msg));
        }
        Ok(Item {
            value,
            binding,
            temp,
        })
    }

    /// Python 3.10's `with (a as x, b as y):` is a named error (D113); a
    /// parenthesized value alone (`with (open(p)) as f:`) is not.
    fn parenthesized_items(&self) -> Result<()> {
        if !self.at(&TokenKind::LParen) {
            return Ok(());
        }
        let mut depth = 0usize;
        for ahead in 0.. {
            let tok = self.tok(ahead);
            match tok.kind {
                TokenKind::LParen | TokenKind::LBracket | TokenKind::LBrace => depth += 1,
                TokenKind::RParen | TokenKind::RBracket | TokenKind::RBrace => depth -= 1,
                TokenKind::As if depth == 1 => {
                    let msg = "parenthesized `with` items are not supported yet";
                    return Err(error(tok.span, msg));
                }
                TokenKind::Newline | TokenKind::EndMarker => break,
                _ => {}
            }
            if depth == 0 {
                break;
            }
        }
        Ok(())
    }
}

/// One item around `body`: an executor `with`, or a file's desugaring.
fn with_one(item: Item, body: Block, start: Span, span: Span) -> Vec<Stmt> {
    let Item {
        value,
        binding,
        temp,
    } = item;
    let Some(temp) = temp else {
        let Some(binding) = binding else {
            return body;
        };
        let kind = StmtKind::With {
            value,
            binding,
            body,
        };
        return vec![Stmt { kind, span }];
    };
    let at = value.span;
    let mut stmts = vec![assign(&temp, at, value)];
    if let Some(b) = binding {
        let read = name(&temp, at);
        stmts.push(assign(&b.name, b.span, read));
    }
    stmts.push(Stmt {
        kind: StmtKind::Try {
            body,
            handlers: Vec::new(),
            finally: Some(vec![close(&temp, start)]),
        },
        span,
    });
    stmts
}

/// Whether the program has a `def open` anywhere: a `def` replaces the
/// builtin in the whole module, before or after its `with` (D115).
pub(super) fn defines_open(tokens: &[Token]) -> bool {
    tokens.windows(2).any(|w| {
        w[0].kind == TokenKind::Def && matches!(&w[1].kind, TokenKind::Name(n) if n == "open")
    })
}

/// A direct call of `open` (D109).
fn is_open(value: &Expr) -> bool {
    let ExprKind::Call { func, .. } = &value.kind else {
        return false;
    };
    matches!(&func.kind, ExprKind::Name(n) if n == "open")
}

fn name(n: &str, span: Span) -> Expr {
    Expr {
        kind: ExprKind::Name(n.to_string()),
        span,
    }
}

fn assign(target: &str, span: Span, value: Expr) -> Stmt {
    let target = Target {
        kind: TargetKind::Name(target.to_string()),
        span,
    };
    Stmt {
        kind: StmtKind::Assign { target, value },
        span,
    }
}

/// `temp.close()`, at the `with` keyword, where CPython reports an
/// error leaving the block.
fn close(temp: &str, span: Span) -> Stmt {
    let func = Expr {
        kind: ExprKind::Attr {
            value: Box::new(name(temp, span)),
            attr: Ident {
                name: "close".to_string(),
                span,
            },
        },
        span,
    };
    let call = Expr {
        kind: ExprKind::Call {
            func: Box::new(func),
            args: Vec::new(),
        },
        span,
    };
    Stmt {
        kind: StmtKind::Expr(call),
        span,
    }
}
