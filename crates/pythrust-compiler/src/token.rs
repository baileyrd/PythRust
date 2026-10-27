//! Tokens: the lexer's output and the parser's input.
//!
//! Reserved-for-later words (`docs/grammar.md`, Lexical) have no kind:
//! the lexer rejects them.

use crate::span::Span;

/// One token with its source span.
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

/// Every token the phase-2 grammar uses.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Literals and names. Values are decoded: `1_000` is `Int(1000)`,
    // string escapes are resolved. `Int` is `u64` so the lexer can carry
    // `9223372036854775808`; the parser folds it under unary `-` to
    // `i64::MIN` and rejects any other literal above `i64::MAX`.
    Name(String),
    Int(u64),
    Float(f64),
    Str(String),

    // Keywords.
    Def,
    Return,
    If,
    Elif,
    Else,
    While,
    For,
    In,
    Break,
    Continue,
    Pass,
    And,
    Or,
    Not,
    True,
    False,
    None,
    Temper,
    /// `is`, only as `is None` / `is not None`.
    Is,
    Try,
    Except,
    Raise,
    /// `as`, only in `except Name as e`.
    As,

    // Arithmetic operators.
    Plus,
    Minus,
    Star,
    Slash,
    DoubleSlash,
    Percent,
    DoubleStar,

    // Comparison operators. `not in` is two tokens: `Not`, `In`.
    EqEq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,

    // Assignment. `Assign` is `=`; `EqEq` is `==`.
    Assign,
    PlusEq,
    MinusEq,
    StarEq,
    SlashEq,
    DoubleSlashEq,
    PercentEq,
    DoubleStarEq,

    // Delimiters.
    LParen,
    RParen,
    LBracket,
    RBracket,
    Comma,
    Colon,
    Dot,
    Arrow,
    /// `|`, only in `T | None` annotations.
    Pipe,

    // Layout.
    Newline,
    Indent,
    Dedent,
    EndMarker,
}
