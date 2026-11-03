//! `pythrust build` of programs importing Rust bindings end to end (M6b
//! native plumbing, `docs/research/rust-interop.md#build`), each named in
//! a `pythrust.toml` beside the program, with every crates.io version
//! decided by the project's `pythrust.lock` (M6d,
//! `docs/research/package-tool.md`). Tests with registry dependencies
//! fetch `itoa` from crates.io, as a user's first `pythrust lock` would.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

mod common;
use common::{Scratch, KEEP};

/// A scratch directory for `test`; each holds a cargo build of about
/// 1 GB, so a passing test's goes.
fn scratch(test: &str) -> Scratch {
    common::scratch("interop", test)
}

#[test]
fn a_scratch_directory_goes_when_its_test_passes() {
    if std::env::var_os(KEEP).is_some() {
        return;
    }
    let dir = scratch("goes");
    fs::write(dir.join("file"), "x").unwrap();
    let path = dir.to_path_buf();
    drop(dir);
    assert!(!path.exists(), "{}", path.display());
}

#[test]
fn a_failing_tests_scratch_directory_stays() {
    let dir = scratch("stays");
    let path = dir.to_path_buf();
    // Unwinds as a failed assertion does, without the panic message.
    let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
        let _dir = dir;
        std::panic::resume_unwind(Box::new("the test fails"));
    }));
    assert!(failed.is_err());
    assert!(path.exists(), "{}", path.display());
    fs::remove_dir_all(&path).unwrap();
}

/// The committed probe binding.
fn probe() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/rust/probe")
}

/// A binding crate `name` in `dir`: `manifest` lines after `[package]`,
/// and a function `digits(x: int) -> int` counting `x`'s decimal digits
/// with `itoa` when `uses_itoa`.
fn binding(dir: &Path, name: &str, manifest: &str, uses_itoa: bool) -> PathBuf {
    let dir = dir.join(name);
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n{manifest}\n[workspace]\n"),
    )
    .unwrap();
    let body = match uses_itoa {
        true => "let mut b = itoa::Buffer::new();\n    b.format(x).len() as i64",
        false => "x.to_string().len() as i64",
    };
    fs::write(
        dir.join("src/lib.rs"),
        format!("pub fn digits(x: i64) -> i64 {{\n    {body}\n}}\n"),
    )
    .unwrap();
    fs::write(
        dir.join(format!("{name}.pyr")),
        "temper def digits(x: int) -> int: ...\n",
    )
    .unwrap();
    dir
}

/// `pythrust build` of `src` as `dir/main.pyr` with each `(name, dir)`
/// binding, then the program's output if the build succeeded.
fn build(dir: &Path, src: &str, bindings: &[(&str, &Path)]) -> (Output, Option<Output>) {
    build_as(dir, "main.pyr", src, bindings)
}

/// [`build`] of the file `dir/file`.
fn build_as(
    dir: &Path,
    file: &str,
    src: &str,
    bindings: &[(&str, &Path)],
) -> (Output, Option<Output>) {
    let built = compile(dir, file, src, bindings);
    let exe = dir
        .join(file)
        .with_extension(std::env::consts::EXE_EXTENSION);
    let ran = built
        .status
        .success()
        .then(|| Command::new(&exe).output().unwrap());
    (built, ran)
}

/// `src` as `dir/file` under a `pythrust.toml` naming `bindings`, then
/// `pythrust lock` and, if that succeeded, `pythrust build`, without
/// running the program: the output of the step that ran last.
fn compile(dir: &Path, file: &str, src: &str, bindings: &[(&str, &Path)]) -> Output {
    fs::write(dir.join(file), src).unwrap();
    fs::write(dir.join("pythrust.toml"), manifest(dir, bindings)).unwrap();
    let locked = lock(dir);
    match locked.status.success() {
        true => build_only(dir, file),
        false => locked,
    }
}

/// `pythrust`, with the embedded runtime: `PYTHRUST_RT_SRC` unset.
fn pythrust() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_pythrust"));
    c.env_remove("PYTHRUST_RT_SRC");
    c
}

/// `pythrust lock` in `dir`.
fn lock(dir: &Path) -> Output {
    pythrust().arg("lock").current_dir(dir).output().unwrap()
}

/// `pythrust build` of `dir/file`, as it stands.
fn build_only(dir: &Path, file: &str) -> Output {
    pythrust()
        .arg("build")
        .arg(dir.join(file))
        .output()
        .unwrap()
}

/// A `pythrust.toml` in `dir` naming each `(name, dir)` binding by its
/// path from `dir`. A binding outside `dir` is copied in as `dir/name`
/// first: a manifest's paths are relative, and on Windows the checkout
/// and the temp dir may be on different drives.
fn manifest(dir: &Path, bindings: &[(&str, &Path)]) -> String {
    let mut out = String::from("[package]\nname = \"t\"\nversion = \"0.1.0\"\n\n[rust]\n");
    for (name, binding) in bindings {
        let inside = match binding.strip_prefix(dir) {
            Ok(rel) => rel.to_path_buf(),
            Err(_) => {
                copy_tree(binding, &dir.join(name));
                PathBuf::from(name)
            }
        };
        let parts: Vec<String> = inside
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect();
        out.push_str(&format!("{name} = {{ path = \"{}\" }}\n", parts.join("/")));
    }
    out
}

/// Copies the binding `from` to `to`, leaving out any `target` dir.
fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            if entry.file_name() != "target" {
                copy_tree(&path, &to.join(entry.file_name()));
            }
        } else {
            fs::copy(&path, to.join(entry.file_name())).unwrap();
        }
    }
}

/// The one entry of `dir/.pythrust/<kind>/` whose name starts with
/// `prefix`, leaving out temporary ones.
fn only_entry(dir: &Path, kind: &str, prefix: &str) -> PathBuf {
    let found: Vec<PathBuf> = fs::read_dir(dir.join(".pythrust").join(kind))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            let name = p.file_name().unwrap().to_string_lossy();
            name.starts_with(prefix) && !name.starts_with('.')
        })
        .collect();
    assert_eq!(found.len(), 1, "{found:?}");
    found[0].clone()
}

/// The build directory `.pythrust/build/<stem>-<hash>/` of the program
/// `stem` in the project at `dir`.
fn build_dir(dir: &Path, stem: &str) -> PathBuf {
    only_entry(dir, "build", &format!("{stem}-"))
}

/// The extracted runtimes in the project at `dir`.
fn runtimes(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir.join(".pythrust/rt")) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .map(|e| e.unwrap().path())
        .filter(|p| !p.file_name().unwrap().to_string_lossy().starts_with('.'))
        .collect();
    found.sort();
    found
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// The program's stdout, failing with the build's output if it did not build.
fn stdout(result: (Output, Option<Output>)) -> String {
    let (built, ran) = result;
    let Some(ran) = ran else {
        panic!("build failed:\n{}", text(&built.stderr));
    };
    assert!(ran.status.success(), "{ran:?}");
    text(&ran.stdout)
}

/// The build's stderr, which must have failed, after the line naming
/// the manifest it used.
fn failure(result: (Output, Option<Output>)) -> String {
    let (built, ran) = result;
    assert!(ran.is_none(), "built: {}", text(&built.stderr));
    failed(&built)
}

/// The stderr of a `lock` or `build` that must have failed, after the
/// line naming the manifest it used.
fn failed(out: &Output) -> String {
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    let err = text(&out.stderr);
    let Some((first, rest)) = err.split_once('\n') else {
        panic!("no manifest line: {err}");
    };
    assert!(first.starts_with("pythrust: using "), "{err}");
    rest.to_string()
}

const DIGITS: &str = "import a\nprint(a.digits(12345))\n";

#[test]
fn build_calls_the_probe_and_builds_again() {
    let dir = scratch("probe");
    let src = "import probe\n\ndef main() -> None:\n    print(probe.double(21), probe.half(5.0))\n\nmain()\n";
    let probe = probe();
    assert_eq!(stdout(build(&dir, src, &[("probe", &probe)])), "42 2.5\n");
    // A warm rebuild still learns the native libraries it links.
    assert_eq!(stdout(build(&dir, src, &[("probe", &probe)])), "42 2.5\n");
    let generated = fs::read_to_string(build_dir(&dir, "main").join("lib.rs")).unwrap();
    assert!(
        generated.contains("fn pt_rs_5probe_6double(\n    p0: i64,\n"),
        "{generated}"
    );
}

/// A copy of the probe binding in `dir` with the stub `stub`.
fn probe_with_stub(dir: &Path, stub: &str) -> PathBuf {
    let probe = dir.join("probe");
    fs::create_dir_all(probe.join("src")).unwrap();
    for file in ["Cargo.toml", "src/lib.rs"] {
        fs::copy(self::probe().join(file), probe.join(file)).unwrap();
    }
    fs::write(probe.join("probe.pyr"), stub).unwrap();
    probe
}

/// Each mismatch rustc catches through the shim names the stub: a type,
/// a missing or an extra `raises`, a parameter Rust would keep past the
/// call, and an `unsafe fn`, which no shim may call. One program, so
/// each rebuild is warm.
#[test]
fn a_stub_that_does_not_match_its_function_names_the_stub() {
    let dir = scratch("mismatch");
    let cases = [
        (
            "temper def double(x: int) -> str: ...\n",
            "mismatched types",
        ),
        (
            "temper def size(path: str) -> int: ...\n",
            "mismatched types",
        ),
        (
            "temper def double(x: int) -> int raises OSError: ...\n",
            "mismatched types",
        ),
        (
            "temper def keep(text: str) -> int: ...\n",
            "does not live long enough",
        ),
        ("temper def danger(x: int) -> int: ...\n", "requires unsafe"),
        (
            "temper def keep_bytes(data: bytes) -> int: ...\n",
            "does not live long enough",
        ),
    ];
    for (stub, rustc) in cases {
        let name = stub["temper def ".len()..]
            .split('(')
            .next()
            .unwrap_or_default();
        let probe = probe_with_stub(&dir, &format!("{ENCODE}{stub}"));
        let src = format!("import probe\nprint(probe.{name}({}))\n", arg(stub));
        let err = failure(build(&dir, &src, &[("probe", &probe)]));
        let head = err.lines().next().unwrap_or_default();
        let want = format!("probe.pyr:2: the stub for `{name}` does not match the Rust function");
        assert!(head.ends_with(&want), "{stub}{err}");
        assert!(err.contains(rustc), "{stub}{err}");
    }
}

/// The first line of the mismatch tests' stubs, which makes `bytes`.
const ENCODE: &str = "temper def encode(text: str) -> bytes: ...\n";

/// An argument of the stub's first parameter's type.
fn arg(stub: &str) -> &'static str {
    let param = stub
        .split(':')
        .nth(1)
        .unwrap_or_default()
        .split(')')
        .next()
        .unwrap_or_default();
    match param.trim() {
        "int" => "1",
        "float" => "1.0",
        "str" => "\"x\"",
        _ => "probe.encode(\"x\")",
    }
}

/// Every type row crosses both ways, an `Err` raises the mapped
/// `OSError` that `try` catches, a scoped thread reads a `str` argument,
/// and the binding's output keeps program order.
const EVERY_TYPE: &str = "import probe

def main(path: str) -> None:
    print(probe.double(21), probe.half(5.0), probe.negate(False))
    print(probe.greet(\"pythrust\"))
    b = probe.reversed(probe.encode(\"abc\"))
    print(b, len(b))
    print(\"before\")
    print(probe.note(3))
    print(probe.scoped_len(\"h\\u00e9llo\"))
    tagged = probe.tag_all([\"a\", \"b\"])
    print(tagged, len(tagged))
    none: list[str] = []
    print(probe.tag_all(none))
    print(probe.size(path))
    try:
        print(probe.size(path + \".missing\"))
    except FileNotFoundError:
        print(\"caught a missing file\")
    try:
        print(probe.refuse(\"nope\"))
    except Exception as e:
        print(\"caught\", e)

main(PATH)
";

const EVERY_TYPE_OUT: &str = "42 2.5 True
hello, pythrust
b'cba' 3
before
note 3
None
6
['a!', 'b!'] 2
[]
5
caught a missing file
caught nope
";

/// [`EVERY_TYPE`] built in `dir` over a five-byte file there.
fn every_type(dir: &Path) -> (Output, Option<Output>) {
    let data = dir.join("data.txt");
    fs::write(&data, "12345").unwrap();
    let path = data.display().to_string().replace('\\', "/");
    let src = EVERY_TYPE.replace("PATH", &format!("{path:?}"));
    build(dir, &src, &[("probe", &probe())])
}

#[test]
fn every_type_crosses_and_errors_raise_os_error() {
    let dir = scratch("types");
    assert_eq!(stdout(every_type(&dir)), EVERY_TYPE_OUT);
}

/// The probe program under valgrind's memcheck when `PYTHRUST_VALGRIND=1`
/// (the memcheck CI job): no invalid access, no definitely lost block.
#[test]
fn every_type_runs_clean_under_memcheck() {
    if std::env::var_os("PYTHRUST_VALGRIND").is_none_or(|v| v != "1") {
        return;
    }
    let dir = scratch("memcheck");
    let (built, _) = every_type(&dir);
    assert!(built.status.success(), "{}", text(&built.stderr));
    let ran = Command::new("valgrind")
        .args([
            "--leak-check=full",
            "--errors-for-leak-kinds=definite",
            "--error-exitcode=97",
        ])
        .arg(dir.join("main"))
        .output()
        .unwrap();
    assert!(ran.status.success(), "{}", text(&ran.stderr));
    assert_eq!(text(&ran.stdout), EVERY_TYPE_OUT);
}

/// The `itoa` version `lock_text`, a `pythrust.lock` or a `Cargo.lock`,
/// pins, if it pins exactly one.
fn itoa_pin(lock_text: &str) -> Option<String> {
    let mut pins = lock_text
        .split("[[package]]")
        .filter(|entry| entry.contains("\nname = \"itoa\"\n"))
        .filter_map(|entry| entry.split("\nversion = \"").nth(1))
        .filter_map(|rest| rest.split('"').next());
    let first = pins.next()?.to_string();
    pins.next().is_none().then_some(first)
}

fn read(path: PathBuf) -> String {
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// A relock keeps every pin that still fits, even after the binding
/// widens its range; a lock deleted and made again starts from nothing,
/// so the old pin goes.
#[test]
fn a_relock_keeps_its_pins_and_a_new_lock_does_not() {
    let dir = scratch("pins");
    let a = binding(&dir, "a", "[dependencies]\nitoa = \"=1.0.1\"\n", true);
    assert_eq!(stdout(build(&dir, DIGITS, &[("a", &a)])), "5\n");
    let manifest = read(a.join("Cargo.toml"));
    fs::write(a.join("Cargo.toml"), manifest.replace("=1.0.1", "1.0")).unwrap();
    assert_eq!(stdout(build(&dir, DIGITS, &[("a", &a)])), "5\n");
    let locked = read(dir.join("pythrust.lock"));
    assert!(locked.starts_with("# Generated by pythrust lock; do not edit.\n"));
    assert_eq!(itoa_pin(&locked).as_deref(), Some("1.0.1"), "{locked}");
    let generated = read(build_dir(&dir, "main").join("Cargo.lock"));
    assert_eq!(itoa_pin(&generated).as_deref(), Some("1.0.1"));

    fs::remove_file(dir.join("pythrust.lock")).unwrap();
    assert!(lock(&dir).status.success());
    let fresh = itoa_pin(&read(dir.join("pythrust.lock"))).unwrap_or_default();
    assert!(fresh.starts_with("1.0.") && fresh != "1.0.1", "{fresh}");
}

#[test]
fn a_dev_dependency_drops_out_of_the_build() {
    let dir = scratch("dev");
    let a = binding(&dir, "a", "[dev-dependencies]\nitoa = \"0.4\"\n", false);
    assert_eq!(stdout(build(&dir, DIGITS, &[("a", &a)])), "5\n");
}

#[test]
fn two_bindings_pin_incompatible_versions_side_by_side() {
    let dir = scratch("two");
    let a = binding(&dir, "a", "[dependencies]\nitoa = \"0.4\"\n", true);
    let b = binding(&dir, "b", "[dependencies]\nitoa = \"1.0\"\n", true);
    let src = "import a\nimport b\nprint(a.digits(12), b.digits(-345))\n";
    assert_eq!(stdout(build(&dir, src, &[("a", &a), ("b", &b)])), "2 4\n");
}

/// Two bindings asking for one crate share one resolution: there is
/// nothing left to conflict.
#[test]
fn two_bindings_share_one_resolution() {
    let dir = scratch("share");
    let a = binding(&dir, "a", "[dependencies]\nitoa = \"=1.0.1\"\n", true);
    let b = binding(&dir, "b", "[dependencies]\nitoa = \"1.0\"\n", true);
    let src = "import a\nimport b\nprint(a.digits(1), b.digits(22))\n";
    assert_eq!(stdout(build(&dir, src, &[("a", &a), ("b", &b)])), "1 2\n");
    let locked = read(dir.join("pythrust.lock"));
    assert_eq!(itoa_pin(&locked).as_deref(), Some("1.0.1"), "{locked}");
}

/// One lock over every binding of `[rust]` builds a program importing
/// only some of them, and stays as it was.
#[test]
fn a_program_importing_some_bindings_builds_from_the_whole_lock() {
    let dir = scratch("unused");
    let a = binding(&dir, "a", "", false);
    let b = binding(&dir, "b", "[dependencies]\nitoa = \"1.0\"\n", true);
    fs::write(dir.join("main.pyr"), DIGITS).unwrap();
    fs::write(
        dir.join("pythrust.toml"),
        manifest(&dir, &[("a", &a), ("b", &b)]),
    )
    .unwrap();
    assert!(lock(&dir).status.success());
    let locked = read(dir.join("pythrust.lock"));
    assert!(itoa_pin(&locked).is_some(), "{locked}");
    let built = build_only(&dir, "main.pyr");
    assert!(built.status.success(), "{}", text(&built.stderr));
    assert_eq!(read(dir.join("pythrust.lock")), locked);
    // The program's own resolution holds only what it imports.
    let generated = read(build_dir(&dir, "main").join("Cargo.lock"));
    assert!(!generated.contains("name = \"b\""), "{generated}");
    assert_eq!(itoa_pin(&generated), None, "{generated}");
}

/// A build needs the lock, and names the command that writes it.
#[test]
fn a_build_without_a_lock_names_pythrust_lock() {
    let dir = scratch("nolock");
    let a = binding(&dir, "a", "", false);
    fs::write(dir.join("main.pyr"), DIGITS).unwrap();
    fs::write(dir.join("pythrust.toml"), manifest(&dir, &[("a", &a)])).unwrap();
    let err = failed(&build_only(&dir, "main.pyr"));
    assert_eq!(err, "pythrust: no pythrust.lock; run `pythrust lock`\n");
}

/// A binding that starts using a crate after the lock was made: the
/// build names the entries that changed and the command that fixes it,
/// and works once it is run.
#[test]
fn a_stale_lock_names_what_changed() {
    let dir = scratch("stale");
    let a = binding(&dir, "a", "", false);
    assert_eq!(stdout(build(&dir, DIGITS, &[("a", &a)])), "5\n");
    binding(&dir, "a", "[dependencies]\nitoa = \"1.0\"\n", true);
    let err = failed(&build_only(&dir, "main.pyr"));
    assert!(
        err.starts_with("pythrust: pythrust.lock is out of date; run `pythrust lock`\n  changed `a 0.1.0`\n  added `itoa 1.0."),
        "{err}"
    );
    assert!(lock(&dir).status.success());
    let built = build_only(&dir, "main.pyr");
    assert!(built.status.success(), "{}", text(&built.stderr));
}

/// The lock is checked for what each program imports: a binding that
/// outgrew a pin it shares fails its own programs as stale, while a
/// program importing only the other still builds from the lock as it is.
#[test]
fn staleness_is_checked_for_what_a_program_imports() {
    let dir = scratch("per-program");
    let a = binding(&dir, "a", "[dependencies]\nitoa = \"1.0\"\n", true);
    let b = binding(&dir, "b", "[dependencies]\nitoa = \"=1.0.1\"\n", true);
    let both = "import a\nimport b\nprint(a.digits(1), b.digits(22))\n";
    assert_eq!(stdout(build(&dir, both, &[("a", &a), ("b", &b)])), "1 2\n");
    binding(&dir, "b", "[dependencies]\nitoa = \"1.0.5\"\n", true);
    fs::write(dir.join("only_a.pyr"), DIGITS).unwrap();
    let built = build_only(&dir, "only_a.pyr");
    assert!(built.status.success(), "{}", text(&built.stderr));
    let err = failed(&build_only(&dir, "main.pyr"));
    assert!(
        err.starts_with("pythrust: pythrust.lock is out of date; run `pythrust lock`\n"),
        "{err}"
    );
}

/// A build compiles only the bindings its program imports: one added to
/// `[rust]` after the lock, and broken, does not stop it.
#[test]
fn a_broken_unlocked_binding_left_unimported_does_not_stop_a_build() {
    let dir = scratch("unimported");
    let a = binding(&dir, "a", "", false);
    assert_eq!(stdout(build(&dir, DIGITS, &[("a", &a)])), "5\n");
    let b = binding(&dir, "b", "", false);
    fs::write(b.join("Cargo.toml"), "[package\n").unwrap();
    fs::write(
        dir.join("pythrust.toml"),
        manifest(&dir, &[("a", &a), ("b", &b)]),
    )
    .unwrap();
    let built = build_only(&dir, "main.pyr");
    assert!(built.status.success(), "{}", text(&built.stderr));
}

/// A program importing a binding the lock does not hold gets the
/// stale-lock error naming it.
#[test]
fn an_imported_binding_missing_from_the_lock_is_stale() {
    let dir = scratch("unlocked");
    let a = binding(&dir, "a", "", false);
    assert_eq!(stdout(build(&dir, DIGITS, &[("a", &a)])), "5\n");
    let b = binding(&dir, "b", "", false);
    fs::write(
        dir.join("pythrust.toml"),
        manifest(&dir, &[("a", &a), ("b", &b)]),
    )
    .unwrap();
    fs::write(
        dir.join("both.pyr"),
        "import a\nimport b\nprint(a.digits(1), b.digits(22))\n",
    )
    .unwrap();
    let err = failed(&build_only(&dir, "both.pyr"));
    assert_eq!(
        err,
        "pythrust: pythrust.lock is out of date; run `pythrust lock`\n  added `b 0.1.0`\n"
    );
}

/// `pythrust lock --check` in `dir`.
fn lock_check(dir: &Path) -> Output {
    pythrust()
        .args(["lock", "--check"])
        .current_dir(dir)
        .output()
        .unwrap()
}

/// `pythrust lock --check` passes on a current lock and names each entry
/// that differs from a fresh lock of every binding otherwise, leaving
/// `pythrust.lock` as it is.
#[test]
fn lock_check_compares_the_lock_with_every_binding() {
    let dir = scratch("check");
    let a = binding(&dir, "a", "", false);
    fs::write(dir.join("pythrust.toml"), manifest(&dir, &[("a", &a)])).unwrap();
    let err = failed(&lock_check(&dir));
    assert_eq!(err, "pythrust: no pythrust.lock; run `pythrust lock`\n");
    assert!(lock(&dir).status.success());
    let locked = read(dir.join("pythrust.lock"));
    let ok = lock_check(&dir);
    assert!(ok.status.success(), "{}", text(&ok.stderr));
    assert!(
        text(&ok.stderr).ends_with("pythrust.lock is up to date\n"),
        "{}",
        text(&ok.stderr)
    );

    let b = binding(&dir, "b", "[dependencies]\nitoa = \"1.0\"\n", true);
    fs::write(
        dir.join("pythrust.toml"),
        manifest(&dir, &[("a", &a), ("b", &b)]),
    )
    .unwrap();
    let err = failed(&lock_check(&dir));
    assert!(
        err.starts_with("pythrust: pythrust.lock does not match the bindings; run `pythrust lock`\n  added `b 0.1.0`\n  added `itoa 1.0."),
        "{err}"
    );
    assert_eq!(read(dir.join("pythrust.lock")), locked);

    assert!(lock(&dir).status.success());
    fs::write(dir.join("pythrust.toml"), manifest(&dir, &[("a", &a)])).unwrap();
    let err = failed(&lock_check(&dir));
    assert!(err.contains("\n  removed `b 0.1.0`\n"), "{err}");
}

/// A build outside the checkout extracts the embedded runtime once into
/// the project, builds against that copy, and rewrites a copy that no
/// longer matches.
#[test]
fn the_embedded_runtime_is_extracted_and_repaired() {
    let dir = scratch("rt");
    let a = binding(&dir, "a", "", false);
    assert_eq!(stdout(build(&dir, DIGITS, &[("a", &a)])), "5\n");
    let [rt] = runtimes(&dir).try_into().unwrap();
    let name = rt.file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        name.starts_with(&format!("{}-", env!("CARGO_PKG_VERSION"))),
        "{name}"
    );
    let manifest = read(rt.join("Cargo.toml"));
    assert!(
        manifest.ends_with("\n[workspace]\n") && !manifest.contains(".workspace"),
        "{manifest}"
    );
    let generated = read(build_dir(&dir, "main").join("Cargo.toml"));
    let rt_path = rt.to_string_lossy().replace('\\', "/");
    assert!(
        generated.replace("\\\\", "/").contains(&rt_path),
        "{generated}"
    );

    let lib = rt.join("src/lib.rs");
    let original = read(lib.clone());
    fs::write(&lib, "broken").unwrap();
    fs::remove_file(rt.join("src/abi.rs")).unwrap();
    fs::write(rt.join("build.rs"), "fn main() {}\n").unwrap();
    let built = build_only(&dir, "main.pyr");
    assert!(built.status.success(), "{}", text(&built.stderr));
    assert_eq!(read(lib), original);
    assert!(rt.join("src/abi.rs").is_file());
    assert!(!rt.join("build.rs").exists());
    assert_eq!(runtimes(&dir), std::slice::from_ref(&rt));

    let hash = name.rsplit('-').next().unwrap();
    let tmp = rt.with_file_name(format!(".{hash}.tmp"));
    fs::remove_dir_all(&rt).unwrap();
    fs::write(&rt, "not a directory").unwrap();
    fs::write(&tmp, "not a directory").unwrap();
    let built = build_only(&dir, "main.pyr");
    assert!(built.status.success(), "{}", text(&built.stderr));
    assert_eq!(read(rt.join("src/lib.rs")), original);
    assert!(!tmp.exists());
}

/// Builds at once share one extraction: three of two programs into a
/// fresh project, then two of one program while the copy needs repair.
#[test]
fn builds_at_once_extract_and_repair_the_runtime_in_turn() {
    let dir = scratch("rt-turns");
    let a = binding(&dir, "a", "", false);
    fs::write(dir.join("pythrust.toml"), manifest(&dir, &[("a", &a)])).unwrap();
    fs::write(dir.join("main.pyr"), DIGITS).unwrap();
    fs::write(dir.join("other.pyr"), "import a\nprint(a.digits(7))\n").unwrap();
    assert!(lock(&dir).status.success());
    fs::remove_dir_all(dir.join(".pythrust")).unwrap();
    let spawn = |file: &str| {
        pythrust()
            .arg("build")
            .arg(dir.join(file))
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap()
    };
    let all_succeed = |runs: Vec<std::process::Child>| {
        for run in runs {
            let out = run.wait_with_output().unwrap();
            assert!(out.status.success(), "{}", text(&out.stderr));
        }
    };
    all_succeed(vec![
        spawn("main.pyr"),
        spawn("other.pyr"),
        spawn("main.pyr"),
    ]);
    let [rt] = runtimes(&dir).try_into().unwrap();
    fs::write(rt.join("src/lib.rs"), "broken").unwrap();
    all_succeed(vec![spawn("main.pyr"), spawn("main.pyr")]);
    assert_eq!(runtimes(&dir), std::slice::from_ref(&rt));
    assert_ne!(read(rt.join("src/lib.rs")), "broken");
    let ran = Command::new(dir.join("other")).output().unwrap();
    assert_eq!(text(&ran.stdout), "1\n");
}

/// `PYTHRUST_RT_SRC` is announced once, read at the start of the build,
/// and extracted by its own hash: the same files share the embedded
/// copy, and a changed file gets a copy of its own.
#[test]
fn an_override_runtime_is_keyed_by_its_source() {
    let dir = scratch("rt-override");
    let a = binding(&dir, "a", "", false);
    assert_eq!(stdout(build(&dir, DIGITS, &[("a", &a)])), "5\n");
    let [embedded] = runtimes(&dir).try_into().unwrap();
    let src = dir.join("rt-src");
    copy_tree(&embedded, &src);
    let with_override = || {
        pythrust()
            .arg("build")
            .arg(dir.join("main.pyr"))
            .env("PYTHRUST_RT_SRC", &src)
            .output()
            .unwrap()
    };
    let out = with_override();
    assert!(out.status.success(), "{}", text(&out.stderr));
    let err = text(&out.stderr);
    let line = format!(
        "pythrust: runtime source from {} (PYTHRUST_RT_SRC)\n",
        src.display()
    );
    assert_eq!(err.matches(&line).count(), 1, "{err}");
    assert_eq!(runtimes(&dir), std::slice::from_ref(&embedded));

    let lib = src.join("src/lib.rs");
    fs::write(&lib, read(lib.clone()) + "// changed\n").unwrap();
    let out = with_override();
    assert!(out.status.success(), "{}", text(&out.stderr));
    let copies = runtimes(&dir);
    assert_eq!(copies.len(), 2, "{copies:?}");
    let changed = copies.iter().find(|c| **c != embedded).unwrap();
    assert!(read(changed.join("src/lib.rs")).ends_with("// changed\n"));
}

/// A relative `PYTHRUST_RT_SRC` naming the runtime as the repository
/// writes it, inheriting from a workspace above: made standalone, it is
/// the embedded runtime, so it shares that copy.
#[test]
fn an_override_inheriting_from_its_workspace_is_made_standalone() {
    let dir = scratch("rt-inherit");
    let a = binding(&dir, "a", "", false);
    assert_eq!(stdout(build(&dir, DIGITS, &[("a", &a)])), "5\n");
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let ws = dir.join("ws");
    let rt = ws.join("crates/rt");
    copy_tree(&repo.join("crates/pythrust-rt"), &rt);
    fs::copy(repo.join("Cargo.toml"), ws.join("Cargo.toml")).unwrap();
    let out = pythrust()
        .arg("build")
        .arg(dir.join("main.pyr"))
        .current_dir(&rt)
        .env("PYTHRUST_RT_SRC", "../rt")
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out.stderr));
    assert_eq!(runtimes(&dir).len(), 1, "{:?}", runtimes(&dir));
}

/// A lock that cannot be resolved leaves the previous one as it was.
#[test]
fn a_failed_lock_keeps_the_old_one() {
    let dir = scratch("keep");
    let a = binding(&dir, "a", "", false);
    assert_eq!(stdout(build(&dir, DIGITS, &[("a", &a)])), "5\n");
    let before = read(dir.join("pythrust.lock"));
    binding(&dir, "a", "[dependencies]\nitoa = \"=99.0.0\"\n", true);
    let err = failed(&lock(&dir));
    assert!(
        err.starts_with("pythrust: cannot resolve the bindings' dependencies\n"),
        "{err}"
    );
    assert_eq!(read(dir.join("pythrust.lock")), before);
}

/// `pythrust lock` finds the manifest above the current directory, and
/// names where it looked when there is none.
#[test]
fn lock_finds_the_manifest_above() {
    let dir = scratch("above");
    let a = binding(&dir, "a", "", false);
    fs::write(dir.join("pythrust.toml"), manifest(&dir, &[("a", &a)])).unwrap();
    let sub = dir.join("src/app");
    fs::create_dir_all(&sub).unwrap();
    let out = lock(&sub);
    assert!(out.status.success(), "{}", text(&out.stderr));
    let err = text(&out.stderr);
    assert!(err.starts_with("pythrust: using "), "{err}");
    assert!(dir.join("pythrust.lock").is_file());

    // A manifest with no `[rust]` locks just the runtime.
    fs::write(
        dir.join("pythrust.toml"),
        "[package]\nname = \"p\"\nversion = \"1\"\n",
    )
    .unwrap();
    assert!(lock(&sub).status.success());
    assert!(read(dir.join("pythrust.lock")).contains("name = \"pythrust-rt\""));

    let bare = scratch("bare");
    let out = lock(&bare);
    assert_eq!(out.status.code(), Some(1));
    let err = text(&out.stderr);
    assert!(err.starts_with("pythrust: no pythrust.toml in "), "{err}");
}

/// A binding inside another workspace is refused by `pythrust lock`: that
/// workspace's lock, not the project's, would decide its versions.
#[test]
fn a_binding_must_be_its_own_workspace() {
    let dir = scratch("member");
    fs::write(dir.join("Cargo.toml"), "[workspace]\nmembers = [\"a\"]\n").unwrap();
    let a = binding(&dir, "a", "", false);
    let manifest = read(a.join("Cargo.toml"));
    fs::write(a.join("Cargo.toml"), manifest.replace("[workspace]\n", "")).unwrap();
    let err = failure(build(&dir, DIGITS, &[("a", &a)]));
    assert!(
        err.starts_with("pythrust: binding `a` is part of the workspace at "),
        "{err}"
    );
}

/// A git package is refused: by `pythrust lock`, which keeps the lock it
/// had, and in a lock written by hand.
#[test]
fn a_git_dependency_is_refused() {
    let dir = scratch("git");
    let g = binding(&dir, "g", "", false);
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .current_dir(&g)
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
    };
    git(&["init", "-q"]);
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "g"]);
    let url = format!(
        "file:///{}",
        g.display()
            .to_string()
            .replace('\\', "/")
            .trim_start_matches('/')
    );
    let a = binding(&dir, "a", "", false);
    assert_eq!(stdout(build(&dir, DIGITS, &[("a", &a)])), "5\n");
    let before = read(dir.join("pythrust.lock"));
    let git_dep = format!("[dependencies]\ng = {{ git = \"{url}\" }}\n");
    binding(&dir, "a", &git_dep, false);
    let err = failed(&lock(&dir));
    assert!(err.ends_with("pythrust.lock: git dependencies are not supported yet; use a crates.io version (`g`)\n"), "{err}");
    assert_eq!(read(dir.join("pythrust.lock")), before);

    let by_hand = "version = 4\n\n[[package]]\nname = \"g\"\nversion = \"0.1.0\"\nsource = \"git+https://example.invalid/g#0123\"\n";
    fs::write(dir.join("pythrust.lock"), by_hand).unwrap();
    let err = failed(&build_only(&dir, "main.pyr"));
    assert!(err.ends_with("pythrust.lock: git dependencies are not supported yet; use a crates.io version (`g`)\n"), "{err}");
}

/// Two `pythrust lock` runs at once, and one beside a running build: the
/// project lock puts them in turn, so each succeeds and the lock is whole.
#[test]
fn locks_and_builds_at_once_take_turns() {
    let dir = scratch("turns");
    let a = binding(&dir, "a", "[dependencies]\nitoa = \"1.0\"\n", true);
    let src = "import a\nprint(a.digits(12345))\n";
    assert_eq!(stdout(build(&dir, src, &[("a", &a)])), "5\n");
    let locked = read(dir.join("pythrust.lock"));
    fs::remove_dir_all(dir.join(".pythrust")).unwrap();

    let spawn = |args: &[&str]| {
        pythrust()
            .args(args)
            .current_dir(&*dir)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap()
    };
    let runs = [
        spawn(&["build", "main.pyr"]),
        spawn(&["lock"]),
        spawn(&["lock"]),
        spawn(&["build", "main.pyr"]),
    ];
    for run in runs {
        let out = run.wait_with_output().unwrap();
        assert!(out.status.success(), "{}", text(&out.stderr));
    }
    assert_eq!(read(dir.join("pythrust.lock")), locked);
    let ran = Command::new(dir.join("main")).output().unwrap();
    assert_eq!(text(&ran.stdout), "5\n");
}

/// A panic in a binding ends the program with status 1, after what the
/// program printed before the call, and without aborting.
#[test]
fn a_panic_in_a_binding_keeps_earlier_output_and_exits_1() {
    let dir = scratch("panic");
    let src = "import probe\nprint(\"before\")\nprint(probe.boom(1))\nprint(\"after\")\n";
    let (built, ran) = build(&dir, src, &[("probe", &probe())]);
    let Some(ran) = ran else {
        panic!("build failed:\n{}", text(&built.stderr));
    };
    assert_eq!(ran.status.code(), Some(1), "{ran:?}");
    assert_eq!(text(&ran.stdout), "before\n");
    let err = text(&ran.stderr);
    // Only Pythrust's diagnostic, at the call: Rust's hook stays quiet.
    assert!(
        err.contains("main.pyr:3:7: runtime error: Rust panic: boom 1\n"),
        "{err}"
    );
    assert!(!err.contains("panicked"), "{err}");
    assert_eq!(err.lines().count(), 3, "{err}");
}

/// A panic outside the call's own thread, in a thread the binding
/// spawns, still prints Rust's message; the call goes on.
#[test]
fn a_panic_in_a_bindings_thread_still_prints() {
    let dir = scratch("worker");
    let (built, ran) = build(
        &dir,
        "import probe\nprint(probe.worker_panics())\n",
        &[("probe", &probe())],
    );
    let Some(ran) = ran else {
        panic!("build failed:\n{}", text(&built.stderr));
    };
    assert!(ran.status.success(), "{ran:?}");
    assert_eq!(text(&ran.stdout), "True\n");
    let err = text(&ran.stderr);
    assert!(
        err.contains("panicked") && err.contains("worker gave up"),
        "{err}"
    );
}

/// A binding's own output comes where the call is in the program.
#[test]
fn a_bindings_output_keeps_program_order() {
    let dir = scratch("order");
    let src = "import probe\nprint(\"a\")\nx = probe.say(1)\nprint(\"c\", x)\n";
    let out = stdout(build(&dir, src, &[("probe", &probe())]));
    assert_eq!(out, "a\nrust says 1\nc 1\n");
}

/// `a.pyr` and `a.b.pyr` in one directory get their own generated crates.
#[test]
fn programs_named_alike_build_apart() {
    let dir = scratch("alike");
    let probe = probe();
    let a = "import probe\nprint(probe.double(1))\n";
    let ab = "import probe\nprint(probe.double(2))\n";
    assert_eq!(
        stdout(build_as(&dir, "a.pyr", a, &[("probe", &probe)])),
        "2\n"
    );
    assert_eq!(
        stdout(build_as(&dir, "a.b.pyr", ab, &[("probe", &probe)])),
        "4\n"
    );
    assert!(build_dir(&dir, "a").join("lib.rs").is_file());
    assert!(build_dir(&dir, "a.b").join("lib.rs").is_file());
    // Programs of one name in two directories build apart.
    for sub in ["x", "y"] {
        fs::create_dir_all(dir.join(sub)).unwrap();
        let src = "import probe\nprint(probe.double(4))\n";
        let built = compile(&dir, &format!("{sub}/main.pyr"), src, &[("probe", &probe)]);
        assert!(built.status.success(), "{}", text(&built.stderr));
    }
    // Another spelling of one program's path is the same build.
    let again = build_only(&dir, "y/../x/main.pyr");
    assert!(again.status.success(), "{}", text(&again.stderr));
    let builds = fs::read_dir(dir.join(".pythrust/build")).unwrap();
    let mains = builds
        .filter(|e| {
            e.as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("main-")
        })
        .count();
    assert_eq!(mains, 2);
    // Programs named like the project's own files build apart from them.
    for name in ["lock", "project.lock"] {
        let src = "import probe\nprint(probe.double(3))\n";
        let file = format!("{name}.pyr");
        assert_eq!(
            stdout(build_as(&dir, &file, src, &[("probe", &probe)])),
            "6\n"
        );
    }
}

/// A binding's package may be named apart from the binding.
#[test]
fn a_binding_package_may_have_another_name() {
    let dir = scratch("package");
    let a = binding(&dir, "a", "", false);
    let text = fs::read_to_string(a.join("Cargo.toml")).unwrap();
    fs::write(
        a.join("Cargo.toml"),
        text.replace("name = \"a\"", "name = \"a-binding\""),
    )
    .unwrap();
    assert_eq!(stdout(build(&dir, DIGITS, &[("a", &a)])), "5\n");
}

/// A search path a build script adds reaches the final link, beside a
/// library from the default paths (as the `cc` crate adds `OUT_DIR` and
/// links the C++ runtime).
#[test]
fn a_build_script_search_path_reaches_the_link() {
    let dir = scratch("search");
    let a = binding(&dir, "a", "build = \"build.rs\"\n", false);
    let lib = if cfg!(windows) {
        ""
    } else {
        "    println!(\"cargo:rustc-link-lib=dylib=m\");\n"
    };
    fs::write(
        a.join("build.rs"),
        format!(
            "fn main() {{\n    let out = std::env::var(\"OUT_DIR\").unwrap();\n    \
             println!(\"cargo:rustc-link-search=native={{out}}\");\n{lib}}}\n"
        ),
    )
    .unwrap();
    assert_eq!(stdout(build(&dir, DIGITS, &[("a", &a)])), "5\n");
}

/// A binding at `version` whose build script builds `libptseven.a`, with
/// `seven()` returning `value`, in a directory it adds only as a search
/// path; the binding names the library in a `#[link]` attribute, so only
/// the final link finds it.
#[cfg(target_os = "linux")]
fn seven_binding(dir: &Path, version: &str, value: i64) -> PathBuf {
    let a = binding(dir, "a", "build = \"build.rs\"\n", false);
    let text = fs::read_to_string(a.join("Cargo.toml")).unwrap();
    fs::write(
        a.join("Cargo.toml"),
        text.replace("version = \"0.1.0\"", &format!("version = \"{version}\"")),
    )
    .unwrap();
    fs::write(
        a.join("build.rs"),
        format!(
            "use std::process::Command;\n\
             fn main() {{\n    \
                 let out = std::env::var(\"OUT_DIR\").unwrap();\n    \
                 std::fs::write(format!(\"{{out}}/seven.c\"), \"long seven(void) {{ return {value}; }}\\n\").unwrap();\n    \
                 let cc = Command::new(\"cc\").args([\"-c\", \"-fPIC\", \"-o\"]).arg(format!(\"{{out}}/seven.o\")).arg(format!(\"{{out}}/seven.c\")).status().unwrap();\n    \
                 assert!(cc.success());\n    \
                 let ar = Command::new(\"ar\").arg(\"rcs\").arg(format!(\"{{out}}/libptseven.a\")).arg(format!(\"{{out}}/seven.o\")).status().unwrap();\n    \
                 assert!(ar.success());\n    \
                 println!(\"cargo:rustc-link-search=native={{out}}\");\n\
             }}\n"
        ),
    )
    .unwrap();
    fs::write(
        a.join("src/lib.rs"),
        "#[link(name = \"ptseven\")]\nunsafe extern \"C\" {\n    fn seven() -> i64;\n}\n\n\
         pub fn digits(x: i64) -> i64 {\n    x + unsafe { seven() }\n}\n",
    )
    .unwrap();
    a
}

/// A library only in a build script's search path reaches the final
/// link, and a warm rebuild after a version bump links this build's
/// library, not one an older build left in the target directory.
#[cfg(target_os = "linux")]
#[test]
fn the_link_uses_this_builds_search_paths() {
    let dir = scratch("onlypath");
    let a = seven_binding(&dir, "0.1.0", 7);
    assert_eq!(stdout(build(&dir, DIGITS, &[("a", &a)])), "12352\n");
    let a = seven_binding(&dir, "0.2.0", 8);
    assert_eq!(stdout(build(&dir, DIGITS, &[("a", &a)])), "12353\n");
    let builds = fs::read_dir(build_dir(&dir, "main").join("target"))
        .unwrap()
        .flatten()
        .filter_map(|t| fs::read_dir(t.path().join("debug/build")).ok())
        .flatten()
        .flatten()
        .filter(|e| e.path().join("out/libptseven.a").is_file())
        .count();
    assert!(
        builds >= 2,
        "the old build's library stays in the target dir"
    );
}

/// The driver's groups, as a set: each `N bytes:` block with its paths
/// sorted, since `fclone_lite.py` orders a size's groups by SHA-256 and
/// `fclone_fast` by xxh3.
#[cfg(unix)]
fn group_set(out: &str) -> std::collections::BTreeSet<Vec<String>> {
    out.split("\n\n")
        .filter(|block| !block.trim().is_empty())
        .map(|block| {
            let mut lines: Vec<String> = block.lines().map(str::to_string).collect();
            lines[1..].sort();
            lines
        })
        .collect()
}

/// `fclone_fast` (`apps/fclone-lite`) finds the groups `fclone_lite.py`
/// finds under CPython: two duplicated payloads of one size, an empty
/// pair, and a file it cannot read, which both skip. The unreadable file
/// is mode 000; when that does not stop this user (root), both programs
/// run as `nobody`, so the case holds for root too. Unix only: the
/// programs print `/`-joined paths, as `fclone_lite_matches_cpython`.
#[cfg(unix)]
#[test]
fn fclone_fast_finds_the_groups_fclone_lite_finds() {
    use std::os::unix::fs::PermissionsExt;
    use std::os::unix::process::CommandExt;

    let dir = scratch("fclone-fast");
    let root = dir.join("tree");
    fs::create_dir_all(root.join("a/b")).unwrap();
    let files = [
        ("one", "ab"),
        ("a/two", "ab"),
        ("a/b/three", "cd"),
        ("four", "cd"),
        ("five", "ef"),
        ("empty", ""),
        ("a/empty", ""),
        ("locked", "ab"),
    ];
    for (name, text) in files {
        fs::write(root.join(name), text).unwrap();
    }
    let locked = root.join("locked");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let as_nobody = fs::read(&locked).is_ok();
    let apps = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/fclone-lite");
    let oracle = dir.join("fclone_lite.py");
    fs::copy(apps.join("fclone_lite.py"), &oracle).unwrap();
    let src = fs::read_to_string(apps.join("fclone_fast.pyr")).unwrap();
    let built = compile(
        &dir,
        "fclone_fast.pyr",
        &src,
        &[("fastfs", &apps.join("fastfs"))],
    );
    assert!(built.status.success(), "{}", text(&built.stderr));
    let python = std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string());
    let run = |command: &mut Command| -> String {
        if as_nobody {
            command.uid(65534).gid(65534);
        }
        let out = command.arg(&root).output().unwrap();
        assert!(out.status.success(), "{out:?}");
        text(&out.stdout)
    };
    let want = run(Command::new(&python)
        .args(["-I", "-X", "utf8"])
        .arg(&oracle));
    let got = run(&mut Command::new(dir.join("fclone_fast")));
    let groups = group_set(&want);
    assert_eq!(groups.len(), 3, "{want}");
    assert!(!want.contains("locked"), "{want}");
    assert_eq!(group_set(&got), groups, "{got}");
}
