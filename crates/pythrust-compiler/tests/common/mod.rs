//! Helpers the integration tests share: each test's scratch directory.

use std::fs;
use std::path::{Path, PathBuf};

/// Set to keep every test's scratch directory, as a failing test's is.
pub const KEEP: &str = "PYTHRUST_KEEP_SCRATCH";

/// A test's scratch directory, removed when the test ends. A failing
/// test's, or every one under [`KEEP`], stays, and its path is printed.
pub struct Scratch {
    path: PathBuf,
    /// Whether it goes when dropped outside a panic; otherwise only
    /// [`Scratch::pass`] removes it.
    remove: bool,
}

impl std::ops::Deref for Scratch {
    type Target = Path;

    fn deref(&self) -> &Path {
        &self.path
    }
}

impl Scratch {
    /// Ends a test that passed: the directory goes, unless under [`KEEP`].
    #[allow(dead_code)] // Used by the suites made with `scratch_until_pass`.
    pub fn pass(mut self) {
        self.remove = true;
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let failed = std::thread::panicking() || !self.remove;
        if failed || std::env::var_os(KEEP).is_some() {
            eprintln!("kept scratch directory {}", self.path.display());
            return;
        }
        if let Err(e) = fs::remove_dir_all(&self.path) {
            eprintln!("cannot remove {}: {e}", self.path.display());
        }
    }
}

/// A fresh directory `pythrust-<suite>-<pid>-<test>`, flat in the temp
/// dir: no parent shared with other tests, so none is created or removed
/// under a test running beside it.
pub fn scratch(suite: &str, test: &str) -> Scratch {
    Scratch {
        path: fresh(suite, test),
        remove: true,
    }
}

/// [`scratch`], for a test that returns errors: kept, its path printed,
/// unless the test calls [`Scratch::pass`].
#[allow(dead_code)] // Used by the suites whose tests return `Result`.
pub fn scratch_until_pass(suite: &str, test: &str) -> Scratch {
    Scratch {
        path: fresh(suite, test),
        remove: false,
    }
}

fn fresh(suite: &str, test: &str) -> PathBuf {
    let name = format!("pythrust-{suite}-{}-{test}", std::process::id());
    let dir = std::env::temp_dir().join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}
