//! The source files diagnostics render against: the program's file,
//! each Rust binding's stub (M6b) and each local module (D82).
//!
//! Every file's spans are offset by where it starts in one shared span
//! space, so a span alone says which file it belongs to, and `Span`
//! stays `{start, end}`.

use pythrust_compiler::{lexer, DiagKind, Diagnostic, Span, Token, Warning};

/// A source file and the offset its spans start at.
struct Source {
    path: String,
    text: String,
    base: usize,
    /// The module's dotted name: its own names render without it. A
    /// binding's stub has none.
    module: Option<String>,
}

/// The program's file and each other file (a binding's stub or a local
/// module), in the order added.
pub(crate) struct Sources {
    main: Source,
    others: Vec<Source>,
}

impl Sources {
    /// The program's file, whose spans start at 0. It is the module
    /// `__main__`, which qualifies a global a local module's free name
    /// would meet; it renders unqualified there too.
    pub(crate) fn new(path: &str, text: &str) -> Sources {
        let main = Source {
            path: path.to_string(),
            text: text.to_string(),
            base: 0,
            module: Some("__main__".to_string()),
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
        self.push(path, text, None)
    }

    /// Adds the local module `name`'s file, as [`Sources::add`] does a
    /// stub. Its own names render as written there: `x`, not `a.x` (D77).
    pub(crate) fn add_module(&mut self, path: &str, text: &str, name: &str) -> usize {
        self.push(path, text, Some(name.to_string()))
    }

    fn push(&mut self, path: &str, text: &str, module: Option<String>) -> usize {
        let last = self.others.last().unwrap_or(&self.main);
        let base = last.base + last.text.len() + 1;
        self.others.push(Source {
            path: path.to_string(),
            text: text.to_string(),
            base,
            module,
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

    /// Renders `diag` against the file it belongs to. A runtime error's
    /// message is the program's own text (an exception's), so it is
    /// shown as raised, as native code shows it (D127 names only the
    /// compiler's names).
    pub(crate) fn render(&self, diag: &Diagnostic) -> String {
        let (file, span) = self.locate(diag.span);
        let message = match diag.kind {
            DiagKind::Runtime => diag.message.clone(),
            _ => file.own_names(&diag.message),
        };
        let local = Diagnostic {
            span,
            message,
            kind: diag.kind,
        };
        crate::render(&file.path, &file.text, &local)
    }

    /// Renders `w` as [`Sources::render`] does an error.
    pub(crate) fn render_warning(&self, w: &Warning) -> String {
        let (file, span) = self.locate(w.span);
        let local = Warning {
            span,
            message: file.own_names(&w.message),
        };
        crate::render_warning(&file.path, &file.text, &local)
    }
}

impl Source {
    /// `message` with this module's own names unqualified: `` `a.x` ``
    /// becomes `` `x` `` in `a.pyr`. Other modules' names keep theirs.
    fn own_names(&self, message: &str) -> String {
        let Some(module) = &self.module else {
            return message.to_string();
        };
        let prefix = format!("`{module}.");
        let mut out = String::with_capacity(message.len());
        let mut rest = message;
        while let Some(at) = rest.find(&prefix) {
            let after = &rest[at + prefix.len()..];
            let own = after
                .find('`')
                .is_some_and(|end| end > 0 && !after[..end].contains(['.', ' ']));
            out.push_str(&rest[..at]);
            out.push('`');
            if !own {
                out.push_str(&prefix[1..]);
            }
            rest = after;
        }
        out.push_str(rest);
        out
    }
}

/// Lexes `text`, a file whose spans start at `base`, into that span
/// space; an error comes in it too.
pub(crate) fn lex_at(text: &str, base: usize) -> pythrust_compiler::Result<Vec<Token>> {
    let mut tokens = lexer::lex(text).map_err(|d| Diagnostic {
        span: shift(d.span, base),
        ..d
    })?;
    for t in &mut tokens {
        t.span = shift(t.span, base);
    }
    Ok(tokens)
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
    fn a_module_renders_its_own_names_unqualified() {
        let mut sources = Sources::new("main.pyr", "import a\n");
        let a = sources.add_module("a.pyr", "x = 1\n", "a");
        let d = Diagnostic {
            message: "`a.x` and `a.f` call `b.g`, `a.b.c`, `sys.argv`, `a.`".to_string(),
            ..diag(a, a + 1)
        };
        let out = sources.render(&d);
        let want = "`x` and `f` call `b.g`, `a.b.c`, `sys.argv`, `a.`";
        assert!(out.contains(want), "{out}");
        let in_main = Diagnostic {
            span: Span { start: 0, end: 1 },
            ..d
        };
        assert!(sources.render(&in_main).contains("`a.x` and `a.f`"));
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

    #[test]
    fn a_runtime_message_is_shown_as_raised() {
        let mut sources = Sources::new("main.pyr", "import a\n");
        let a = sources.add_module("a.pyr", "raise E\n", "a");
        let d = Diagnostic {
            message: "ValueError: `a.x`".to_string(),
            kind: pythrust_compiler::DiagKind::Runtime,
            ..diag(a, a + 1)
        };
        assert!(sources.render(&d).contains("ValueError: `a.x`"));
    }

    #[test]
    fn the_main_files_qualified_names_render_unqualified_there() {
        let mut sources = Sources::new("main.pyr", "len = 1\n");
        let a = sources.add_module("a.pyr", "x = 1\n", "a");
        let message = "`__main__.len` and `a.x`".to_string();
        let in_main = Diagnostic {
            message: message.clone(),
            ..diag(0, 1)
        };
        assert!(sources.render(&in_main).contains("`len` and `a.x`"));
        let in_a = Diagnostic {
            message,
            ..diag(a, a + 1)
        };
        assert!(sources.render(&in_a).contains("`__main__.len` and `x`"));
    }
}
