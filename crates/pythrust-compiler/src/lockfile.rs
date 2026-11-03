//! `pythrust.lock` (M6d, `docs/research/package-tool.md`): `pythrust
//! lock` writes it; `build` resolves the bindings a program imports
//! from it and refuses anything it does not cover. The project's file
//! locks keep the shared paths consistent between runs.

use std::fs::File;
use std::path::{Path, PathBuf};

use pythrust_compiler::interop::gen::{CRATE, CRATE_VERSION};
use pythrust_compiler::interop::lock::{self, Lock};

use crate::cargo::{self, Binding};
use crate::project::Found;

/// The lock's file name, beside `pythrust.toml`.
const FILE: &str = "pythrust.lock";

/// The project's directory for generated files.
const DOT: &str = ".pythrust";

/// The project lock's file, in [`DOT`].
const PROJECT_LOCK: &str = "project.lock";

/// The build lock's file, in a program's build directory.
const BUILD_LOCK: &str = "build.lock";

/// Lets resolution prefer crates that build on the generated crate's
/// `rust-version` (`gen::MIN_RUST`).
const FALLBACK: (&str, &str) = ("CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS", "fallback");

/// How a run holds the project lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Hold {
    /// A build: nothing it reads changes under it.
    Shared,
    /// `pythrust lock`: no build runs while it writes.
    Exclusive,
}

/// The project lock of the project in `dir`, held until dropped.
pub(crate) fn hold(dir: &Path, how: Hold) -> Result<File, String> {
    let dot = dir.join(DOT);
    std::fs::create_dir_all(&dot)
        .map_err(|e| format!("pythrust: cannot create {}: {e}\n", dot.display()))?;
    let path = dot.join(PROJECT_LOCK);
    let file = open(&path)?;
    take(&file, &path, how)?;
    Ok(file)
}

/// The build lock of the build directory `dir`, held until dropped, so
/// two builds of one program run one after the other.
pub(crate) fn hold_build(dir: &Path) -> Result<File, String> {
    let path = dir.join(BUILD_LOCK);
    let file = open(&path)?;
    take(&file, &path, Hold::Exclusive)?;
    Ok(file)
}

/// Locks `file`, at `path`, as `how` says. When another run holds it,
/// says so once before waiting, as cargo does.
fn take(file: &File, path: &Path, how: Hold) -> Result<(), String> {
    let tried = match how {
        Hold::Shared => file.try_lock_shared(),
        Hold::Exclusive => file.try_lock(),
    };
    let failed = |e: std::io::Error| format!("pythrust: cannot lock {}: {e}\n", path.display());
    match tried {
        Ok(()) => return Ok(()),
        Err(std::fs::TryLockError::Error(e)) => return Err(failed(e)),
        Err(std::fs::TryLockError::WouldBlock) => {}
    }
    eprintln!("pythrust: waiting for {}", path.display());
    let held = match how {
        Hold::Shared => file.lock_shared(),
        Hold::Exclusive => file.lock(),
    };
    held.map_err(failed)
}

fn open(path: &Path) -> Result<File, String> {
    File::options()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| format!("pythrust: cannot open {}: {e}\n", path.display()))
}

/// A project's `pythrust.lock`: cargo's text, without the header, and
/// what it holds.
pub(crate) struct Locked {
    pub(crate) text: String,
    lock: Lock,
}

/// After `build` resolved a program's bindings from `locked` into
/// `text`: the stale-lock error naming each entry `locked` does not
/// cover, if any.
pub(crate) fn check_covered(locked: &Locked, text: &str, path: &Path) -> Result<(), String> {
    let built = lock::parse(text).map_err(|e| format!("pythrust: {}: {e}\n", path.display()))?;
    let changes = lock::uncovered(&built, &locked.lock, (CRATE, CRATE_VERSION));
    if changes.is_empty() {
        return Ok(());
    }
    let listed: String = changes.iter().map(|c| format!("  {c}\n")).collect();
    Err(format!(
        "pythrust: pythrust.lock is out of date; run `pythrust lock`\n{listed}"
    ))
}

/// Reads the lock of `project`. Missing, it is the named error.
pub(crate) fn read(project: &Found) -> Result<Locked, String> {
    let path = project.dir().join(FILE);
    let exists = path
        .try_exists()
        .map_err(|e| format!("pythrust: cannot look for {}: {e}\n", path.display()))?;
    if !exists {
        return Err("pythrust: no pythrust.lock; run `pythrust lock`\n".to_string());
    }
    let full = std::fs::read_to_string(&path)
        .map_err(|e| format!("pythrust: cannot read {}: {e}\n", path.display()))?;
    let text = lock::strip_header(&full).to_string();
    let lock = parsed(&path, &text)?;
    Ok(Locked { text, lock })
}

/// `text` read as a lock, which may not hold a git package.
fn parsed(path: &Path, text: &str) -> Result<Lock, String> {
    let lock = lock::parse(text).map_err(|e| format!("pythrust: {}: {e}\n", path.display()))?;
    if let Some(p) = lock::git_package(&lock) {
        return Err(format!(
            "pythrust: {}: git dependencies are not supported yet; use a crates.io version (`{}`)\n",
            path.display(),
            p.name
        ));
    }
    Ok(lock)
}

/// `pythrust lock`: resolves every binding of `project` from its current
/// lock, keeping each pin that still fits, and replaces the lock with
/// the result. A failure leaves the old lock as it was. Gives its path.
pub(crate) fn lock(project: &Found) -> Result<PathBuf, String> {
    let _held = hold(project.dir(), Hold::Exclusive)?;
    let path = project.dir().join(FILE);
    let old = seed(&path)?;
    let runtime = cargo::runtime_src()?;
    let bindings = all_bindings(project)?;
    let dir = project.dir().join(DOT).join("lock");
    let text = resolve(&dir, &runtime, &bindings, old.as_deref())?;
    parsed(&path, &text)?;
    let tmp = project.dir().join(format!(".{FILE}.tmp"));
    let written = std::fs::write(&tmp, lock::with_header(&text))
        .and_then(|()| std::fs::rename(&tmp, &path))
        .map_err(|e| format!("pythrust: cannot write {}: {e}\n", path.display()));
    if written.is_err() {
        // The old lock stands; the half-made copy goes, best effort.
        let _ = std::fs::remove_file(&tmp);
    }
    written.map(|()| path)
}

/// The current lock's text without its header, to resolve from; `None`
/// when there is none.
fn seed(path: &Path) -> Result<Option<String>, String> {
    match std::fs::read_to_string(path) {
        Ok(full) => Ok(Some(lock::strip_header(&full).to_string())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("pythrust: cannot read {}: {e}\n", path.display())),
    }
}

/// Every binding of `project`, checked for cargo.
fn all_bindings(project: &Found) -> Result<Vec<Binding>, String> {
    project.all.iter().map(cargo::binding).collect()
}

/// The lock cargo resolves in `dir` for the generated crate over
/// `bindings`, starting from `seed` (none: from nothing).
fn resolve(
    dir: &Path,
    runtime: &Path,
    bindings: &[Binding],
    seed: Option<&str>,
) -> Result<String, String> {
    std::fs::create_dir_all(dir)
        .map_err(|e| format!("pythrust: cannot create {}: {e}\n", dir.display()))?;
    cargo::write(dir, "Cargo.toml", &cargo::manifest(runtime, bindings)?)?;
    cargo::write(dir, "lib.rs", "")?;
    resolve_in(dir, seed)
}

/// The lock cargo resolves for the crate already written in `dir`,
/// starting from `seed` (none: from nothing). Cargo keeps every pin that
/// still fits, resolves only what is new, and drops what the crate does
/// not use.
pub(crate) fn resolve_in(dir: &Path, seed: Option<&str>) -> Result<String, String> {
    let lock = dir.join("Cargo.lock");
    match seed {
        Some(text) => cargo::write(dir, "Cargo.lock", text)?,
        None => remove(&lock)?,
    }
    let mut metadata = cargo::cargo();
    metadata
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(dir.join("Cargo.toml"))
        .env(FALLBACK.0, FALLBACK.1);
    let output = cargo::run(metadata)?;
    if !output.status.success() {
        return Err(format!(
            "pythrust: cannot resolve the bindings' dependencies\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    std::fs::read_to_string(&lock)
        .map_err(|e| format!("pythrust: cannot read {}: {e}\n", lock.display()))
}

/// Removes `path` if it is there.
fn remove(path: &Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            Err(format!("pythrust: cannot remove {}: {e}\n", path.display()))
        }
        _ => Ok(()),
    }
}
