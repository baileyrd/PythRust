//! Parser: `[Token]` to `ast::Module`. Thread B.

use crate::ast::Module;
use crate::diag::{DiagKind, Diagnostic, Result};
use crate::span::Span;
use crate::token::Token;

/// Parses a token stream ending in `EndMarker`. Stub: always returns an
/// error until thread B lands.
pub fn parse(_tokens: &[Token]) -> Result<Module> {
    Err(Diagnostic {
        span: Span { start: 0, end: 0 },
        message: "parser not implemented".to_string(),
        kind: DiagKind::Parse,
    })
}
