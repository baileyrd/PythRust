//! Source locations.

/// A half-open byte range `start..end` into the source text.
///
/// Invariant: `start <= end`. Producers (lexer, parser) uphold it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    /// The smallest span covering both `self` and `other`.
    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span(start: usize, end: usize) -> Span {
        Span { start, end }
    }

    #[test]
    fn merge_covers_both_in_either_order() {
        assert_eq!(span(2, 4).merge(span(7, 9)), span(2, 9));
        assert_eq!(span(7, 9).merge(span(2, 4)), span(2, 9));
    }

    #[test]
    fn merge_with_nested_or_empty_span() {
        assert_eq!(span(0, 10).merge(span(3, 5)), span(0, 10));
        assert_eq!(span(5, 5).merge(span(5, 5)), span(5, 5));
    }
}
