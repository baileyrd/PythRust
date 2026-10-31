//! `--rust name=path` (M6b, interim until M6d's manifest): loading each
//! Rust binding's stub, and the source files diagnostics render against
//! (`docs/research/rust-interop.md#resolution`).
//!
//! Every file's spans are offset by where it starts in one shared span
//! space, so a span alone says which file it belongs to.

use std::path::Path;

use pythrust_compiler::ast::Binding;
use pythrust_compiler::{infer, lexer, parser, stdlib, Diagnostic, Span, TokenKind, Warning};

/// A source file and the offset its spans start at.
struct Source {
    path: String,
    text: String,
    base: usize,
}

/// The program's file, then each stub's.
pub(crate) struct Sources {
    files: Vec<Source>,
}

impl Sources {
    /// The program's file, whose spans start at 0.
    pub(crate) fn new(path: &str, text: &str) -> Sources {
        let main = Source {
            path: path.to_string(),
            text: text.to_string(),
            base: 0,
        };
        Sources { files: vec![main] }
    }

    /// The program's file name.
    pub(crate) fn path(&self) -> &str {
        &self.files[0].path
    }

    /// Adds a file after the others, one past the last one's end so an
    /// end-of-file span stays in its own file; returns its base.
    fn add(&mut self, path: &str, text: &str) -> usize {
        let base = self.files.last().map_or(0, |f| f.base + f.text.len() + 1);
        self.files.push(Source {
            path: path.to_string(),
            text: text.to_string(),
            base,
        });
        base
    }

    /// The file `span` belongs to, and `span` within it.
    fn locate(&self, span: Span) -> (&Source, Span) {
        let file = self
            .files
            .iter()
            .rev()
            .find(|f| f.base <= span.start)
            .unwrap_or(&self.files[0]);
        (file, unshift(span, file.base))
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

/// Splits one `--rust` value, `name=path`.
pub(crate) fn split(arg: &str) -> Result<(&str, &str), String> {
    match arg.split_once('=') {
        Some((name, path)) if !name.is_empty() && !path.is_empty() => Ok((name, path)),
        _ => Err(format!("`--rust` takes `name=path`, got `{arg}`")),
    }
}

/// Checks `name` for a binding given after `earlier` ones: a plain name
/// that is no standard module, builtin, exception class, keyword or
/// reserved word, given once.
pub(crate) fn check_name(name: &str, earlier: &[&str]) -> Result<(), String> {
    let what = if !is_plain_name(name) {
        "is not a name"
    } else if stdlib::module(name).is_some() {
        "is a standard module"
    } else if infer::is_builtin(name) {
        "is a builtin"
    } else if earlier.contains(&name) {
        "is given twice"
    } else {
        return Ok(());
    };
    Err(format!("`--rust {name}=...`: `{name}` {what}"))
}

/// Whether `name` lexes as itself, one name: not a keyword, a reserved
/// word or anything else. ASCII only, as a Rust crate's name is.
fn is_plain_name(name: &str) -> bool {
    let ascii = name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_');
    let Ok(tokens) = lexer::lex(name) else {
        return false;
    };
    let one = matches!(tokens.first(), Some(t) if t.kind == TokenKind::Name(name.to_string()));
    let rest = tokens
        .iter()
        .skip(1)
        .all(|t| matches!(t.kind, TokenKind::Newline | TokenKind::EndMarker));
    ascii && one && rest
}

/// Reads and parses the stub `dir/name.pyr` of each `(name, dir)`,
/// adding it to `sources`. An error comes rendered.
pub(crate) fn load(rust: &[(&str, &str)], sources: &mut Sources) -> Result<Vec<Binding>, String> {
    let mut names: Vec<&str> = Vec::new();
    let mut bindings = Vec::new();
    for &(name, dir) in rust {
        check_name(name, &names).map_err(|m| format!("pythrust: {m}\n"))?;
        names.push(name);
        let stub = stub_text(name, Path::new(dir))?;
        let path = Path::new(dir).join(format!("{name}.pyr"));
        let base = sources.add(&path.to_string_lossy(), &stub);
        let parsed = parse(&stub, name, base).map_err(|d| sources.render(&d))?;
        bindings.push(parsed);
    }
    Ok(bindings)
}

/// The text of `dir`'s stub for `name`.
fn stub_text(name: &str, dir: &Path) -> Result<String, String> {
    if !dir.is_dir() {
        let shown = dir.display();
        return Err(format!(
            "pythrust: binding `{name}`: no directory `{shown}`\n"
        ));
    }
    let stub = dir.join(format!("{name}.pyr"));
    std::fs::read_to_string(&stub).map_err(|e| {
        let shown = stub.display();
        format!("pythrust: binding `{name}`: cannot read its stub `{shown}`: {e}\n")
    })
}

/// Lexes and parses the stub `text` of `name`, its spans offset by `base`.
fn parse(text: &str, name: &str, base: usize) -> pythrust_compiler::Result<Binding> {
    let shift_diag = |d: Diagnostic| Diagnostic {
        span: shift(d.span, base),
        ..d
    };
    let mut tokens = lexer::lex(text).map_err(shift_diag)?;
    for t in &mut tokens {
        t.span = shift(t.span, base);
    }
    parser::parse_stub(&tokens, name)
}

fn shift(span: Span, by: usize) -> Span {
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

    #[test]
    fn split_needs_a_name_and_a_path() {
        assert_eq!(split("fastfs=../fastfs"), Ok(("fastfs", "../fastfs")));
        assert_eq!(split("a=b=c"), Ok(("a", "b=c")));
        for bad in ["fastfs", "=p", "n=", ""] {
            assert!(split(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn names_that_mean_something_else_are_refused() {
        assert_eq!(check_name("fastfs", &[]), Ok(()));
        assert_eq!(check_name("fast_fs2", &["other"]), Ok(()));
        let cases = [
            ("os", "is a standard module"),
            ("hashlib", "is a standard module"),
            ("len", "is a builtin"),
            ("print", "is a builtin"),
            ("ValueError", "is a builtin"),
            ("def", "is not a name"),
            ("pause", "is not a name"),
            ("shared", "is not a name"),
            ("2fast", "is not a name"),
            ("fast-fs", "is not a name"),
            ("fäst", "is not a name"),
            ("a b", "is not a name"),
        ];
        for (name, why) in cases {
            let err = check_name(name, &[]).err().unwrap_or_default();
            assert!(err.ends_with(why), "{name}: {err}");
        }
        let twice = check_name("fastfs", &["fastfs"]).err().unwrap_or_default();
        assert!(twice.ends_with("is given twice"), "{twice}");
    }

    #[test]
    fn spans_render_against_their_own_file() {
        let mut sources = Sources::new("main.pyr", "x = 1\n");
        let stub = "temper def f() -> int:\n    ...\noops\n";
        let base = sources.add("fastfs/fastfs.pyr", stub);
        assert_eq!(base, 7);
        let Err(diag) = parse(stub, "fastfs", base) else {
            panic!("`oops` is not a signature");
        };
        let out = sources.render(&diag);
        assert!(
            out.starts_with("fastfs/fastfs.pyr:3:1: parse error"),
            "{out}"
        );
        let main = Diagnostic {
            span: Span { start: 4, end: 5 },
            ..diag
        };
        assert!(sources.render(&main).starts_with("main.pyr:1:5:"));
    }

    #[test]
    fn missing_directory_and_stub_are_named() {
        let mut sources = Sources::new("m.pyr", "");
        let gone = load(&[("fastfs", "/no/such/dir")], &mut sources);
        let err = gone.err().unwrap_or_default();
        assert!(err.contains("binding `fastfs`: no directory"), "{err}");
        let tmp = std::env::temp_dir();
        let dir = tmp.to_string_lossy();
        let missing = load(&[("no_such_stub_xyz", &dir)], &mut sources);
        let err = missing.err().unwrap_or_default();
        assert!(err.contains("cannot read its stub"), "{err}");
    }
}
