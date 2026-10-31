//! Process-level regressions for the `pythrust` CLI: what one program
//! prints, on which stream, in what order and with what exit status,
//! through the whole pipeline (`check` and `run` as a user invokes them).
//! The corpus compares stdout and status only (`conformance.rs`); these
//! hold the cases its goldens cannot (roadmap phase-2 C/D/E and F).

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

/// A source file named for its test, so parallel tests never share one.
fn source(name: &str, src: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pythrust-cli-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.pyr"));
    fs::write(&path, src).unwrap();
    path
}

fn pythrust(command: &str, name: &str, src: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pythrust"))
        .arg(command)
        .arg(source(name, src))
        .output()
        .unwrap()
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// `total = 0` then `total += 1.5` in a loop is a compile error whether
/// the loop runs or not, so nothing prints either way; widening would
/// print `0` or `0.0` depending on the data.
#[test]
fn int_accumulator_errors_whatever_the_loop_runs() {
    for (name, items) in [("loop_empty", "[]"), ("loop_nonempty", "[1, 2]")] {
        let src = format!(
            "print(\"start\")\ntotal = 0\nfor x in {items}:\n    total += 1.5\nprint(total)\n"
        );
        for command in ["check", "run"] {
            let out = pythrust(command, name, &src);
            let err = text(&out.stderr);
            assert_eq!(out.status.code(), Some(1), "{command} {name}: {err}");
            assert_eq!(text(&out.stdout), "", "{command} {name}");
            assert!(err.contains("total") && err.contains("0.0"), "{err}");
        }
    }
}

/// `2 ** 64` overflows `int` at runtime; output already printed stays.
#[test]
fn int_power_overflow_is_a_runtime_error() {
    let src = "print(1)\nprint(2 ** 62)\nprint(2 ** 64)\nprint(3)\n";
    assert!(pythrust("check", "pow_overflow", src).status.success());
    let out = pythrust("run", "pow_overflow", src);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(text(&out.stdout), "1\n4611686018427387904\n");
    assert!(text(&out.stderr).contains("OverflowError"), "{out:?}");
}

/// A negative exponent the checker cannot see is a runtime error naming
/// the fix; a literal one is a compile error; a float base stays legal.
#[test]
fn negative_int_exponent_says_use_a_float_base() {
    let src = "print(2.0 ** -1)\nn = -1\nprint(2 ** n)\nprint(3)\n";
    assert!(pythrust("check", "pow_nonliteral", src).status.success());
    let out = pythrust("run", "pow_nonliteral", src);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(text(&out.stdout), "0.5\n");
    assert!(text(&out.stderr).contains("use a float base"), "{out:?}");

    let out = pythrust("run", "pow_literal", "print(1)\nprint(2 ** -1)\n");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(text(&out.stdout), "");
    assert!(text(&out.stderr).contains("use a float base"), "{out:?}");
}

/// With stdout and stderr on one file, output printed before a runtime
/// error comes before its diagnostic, as in CPython.
#[test]
fn output_precedes_the_runtime_error_on_one_stream() {
    let path = source("ordering", "print(\"before\")\nx = 0\nprint(1 // x)\n");
    let log = path.with_extension("log");
    let file = fs::File::create(&log).unwrap();
    let status = Command::new(env!("CARGO_BIN_EXE_pythrust"))
        .arg("run")
        .arg(&path)
        .stdout(Stdio::from(file.try_clone().unwrap()))
        .stderr(Stdio::from(file))
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(1));
    let both = fs::read_to_string(&log).unwrap();
    assert!(both.starts_with("before\n"), "{both}");
    assert!(both.contains("ZeroDivisionError"), "{both}");
}

/// Output fits the buffer, so the first write is the final flush; into a
/// pipe nobody reads it fails, which is a runtime error, not a success.
#[test]
#[cfg_attr(
    windows,
    ignore = "untested on Windows: a closed pipe fails a write differently there; the flush error path is the same code"
)]
fn a_failed_final_flush_is_a_runtime_error() {
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    let out = Command::new(env!("CARGO_BIN_EXE_pythrust"))
        .arg("run")
        .arg(source("flush", "print(1)\n"))
        .stdout(writer)
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    let err = text(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{err}");
    assert!(err.contains("cannot write output"), "{err}");
}

/// Prints each of `sys.argv` but the first, which names the program
/// differently under `run` (the file) and a built program (itself).
const ARGS: &str = "import sys\nprint(len(sys.argv))\ni = 1\nwhile i < len(sys.argv):\n    print(sys.argv[i])\n    i += 1\n";

/// `run <file> args...` passes the arguments after the file as
/// `sys.argv[1:]`, as `python3 <file> args...` does; `sys.argv[0]` is
/// the file.
#[test]
fn run_passes_the_arguments_after_the_file_to_sys_argv() {
    let path = source("argv_run", "import sys\nprint(sys.argv[0])\n");
    let out = Command::new(env!("CARGO_BIN_EXE_pythrust"))
        .arg("run")
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(text(&out.stdout), format!("{}\n", path.display()));

    let path = source("argv_args", ARGS);
    let out = Command::new(env!("CARGO_BIN_EXE_pythrust"))
        .args([
            "run".as_ref(),
            path.as_os_str(),
            "a b".as_ref(),
            "-x".as_ref(),
        ])
        .output()
        .unwrap();
    assert_eq!(text(&out.stdout), "3\na b\n-x\n", "{out:?}");

    let out = Command::new(env!("CARGO_BIN_EXE_pythrust"))
        .args(["check".as_ref(), path.as_os_str(), "a".as_ref()])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2), "{out:?}");
}

/// A Rust binding's stub, `fastfs.pyr` in a directory of its own, for
/// `--rust fastfs=<dir>` (M6b).
fn binding(test: &str, stub: &str) -> PathBuf {
    let dir = std::env::temp_dir()
        .join(format!("pythrust-cli-{}", std::process::id()))
        .join(format!("{test} binding"));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("fastfs.pyr"), stub).unwrap();
    dir
}

fn pythrust_rust(command: &str, dir: &PathBuf, name: &str, src: &str) -> Output {
    let mut flag = std::ffi::OsString::from("fastfs=");
    flag.push(dir);
    Command::new(env!("CARGO_BIN_EXE_pythrust"))
        .arg(command)
        .arg("--rust")
        .arg(flag)
        .arg(source(name, src))
        .output()
        .unwrap()
}

const FASTFS: &str = "\
temper def file_xxh3(path: str) -> str raises OSError: ...

temper def add(a: int, b: int) -> int:
    ...
";

/// `check` checks calls against a binding's stub; `run` and `build`
/// refuse with the named gaps until the native M6b PRs land.
#[test]
fn rust_binding_checks_and_run_and_build_name_their_gaps() {
    let dir = binding("gaps", FASTFS);
    let src = "\
import fastfs

def main() -> None:
    try:
        print(fastfs.file_xxh3(\"x\"), fastfs.add(b=2, a=1))
    except OSError as e:
        print(e)

print(\"start\")
main()
";
    let out = pythrust_rust("check", &dir, "rust_gaps", src);
    assert!(out.status.success(), "{out:?}");
    let out = pythrust_rust("run", &dir, "rust_gaps", src);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(text(&out.stdout), "");
    let err = text(&out.stderr);
    assert_eq!(
        err,
        "pythrust: run cannot call Rust bindings yet; use build\n"
    );
    let out = pythrust_rust("build", &dir, "rust_gaps", src);
    assert_eq!(out.status.code(), Some(1));
    let err = text(&out.stderr);
    assert!(err.contains("build cannot call Rust bindings yet"), "{err}");
    // Imported but never called, a binding still stops `build`.
    let out = pythrust_rust("build", &dir, "rust_uncalled", "import fastfs\nprint(1)\n");
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains("build cannot call Rust bindings yet"));
    // Given but not imported, a binding changes nothing.
    let out = pythrust_rust("run", &dir, "rust_unused", "print(1)\n");
    assert_eq!(text(&out.stdout), "1\n", "{out:?}");
}

/// A call that does not match the stub is a type error in the program;
/// an error in the stub renders against the stub's own file.
#[test]
fn rust_binding_errors_render_against_their_file() {
    let dir = binding("errors", FASTFS);
    let src = "import fastfs\nx: int = fastfs.add(1, \"a\")\n";
    let out = pythrust_rust("check", &dir, "rust_mismatch", src);
    let err = text(&out.stderr);
    assert!(err.contains("rust_mismatch.pyr:2:24: type error"), "{err}");
    assert!(err.contains("argument `b` of `fastfs.add`"), "{err}");

    let dir = binding("bad stub", "temper def f(mut x: int) -> int: ...\n");
    let out = pythrust_rust("check", &dir, "rust_bad_stub", "import fastfs\n");
    let err = text(&out.stderr);
    assert!(err.contains("fastfs.pyr:1:14: parse error"), "{err}");
    assert!(err.contains("not supported in stubs yet"), "{err}");
}

/// A stub with no functions is refused by every command, so `run` and
/// `build` cannot miss an import of it.
#[test]
fn rust_binding_without_functions_is_refused() {
    for (test, stub) in [("empty", ""), ("comments", "# nothing yet\n")] {
        let dir = binding(test, stub);
        for command in ["check", "run", "build"] {
            let name = format!("rust_{test}_{command}");
            let out = pythrust_rust(command, &dir, &name, "import fastfs\nprint(1)\n");
            assert_eq!(out.status.code(), Some(1), "{command}: {out:?}");
            assert_eq!(text(&out.stdout), "", "{command}");
            let err = text(&out.stderr);
            assert!(
                err.contains("the stub for `fastfs` declares no functions"),
                "{err}"
            );
        }
    }
}

/// `import` of a name no `--rust` gives says how to give one; a binding
/// named like a standard module is refused before anything is read.
#[test]
fn rust_binding_names_are_resolved() {
    let out = pythrust("check", "rust_not_given", "import fastfs\n");
    let err = text(&out.stderr);
    assert!(err.contains("needs `--rust fastfs=path`"), "{err}");

    let out = Command::new(env!("CARGO_BIN_EXE_pythrust"))
        .args(["check", "--rust", "os=anywhere"])
        .arg(source("rust_os", "import os\n"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let err = text(&out.stderr);
    assert!(err.contains("`os` is a standard module"), "{err}");
}
