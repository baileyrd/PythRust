//! Fatal paths through constant-divisor clones (D69) match `run` exactly.
use std::fs;
use std::path::Path;
use std::process::Command;

mod common;

#[test]
fn clones_keep_fatal_overflow_recursion_and_order() {
    let dir = common::scratch("const_divisor", "fatal");
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
        // Overflow inside a clone, after its own output.
        "def f(a: int, b: int) -> int:\n    print(a // b)\n    return (a // b) * a\nprint(f(9223372036854775807, 10))\n",
        // A pair clone: the overflow is in the second field.
        "def f(a: int, b: int) -> tuple[int, int]:\n    return a // b, a % b + a\nq, r = f(9223372036854775807, 10)\nprint(q, r)\n",
        // Unbounded recursion through the same clone keeps the depth limit.
        "def f(a: int, b: int) -> int:\n    return f(a // b + 1, 10)\nprint(f(5, 10))\n",
        // An uncaught ZeroDivisionError inside a clone.
        "def f(a: int, b: int) -> int:\n    return a // b + 1 // (a - a)\nprint(f(5, 10))\n",
        // An argument that fails before the call.
        "def f(a: int, b: int) -> int:\n    return a // b\nprint(f(9223372036854775807 + 1, 10))\n",
    ];
    for source in cases {
        let path = dir.join("input.pyr");
        fs::write(&path, source).unwrap();
        let interpreted = Command::new(&compiler)
            .args([Path::new("run"), &path])
            .output()
            .unwrap();
        assert_eq!(interpreted.status.code(), Some(1), "{interpreted:?}");
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
