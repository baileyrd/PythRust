//! The runtime a program calling Rust bindings builds against (M6d,
//! `docs/research/package-tool.md#the-manifest`). The selected source is
//! the embedded one, or a snapshot of `$PYTHRUST_RT_SRC`. It is extracted
//! once per project to `.pythrust/rt/<version>-<hash>/`, and a copy that
//! does not match it byte for byte is rewritten under the exclusive
//! project lock.

use std::fs::File;
use std::path::{Path, PathBuf};

use pythrust_compiler::interop::runtime::{self, File as Source};

use crate::lockfile::{self, Hold};

/// A development override: the runtime's source directory, used in place
/// of the embedded one.
const RT_SRC_ENV: &str = "PYTHRUST_RT_SRC";

/// The runtime's source for this run: embedded, or read once from
/// `$PYTHRUST_RT_SRC`, which is announced on stderr.
pub(crate) fn selected() -> Result<Vec<Source>, String> {
    let Some(dir) = std::env::var_os(RT_SRC_ENV) else {
        return runtime::embedded().map_err(|e| format!("pythrust: the embedded runtime: {e}\n"));
    };
    let dir = PathBuf::from(dir);
    eprintln!(
        "pythrust: runtime source from {} ({RT_SRC_ENV})",
        dir.display()
    );
    let dir = std::fs::canonicalize(&dir).map_err(|e| {
        format!(
            "pythrust: {RT_SRC_ENV}: cannot resolve {}: {e}\n",
            dir.display()
        )
    })?;
    snapshot(&dir)
}

/// The runtime crate at the canonical `dir`, as the embedded one is
/// taken: every file but cargo's `target`, sorted by path, with its
/// `Cargo.toml` made standalone.
fn snapshot(dir: &Path) -> Result<Vec<Source>, String> {
    let read_failed = |e: String| format!("pythrust: {RT_SRC_ENV}: {e}\n");
    let mut files = files_in(dir).map_err(read_failed)?;
    let workspace = workspace_manifest(dir)?;
    let manifest = files
        .iter_mut()
        .find(|f| f.path == "Cargo.toml")
        .ok_or_else(|| read_failed(format!("no Cargo.toml in {}", dir.display())))?;
    let text = String::from_utf8(std::mem::take(&mut manifest.text))
        .map_err(|_| read_failed("its Cargo.toml is not UTF-8".to_string()))?;
    let text = runtime::standalone(&text, &workspace)
        .map_err(|e| read_failed(format!("{}: {e}", dir.join("Cargo.toml").display())))?;
    manifest.text = text.into_bytes();
    Ok(files)
}

/// The workspace manifest above the crate at the canonical `dir`, for
/// the fields its `Cargo.toml` inherits: the first `Cargo.toml` above
/// with a `[workspace]`. Empty when there is none, so an inherited field
/// is a named error.
fn workspace_manifest(dir: &Path) -> Result<String, String> {
    for up in dir.ancestors().skip(1) {
        let candidate = up.join("Cargo.toml");
        let text = match std::fs::read_to_string(&candidate) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => {
                let shown = candidate.display();
                return Err(format!(
                    "pythrust: {RT_SRC_ENV}: cannot read {shown}: {e}\n"
                ));
            }
        };
        if text.lines().any(|l| l.trim() == "[workspace]") {
            return Ok(text);
        }
    }
    Ok(String::new())
}

/// Every file under `root` but a top-level `target`, sorted by path.
/// A symbolic link is read as the file it names, never followed into a
/// directory.
fn files_in(root: &Path) -> Result<Vec<Source>, String> {
    let mut out = Vec::new();
    walk(root, root, &mut out)?;
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<Source>) -> Result<(), String> {
    let failed = |p: &Path, e: std::io::Error| format!("cannot read {}: {e}", p.display());
    for entry in std::fs::read_dir(dir).map_err(|e| failed(dir, e))? {
        let entry = entry.map_err(|e| failed(dir, e))?;
        let path = entry.path();
        let is_dir = entry.file_type().map_err(|e| failed(&path, e))?.is_dir();
        if is_dir && dir == root && entry.file_name() == "target" {
            continue;
        }
        if is_dir {
            walk(root, &path, out)?;
            continue;
        }
        let text = std::fs::read(&path).map_err(|e| failed(&path, e))?;
        out.push(Source {
            path: relative(root, &path)?,
            text,
        });
    }
    Ok(())
}

/// `path` under `root`, `/`-separated.
fn relative(root: &Path, path: &Path) -> Result<String, String> {
    let rel = path
        .strip_prefix(root)
        .map_err(|_| format!("{} is outside {}", path.display(), root.display()))?;
    let parts: Option<Vec<&str>> = rel.iter().map(|p| p.to_str()).collect();
    parts
        .map(|p| p.join("/"))
        .ok_or_else(|| format!("{} is not UTF-8", path.display()))
}

/// For a build: the extracted runtime of the project in `project`, with
/// the project lock held shared, as the build keeps it. A copy that does
/// not match `files` is repaired under the exclusive lock first.
pub(crate) fn extract(project: &Path, files: &[Source]) -> Result<(PathBuf, File), String> {
    let dir = rt_dir(project, files);
    loop {
        let shared = lockfile::hold(project, Hold::Shared)?;
        if matches(&dir, files) {
            return Ok((dir, shared));
        }
        drop(shared);
        let exclusive = lockfile::hold(project, Hold::Exclusive)?;
        ensure(project, files)?;
        drop(exclusive);
    }
}

/// The extracted runtime of the project in `project`, written afresh
/// unless it matches `files`. The caller holds the project lock
/// exclusively.
pub(crate) fn ensure(project: &Path, files: &[Source]) -> Result<PathBuf, String> {
    let dir = rt_dir(project, files);
    if matches(&dir, files) {
        return Ok(dir);
    }
    let tmp = dir.with_file_name(format!(".{}.tmp", runtime::hash(files)));
    let created = |p: &Path, e: std::io::Error| {
        format!(
            "pythrust: cannot write the runtime to {}: {e}\n",
            p.display()
        )
    };
    remove(&tmp)?;
    for f in files {
        let path = tmp.join(&f.path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| created(parent, e))?;
        }
        std::fs::write(&path, &f.text).map_err(|e| created(&path, e))?;
    }
    remove(&dir)?;
    std::fs::rename(&tmp, &dir).map_err(|e| created(&dir, e))?;
    Ok(dir)
}

/// `.pythrust/rt/<version>-<hash>/` of the project in `project`.
fn rt_dir(project: &Path, files: &[Source]) -> PathBuf {
    let name = format!("{}-{}", env!("CARGO_PKG_VERSION"), runtime::hash(files));
    project.join(".pythrust").join("rt").join(name)
}

/// Whether `dir` holds exactly `files`, byte for byte: no file missing,
/// changed or added (a stray `build.rs` would run). A copy that cannot
/// be read does not match, so it is rewritten, and a write that fails
/// then is the error.
fn matches(dir: &Path, files: &[Source]) -> bool {
    files_in(dir).is_ok_and(|found| found == files)
}

/// Removes the directory `path` if it is there.
fn remove(path: &Path) -> Result<(), String> {
    match std::fs::remove_dir_all(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            Err(format!("pythrust: cannot remove {}: {e}\n", path.display()))
        }
        _ => Ok(()),
    }
}
