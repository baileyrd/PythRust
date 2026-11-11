//! A program's project (M6d, `docs/research/package-tool.md`): the
//! `pythrust.toml` found by walking up from the program's directory,
//! read only when the program imports a module that is neither standard
//! nor local, and the Rust bindings it names for those imports. Its
//! directory is the root local modules are found in (D75).

use std::path::{Component, Path, PathBuf};

use pythrust_compiler::ast::Ident;
use pythrust_compiler::manifest::{self, toml, Manifest, RustDep};
use pythrust_compiler::{stdlib, DiagKind, Diagnostic};

use crate::bindings;
use crate::cargo::Crate;
use crate::modules::Graph;
use crate::sources::Sources;

/// The bindings a program imports and the manifest that names them.
pub(crate) struct Project {
    /// With absolute directories, in import order.
    pub(crate) crates: Vec<Crate>,
    /// `None` when every import is standard: then none is looked for.
    pub(crate) manifest: Option<Found>,
}

/// A manifest read from disk.
pub(crate) struct Found {
    /// The `pythrust.toml` itself, as the walk found it.
    pub(crate) path: PathBuf,
    /// Every binding of `[rust]`, with absolute directories, in order:
    /// `pythrust lock` resolves them all.
    pub(crate) all: Vec<Crate>,
}

impl Found {
    /// The directory the manifest is in: the project's.
    pub(crate) fn dir(&self) -> &Path {
        self.path.parent().unwrap_or(Path::new("."))
    }
}

/// Where a program's modules are found (D75): the directory of the
/// first `pythrust.toml` in or above the main file's, else the main
/// file's. Finding it checks only that the manifest exists (D81).
pub(crate) struct Root {
    pub(crate) dir: PathBuf,
    /// The main file's directory, where the walk started.
    start: PathBuf,
    manifest: Option<PathBuf>,
}

/// The root of the program at `path`.
pub(crate) fn root(path: &str) -> Result<Root, String> {
    let start = normalize(&absolute(&program_dir(path))?);
    let manifest = find(&start)?;
    let dir = match manifest.as_deref().and_then(Path::parent) {
        Some(dir) => dir.to_path_buf(),
        None => start.clone(),
    };
    Ok(Root {
        dir,
        start,
        manifest,
    })
}

/// The project of the program `graph`. A program importing only
/// standard modules never reads the manifest; one importing a local
/// module or package reads it when there is one, so a binding by the
/// same name is an error (D125, D126); one importing a name neither
/// standard nor a local module file needs it (D81), unless the name is
/// a package directory the manifest does not give. An error comes
/// rendered.
pub(crate) fn bindings(root: &Root, graph: &Graph, sources: &Sources) -> Result<Project, String> {
    let none = || Project {
        crates: Vec::new(),
        manifest: None,
    };
    if graph.foreign.is_empty() && graph.locals.is_empty() && graph.packages.is_empty() {
        return Ok(none());
    }
    let package = |i: &Ident| graph.packages.contains(&i.name);
    let Some(file) = &root.manifest else {
        let Some(first) = graph.foreign.iter().find(|i| !package(i)) else {
            return Ok(none());
        };
        let why = format!(
            "no pythrust.toml names it (none in {} or any directory above it)",
            root.start.display()
        );
        let message = stdlib::not_standard(&first.name, &why);
        return Err(render_import(sources, first, message));
    };
    let found = load(file.clone())?;
    for local in &graph.locals {
        let top = local.name.split('.').next().unwrap_or(&local.name);
        if !found.all.iter().any(|c| c.name == top) {
            continue;
        }
        let what = match top == local.name {
            true => format!("the local module `{top}.pyr`"),
            false => format!("the package of the local module `{}`", local.name),
        };
        let message = format!(
            "`{top}` is both {what} and a Rust binding in {}; rename one",
            found.path.display()
        );
        return Err(render_import(sources, &local.import, message));
    }
    let mut crates = Vec::new();
    for import in &graph.foreign {
        match named(&found, import) {
            Ok(c) => crates.push(c),
            Err(_) if package(import) => {}
            Err(m) => return Err(render_import(sources, import, m)),
        }
    }
    Ok(Project {
        crates,
        manifest: Some(found),
    })
}

/// The manifest above the current directory, for `pythrust lock`.
pub(crate) fn here() -> Result<Found, String> {
    let start = normalize(&absolute(Path::new("."))?);
    let Some(file) = find(&start)? else {
        return Err(format!(
            "pythrust: no pythrust.toml in {} or any directory above it\n",
            start.display()
        ));
    };
    load(file)
}

/// Reads the manifest `file` and places each of its bindings.
fn load(file: PathBuf) -> Result<Found, String> {
    let manifest = read(&file)?;
    let dir = file.parent().unwrap_or(Path::new("."));
    let all = manifest.rust.iter().map(|dep| place(dir, dep)).collect();
    Ok(Found { path: file, all })
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
pub(crate) fn normalize(path: &Path) -> PathBuf {
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

/// The binding `import` names in `found`.
fn named(found: &Found, import: &Ident) -> Result<Crate, String> {
    let Some(c) = found.all.iter().find(|c| c.name == import.name) else {
        let why = format!(
            "{} has no `{}` in `[rust]`",
            found.path.display(),
            import.name
        );
        return Err(stdlib::not_standard(&import.name, &why));
    };
    Ok(c.clone())
}

/// The binding `dep` of the manifest in `dir`.
fn place(dir: &Path, dep: &RustDep) -> Crate {
    // `/`-separated in the manifest; split so each OS joins its own way.
    let rel: PathBuf = dep.path.text.split('/').collect();
    Crate {
        name: dep.name.name.clone(),
        dir: dir.join(rel),
    }
}

/// `message` as a parse error at `import`, in whichever file it is.
fn render_import(sources: &Sources, import: &Ident, message: String) -> String {
    let diag = Diagnostic {
        span: import.span,
        message,
        kind: DiagKind::Parse,
    };
    sources.render(&diag)
}

#[cfg(test)]
mod tests {
    use super::*;

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
