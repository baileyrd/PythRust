//! Helpers the integration tests share: each test's scratch directory.

use std::fs;
use std::path::{Path, PathBuf};

/// Set to keep every test's scratch directory, as a failing test's is.
pub const KEEP: &str = "PYTHRUST_KEEP_SCRATCH";

/// A test's scratch directory, removed when the test ends. A failing
/// test's, or every one under [`KEEP`], stays, and its path is printed.
pub struct Scratch(PathBuf);

impl std::ops::Deref for Scratch {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if std::thread::panicking() || std::env::var_os(KEEP).is_some() {
            eprintln!("kept scratch directory {}", self.0.display());
            return;
        }
        if let Err(e) = fs::remove_dir_all(&self.0) {
            eprintln!("cannot remove {}: {e}", self.0.display());
        }
    }
}

/// A fresh directory `pythrust-<suite>-<pid>-<test>`, flat in the temp
/// dir: no parent shared with other tests, so none is created or removed
/// under a test running beside it.
pub fn scratch(suite: &str, test: &str) -> Scratch {
    let name = format!("pythrust-{suite}-{}-{test}", std::process::id());
    let dir = std::env::temp_dir().join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    Scratch(dir)
}
