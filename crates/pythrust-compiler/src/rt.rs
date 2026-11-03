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
    snapshot(&dir)
}

/// The runtime crate at `dir`: its `Cargo.toml`, made standalone, and
/// every file under `src/`, sorted by path.
fn snapshot(dir: &Path) -> Result<Vec<Source>, String> {
    let manifest = dir.join("Cargo.toml");
    let text = std::fs::read_to_string(&manifest).map_err(|e| {
        format!(
            "pythrust: {RT_SRC_ENV}: cannot read {}: {e}\n",
            manifest.display()
        )
    })?;
    let workspace = workspace_manifest(dir)?;
    let text = runtime::standalone(&text, &workspace)
        .map_err(|e| format!("pythrust: {RT_SRC_ENV}: {}: {e}\n", manifest.display()))?;
    let mut files = vec![Source {
        path: "Cargo.toml".to_string(),
        text: text.into_bytes(),
    }];
    let mut found = Vec::new();
    walk(dir, &dir.join("src"), &mut found)?;
    found.sort_by(|a, b| a.path.cmp(&b.path));
    files.extend(found);
    Ok(files)
}

/// The workspace manifest above the crate at `dir`, if its `Cargo.toml`
/// inherits from one: the first `Cargo.toml` above with a `[workspace]`.
/// Empty when there is none, so an inherited field is a named error.
fn workspace_manifest(dir: &Path) -> Result<String, String> {
    for up in dir.ancestors().skip(1) {
        let candidate = up.join("Cargo.toml");
        let Ok(text) = std::fs::read_to_string(&candidate) else {
            continue;
        };
        if text.lines().any(|l| l.trim() == "[workspace]") {
            return Ok(text);
        }
    }
    Ok(String::new())
}

/// Every file under `dir`, inside the crate at `root`, into `out`.
fn walk(root: &Path, dir: &Path, out: &mut Vec<Source>) -> Result<(), String> {
    let failed = |p: &Path, e: std::io::Error| {
        format!("pythrust: {RT_SRC_ENV}: cannot read {}: {e}\n", p.display())
    };
    for entry in std::fs::read_dir(dir).map_err(|e| failed(dir, e))? {
        let path = entry.map_err(|e| failed(dir, e))?.path();
        if path.is_dir() {
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
    let rel = path.strip_prefix(root).map_err(|_| {
        format!(
            "pythrust: {} is outside {}\n",
            path.display(),
            root.display()
        )
    })?;
    let parts: Option<Vec<&str>> = rel.iter().map(|p| p.to_str()).collect();
    parts
        .map(|p| p.join("/"))
        .ok_or_else(|| format!("pythrust: {} is not UTF-8\n", path.display()))
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

/// Whether every file of `files` is in `dir` byte for byte. A file that
/// cannot be read does not match, so the copy is rewritten, and a write
/// that fails then is the error.
fn matches(dir: &Path, files: &[Source]) -> bool {
    files
        .iter()
        .all(|f| std::fs::read(dir.join(&f.path)).is_ok_and(|text| text == f.text))
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
