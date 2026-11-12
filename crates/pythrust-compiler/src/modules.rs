//! The program's local modules (step 3, D75-D84): every `<root>/a.pyr`
//! or `<root>/a/b.pyr` the program imports, found from the main file and
//! lexed into the program's one span space ([`Sources`], D82), in the
//! order their top levels run: depth-first, in import order, each once
//! (D83). Directories are namespace packages, with no code (D78).
//!
//! A name is a standard module first, else a local module file, else a
//! Rust binding (D81). What cannot be told apart is a named error: a
//! local file named like a standard module, a file beside a directory,
//! a package's `__init__.pyr`, a file whose name differs only in case
//! (D78), and a circular import (D75).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use pythrust_compiler::ast::Ident;
use pythrust_compiler::{stdlib, DiagKind, Diagnostic, Span, Token, TokenKind};

use crate::sources::{lex_at, Sources};

/// A local module, lexed.
pub(crate) struct Local {
    /// Its dotted name, `a` or `a.b`.
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
    /// Imports neither standard nor local module files, each once, in
    /// the order met: Rust bindings, if the manifest names them, or else
    /// packages.
    pub(crate) foreign: Vec<Ident>,
    /// The package directories imported, by dotted name.
    pub(crate) packages: Vec<String>,
}

/// A module a leading import names: `a.b` of `import a.b` or of
/// `from a.b import x, y`, with the `x, y`, which may be modules too.
pub(crate) struct Request {
    pub(crate) parts: Vec<Ident>,
    pub(crate) names: Vec<Ident>,
}

/// What an imported path is.
#[derive(Debug, PartialEq)]
enum Found {
    /// Standard, loaded already, or a package: nothing to load.
    Nothing,
    /// A package: its `from` names may be its modules.
    Package,
    /// Not standard and not under the root: a Rust binding, if any.
    Foreign,
    Local(PathBuf),
    /// In a package, no such module: why.
    Missing(String),
}

struct Loader<'s> {
    root: &'s Path,
    /// The names in each directory looked in, as written on disk.
    listings: BTreeMap<PathBuf, BTreeSet<String>>,
    sources: &'s mut Sources,
    /// The modules being loaded, outermost first: a cycle comes back to
    /// one. The main file's own dotted name is first when it is under
    /// the root.
    open: Vec<String>,
    locals: Vec<Local>,
    foreign: Vec<Ident>,
    packages: Vec<String>,
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
        listings: BTreeMap::new(),
        sources,
        open: main_name(root, path).into_iter().collect(),
        locals: Vec::new(),
        foreign: Vec::new(),
        packages: Vec::new(),
    };
    loader.imports(&main)?;
    Ok(Graph {
        locals: loader.locals,
        main,
        foreign: loader.foreign,
        packages: loader.packages,
    })
}

impl Loader<'_> {
    fn imports(&mut self, tokens: &[Token]) -> Result<(), String> {
        for request in requests(tokens) {
            self.request(&request)?;
        }
        Ok(())
    }

    /// Loads what `request` names: the module, then each `from` name
    /// that is a module of it, a package.
    fn request(&mut self, request: &Request) -> Result<(), String> {
        let found = self.import(&request.parts)?;
        if found != Found::Package {
            return Ok(());
        }
        for name in &request.names {
            let mut parts = request.parts.clone();
            parts.push(name.clone());
            // A name that is no module is one of the package's, which
            // the parser reports where it is written.
            if let Found::Local(file) = self.find(&parts).map_err(|m| self.error(&parts, m))? {
                self.local(&parts, &file)?;
            }
        }
        Ok(())
    }

    fn import(&mut self, parts: &[Ident]) -> Result<Found, String> {
        let found = self.find(parts).map_err(|m| self.error(parts, m))?;
        match &found {
            Found::Foreign => {
                let name = joined(parts);
                if !self.foreign.iter().any(|f| f.name == name.name) {
                    self.foreign.push(name);
                }
            }
            Found::Local(file) => self.local(parts, file)?,
            Found::Missing(why) => return Err(self.error(parts, why.clone())),
            Found::Package => {
                let name = joined(parts);
                // A top-level directory may be a binding's crate: a
                // binding comes before a package with no module file
                // (D81), so the manifest decides.
                if parts.len() == 1 && !self.foreign.iter().any(|f| f.name == name.name) {
                    self.foreign.push(name.clone());
                }
                if !self.packages.contains(&name.name) {
                    self.packages.push(name.name);
                }
            }
            Found::Nothing => {}
        }
        Ok(found)
    }

    /// Reads, lexes and loads the local module `parts` from `file`,
    /// after the modules it imports.
    fn local(&mut self, parts: &[Ident], file: &Path) -> Result<(), String> {
        let import = joined(parts);
        let text = std::fs::read_to_string(file)
            .map_err(|e| format!("pythrust: cannot read {}: {e}\n", file.display()))?;
        let base = self.sources.add_module(&shown(file), &text, &import.name);
        let tokens = lex_at(&text, base).map_err(|d| self.sources.render(&d))?;
        self.open.push(import.name.clone());
        self.imports(&tokens)?;
        self.open.pop();
        self.locals.push(Local {
            name: import.name.clone(),
            tokens,
            import,
        });
        Ok(())
    }

    /// What the dotted path `parts` is (D81, D78), or why it cannot be
    /// imported.
    fn find(&mut self, parts: &[Ident]) -> Result<Found, String> {
        let name = joined(parts).name;
        let first = parts[0].name.as_str();
        if stdlib::is_python_module(first) {
            return self.standard(first);
        }
        if let Some(at) = self.open.iter().position(|m| *m == name) {
            return Err(circular(&self.open[at..], &name));
        }
        if self.locals.iter().any(|m| m.name == name) {
            return Ok(Found::Nothing);
        }
        let mut dir = self.root.to_path_buf();
        let (last, packages) = parts.split_last().ok_or("an empty module path")?;
        for (i, part) in packages.iter().enumerate() {
            let shown = joined(&parts[..=i]).name;
            match self.entry(&dir, &part.name)? {
                Entry::Dir => dir.push(&part.name),
                Entry::File => {
                    return Err(format!(
                        "`{shown}` is a module, not a package: `{name}` cannot be imported"
                    ))
                }
                Entry::Missing if i == 0 => {
                    return Err(format!(
                        "no module `{name}`: no directory `{shown}/` under the root"
                    ))
                }
                Entry::Missing => {
                    return Err(format!("no module `{name}`: `{shown}` is not a package"))
                }
            }
        }
        match self.entry(&dir, &last.name)? {
            Entry::File => Ok(Found::Local(dir.join(format!("{}.pyr", last.name)))),
            Entry::Dir => Ok(Found::Package),
            Entry::Missing if packages.is_empty() => Ok(Found::Foreign),
            Entry::Missing => Ok(Found::Missing(format!(
                "no module `{name}`: no file `{}.pyr`",
                path_of(parts)
            ))),
        }
    }

    /// A standard module: nothing to load, unless a local file takes its
    /// name (D81).
    fn standard(&mut self, name: &str) -> Result<Found, String> {
        let file = format!("{name}.pyr");
        let root = self.root.to_path_buf();
        if !self.listing(&root)?.contains(&file) {
            return Ok(Found::Nothing);
        }
        Err(format!(
            "the local module `{file}` is named like the standard module `{name}`, \
             which `import {name}` finds first; rename the file"
        ))
    }

    /// What `name` is in `dir`: a module file, a package directory, or
    /// neither. Both, a package with an `__init__.pyr`, or a file
    /// differing only in case is an error (D78).
    fn entry(&mut self, dir: &Path, name: &str) -> Result<Entry, String> {
        let file = format!("{name}.pyr");
        let names = self.listing(dir)?;
        let has_file = names.contains(&file);
        let has_dir = names.contains(name) && dir.join(name).is_dir();
        let near = other_case(names, &file);
        match (has_file, has_dir) {
            (true, true) => Err(format!(
                "`{name}` is both the module `{file}` and the directory `{name}/`; rename one"
            )),
            (true, false) => Ok(Entry::File),
            (false, true) if dir.join(name).join("__init__.pyr").exists() => {
                Err("packages with `__init__.pyr` are not supported yet".to_string())
            }
            (false, true) => Ok(Entry::Dir),
            (false, false) => near.map_or(Ok(Entry::Missing), Err),
        }
    }

    /// The names in `dir`, read once.
    fn listing(&mut self, dir: &Path) -> Result<&BTreeSet<String>, String> {
        if !self.listings.contains_key(dir) {
            let names = listing(dir)?;
            self.listings.insert(dir.to_path_buf(), names);
        }
        self.listings
            .get(dir)
            .ok_or_else(|| format!("pythrust: cannot list {}\n", dir.display()))
    }

    /// `message` as a parse error at the import of `parts`.
    fn error(&self, parts: &[Ident], message: String) -> String {
        let diag = Diagnostic {
            span: joined(parts).span,
            message,
            kind: DiagKind::Parse,
        };
        self.sources.render(&diag)
    }
}

/// What a name is in a directory.
enum Entry {
    File,
    Dir,
    Missing,
}

/// Why `file` is not found, when a file whose name differs only in
/// case is: the import must spell it exactly (D78).
fn other_case(names: &BTreeSet<String>, file: &str) -> Option<String> {
    let lower = file.to_lowercase();
    let near = names.iter().find(|e| e.to_lowercase() == lower)?;
    Some(format!(
        "no module file `{file}`; `{near}` differs only in case, \
         and an import must match its file name exactly"
    ))
}

/// The dotted path `parts` as one name, spanning them.
fn joined(parts: &[Ident]) -> Ident {
    let names: Vec<&str> = parts.iter().map(|p| p.name.as_str()).collect();
    let first = parts.first().map_or(Span { start: 0, end: 0 }, |p| p.span);
    Ident {
        name: names.join("."),
        span: parts.iter().fold(first, |s, p| s.merge(p.span)),
    }
}

/// `a/b` of `a.b`, as an error shows a module's file.
fn path_of(parts: &[Ident]) -> String {
    let names: Vec<&str> = parts.iter().map(|p| p.name.as_str()).collect();
    names.join("/")
}

/// The circular-import error for a chain of open modules back to `name`.
fn circular(chain: &[String], name: &str) -> String {
    let path: Vec<&str> = chain.iter().map(String::as_str).chain([name]).collect();
    format!(
        "circular import: {}; Pythrust refuses circular imports (a listed deviation)",
        path.join(" -> ")
    )
}

/// The main file's own dotted name, when it is under the root:
/// importing it closes a cycle (D83).
fn main_name(root: &Path, path: &str) -> Option<String> {
    let file = std::path::absolute(path).ok()?;
    let file = crate::project::normalize(&file);
    let rel = file.strip_prefix(root).ok()?.with_extension("");
    let parts: Option<Vec<&str>> = rel.components().map(|c| c.as_os_str().to_str()).collect();
    Some(parts?.join("."))
}

/// The names in `dir`, as written on disk.
fn listing(dir: &Path) -> Result<BTreeSet<String>, String> {
    let cannot = |e: std::io::Error| format!("pythrust: cannot list {}: {e}\n", dir.display());
    let mut names = BTreeSet::new();
    for entry in std::fs::read_dir(dir).map_err(cannot)? {
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

/// The modules a file's leading imports name, in order. Imports come
/// first in a module, so the scan stops at the first statement that is
/// not one; one the parser will refuse names what it can.
pub(crate) fn requests(tokens: &[Token]) -> Vec<Request> {
    let mut out = Vec::new();
    let mut at = Cursor { tokens, i: 0 };
    loop {
        match at.kind() {
            Some(TokenKind::Newline) => {
                at.i += 1;
                continue;
            }
            Some(TokenKind::Import) => {
                at.i += 1;
                at.after_import(&mut out);
            }
            Some(TokenKind::From) => {
                at.i += 1;
                at.after_from(&mut out);
            }
            _ => return out,
        }
        at.skip_line();
    }
}

/// A scan over a file's leading imports.
struct Cursor<'t> {
    tokens: &'t [Token],
    i: usize,
}

impl Cursor<'_> {
    fn kind(&self) -> Option<&TokenKind> {
        self.tokens.get(self.i).map(|t| &t.kind)
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        let found = self.kind() == Some(kind);
        self.i += usize::from(found);
        found
    }

    fn name(&mut self) -> Option<Ident> {
        let t = self.tokens.get(self.i)?;
        let TokenKind::Name(name) = &t.kind else {
            return None;
        };
        self.i += 1;
        Some(Ident {
            name: name.clone(),
            span: t.span,
        })
    }

    /// `NAME ('.' NAME)*`.
    fn path(&mut self) -> Option<Vec<Ident>> {
        let mut parts = vec![self.name()?];
        while self.eat(&TokenKind::Dot) {
            parts.push(self.name()?);
        }
        Some(parts)
    }

    /// After `import`: `path ['as' NAME] (',' ...)*`.
    fn after_import(&mut self, out: &mut Vec<Request>) {
        while let Some(parts) = self.path() {
            out.push(Request {
                parts,
                names: Vec::new(),
            });
            if self.eat(&TokenKind::As) {
                self.name();
            }
            if !self.eat(&TokenKind::Comma) {
                return;
            }
        }
    }

    /// After `from`: `path 'import' ['('] NAME ['as' NAME] (',' ...)*`.
    fn after_from(&mut self, out: &mut Vec<Request>) {
        let Some(parts) = self.path() else {
            return;
        };
        let mut names = Vec::new();
        if self.eat(&TokenKind::Import) {
            self.eat(&TokenKind::LParen);
            while let Some(name) = self.name() {
                names.push(name);
                if self.eat(&TokenKind::As) {
                    self.name();
                }
                if !self.eat(&TokenKind::Comma) {
                    break;
                }
            }
        }
        out.push(Request { parts, names });
    }

    /// Past the next `NEWLINE`.
    fn skip_line(&mut self) {
        while let Some(t) = self.tokens.get(self.i) {
            self.i += 1;
            if t.kind == TokenKind::Newline {
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pythrust_compiler::lexer;

    fn scanned(src: &str) -> Vec<(String, Vec<String>)> {
        let tokens = lexer::lex(src).unwrap();
        let name = |i: &Ident| i.name.clone();
        requests(&tokens)
            .iter()
            .map(|r| (joined(&r.parts).name, r.names.iter().map(name).collect()))
            .collect()
    }

    fn plain(names: &[&str]) -> Vec<(String, Vec<String>)> {
        names.iter().map(|n| (n.to_string(), Vec::new())).collect()
    }

    #[test]
    fn only_leading_imports_count() {
        assert_eq!(scanned("import sys, os\nprint(1)\n"), plain(&["sys", "os"]));
        let src =
            "# c\n\nimport sys, fastfs\nimport concurrent.futures\nimport b\nx = 1\nimport late\n";
        assert_eq!(
            scanned(src),
            plain(&["sys", "fastfs", "concurrent.futures", "b"])
        );
    }

    #[test]
    fn continued_aliased_and_from_imports_scan_their_modules() {
        assert_eq!(
            scanned("import sys, \\\n    fastfs\n"),
            plain(&["sys", "fastfs"])
        );
        assert_eq!(scanned("import a.b as c, d\n"), plain(&["a.b", "d"]));
        let from = "from a.b import (x as y,\n    z)\nfrom c import w\nfrom . import v\n";
        let want = [
            ("a.b".to_string(), vec!["x".to_string(), "z".to_string()]),
            ("c".to_string(), vec!["w".to_string()]),
        ];
        assert_eq!(scanned(from), want);
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
