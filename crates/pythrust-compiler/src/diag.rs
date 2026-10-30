//! Diagnostics: the single error type every stage returns.

use std::fmt;

use crate::span::Span;

/// The stage that produced a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagKind {
    Lex,
    Parse,
    Type,
    /// The program is valid, but the native backend cannot compile it.
    Backend,
    Runtime,
}

/// An error tied to a source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
    pub kind: DiagKind,
}

/// A note on a valid program, which compiles and runs as written: a
/// lint. Rendered, never returned as an error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    pub span: Span,
    pub message: String,
}

/// Result alias used across the compiler.
pub type Result<T> = std::result::Result<T, Diagnostic>;

impl fmt::Display for DiagKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            DiagKind::Lex => "lex",
            DiagKind::Parse => "parse",
            DiagKind::Type => "type",
            DiagKind::Backend => "backend",
            DiagKind::Runtime => "runtime",
        };
        f.write_str(name)
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Span { start, end } = self.span;
        write!(f, "{} error at {start}..{end}: {}", self.kind, self.message)
    }
}

impl std::error::Error for Diagnostic {}

impl fmt::Display for Warning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Span { start, end } = self.span;
        write!(f, "warning at {start}..{end}: {}", self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names_kind_span_and_message() {
        let d = Diagnostic {
            span: Span { start: 3, end: 7 },
            message: "tab in indentation".to_string(),
            kind: DiagKind::Lex,
        };
        assert_eq!(d.to_string(), "lex error at 3..7: tab in indentation");
    }

    #[test]
    fn a_warning_displays_as_one() {
        let w = Warning {
            span: Span { start: 1, end: 2 },
            message: "vacuous".to_string(),
        };
        assert_eq!(w.to_string(), "warning at 1..2: vacuous");
    }
}
