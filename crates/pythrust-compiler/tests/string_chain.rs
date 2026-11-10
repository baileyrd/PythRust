//! Linker-instrumented string allocation/copy regressions; no production counter.
#![cfg(target_os = "linux")]

use std::path::Path;
use std::process::Command;

#[test]
fn chains_eliminate_prefix_allocations_and_copies_with_fallback() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let compiler = std::env::var("PYTHRUST_BIN")
        .unwrap_or_else(|_| env!("CARGO_BIN_EXE_pythrust").to_string());
    let release = Path::new(&compiler)
        .parent()
        .is_some_and(|p| p.ends_with("release"));
    let mut cargo = Command::new(env!("CARGO"));
    cargo
        .current_dir(&root)
        .args(["build", "--locked", "-p", "pythrust-rt"]);
    if release {
        cargo.arg("--release");
    }
    let built = cargo.output().expect("build runtime");
    assert!(
        built.status.success(),
        "{}",
        String::from_utf8_lossy(&built.stderr)
    );
    let python = std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string());
    let done = Command::new(python)
        .current_dir(root)
        .args([
            "bench/string_chains.py",
            "--after",
            &compiler,
            "--verify-only",
            "--runs",
            "1",
        ])
        .output()
        .expect("allocation probe");
    assert!(
        done.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&done.stderr),
        String::from_utf8_lossy(&done.stdout)
    );
}
