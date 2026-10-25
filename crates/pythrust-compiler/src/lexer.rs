//! Lexer: source text to `Vec<Token>`. Thread A.

use crate::diag::{DiagKind, Diagnostic, Result};
use crate::span::Span;
use crate::token::Token;

/// Tokenizes `src`. Stub: always returns an error until thread A lands.
pub fn lex(_src: &str) -> Result<Vec<Token>> {
    Err(Diagnostic {
        span: Span { start: 0, end: 0 },
        message: "lexer not implemented".to_string(),
        kind: DiagKind::Lex,
    })
}
