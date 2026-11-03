//! A program's project (M6d, `docs/research/package-tool.md`): the
//! `pythrust.toml` found by walking up from the program's directory,
//! read only when the program imports a module that is not standard,
//! and the Rust bindings it names for those imports.

use std::path::{Component, Path, PathBuf};

use pythrust_compiler::ast::Ident;
use pythrust_compiler::manifest::{self, toml, Manifest};
use pythrust_compiler::{lexer, stdlib, DiagKind, Diagnostic, Token, TokenKind};

use crate::bindings;
use crate::cargo::Crate;

/// The bindings a program imports and the manifest that names them.
pub(crate) struct Project {
    /// With absolute directories, in import order.
    pub(crate) crates: Vec<Crate>,
    /// `None` when every import is standard: then none is looked for.
    pub(crate) manifest: Option<PathBuf>,
}

/// The project of the program at `path`, whose text is `src`. An error
/// comes rendered.
pub(crate) fn bindings(path: &str, src: &str) -> Result<Project, String> {
    let imports = foreign_imports(src);
    let Some(first) = imports.first() else {
        return Ok(Project {
            crates: Vec::new(),
            manifest: None,
        });
    };
    let start = normalize(&absolute(&program_dir(path))?);
    let Some(file) = find(&start)? else {
        let why = format!(
            "no pythrust.toml names it (none in {} or any directory above it)",
            start.display()
        );
        let message = stdlib::not_standard(&first.name, &why);
        return Err(render_import(path, src, first, message));
    };
    let manifest = read(&file)?;
    let dir = file.parent().unwrap_or(Path::new("."));
    let crates = imports
        .iter()
        .map(|import| {
            named(&manifest, dir, import).map_err(|m| render_import(path, src, import, m))
        })
        .collect::<Result<_, _>>()?;
    Ok(Project {
        crates,
        manifest: Some(file),
    })
}

/// The program's imports that are not standard modules, each once, in
/// order. A file that does not lex has none here: the front end reports
/// its error.
fn foreign_imports(src: &str) -> Vec<Ident> {
    let Ok(tokens) = lexer::lex(src) else {
        return Vec::new();
    };
    let mut names: Vec<Ident> = Vec::new();
    let mut in_import = false;
    for (i, t) in tokens.iter().enumerate() {
        match &t.kind {
            TokenKind::Import => in_import = true,
            TokenKind::Newline => in_import = false,
            TokenKind::Name(name) if in_import && after_import_or_comma(&tokens, i) => {
                let known = stdlib::module(name).is_some() || names.iter().any(|n| &n.name == name);
                if !known {
                    names.push(Ident {
                        name: name.clone(),
                        span: t.span,
                    });
                }
            }
            _ if !in_import && t.kind != TokenKind::Newline => break,
            _ => {}
        }
    }
    names
}

/// Whether the token before `i` is `import` or `,`: a module name, not
/// the `futures` of `concurrent.futures`.
fn after_import_or_comma(tokens: &[Token], i: usize) -> bool {
    i > 0 && matches!(tokens[i - 1].kind, TokenKind::Import | TokenKind::Comma)
}

/// The directory of the program at `path`: `.` for a bare file name.
fn program_dir(path: &str) -> PathBuf {
    match Path::new(path).parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir.to_path_buf(),
        _ => PathBuf::from("."),
    }
}

fn absolute(path: &Path) -> Result<PathBuf, String> {
    std::path::absolute(path)
        .map_err(|e| format!("pythrust: cannot resolve {}: {e}\n", path.display()))
}

/// `path` with `.` dropped and each `..` taking off the name before it,
/// as text: the directories above a program are those of its path as
/// written, not of the current directory. Symlinks are not resolved.
fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// The first `pythrust.toml` in `start` or a directory above it. A
/// manifest that cannot be checked for (say, a directory it cannot
/// search) is an error, not a reason to look further up.
fn find(start: &Path) -> Result<Option<PathBuf>, String> {
    for dir in start.ancestors() {
        let file = dir.join(manifest::FILE);
        let exists = file
            .try_exists()
            .map_err(|e| format!("pythrust: cannot look for {}: {e}\n", file.display()))?;
        if exists {
            return Ok(Some(file));
        }
    }
    Ok(None)
}

/// Reads and checks the manifest `file`, and the name of each binding
/// it gives.
fn read(file: &Path) -> Result<Manifest, String> {
    let shown = file.display().to_string();
    let bytes = std::fs::read(file).map_err(|e| format!("pythrust: cannot read {shown}: {e}\n"))?;
    let text = String::from_utf8_lossy(&bytes);
    let render =
        |e: toml::Error| crate::render_at(&shown, &text, e.span, "manifest error", &e.message);
    let manifest = manifest::parse(&bytes).map_err(render)?;
    for dep in &manifest.rust {
        bindings::check_name(&dep.name.name).map_err(|message| {
            render(toml::Error {
                span: dep.name.span,
                message,
            })
        })?;
    }
    Ok(manifest)
}

/// The binding `import` names in `manifest`, whose directory is `dir`.
fn named(manifest: &Manifest, dir: &Path, import: &Ident) -> Result<Crate, String> {
    let Some(dep) = manifest.rust.iter().find(|d| d.name.name == import.name) else {
        let why = format!(
            "{} has no `{}` in `[rust]`",
            dir.join(manifest::FILE).display(),
            import.name
        );
        return Err(stdlib::not_standard(&import.name, &why));
    };
    // `/`-separated in the manifest; split so each OS joins its own way.
    let rel: PathBuf = dep.path.text.split('/').collect();
    Ok(Crate {
        name: import.name.clone(),
        dir: dir.join(rel),
    })
}

/// `message` as a parse error at `import` in the program.
fn render_import(path: &str, src: &str, import: &Ident, message: String) -> String {
    let diag = Diagnostic {
        span: import.span,
        message,
        kind: DiagKind::Parse,
    };
    crate::render(path, src, &diag)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(src: &str) -> Vec<String> {
        foreign_imports(src).into_iter().map(|i| i.name).collect()
    }

    #[test]
    fn only_leading_non_standard_imports_count() {
        assert_eq!(names("import sys, os\nprint(1)\n"), Vec::<String>::new());
        assert_eq!(
            names("# c\n\nimport sys, fastfs\nimport concurrent.futures\nimport b, fastfs\nx = 1\nimport late\n"),
            ["fastfs", "b"]
        );
        assert_eq!(names("x = (\n"), Vec::<String>::new());
    }

    #[test]
    fn continued_and_aliased_imports_scan_their_module_names() {
        assert_eq!(names("import sys, \\\n    fastfs\n"), ["fastfs"]);
        assert_eq!(names("import fastfs as f\n"), ["fastfs"]);
    }

    #[test]
    fn normalize_takes_dot_dot_off_the_name_before_it() {
        let root = std::path::absolute("/").unwrap_or_default();
        let path = root.join("x/proj/./../real");
        assert_eq!(normalize(&path), root.join("x/real"));
    }

    #[test]
    fn the_program_dir_of_a_bare_name_is_here() {
        assert_eq!(program_dir("main.pyr"), PathBuf::from("."));
        assert_eq!(program_dir("a/b/main.pyr"), PathBuf::from("a/b"));
    }
}
