//! Native output publication, through the real compiler and system linker.

use super::{build_runtime, pythrust_bin};
use crate::common;
use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn build(dir: &Path, source: &Path, runtime: Option<&Path>) -> Output {
    let mut command = Command::new(pythrust_bin().unwrap());
    command.current_dir(dir).arg("build").arg(source);
    // Overrides belong to this child alone; parallel tests keep their runtime.
    command.env_remove("PYTHRUST_RT");
    if let Some(runtime) = runtime {
        command.env("PYTHRUST_RT", runtime);
    }
    command.output().unwrap()
}

fn success(output: Output) {
    assert!(output.status.success(), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
}

fn runs(executable: &Path, expected: &[u8]) {
    let output = Command::new(executable).output().unwrap();
    assert!(output.status.success(), "{output:?}");
    assert_eq!(output.stdout, expected);
    assert!(output.stderr.is_empty(), "{output:?}");
}

fn no_staging(dir: &Path) {
    for entry in fs::read_dir(dir).unwrap() {
        let name = entry.unwrap().file_name();
        assert!(
            !name.to_string_lossy().starts_with(".pythrust-build-"),
            "{name:?}"
        );
    }
}

/// A genuine static archive, deliberately lacking the runtime entry point.
fn missing_runtime(dir: &Path) -> std::path::PathBuf {
    let source = dir.join("unrelated.rs");
    fs::write(&source, "pub fn unrelated() {}\n").unwrap();
    let archive = dir.join(if cfg!(windows) {
        "unrelated.lib"
    } else {
        "unrelated.a"
    });
    let output = Command::new("rustc")
        .args(["--crate-name", "unrelated", "--crate-type", "staticlib"])
        .arg(source)
        .arg("-o")
        .arg(&archive)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    assert!(fs::read(&archive).unwrap().starts_with(b"!<arch>\n"));
    archive
}

#[test]
fn failed_native_link_preserves_executable_and_success_replaces_it() {
    build_runtime().unwrap();
    let tmp = common::scratch("link", "preserve");
    let archive = missing_runtime(&tmp);
    check_replacement(&tmp, &archive);
}

/// Exercise the same failure and success checks through a symlinked cwd,
/// as macOS's `/var` temporary roots are aliases of `/private/var`.
#[cfg(unix)]
#[test]
fn native_link_preserves_executable_through_a_symlinked_working_directory() {
    build_runtime().unwrap();
    let tmp = common::scratch("link", "symlinked_cwd");
    let archive = missing_runtime(&tmp);
    let actual = tmp.join("actual");
    let alias = tmp.join("alias");
    fs::create_dir(&actual).unwrap();
    std::os::unix::fs::symlink(&actual, &alias).unwrap();
    check_replacement(&alias, &archive);
}

fn check_replacement(root: &Path, archive: &Path) {
    // Object-like names must never collide with the private object file.
    for name in [
        "app",
        "program.o",
        "program.obj",
        "space é",
        "-option",
        "@response",
    ] {
        let dir = root.join(format!("nested {name}"));
        fs::create_dir(&dir).unwrap();
        let source = dir.join(format!("{name}.pyr"));
        let executable = source.with_extension(std::env::consts::EXE_EXTENSION);
        fs::write(&source, "print(42)\n").unwrap();
        // Relative source path, with spaces and option/response-file prefixes.
        let relative = Path::new(".").join(source.file_name().unwrap());
        success(build(&dir, &relative, None));
        runs(&executable, b"42\n");
        let previous = fs::read(&executable).unwrap();
        let permissions = fs::metadata(&executable).unwrap().permissions();
        fs::write(&source, "print(43)\n").unwrap();
        let output = build(&dir, &relative, Some(archive));
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        let error = String::from_utf8_lossy(&output.stderr);
        // Unix getcwd resolves directory aliases (including macOS /var).
        // Match the child's cwd identity, keeping the exact destination.
        let diagnostic = if cfg!(unix) {
            fs::canonicalize(&dir)
                .unwrap()
                .join(executable.file_name().unwrap())
        } else {
            executable.clone()
        };
        assert!(
            error.contains(&format!("linking {} failed:", diagnostic.display())),
            "{error}"
        );
        let lower = error.to_lowercase();
        assert!(
            lower.contains("undefined") || lower.contains("unresolved"),
            "{error}"
        );
        assert_eq!(fs::read(&executable).unwrap(), previous);
        assert_eq!(
            fs::metadata(&executable).unwrap().permissions(),
            permissions
        );
        runs(&executable, b"42\n");
        no_staging(&dir);
        success(build(&dir, &source, None));
        assert_ne!(fs::read(&executable).unwrap(), previous);
        runs(&executable, b"43\n");
        no_staging(&dir);
        fs::remove_file(&executable).unwrap();
        let output = build(&dir, &source, Some(archive));
        assert_eq!(output.status.code(), Some(1), "{output:?}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("linking "),
            "{output:?}"
        );
        assert!(!executable.exists());
        no_staging(&dir);
    }
}

#[test]
fn failed_native_publication_keeps_destination_and_cleans_staging() {
    build_runtime().unwrap();
    let tmp = common::scratch("link", "publication");
    let source = tmp.join("app.pyr");
    let executable = source.with_extension(std::env::consts::EXE_EXTENSION);
    fs::write(&source, "print(42)\n").unwrap();
    fs::create_dir(&executable).unwrap();
    let marker = executable.join("unrelated");
    fs::write(&marker, b"keep me").unwrap();
    let output = build(&tmp, &source, None);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("cannot publish"),
        "{output:?}"
    );
    assert_eq!(fs::read(&marker).unwrap(), b"keep me");
    no_staging(&tmp);
}

#[cfg(windows)]
#[test]
fn failed_native_publication_preserves_a_locked_executable() {
    use std::os::windows::fs::OpenOptionsExt;
    build_runtime().unwrap();
    let tmp = common::scratch("link", "locked");
    let source = tmp.join("app.pyr");
    let executable = source.with_extension(std::env::consts::EXE_EXTENSION);
    fs::write(&source, "print(42)\n").unwrap();
    success(build(&tmp, &source, None));
    let previous = fs::read(&executable).unwrap();
    let held = fs::OpenOptions::new()
        .read(true)
        // Permit inspection/reads, but deny deletion/replacement.
        .share_mode(0x1 | 0x2)
        .open(&executable)
        .unwrap();
    fs::write(&source, "print(43)\n").unwrap();
    let output = build(&tmp, &source, None);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("cannot publish"),
        "{output:?}"
    );
    no_staging(&tmp);
    drop(held);
    assert_eq!(fs::read(&executable).unwrap(), previous);
    runs(&executable, b"42\n");
}

#[cfg(unix)]
#[test]
fn native_publication_replaces_links_without_changing_their_targets() {
    build_runtime().unwrap();
    let tmp = common::scratch("link", "aliases");
    let source = tmp.join("app.pyr");
    let executable = source.with_extension(std::env::consts::EXE_EXTENSION);
    let unrelated = tmp.join("unrelated");
    fs::write(&source, "print(42)\n").unwrap();
    for symbolic in [true, false] {
        fs::write(&unrelated, b"keep me").unwrap();
        if symbolic {
            std::os::unix::fs::symlink(&unrelated, &executable).unwrap();
        } else {
            fs::hard_link(&unrelated, &executable).unwrap();
        }
        success(build(&tmp, &source, None));
        assert_eq!(fs::read(&unrelated).unwrap(), b"keep me");
        assert!(!fs::symlink_metadata(&executable)
            .unwrap()
            .file_type()
            .is_symlink());
        runs(&executable, b"42\n");
        no_staging(&tmp);
        fs::remove_file(&executable).unwrap();
    }
}
