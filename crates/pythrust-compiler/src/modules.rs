//! The program's local modules (step 3, D75-D83): every `<root>/a.pyr`
//! the program imports, found from the main file and lexed into the
//! program's one span space ([`Sources`], D82), in the order their top
//! levels run: depth-first, in import order, each once (D83).
//!
//! A name is a standard module first, else a local module file, else a
//! Rust binding (D81). What cannot be told apart is a named error: a
//! local file named like a standard module, a file beside a directory,
//! a package's `__init__.pyr`, a file whose name differs only in case
//! (D78), and a circular import (D75).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use pythrust_compiler::ast::Ident;
use pythrust_compiler::{stdlib, DiagKind, Diagnostic, Token, TokenKind};

use crate::sources::{lex_at, Sources};

/// A local module, lexed.
pub(crate) struct Local {
    pub(crate) name: String,
    pub(crate) tokens: Vec<Token>,
    /// The first import of it, for errors that name it.
    pub(crate) import: Ident,
}

/// The program's files, lexed, and what they import from elsewhere.
pub(crate) struct Graph {
    /// In the order their top levels run (D83).
    pub(crate) locals: Vec<Local>,
    pub(crate) main: Vec<Token>,
    /// Imports neither standard nor local, each once, in the order met:
    /// Rust bindings, if the manifest names them.
    pub(crate) foreign: Vec<Ident>,
}

/// What an imported name is.
enum Found {
    /// Standard, foreign or loaded already: nothing to load.
    Nothing,
    Foreign,
    Local(PathBuf),
}

struct Loader<'s> {
    root: &'s Path,
    /// The names in the root directory.
    entries: BTreeSet<String>,
    sources: &'s mut Sources,
    /// The modules being loaded, outermost first: a cycle comes back to
    /// one. The main file's own name is first when it is in the root.
    open: Vec<String>,
    locals: Vec<Local>,
    foreign: Vec<Ident>,
}

/// Loads the program whose main file is `path`, with text `src`, and
/// every local module under `root` it imports. An error comes rendered.
pub(crate) fn load(
    root: &Path,
    path: &str,
    src: &str,
    sources: &mut Sources,
) -> Result<Graph, String> {
    let main = lex_at(src, 0).map_err(|d| sources.render(&d))?;
    let mut loader = Loader {
        root,
        entries: listing(root)?,
        sources,
        open: main_name(root, path).into_iter().collect(),
        locals: Vec::new(),
        foreign: Vec::new(),
    };
    loader.imports(&main)?;
    Ok(Graph {
        locals: loader.locals,
        main,
        foreign: loader.foreign,
    })
}

impl Loader<'_> {
    fn imports(&mut self, tokens: &[Token]) -> Result<(), String> {
        for name in imported(tokens) {
            self.import(name)?;
        }
        Ok(())
    }

    fn import(&mut self, name: Ident) -> Result<(), String> {
        let found = self.find(&name).map_err(|m| self.error(&name, m))?;
        match found {
            Found::Nothing => Ok(()),
            Found::Foreign => {
                if !self.foreign.iter().any(|f| f.name == name.name) {
                    self.foreign.push(name);
                }
                Ok(())
            }
            Found::Local(file) => self.local(name, &file),
        }
    }

    /// Reads, lexes and loads the local module `name` from `file`, after
    /// the modules it imports.
    fn local(&mut self, name: Ident, file: &Path) -> Result<(), String> {
        let text = std::fs::read_to_string(file)
            .map_err(|e| format!("pythrust: cannot read {}: {e}\n", file.display()))?;
        let base = self.sources.add_module(&shown(file), &text, &name.name);
        let tokens = lex_at(&text, base).map_err(|d| self.sources.render(&d))?;
        self.open.push(name.name.clone());
        self.imports(&tokens)?;
        self.open.pop();
        self.locals.push(Local {
            name: name.name.clone(),
            tokens,
            import: name,
        });
        Ok(())
    }

    /// What `name` is (D81, D78), or why it cannot be imported.
    fn find(&self, name: &Ident) -> Result<Found, String> {
        let n = name.name.as_str();
        let file = format!("{n}.pyr");
        let has_file = self.entries.contains(&file);
        if stdlib::is_python_module(n) {
            return match has_file {
                true => Err(format!(
                    "the local module `{file}` is named like the standard module `{n}`, \
                     which `import {n}` finds first; rename the file"
                )),
                false => Ok(Found::Nothing),
            };
        }
        if let Some(at) = self.open.iter().position(|m| m == n) {
            return Err(circular(&self.open[at..], n));
        }
        if self.locals.iter().any(|m| m.name == n) {
            return Ok(Found::Nothing);
        }
        let dir = self.root.join(n);
        let has_dir = self.entries.contains(n) && dir.is_dir();
        match (has_file, has_dir) {
            (true, true) => Err(format!(
                "`{n}` is both the module `{file}` and the directory `{n}/`; rename one"
            )),
            (true, false) => Ok(Found::Local(self.root.join(file))),
            (false, true) if dir.join("__init__.pyr").exists() => {
                Err("packages with `__init__.pyr` are not supported yet".to_string())
            }
            _ => self.other_case(&file).map_or(Ok(Found::Foreign), Err),
        }
    }

    /// Why `file` is not found, when a file whose name differs only in
    /// case is: the import must spell it exactly (D78).
    fn other_case(&self, file: &str) -> Option<String> {
        let lower = file.to_lowercase();
        let near = self.entries.iter().find(|e| e.to_lowercase() == lower)?;
        Some(format!(
            "no module file `{file}`; `{near}` differs only in case, \
             and an import must match its file name exactly"
        ))
    }

    /// `message` as a parse error at the import `name`.
    fn error(&self, name: &Ident, message: String) -> String {
        let diag = Diagnostic {
            span: name.span,
            message,
            kind: DiagKind::Parse,
        };
        self.sources.render(&diag)
    }
}

/// The circular-import error for a chain of open modules back to `name`.
fn circular(chain: &[String], name: &str) -> String {
    let path: Vec<&str> = chain.iter().map(String::as_str).chain([name]).collect();
    format!(
        "circular import: {}; Pythrust refuses circular imports (a listed deviation)",
        path.join(" -> ")
    )
}

/// The main file's own module name, when it is in the root: importing
/// it closes a cycle (D83).
fn main_name(root: &Path, path: &str) -> Option<String> {
    let file = std::path::absolute(path).ok()?;
    let dir = crate::project::normalize(file.parent()?);
    let stem = file.file_stem()?.to_str()?;
    (dir == root).then(|| stem.to_string())
}

/// The names in `root`, as written on disk.
fn listing(root: &Path) -> Result<BTreeSet<String>, String> {
    let cannot = |e: std::io::Error| format!("pythrust: cannot list {}: {e}\n", root.display());
    let mut names = BTreeSet::new();
    for entry in std::fs::read_dir(root).map_err(cannot)? {
        let entry = entry.map_err(cannot)?;
        names.insert(entry.file_name().to_string_lossy().into_owned());
    }
    Ok(names)
}

/// `file` relative to the current directory where it can be (D82).
fn shown(file: &Path) -> String {
    let cwd = std::env::current_dir().ok();
    let rel = cwd.as_deref().and_then(|c| file.strip_prefix(c).ok());
    rel.unwrap_or(file).display().to_string()
}

/// The module names a file's leading imports name, in order: the
/// `concurrent` of `import concurrent.futures`, not `futures` or an
/// `as` name. Imports come first in a module, so the scan stops at the
/// first statement that is not one.
pub(crate) fn imported(tokens: &[Token]) -> Vec<Ident> {
    let mut names = Vec::new();
    let mut in_import = false;
    for (i, t) in tokens.iter().enumerate() {
        match &t.kind {
            TokenKind::Import => in_import = true,
            TokenKind::Newline => in_import = false,
            TokenKind::Name(name) if in_import && after_import_or_comma(tokens, i) => {
                names.push(Ident {
                    name: name.clone(),
                    span: t.span,
                });
            }
            _ if !in_import => break,
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

#[cfg(test)]
mod tests {
    use super::*;
    use pythrust_compiler::lexer;

    fn names(src: &str) -> Vec<String> {
        let tokens = lexer::lex(src).unwrap();
        imported(&tokens).into_iter().map(|i| i.name).collect()
    }

    #[test]
    fn only_leading_imports_count() {
        assert_eq!(names("import sys, os\nprint(1)\n"), ["sys", "os"]);
        assert_eq!(
            names("# c\n\nimport sys, fastfs\nimport concurrent.futures\nimport b\nx = 1\nimport late\n"),
            ["sys", "fastfs", "concurrent", "b"]
        );
    }

    #[test]
    fn continued_and_aliased_imports_scan_their_module_names() {
        assert_eq!(names("import sys, \\\n    fastfs\n"), ["sys", "fastfs"]);
        assert_eq!(names("import fastfs as f\n"), ["fastfs"]);
    }

    #[test]
    fn a_cycle_names_its_chain() {
        let chain = ["main".to_string(), "a".to_string(), "b".to_string()];
        assert_eq!(
            circular(&chain[1..], "a"),
            "circular import: a -> b -> a; Pythrust refuses circular imports (a listed deviation)"
        );
    }
}
