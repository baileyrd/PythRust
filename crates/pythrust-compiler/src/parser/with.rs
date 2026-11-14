//! `with` statements: the scoped task executor (M6c) and `open(...)`
//! (#1374, D109-D113, D115).
//!
//! A file item is desugared here, once, so every later stage sees only
//! the try/finally of 1b-4 (D110). An item that is neither `open(...)`
//! nor a `ThreadPoolExecutor(...)` call is an object, entered and exited
//! by PEP 343's steps over the same try/finally (D153, `object_with`):
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
use crate::ast::{
    Arg, Block, Branch, Expr, ExprKind, Handler, Ident, Stmt, StmtKind, Target, TargetKind,
    TypeExpr, UnaryOp,
};
use crate::diag::Result;
use crate::span::Span;
use crate::token::{Token, TokenKind};

/// One `with` item: `value ['as' NAME]`, and the hidden temp of an
/// `open(...)` or object item, numbered left to right.
struct Item {
    value: Expr,
    binding: Option<Ident>,
    temp: Option<String>,
    object: bool,
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
        let file = !self.user_open && is_open(&value);
        let object = !file && !is_executor(&value);
        let temp = (file || object).then(|| {
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
            object,
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
        object,
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
    if object {
        return object_with(&temp, value, binding, body, start, span);
    }
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

/// `with#N = value`, `x = with#N.__enter__()`, then the body run so that
/// `__exit__` runs once on every way out: with the exception from a
/// handler (suppressing it when it returns true), else from `finally`
/// with `None`s. `exc_type`, `exc` and `tb` all get the one
/// `Exception | None` value, so the parameters have one type (D153).
///
/// Inference keys types by span, so each synthesized node of a type
/// shares a span with the others of that type only (`Marks`).
fn object_with(
    t: &str,
    value: Expr,
    binding: Option<Ident>,
    body: Block,
    start: Span,
    span: Span,
) -> Vec<Stmt> {
    let at = value.span;
    let m = Marks::at(start.start);
    let (exc, done, err) = (format!("{t}.exc"), format!("{t}.done"), format!("{t}.err"));
    let mut stmts = vec![assign(t, at, value)];
    let enter_at = binding.as_ref().map_or(Span::at(at.start), |b| b.span);
    let enter = method(t, "__enter__", Vec::new(), at, enter_at);
    stmts.push(match binding {
        Some(b) => assign(&b.name, b.span, enter),
        None => stmt(StmtKind::Expr(enter), enter_at),
    });
    let optional = TypeExpr {
        name: Ident {
            name: "Exception".to_string(),
            span: m.optional,
        },
        args: Vec::new(),
        optional: true,
        span: m.optional,
    };
    let none = Expr {
        kind: ExprKind::None,
        span: m.none,
    };
    let kind = StmtKind::AnnAssign {
        target: Ident {
            name: exc.clone(),
            span: m.optional,
        },
        annotation: optional,
        value: none,
    };
    stmts.push(stmt(kind, start));
    stmts.push(assign(&done, m.flag, flag(false, m.flag)));
    let exit = || {
        let args = (0..3).map(|_| name(&exc, m.optional)).collect();
        method(t, "__exit__", args, at, m.flag)
    };
    let suppress = not(exit(), m.flag);
    let reraise = stmt(StmtKind::Raise(name(&err, m.err)), start);
    let handler = Handler {
        classes: vec![Ident {
            name: "Exception".to_string(),
            span: start,
        }],
        binding: Some(Ident {
            name: err.clone(),
            span: m.err,
        }),
        body: vec![
            assign(&exc, m.optional, name(&err, m.err)),
            assign(&done, m.flag, flag(true, m.flag)),
            if_then(suppress, vec![reraise], start),
        ],
        span: start,
    };
    let not_done = not(name(&done, m.flag), m.flag);
    let exit_clean = stmt(StmtKind::Expr(exit()), m.flag);
    stmts.push(Stmt {
        kind: StmtKind::Try {
            body,
            handlers: vec![handler],
            finally: Some(vec![if_then(not_done, vec![exit_clean], start)]),
        },
        span,
    });
    stmts
}

/// The spans of an object `with`'s synthesized nodes of one type each,
/// zero-width inside the `with` keyword: `Exception | None`, `None`,
/// `bool` and `Exception`.
struct Marks {
    optional: Span,
    none: Span,
    flag: Span,
    err: Span,
}

impl Marks {
    fn at(start: usize) -> Marks {
        Marks {
            optional: Span::at(start),
            none: Span::at(start + 1),
            flag: Span::at(start + 2),
            err: Span::at(start + 3),
        }
    }
}

fn stmt(kind: StmtKind, span: Span) -> Stmt {
    Stmt { kind, span }
}

fn flag(value: bool, span: Span) -> Expr {
    Expr {
        kind: ExprKind::Bool(value),
        span,
    }
}

fn not(operand: Expr, span: Span) -> Expr {
    Expr {
        kind: ExprKind::Unary {
            op: UnaryOp::Not,
            operand: Box::new(operand),
        },
        span,
    }
}

fn if_then(test: Expr, body: Block, span: Span) -> Stmt {
    let branch = Branch { test, body, span };
    let kind = StmtKind::If {
        branch,
        elifs: Vec::new(),
        orelse: Vec::new(),
    };
    stmt(kind, span)
}

/// `recv.attr(args)`: the receiver at `at`, the call at `call`.
fn method(recv: &str, attr: &str, args: Vec<Expr>, at: Span, call: Span) -> Expr {
    let func = Expr {
        kind: ExprKind::Attr {
            value: Box::new(name(recv, at)),
            attr: Ident {
                name: attr.to_string(),
                span: at,
            },
        },
        span: at,
    };
    let args = args
        .into_iter()
        .map(|value| Arg {
            name: None,
            span: value.span,
            value,
        })
        .collect();
    Expr {
        kind: ExprKind::Call {
            func: Box::new(func),
            args,
        },
        span: call,
    }
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

/// A call of `ThreadPoolExecutor`, bare or qualified (M6c).
fn is_executor(value: &Expr) -> bool {
    let ExprKind::Call { func, .. } = &value.kind else {
        return false;
    };
    match &func.kind {
        ExprKind::Name(n) => n.rsplit('.').next() == Some("ThreadPoolExecutor"),
        ExprKind::Attr { attr, .. } => attr.name == "ThreadPoolExecutor",
        _ => false,
    }
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
