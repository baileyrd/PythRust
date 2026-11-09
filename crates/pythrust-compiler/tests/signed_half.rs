//! Fatal numerator/fallback controls supplement the oracle corpus.
use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

#[test]
fn half_preserves_checked_overflow_and_fault_order() {
    let dir = common::scratch("signed_half", "overflow");
    let compiler =
        std::env::var_os("PYTHRUST_BIN").unwrap_or_else(|| env!("CARGO_BIN_EXE_pythrust").into());
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut cargo = Command::new(env!("CARGO"));
    cargo
        .current_dir(root)
        .args(["build", "--locked", "-p", "pythrust-rt"]);
    if Path::new(&compiler)
        .parent()
        .is_some_and(|p| p.ends_with("release"))
    {
        cargo.arg("--release");
    }
    let runtime = cargo.output().unwrap();
    assert!(runtime.status.success(), "{runtime:?}");
    let cases = [
        "print((9223372036854775807 + 1) // 2)\n",
        "print((9223372036854775807 + 1) % 2)\n",
        "def f(n: int, d: int) -> int:\n    return n // d\nprint(f(-9223372036854775808, -1))\n",
        "def f(n: int) -> int:\n    print(17)\n    return n + 1\nprint(f(9223372036854775807) // 2)\n",
        "def f(n: int) -> int:\n    print(17)\n    return n + 1\nprint(f(9223372036854775807) % 2)\n",
    ];
    for source in cases {
        let path = dir.join("input.pyr");
        fs::write(&path, source).unwrap();
        let interpreted = Command::new(&compiler)
            .args([Path::new("run"), &path])
            .output()
            .unwrap();
        assert_eq!(interpreted.status.code(), Some(1), "{interpreted:?}");
        assert!(String::from_utf8_lossy(&interpreted.stderr).contains("overflow"));
        let built = Command::new(&compiler)
            .args([Path::new("build"), &path])
            .output()
            .unwrap();
        assert!(built.status.success(), "{built:?}");
        let native = Command::new(path.with_extension(std::env::consts::EXE_EXTENSION))
            .output()
            .unwrap();
        assert_eq!(native.status.code(), Some(1), "{native:?}");
        assert_eq!(native.stdout, interpreted.stdout);
        assert_eq!(native.stderr, interpreted.stderr);
    }
}
