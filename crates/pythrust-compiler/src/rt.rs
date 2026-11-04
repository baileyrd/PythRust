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
    let mut files = files_in(dir, Links::Read).map_err(read_failed)?;
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
/// with a `workspace` table. Empty when there is none, so an inherited field
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
        let root = runtime::is_workspace_root(&text)
            .map_err(|e| format!("pythrust: {RT_SRC_ENV}: {}: {e}\n", candidate.display()))?;
        if root {
            return Ok(text);
        }
    }
    Ok(String::new())
}

/// How a walk treats a symbolic link.
#[derive(Clone, Copy)]
enum Links {
    /// An override's: read as the file it names, never followed into a
    /// directory.
    Read,
    /// The extracted copy's: refused, so the copy holds only its own files.
    Refuse,
}

/// Every file under `root` but a top-level `target`, sorted by path. Any
/// entry that is not a directory or a regular file (a FIFO, a socket, a
/// device) is refused before it is read, and so is a link, as `links` says.
fn files_in(root: &Path, links: Links) -> Result<Vec<Source>, String> {
    let mut out = Vec::new();
    walk(root, root, links, &mut out)?;
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn walk(root: &Path, dir: &Path, links: Links, out: &mut Vec<Source>) -> Result<(), String> {
    let failed = |p: &Path, e: std::io::Error| format!("cannot read {}: {e}", p.display());
    for entry in std::fs::read_dir(dir).map_err(|e| failed(dir, e))? {
        let entry = entry.map_err(|e| failed(dir, e))?;
        let path = entry.path();
        let kind = entry.file_type().map_err(|e| failed(&path, e))?;
        if kind.is_dir() && dir == root && entry.file_name() == "target" {
            continue;
        }
        if kind.is_dir() {
            walk(root, &path, links, out)?;
            continue;
        }
        regular(&path, kind, links)?;
        let text = std::fs::read(&path).map_err(|e| failed(&path, e))?;
        out.push(Source {
            path: relative(root, &path)?,
            text,
        });
    }
    Ok(())
}

/// Whether the non-directory entry at `path`, of type `kind`, may be
/// read: a regular file, or a link to one where `links` reads links.
fn regular(path: &Path, kind: std::fs::FileType, links: Links) -> Result<(), String> {
    let target = match (kind.is_symlink(), links) {
        (false, _) => kind.is_file(),
        (true, Links::Refuse) => false,
        (true, Links::Read) => std::fs::metadata(path).is_ok_and(|m| m.is_file()),
    };
    match target {
        true => Ok(()),
        false => Err(format!("{} is not a regular file", path.display())),
    }
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

/// Whether `dir` is a directory of its own, not a link, holding exactly
/// `files` as regular files, byte for byte: no file missing, changed or
/// added (a stray `build.rs` would run). A copy that cannot be read does
/// not match, so it is rewritten, and a write that fails then is the
/// error.
fn matches(dir: &Path, files: &[Source]) -> bool {
    let own = std::fs::symlink_metadata(dir).is_ok_and(|m| m.is_dir());
    own && files_in(dir, Links::Refuse).is_ok_and(|found| found == files)
}

/// Removes whatever is at `path`, if anything: a directory with all it
/// holds, or a file or symbolic link, which is never followed.
fn remove(path: &Path) -> Result<(), String> {
    let failed = |e: std::io::Error| format!("pythrust: cannot remove {}: {e}\n", path.display());
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(failed(e)),
    };
    let removed = if meta.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };
    removed.map_err(failed)
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn files() -> Vec<Source> {
        let file = |path: &str, text: &str| Source {
            path: path.to_string(),
            text: text.as_bytes().to_vec(),
        };
        vec![
            file("Cargo.toml", "[package]\n"),
            file("src/lib.rs", "//! rt\n"),
        ]
    }

    /// A fresh directory for `name`, removed again by the test on pass.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("pythrust-rt-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn mkfifo(path: &Path) {
        let made = std::process::Command::new("mkfifo").arg(path).status();
        assert!(made.unwrap().success());
    }

    /// The copy is the project's own: a link in it, a special file or a
    /// linked root does not match and is replaced, never read through.
    #[test]
    fn a_copy_holding_links_or_special_files_is_rewritten() {
        let tmp = scratch("links");
        let (project, outside) = (tmp.join("p"), tmp.join("outside"));
        std::fs::create_dir_all(&outside).unwrap();
        let files = files();
        let dir = ensure(&project, &files).unwrap();
        let lib = dir.join("src/lib.rs");

        std::fs::write(outside.join("lib.rs"), "//! rt\n").unwrap();
        std::fs::remove_file(&lib).unwrap();
        symlink(outside.join("lib.rs"), &lib).unwrap();
        assert!(!matches(&dir, &files));
        ensure(&project, &files).unwrap();
        assert!(!lib.symlink_metadata().unwrap().is_symlink());

        mkfifo(&dir.join("stray"));
        assert!(!matches(&dir, &files));
        ensure(&project, &files).unwrap();
        assert!(!dir.join("stray").exists());

        std::fs::rename(&dir, outside.join("rt")).unwrap();
        symlink(outside.join("rt"), &dir).unwrap();
        assert!(!matches(&dir, &files));
        let (got, _shared) = extract(&project, &files).unwrap();
        assert!(got.symlink_metadata().unwrap().is_dir());
        assert!(outside.join("rt/src/lib.rs").is_file());
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    /// The workspace above an override is found with a commented header.
    #[test]
    fn an_override_inherits_from_a_commented_workspace_above_it() {
        let tmp = scratch("workspace");
        std::fs::create_dir_all(tmp.join("rt")).unwrap();
        let ws = "[workspace] # root\n[workspace.package]\nversion = \"2.0.0\"\n";
        std::fs::write(tmp.join("Cargo.toml"), ws).unwrap();
        std::fs::write(
            tmp.join("rt/Cargo.toml"),
            "[package]\nversion.workspace = true\n",
        )
        .unwrap();
        let files = snapshot(&tmp.join("rt")).unwrap();
        let text = String::from_utf8(files[0].text.clone()).unwrap();
        assert!(text.contains("version = \"2.0.0\""), "{text}");
        std::fs::remove_dir_all(&tmp).unwrap();
    }

    /// An override is read through its links, but a special file in it
    /// is a named error, not a read that waits forever.
    #[test]
    fn an_override_holding_a_special_file_is_a_named_error() {
        let tmp = scratch("override");
        std::fs::write(tmp.join("Cargo.toml"), "[package]\n").unwrap();
        mkfifo(&tmp.join("pipe"));
        let got = snapshot(&tmp).map(|_| ());
        assert!(
            got.as_ref()
                .is_err_and(|e| e.contains("not a regular file")),
            "{got:?}"
        );
        std::fs::remove_dir_all(&tmp).unwrap();
    }
}
