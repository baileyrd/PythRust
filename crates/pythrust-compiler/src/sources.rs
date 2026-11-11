//! The source files diagnostics render against: the program's file,
//! each Rust binding's stub (M6b) and each local module (D82).
//!
//! Every file's spans are offset by where it starts in one shared span
//! space, so a span alone says which file it belongs to, and `Span`
//! stays `{start, end}`.

use pythrust_compiler::{Diagnostic, Span, Warning};

/// A source file and the offset its spans start at.
struct Source {
    path: String,
    text: String,
    base: usize,
}

/// The program's file and each other file (a binding's stub or a local
/// module), in the order added.
pub(crate) struct Sources {
    main: Source,
    others: Vec<Source>,
}

impl Sources {
    /// The program's file, whose spans start at 0.
    pub(crate) fn new(path: &str, text: &str) -> Sources {
        let main = Source {
            path: path.to_string(),
            text: text.to_string(),
            base: 0,
        };
        Sources {
            main,
            others: Vec::new(),
        }
    }

    /// The program's file name.
    pub(crate) fn path(&self) -> &str {
        &self.main.path
    }

    /// Adds a file after the others, one past the last one's end so an
    /// end-of-file span stays in its own file; returns its base, which
    /// its spans are shifted by.
    pub(crate) fn add(&mut self, path: &str, text: &str) -> usize {
        let last = self.others.last().unwrap_or(&self.main);
        let base = last.base + last.text.len() + 1;
        self.others.push(Source {
            path: path.to_string(),
            text: text.to_string(),
            base,
        });
        base
    }

    /// The file `span` belongs to, and `span` within it.
    fn locate(&self, span: Span) -> (&Source, Span) {
        let file = self
            .others
            .iter()
            .rev()
            .find(|f| f.base <= span.start)
            .unwrap_or(&self.main);
        (file, unshift(span, file.base))
    }

    /// `file:line` of `span`, as a shim's comment names its stub.
    pub(crate) fn place(&self, span: Span) -> String {
        let (file, span) = self.locate(span);
        let line = crate::line_col(&file.text, span.start).line;
        format!("{}:{line}", file.path)
    }

    /// Renders `diag` against the file it belongs to.
    pub(crate) fn render(&self, diag: &Diagnostic) -> String {
        let (file, span) = self.locate(diag.span);
        let local = Diagnostic {
            span,
            ..diag.clone()
        };
        crate::render(&file.path, &file.text, &local)
    }

    /// Renders `w` as [`Sources::render`] does an error.
    pub(crate) fn render_warning(&self, w: &Warning) -> String {
        let (file, span) = self.locate(w.span);
        let local = Warning {
            span,
            message: w.message.clone(),
        };
        crate::render_warning(&file.path, &file.text, &local)
    }
}

/// `span` moved `by` bytes on, into the file whose base is `by`.
pub(crate) fn shift(span: Span, by: usize) -> Span {
    Span {
        start: span.start + by,
        end: span.end + by,
    }
}

fn unshift(span: Span, by: usize) -> Span {
    Span {
        start: span.start.saturating_sub(by),
        end: span.end.saturating_sub(by),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diag(start: usize, end: usize) -> Diagnostic {
        Diagnostic {
            span: Span { start, end },
            message: "boom".to_string(),
            kind: pythrust_compiler::DiagKind::Type,
        }
    }

    #[test]
    fn each_module_renders_against_its_own_file() {
        let mut sources = Sources::new("main.pyr", "import a\nimport b\n");
        let a = sources.add("a.pyr", "x = 1\ny = 2\n");
        let b = sources.add("pkg/b.pyr", "z = 3\n");
        assert_eq!((a, b), (19, 32));
        let at = |span| sources.render(&diag(span, span + 1));
        assert!(at(9).starts_with("main.pyr:2:1:"), "{}", at(9));
        assert!(at(a + 6).starts_with("a.pyr:2:1:"), "{}", at(a + 6));
        assert!(at(b + 4).starts_with("pkg/b.pyr:1:5:"), "{}", at(b + 4));
        assert_eq!(
            sources.place(Span {
                start: a + 6,
                end: a + 7
            }),
            "a.pyr:2"
        );
    }

    #[test]
    fn a_span_at_a_file_end_stays_in_that_file() {
        let mut sources = Sources::new("main.pyr", "x\n");
        let a = sources.add("a.pyr", "y\n");
        let b = sources.add("b.pyr", "");
        assert!(sources.render(&diag(2, 2)).starts_with("main.pyr:2:1:"));
        assert!(sources.render(&diag(a, a)).starts_with("a.pyr:1:1:"));
        assert!(sources
            .render(&diag(a + 2, a + 2))
            .starts_with("a.pyr:2:1:"));
        assert!(sources.render(&diag(b, b)).starts_with("b.pyr:1:1:"));
    }

    #[test]
    fn warnings_render_against_their_own_file() {
        let mut sources = Sources::new("main.pyr", "x = 1\n");
        let a = sources.add("a.pyr", "y = 2\n");
        let w = Warning {
            span: Span {
                start: a + 4,
                end: a + 5,
            },
            message: "hm".to_string(),
        };
        let out = sources.render_warning(&w);
        assert!(out.starts_with("a.pyr:1:5:"), "{out}");
    }
}
