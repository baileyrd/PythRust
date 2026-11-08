//! Process-level regressions for the `pythrust` CLI: what one program
//! prints, on which stream, in what order and with what exit status,
//! through the whole pipeline (`check` and `run` as a user invokes them).
//! The corpus compares stdout and status only (`conformance.rs`); these
//! hold the cases its goldens cannot (roadmap phase-2 C/D/E and F).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

mod common;

/// A scratch directory for `test`, gone when the test passes.
fn scratch(test: &str) -> common::Scratch {
    common::scratch("cli", test)
}

/// `src`, written as `dir/<name>.pyr`.
fn source(dir: &Path, name: &str, src: &str) -> PathBuf {
    fs::create_dir_all(dir).unwrap();
    let path = dir.join(format!("{name}.pyr"));
    fs::write(&path, src).unwrap();
    path
}

/// `pythrust <command>` of `src`, written as `dir/<name>.pyr`.
fn pythrust(command: &str, dir: &Path, name: &str, src: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_pythrust"))
        .arg(command)
        .arg(source(dir, name, src))
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
    let tmp = scratch("int_accumulator_errors_whatever_the_loop_runs");
    for (name, items) in [("loop_empty", "[]"), ("loop_nonempty", "[1, 2]")] {
        let src = format!(
            "print(\"start\")\ntotal = 0\nfor x in {items}:\n    total += 1.5\nprint(total)\n"
        );
        for command in ["check", "run"] {
            let out = pythrust(command, &tmp, name, &src);
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
    let tmp = scratch("int_power_overflow_is_a_runtime_error");
    let src = "print(1)\nprint(2 ** 62)\nprint(2 ** 64)\nprint(3)\n";
    assert!(pythrust("check", &tmp, "pow_overflow", src)
        .status
        .success());
    let out = pythrust("run", &tmp, "pow_overflow", src);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(text(&out.stdout), "1\n4611686018427387904\n");
    assert!(text(&out.stderr).contains("OverflowError"), "{out:?}");
}

/// A negative exponent the checker cannot see is a runtime error naming
/// the fix; a literal one is a compile error; a float base stays legal.
#[test]
fn negative_int_exponent_says_use_a_float_base() {
    let tmp = scratch("negative_int_exponent_says_use_a_float_base");
    let src = "print(2.0 ** -1)\nn = -1\nprint(2 ** n)\nprint(3)\n";
    assert!(pythrust("check", &tmp, "pow_nonliteral", src)
        .status
        .success());
    let out = pythrust("run", &tmp, "pow_nonliteral", src);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(text(&out.stdout), "0.5\n");
    assert!(text(&out.stderr).contains("use a float base"), "{out:?}");

    let out = pythrust("run", &tmp, "pow_literal", "print(1)\nprint(2 ** -1)\n");
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(text(&out.stdout), "");
    assert!(text(&out.stderr).contains("use a float base"), "{out:?}");
}

/// With stdout and stderr on one file, output printed before a runtime
/// error comes before its diagnostic, as in CPython.
#[test]
fn output_precedes_the_runtime_error_on_one_stream() {
    let tmp = scratch("output_precedes_the_runtime_error_on_one_stream");
    let path = source(
        &tmp,
        "ordering",
        "print(\"before\")\nx = 0\nprint(1 // x)\n",
    );
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
    let tmp = scratch("a_failed_final_flush_is_a_runtime_error");
    let (reader, writer) = std::io::pipe().unwrap();
    drop(reader);
    let out = Command::new(env!("CARGO_BIN_EXE_pythrust"))
        .arg("run")
        .arg(source(&tmp, "flush", "print(1)\n"))
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
    let tmp = scratch("run_passes_the_arguments_after_the_file_to_sys_argv");
    let path = source(&tmp, "argv_run", "import sys\nprint(sys.argv[0])\n");
    let out = Command::new(env!("CARGO_BIN_EXE_pythrust"))
        .arg("run")
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(text(&out.stdout), format!("{}\n", path.display()));

    let path = source(&tmp, "argv_args", ARGS);
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

/// A project for `test` (M6d): a `pythrust.toml` naming the binding
/// `fastfs`, whose directory holds only its stub `stub`.
fn project(tmp: &Path, test: &str, stub: &str) -> PathBuf {
    let dir = tmp.join(format!("{test} project"));
    fs::create_dir_all(dir.join("fastfs")).unwrap();
    fs::write(dir.join("fastfs/fastfs.pyr"), stub).unwrap();
    let manifest =
        "[package]\nname = \"p\"\nversion = \"0.1.0\"\n\n[rust]\nfastfs = { path = \"fastfs\" }\n";
    fs::write(dir.join("pythrust.toml"), manifest).unwrap();
    dir
}

/// An empty directory `test` in `tmp`, with no manifest in it.
fn scratch_dir(tmp: &Path, test: &str) -> PathBuf {
    let dir = tmp.join(test);
    fs::create_dir_all(&dir).unwrap();
    dir
}

const FASTFS: &str = "\
temper def file_xxh3(path: str) -> str raises OSError: ...

temper def add(a: int, b: int) -> int:
    ...
";

/// `check` checks calls against a binding's stub alone; `run` refuses
/// with its named gap, and `build` needs the binding's crate.
#[test]
fn rust_binding_checks_and_run_and_build_name_their_gaps() {
    let tmp = scratch("rust_binding_checks_and_run_and_build_name_their_gaps");
    let dir = project(&tmp, "gaps", FASTFS);
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
    let out = pythrust("check", &dir, "rust_gaps", src);
    assert!(out.status.success(), "{out:?}");
    let out = pythrust("run", &dir, "rust_gaps", src);
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(text(&out.stdout), "");
    let err = text(&out.stderr);
    assert_eq!(
        err,
        "pythrust: run cannot call Rust bindings yet; use build\n"
    );
    let out = pythrust("build", &dir, "rust_gaps", src);
    assert_eq!(out.status.code(), Some(1));
    let err = text(&out.stderr);
    assert!(err.contains("binding `fastfs`: no Cargo.toml in"), "{err}");
    // Named but not imported, a binding changes nothing.
    let out = pythrust("run", &dir, "rust_unused", "print(1)\n");
    assert_eq!(text(&out.stdout), "1\n", "{out:?}");
}

/// A call that does not match the stub is a type error in the program;
/// an error in the stub renders against the stub's own file.
#[test]
fn rust_binding_errors_render_against_their_file() {
    let tmp = scratch("rust_binding_errors_render_against_their_file");
    let dir = project(&tmp, "errors", FASTFS);
    let src = "import fastfs\nx: int = fastfs.add(1, \"a\")\n";
    let out = pythrust("check", &dir, "rust_mismatch", src);
    let err = text(&out.stderr);
    assert!(err.contains("rust_mismatch.pyr:2:24: type error"), "{err}");
    assert!(err.contains("argument `b` of `fastfs.add`"), "{err}");

    let dir = project(&tmp, "bad stub", "temper def f(mut x: int) -> int: ...\n");
    let out = pythrust("check", &dir, "rust_bad_stub", "import fastfs\n");
    let err = text(&out.stderr);
    assert!(err.contains("fastfs.pyr:1:14: parse error"), "{err}");
    assert!(err.contains("not supported in stubs yet"), "{err}");
}

/// A stub with no functions is refused by every command, so `run` and
/// `build` cannot miss an import of it.
#[test]
fn rust_binding_without_functions_is_refused() {
    let tmp = scratch("rust_binding_without_functions_is_refused");
    for (test, stub) in [("empty", ""), ("comments", "# nothing yet\n")] {
        let dir = project(&tmp, test, stub);
        for command in ["check", "run", "build"] {
            let name = format!("rust_{test}_{command}");
            let out = pythrust(command, &dir, &name, "import fastfs\nprint(1)\n");
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

/// `import` of a name no manifest gives says where it looked; a manifest
/// that does not name it says which one; a binding named like a standard
/// module is refused at its line in the manifest.
#[test]
fn rust_binding_names_are_resolved() {
    let tmp = scratch("rust_binding_names_are_resolved");
    let out = pythrust("check", &tmp, "rust_not_given", "import fastfs\n");
    assert_eq!(out.status.code(), Some(1));
    let err = text(&out.stderr);
    let named = "rust_not_given.pyr:1:8: parse error: `fastfs` is not a standard module, \
                 and no pythrust.toml names it";
    assert!(err.contains(named), "{err}");
    assert!(err.contains("or any directory above it)"), "{err}");

    let dir = project(&tmp, "unnamed", FASTFS);
    let out = pythrust("check", &dir, "main", "import sys, other\n");
    let err = text(&out.stderr);
    assert!(err.contains("main.pyr:1:13: parse error"), "{err}");
    assert!(
        err.contains("pythrust.toml has no `other` in `[rust]`"),
        "{err}"
    );

    let dir = project(&tmp, "os", FASTFS);
    let manifest = fs::read_to_string(dir.join("pythrust.toml")).unwrap();
    fs::write(
        dir.join("pythrust.toml"),
        format!("{manifest}os = {{ path = \"x\" }}\n"),
    )
    .unwrap();
    let out = pythrust("check", &dir, "main", "import fastfs\n");
    let err = text(&out.stderr);
    assert!(
        err.contains("pythrust.toml:7:1: manifest error: binding `os` is a standard module"),
        "{err}"
    );
}

/// The manifest is the first one above the program, and is read only
/// for a program importing a module that is not standard.
#[test]
fn the_manifest_is_found_above_and_read_only_when_needed() {
    let tmp = scratch("the_manifest_is_found_above_and_read_only_when_needed");
    let dir = project(&tmp, "above", FASTFS);
    let out = pythrust(
        "check",
        &dir.join("src/app"),
        "main",
        "import fastfs\nfastfs.add(1, 2)\n",
    );
    assert!(out.status.success(), "{out:?}");

    fs::write(dir.join("pythrust.toml"), "[package]\nname = 'p'\n").unwrap();
    let out = pythrust("run", &dir, "plain", "import sys\nprint(1)\n");
    assert_eq!(text(&out.stdout), "1\n", "{out:?}");
    let out = pythrust("check", &dir, "main", "import fastfs\n");
    let err = text(&out.stderr);
    assert!(
        err.contains("pythrust.toml:2:8: manifest error: literal strings"),
        "{err}"
    );

    fs::write(
        dir.join("pythrust.toml"),
        "[package]\nname = \"p\"\nversion = \"1\"\n[python]\n",
    )
    .unwrap();
    let out = pythrust("check", &dir, "main", "import fastfs\n");
    let err = text(&out.stderr);
    let reserved = "pythrust.toml:4:2: manifest error: `[python]` is reserved: Python packages arrive with M6e";
    assert!(err.contains(reserved), "{err}");
}

/// A program reached through `..` looks above its own directory, not
/// above the current one; a binding path may leave the project with
/// `..`; and a `[rust]` key that is not a plain name is refused at its
/// line. These tests assume no `pythrust.toml` above the temp dir.
#[test]
fn discovery_follows_the_programs_path_and_paths_may_climb() {
    let tmp = scratch("discovery_follows_the_programs_path_and_paths_may_climb");
    // `bare/proj` holds a manifest naming `fastfs`; `../real/main.pyr`
    // from there is in `bare/real`, with no manifest above it, so the
    // walk must not reach `bare/proj` through `..`.
    let bare = scratch_dir(&tmp, "climb bare");
    fs::create_dir_all(bare.join("real")).unwrap();
    fs::write(bare.join("real/main.pyr"), "import fastfs\n").unwrap();
    let decoy = project(&tmp, "climb decoy", FASTFS);
    let proj = bare.join("proj");
    fs::create_dir_all(&proj).unwrap();
    fs::copy(decoy.join("pythrust.toml"), proj.join("pythrust.toml")).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_pythrust"))
        .args(["check", "../real/main.pyr"])
        .current_dir(&proj)
        .output()
        .unwrap();
    let err = text(&out.stderr);
    // The current directory may be reported through a symlink (macOS's
    // `/var` is `/private/var`), so match the path's tail.
    let looked = format!(
        "climb bare{}real or any directory above it",
        std::path::MAIN_SEPARATOR
    );
    assert!(err.contains("none in ") && err.contains(&looked), "{err}");

    let root = project(&tmp, "climb", FASTFS);
    let shared = root.join("app");
    let manifest =
        "[package]\nname = \"app\"\nversion = \"1\"\n[rust]\nfastfs = { path = \"../fastfs\" }\n";
    fs::create_dir_all(&shared).unwrap();
    fs::write(shared.join("pythrust.toml"), manifest).unwrap();
    let out = pythrust(
        "check",
        &shared,
        "main",
        "import fastfs\nfastfs.add(1, 2)\n",
    );
    assert!(out.status.success(), "{out:?}");

    let bad = "[package]\nname = \"app\"\nversion = \"1\"\n[rust]\nfast-fs = { path = \"x\" }\n";
    fs::write(shared.join("pythrust.toml"), bad).unwrap();
    let out = pythrust("check", &shared, "main", "import fastfs\n");
    let err = text(&out.stderr);
    assert!(
        err.contains("pythrust.toml:5:1: manifest error: binding `fast-fs` is not a name"),
        "{err}"
    );
}

/// The walk follows the program's path as written: a symlinked directory
/// is not resolved, and a `..` after one takes off the link's name, not
/// its target's (`docs/grammar.md#manifest-pythrusttoml`).
#[cfg(unix)]
#[test]
fn discovery_does_not_resolve_symlinks() {
    let tmp = scratch("discovery_does_not_resolve_symlinks");
    let base = scratch_dir(&tmp, "symlinks");
    // The program lives in `proj/src`, under a manifest naming `fastfs`.
    let proj = project(&tmp, "symlinks proj", FASTFS);
    fs::create_dir_all(proj.join("src")).unwrap();
    fs::create_dir_all(proj.join("x")).unwrap();
    fs::write(proj.join("src/main.pyr"), "import fastfs\n").unwrap();
    fs::write(proj.join("x/main.pyr"), "import fastfs\nfastfs.add(1, 2)\n").unwrap();
    fs::create_dir_all(base.join("other")).unwrap();
    std::os::unix::fs::symlink(proj.join("src"), base.join("other/link")).unwrap();

    // Through the link, the walk sees `other/link`, `other`, ... and not `proj`.
    let out = Command::new(env!("CARGO_BIN_EXE_pythrust"))
        .arg("check")
        .arg(base.join("other/link/main.pyr"))
        .output()
        .unwrap();
    let err = text(&out.stderr);
    let looked = format!(
        "none in {} or any directory above it",
        base.join("other/link").display()
    );
    assert!(err.contains(&looked), "{err}");

    // `link/../x` is `other/x` as text, though the OS reads `proj/x`:
    // the manifest in `other` is the one used.
    let other = project(&tmp, "symlinks other", FASTFS);
    fs::copy(
        other.join("pythrust.toml"),
        base.join("other/pythrust.toml"),
    )
    .unwrap();
    copy_stub(&other, &base.join("other"));
    let out = Command::new(env!("CARGO_BIN_EXE_pythrust"))
        .arg("check")
        .arg(base.join("other/link/../x/main.pyr"))
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let used = format!(
        "pythrust: using {}",
        base.join("other/pythrust.toml").display()
    );
    assert!(text(&out.stderr).contains(&used), "{out:?}");
}

/// Copies `from/fastfs` to `to/fastfs`.
#[cfg(unix)]
fn copy_stub(from: &Path, to: &Path) {
    fs::create_dir_all(to.join("fastfs")).unwrap();
    fs::copy(from.join("fastfs/fastfs.pyr"), to.join("fastfs/fastfs.pyr")).unwrap();
}

/// fclone-lite's committed manifest names the binding its fast driver
/// imports, and `check` says which manifest it used.
#[test]
fn fclone_lites_manifest_checks_its_fast_driver() {
    let app = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/fclone-lite/fclone_fast.pyr");
    let out = Command::new(env!("CARGO_BIN_EXE_pythrust"))
        .arg("check")
        .arg(app)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let used = text(&out.stderr);
    assert!(used.starts_with("pythrust: using "), "{used}");
    assert!(used.trim_end().ends_with("pythrust.toml"), "{used}");
}

/// `--rust` is gone; its error says where bindings are named now.
#[test]
fn the_rust_flag_names_the_manifest() {
    let out = Command::new(env!("CARGO_BIN_EXE_pythrust"))
        .args(["check", "--rust", "a=b", "main.pyr"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(
        text(&out.stderr).contains("`[rust]` table of pythrust.toml"),
        "{out:?}"
    );
    let out = Command::new(env!("CARGO_BIN_EXE_pythrust"))
        .args(["check", "--rust=a=b", "main.pyr"])
        .output()
        .unwrap();
    assert!(text(&out.stderr).contains("pythrust.toml"), "{out:?}");
}

#[test]
fn loose_callee_stores_reach_tempered_call_checks() {
    let tmp = scratch("loose_callee_stores_reach_tempered_call_checks");
    let prefix = "temper def add(mut rows: list[list[int]], owned row: list[int]) -> None:\n    rows.append(row)\nrows: list[list[int]] = []\nrow = [1]\n";
    for (name, store) in [
        ("direct", "rows.append(row)\n"),
        ("captured", "def stash():\n    rows.append(row)\nstash()\n"),
        (
            "parameter",
            "def stash(x):\n    rows.append(x)\nstash(row)\n",
        ),
    ] {
        let src = format!("{prefix}{store}add(rows, row)\nprint(rows)\n");
        for command in ["check", "run", "build"] {
            let out = pythrust(command, &tmp, name, &src);
            let err = text(&out.stderr);
            assert_eq!(out.status.code(), Some(1), "{command} {name}: {err}");
            assert!(
                err.contains("tempered `add` keeps `row` after it returns"),
                "{err}"
            );
            assert!(err.contains("may also pass that list as `rows`"), "{err}");
            assert!(err.contains("pass a copy (`list(...)`)"), "{err}");
            assert!(out.stdout.is_empty(), "{command} {name}: {out:?}");
        }
    }
}

#[test]
fn joined_receiver_aliases_reject_before_any_execution() {
    let tmp = scratch("joined_receiver_aliases");
    for (name, receiver) in [
        ("conditional", "(a if c else b)"),
        ("or", "(b or a)"),
        ("and", "(a and b)"),
    ] {
        let src = format!("temper def f(c: bool) -> int:\n    a = [1]\n    b = a\n    {receiver}.append(2)\n    return len(a) + len(b)\nprint(f(True))\n");
        for command in ["check", "run", "build"] {
            let out = pythrust(command, &tmp, name, &src);
            assert_eq!(out.status.code(), Some(1), "{command} {name}: {out:?}");
            assert!(out.stdout.is_empty());
            let err = text(&out.stderr);
            assert!(
                err.contains("may be the same list and is used later")
                    && err.contains("copy instead of sharing"),
                "{command}: {err}"
            );
            assert!(!tmp
                .join(name)
                .with_extension(std::env::consts::EXE_EXTENSION)
                .exists());
        }
        let legal = src.replace("b = a", "b = [2]");
        assert!(pythrust("check", &tmp, name, &legal).status.success());
        let out = pythrust("run", &tmp, name, &legal);
        assert!(out.status.success(), "{out:?}");
        assert_eq!(text(&out.stdout), "3\n");
        let loose = src.replace("temper ", "");
        let out = pythrust("run", &tmp, name, &loose);
        assert!(out.status.success(), "{out:?}");
        assert_eq!(text(&out.stdout), "4\n");
    }
}
