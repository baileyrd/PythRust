//! Source locations.

/// A half-open byte range `start..end` into the source text.
///
/// Invariant: `start <= end`. Producers (lexer, parser) uphold it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}
