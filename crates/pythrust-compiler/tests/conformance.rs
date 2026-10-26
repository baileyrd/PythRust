//! Differential conformance harness. Thread E.
//!
//! Every `examples/<name>.pyr` program has a golden `<name>.out`: the
//! stdout it must print. The phase-2 subset is Python-compatible, so CPython
//! is the oracle: `corpus_matches_cpython` checks each golden against
//! `python3`, and `corpus_matches_pythrust` checks it against `pythrust run`.
//! `corpus_matches_native` builds each program with `pythrust build` and
//! checks the executable too; programs outside the native subset must fail
//! with a backend "does not support" error, and `NATIVE` must build. A
//! native program that fails must also print `run`'s exact diagnostic.
//! Stdout and exit status are compared; stderr is not, because diagnostics
//! and tracebacks differ by design.
//!
//! Directives in the leading comment block of a program:
//! - `# expect-exit: N` expected exit status (default 0).
//! - `# deviation: why` a documented deviation from Python (see
//!   `docs/grammar.md`). The golden holds Pythrust's output, and CPython is
//!   required to differ, so a stale deviation fails.
//! - `# expect-error: text` the program must fail `check` (lex, parse,
//!   infer) with a diagnostic containing `text`; it implies exit 1. Every
//!   other program must pass `check`. `corpus_checks` asserts both.
//!
//! Environment:
//! - `PYTHON` interpreter to use as the oracle (default `python3`).
//! - `PYTHRUST_BIN` the `pythrust` binary (default: the one Cargo built for
//!   this test run).
//! - `PYTHRUST_BLESS=1` write non-deviation goldens from CPython, creating
//!   missing ones; a golden whose exit status disagrees is kept.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use pythrust_compiler::{infer, lexer, parser};

type Res<T> = std::result::Result<T, String>;

/// What a program is expected to do, from its directives.
#[derive(Debug, PartialEq, Eq)]
struct Expect {
    status: i32,
    deviation: Option<String>,
    /// Substring of the compile error `check` must report, if any.
    check_error: Option<String>,
}

/// One corpus program and its golden stdout.
#[derive(Debug)]
struct Case {
    source: PathBuf,
    stdout: String,
    expect: Expect,
}

/// Observed result of running a program.
#[derive(Debug, PartialEq, Eq)]
struct Run {
    stdout: String,
    status: i32,
}

const EXIT: &str = "# expect-exit:";
const DEVIATION: &str = "# deviation:";
const ERROR: &str = "# expect-error:";

/// Reads directives from the leading `#` comment block of `src`.
fn parse_expect(src: &str) -> Res<Expect> {
    let mut status: Option<i32> = None;
    let mut deviation: Option<String> = None;
    let mut check_error: Option<String> = None;
    for line in src.lines().take_while(|l| l.starts_with('#')) {
        if let Some(v) = line.strip_prefix(EXIT) {
            let n = v
                .trim()
                .parse()
                .map_err(|e| format!("bad {EXIT} {v:?}: {e}"))?;
            set_once(&mut status, n, EXIT)?;
        } else if let Some(v) = line.strip_prefix(DEVIATION) {
            set_once(&mut deviation, text(v, DEVIATION)?, DEVIATION)?;
        } else if let Some(v) = line.strip_prefix(ERROR) {
            set_once(&mut check_error, text(v, ERROR)?, ERROR)?;
        }
    }
    let status = status.unwrap_or(0);
    if check_error.is_some() && status != 1 {
        return Err(format!("{ERROR} needs {EXIT} 1"));
    }
    Ok(Expect {
        status,
        deviation,
        check_error,
    })
}

/// A directive's non-empty value.
fn text(value: &str, name: &str) -> Res<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(format!("{name} needs a value"));
    }
    Ok(value.to_string())
}

fn set_once<T>(slot: &mut Option<T>, value: T, name: &str) -> Res<()> {
    if slot.is_some() {
        return Err(format!("duplicate {name}"));
    }
    *slot = Some(value);
    Ok(())
}

fn repo_root() -> Res<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    root.canonicalize()
        .map_err(|e| format!("{}: {e}", root.display()))
}

fn corpus_dir() -> Res<PathBuf> {
    Ok(repo_root()?.join("examples"))
}

fn golden_path(source: &Path) -> PathBuf {
    source.with_extension("out")
}

fn read(path: &Path) -> Res<String> {
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// All `.pyr` programs in `dir`, sorted.
fn list_sources(dir: &Path) -> Res<Vec<PathBuf>> {
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut sources: Vec<PathBuf> = entries
        .map(|e| e.map(|e| e.path()).map_err(|e| e.to_string()))
        .collect::<Res<_>>()?;
    sources.retain(|p| p.extension().is_some_and(|x| x == "pyr"));
    sources.sort();
    Ok(sources)
}

/// All programs in `dir`, each with its golden; a missing golden is an error.
fn load_corpus(dir: &Path) -> Res<Vec<Case>> {
    list_sources(dir)?.into_iter().map(load_case).collect()
}

fn load_expect(source: &Path) -> Res<Expect> {
    parse_expect(&read(source)?).map_err(|e| format!("{}: {e}", source.display()))
}

fn load_case(source: PathBuf) -> Res<Case> {
    let expect = load_expect(&source)?;
    let stdout = read(&golden_path(&source))?;
    Ok(Case {
        source,
        stdout,
        expect,
    })
}

/// Longest a single program may run before it is killed.
const TIMEOUT: Duration = Duration::from_secs(10);

/// Runs `cmd` to completion, killing and reaping it after `timeout`.
fn run(cmd: &mut Command, timeout: Duration) -> Res<Run> {
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("cannot run {cmd:?}: {e}"))?;
    // Drain stdout on a thread so a full pipe cannot stall the child.
    let mut pipe = child.stdout.take().ok_or("stdout not captured")?;
    let reader = thread::spawn(move || {
        let mut buf = Vec::new();
        pipe.read_to_end(&mut buf).map(|_| buf)
    });
    let status = wait(&mut child, timeout).map_err(|e| format!("{cmd:?}: {e}"))?;
    let bytes = reader
        .join()
        .map_err(|_| "stdout reader panicked".to_string())?
        .map_err(|e| e.to_string())?;
    let stdout = String::from_utf8(bytes).map_err(|e| format!("{cmd:?}: {e}"))?;
    Ok(Run { stdout, status })
}

/// Waits for `child`'s exit code, killing it if it outlives `timeout`.
fn wait(child: &mut Child, timeout: Duration) -> Res<i32> {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            return status
                .code()
                .ok_or_else(|| "killed by a signal".to_string());
        }
        if start.elapsed() > timeout {
            child.kill().map_err(|e| e.to_string())?;
            child.wait().map_err(|e| e.to_string())?;
            return Err(format!("timed out after {timeout:?}"));
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn run_cpython(source: &Path) -> Res<Run> {
    let python = std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string());
    // -I: isolated mode, so nothing from the environment or cwd leaks in.
    run(Command::new(python).arg("-I").arg(source), TIMEOUT)
}

fn pythrust_bin() -> Res<PathBuf> {
    let default = PathBuf::from(env!("CARGO_BIN_EXE_pythrust"));
    Ok(std::env::var_os("PYTHRUST_BIN").map_or(default, PathBuf::from))
}

fn run_pythrust(source: &Path) -> Res<Run> {
    run(
        Command::new(pythrust_bin()?).arg("run").arg(source),
        TIMEOUT,
    )
}

/// Programs the native backend must compile; it grows with the subset.
const NATIVE: &[&str] = &[
    "arithmetic",
    "call_before_def",
    "collatz",
    "fibonacci",
    "fizzbuzz",
    "float_parse_error",
    "float_semantics",
    "float_to_int_nan",
    "float_to_int_range",
    "floats",
    "hello",
    "int_control",
    "int_parse_overflow",
    "int_pow_negative",
    "overflow_add",
    "overflow_factorial",
    "overflow_min_floordiv",
    "recursion_limit",
    "shadow_builtin_late",
    "str_index_error",
    "str_loop_exits",
    "str_repeat_error",
    "strings",
    "unbound_augassign",
    "unbound_local",
    "unbound_loop_target",
    "zero_division",
];

/// What the native backend says about a program outside its subset.
const NATIVE_UNSUPPORTED: &str = "backend error: the native backend does not support";

/// Builds the runtime library `pythrust build` links against, with the
/// profile and target of the `pythrust` under test so it lands beside it.
/// `cargo test` builds the compiler but not this static library.
fn build_runtime() -> Res<()> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let args = runtime_build_args(&pythrust_bin()?, &|dir| dir.join("CACHEDIR.TAG").is_file())?;
    let status = Command::new(cargo)
        .args(&args)
        .status()
        .map_err(|e| format!("cannot run cargo: {e}"))?;
    status
        .success()
        .then_some(())
        .ok_or_else(|| format!("building pythrust-rt failed: {status}"))
}

/// Cargo arguments that build `pythrust-rt` into the directory of `exe`,
/// laid out as `<target-dir>[/<triple>]/<profile-dir>/pythrust`. Cargo
/// marks a target directory with `CACHEDIR.TAG`; `is_target_dir` checks it.
fn runtime_build_args(exe: &Path, is_target_dir: &dyn Fn(&Path) -> bool) -> Res<Vec<String>> {
    let parent = |p: &Path| {
        p.parent()
            .map(Path::to_path_buf)
            .ok_or(format!("{} has no parent", p.display()))
    };
    let name = |p: &Path| {
        p.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .ok_or(format!("{} has no name", p.display()))
    };
    let profile_dir = parent(exe)?;
    let profile = match name(&profile_dir)?.as_str() {
        "debug" => "dev".to_string(),
        other => other.to_string(),
    };
    let mut root = parent(&profile_dir)?;
    let mut args: Vec<String> = ["build", "--quiet", "-p", "pythrust-rt", "--profile"]
        .map(String::from)
        .into();
    args.push(profile);
    if !is_target_dir(&root) {
        args.extend(["--target".to_string(), name(&root)?]);
        root = parent(&root)?;
    }
    args.extend([
        "--target-dir".to_string(),
        root.to_string_lossy().into_owned(),
    ]);
    Ok(args)
}

/// Builds a copy of `source` in `dir`: the executable, or `None` when
/// the program is outside the native subset.
fn build_native(source: &Path, dir: &Path) -> Res<Option<PathBuf>> {
    let name = source.file_name().ok_or("source has no file name")?;
    let copy = dir.join(name);
    fs::copy(source, &copy).map_err(|e| format!("{}: {e}", source.display()))?;
    let out = Command::new(pythrust_bin()?)
        .arg("build")
        .arg(&copy)
        .output()
        .map_err(|e| format!("cannot run pythrust build: {e}"))?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    match (out.status.success(), stderr.contains(NATIVE_UNSUPPORTED)) {
        (true, _) => Ok(Some(copy.with_extension(""))),
        (false, true) => Ok(None),
        (false, false) => Err(format!("{}: build failed:\n{stderr}", source.display())),
    }
}

/// Compares a run against the golden; `None` means it matches.
fn mismatch(case: &Case, got: &Run) -> Option<String> {
    let want = Run {
        stdout: case.stdout.clone(),
        status: case.expect.status,
    };
    if *got == want {
        return None;
    }
    Some(format!(
        "{}\n  want status {} stdout:\n{}\n  got status {} stdout:\n{}",
        case.source.display(),
        want.status,
        want.stdout,
        got.status,
        got.stdout
    ))
}

/// Checks one case against CPython. A deviation must still differ.
fn check_cpython(case: &Case) -> Res<Option<String>> {
    let got = run_cpython(&case.source)?;
    let Some(why) = &case.expect.deviation else {
        return Ok(mismatch(case, &got));
    };
    let stale = got.stdout == case.stdout && got.status == case.expect.status;
    Ok(stale.then(|| {
        format!(
            "{}: CPython agrees; stale deviation: {why}",
            case.source.display()
        )
    }))
}

/// Writes `source`'s golden from CPython; the golden may not exist yet.
/// Deviations are hand-written and skipped. An unexpected exit status
/// leaves the old golden in place and is reported.
fn bless(source: &Path) -> Res<Option<String>> {
    let expect = load_expect(source)?;
    if expect.deviation.is_some() {
        return Ok(None);
    }
    let got = run_cpython(source)?;
    if got.status != expect.status {
        return Ok(Some(format!(
            "{}: CPython exited {}, want {}; golden kept",
            source.display(),
            got.status,
            expect.status
        )));
    }
    let golden = golden_path(source);
    fs::write(&golden, &got.stdout).map_err(|e| format!("{}: {e}", golden.display()))?;
    Ok(None)
}

/// Fails with every collected failure at once, not just the first.
fn report(failures: Vec<String>) {
    assert!(
        failures.is_empty(),
        "{} failing:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

/// Blesses every program in `dir`, or checks each against CPython.
fn cpython_failures(dir: &Path, blessing: bool) -> Res<Vec<String>> {
    let mut failures = Vec::new();
    if blessing {
        for source in list_sources(dir)? {
            failures.extend(bless(&source)?);
        }
        return Ok(failures);
    }
    for case in load_corpus(dir)? {
        failures.extend(check_cpython(&case)?);
    }
    Ok(failures)
}

/// Runs `check` (lex, parse, infer) in process; `Err` holds the message.
fn check_source(src: &str) -> std::result::Result<(), String> {
    let tokens = lexer::lex(src).map_err(|d| d.message)?;
    let module = parser::parse(&tokens).map_err(|d| d.message)?;
    infer::infer(&module).map(|_| ()).map_err(|d| d.message)
}

/// Compares `check` against the case's `expect-error`; `None` means it agrees.
fn check_mismatch(source: &Path, want: Option<&str>, got: Result<(), String>) -> Option<String> {
    let name = source.display();
    match (want, got) {
        (None, Ok(())) => None,
        (None, Err(msg)) => Some(format!("{name}: check failed: {msg}")),
        (Some(want), Ok(())) => Some(format!("{name}: check passed, want error {want:?}")),
        (Some(want), Err(msg)) if msg.contains(want) => None,
        (Some(want), Err(msg)) => Some(format!("{name}: want error {want:?}, got {msg:?}")),
    }
}

#[test]
fn corpus_checks() -> Res<()> {
    let mut failures = Vec::new();
    for case in load_corpus(&corpus_dir()?)? {
        let got = check_source(&read(&case.source)?);
        let want = case.expect.check_error.as_deref();
        failures.extend(check_mismatch(&case.source, want, got));
    }
    report(failures);
    Ok(())
}

#[test]
fn check_mismatch_cases() {
    let p = Path::new("p.pyr");
    assert_eq!(check_mismatch(p, None, Ok(())), None);
    assert_eq!(
        check_mismatch(p, Some("bad"), Err("a bad type".into())),
        None
    );
    assert!(check_mismatch(p, None, Err("x".into())).is_some());
    assert!(check_mismatch(p, Some("bad"), Ok(())).is_some());
    assert!(check_mismatch(p, Some("bad"), Err("other".into())).is_some());
}

#[test]
fn corpus_matches_cpython() -> Res<()> {
    let blessing = std::env::var("PYTHRUST_BLESS").is_ok_and(|v| v == "1");
    report(cpython_failures(&corpus_dir()?, blessing)?);
    Ok(())
}

#[test]
fn corpus_matches_pythrust() -> Res<()> {
    let mut failures = Vec::new();
    for case in load_corpus(&corpus_dir()?)? {
        let got = run_pythrust(&case.source)?;
        failures.extend(mismatch(&case, &got));
    }
    report(failures);
    Ok(())
}

/// A native program that failed must print exactly what `pythrust run`
/// prints for the same file: the error, its location and its carets. Only
/// called once the program is known to terminate.
fn stderr_mismatch(exe: &Path) -> Res<Option<String>> {
    let stderr = |cmd: &mut Command| -> Res<String> {
        let out = cmd
            .output()
            .map_err(|e| format!("cannot run {cmd:?}: {e}"))?;
        Ok(String::from_utf8_lossy(&out.stderr).into_owned())
    };
    let source = exe.with_extension("pyr");
    let native = stderr(&mut Command::new(exe))?;
    let interp = stderr(Command::new(pythrust_bin()?).arg("run").arg(&source))?;
    Ok((native != interp).then(|| {
        format!(
            "{}\n  run stderr:\n{interp}\n  native stderr:\n{native}",
            source.display()
        )
    }))
}

#[test]
fn corpus_matches_native() -> Res<()> {
    build_runtime()?;
    let dir = std::env::temp_dir().join(format!("pythrust-native-{}", std::process::id()));
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let mut failures = Vec::new();
    let mut built = Vec::new();
    for case in load_corpus(&corpus_dir()?)? {
        if case.expect.check_error.is_some() {
            continue;
        }
        let Some(exe) = build_native(&case.source, &dir)? else {
            continue;
        };
        let got = run(&mut Command::new(&exe), TIMEOUT)?;
        let failed = mismatch(&case, &got);
        let errored = failed.is_none() && got.status != 0;
        failures.extend(failed);
        if errored {
            failures.extend(stderr_mismatch(&exe)?);
        }
        built.extend(
            case.source
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned()),
        );
    }
    fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    for name in NATIVE {
        if !built.iter().any(|b| b == name) {
            failures.push(format!("{name}: must build natively"));
        }
    }
    report(failures);
    Ok(())
}

#[test]
fn corpus_is_complete() -> Res<()> {
    let dir = corpus_dir()?;
    let cases = load_corpus(&dir)?;
    assert!(
        cases.len() >= 20,
        "M2 wants 20+ programs, have {}",
        cases.len()
    );
    assert!(cases.iter().any(|c| c.expect.deviation.is_some()));
    for entry in fs::read_dir(&dir).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let orphan =
            path.extension().is_some_and(|x| x == "out") && !path.with_extension("pyr").exists();
        assert!(!orphan, "golden without a program: {}", path.display());
    }
    Ok(())
}

#[test]
fn runtime_follows_the_profile_and_target_under_test() -> Res<()> {
    let root = |dir: &Path| dir.ends_with("target");
    let tail = |args: Vec<String>| args[4..].join(" ");
    let dev = runtime_build_args(Path::new("/w/target/debug/pythrust"), &root)?;
    assert_eq!(tail(dev), "--profile dev --target-dir /w/target");
    let release = runtime_build_args(Path::new("/w/target/release/pythrust"), &root)?;
    assert_eq!(tail(release), "--profile release --target-dir /w/target");
    let cross = runtime_build_args(
        Path::new("/w/target/x86_64-unknown-linux-gnu/bench/pythrust"),
        &root,
    )?;
    assert_eq!(
        tail(cross),
        "--profile bench --target x86_64-unknown-linux-gnu --target-dir /w/target"
    );
    Ok(())
}

#[test]
fn expect_defaults() {
    let want = || Expect {
        status: 0,
        deviation: None,
        check_error: None,
    };
    assert_eq!(parse_expect("print(1)\n"), Ok(want()));
    assert_eq!(parse_expect(""), Ok(want()));
}

#[test]
fn expect_reads_directives() {
    let src = "# a plain comment\n# expect-exit: 1\n# deviation: checked overflow\n# expect-error: overflow\nx = 1\n";
    let want = Expect {
        status: 1,
        deviation: Some("checked overflow".to_string()),
        check_error: Some("overflow".to_string()),
    };
    assert_eq!(parse_expect(src), Ok(want));
}

#[test]
fn expect_ignores_directives_after_code() {
    let src = "x = 1\n# expect-exit: 1\n";
    assert_eq!(parse_expect(src).map(|e| e.status), Ok(0));
}

#[test]
fn expect_rejects_bad_directives() {
    assert!(parse_expect("# expect-exit: one\n").is_err());
    assert!(parse_expect("# expect-exit: 1\n# expect-exit: 2\n").is_err());
    assert!(parse_expect("# deviation:\n").is_err());
    assert!(parse_expect("# deviation: a\n# deviation: b\n").is_err());
    assert!(parse_expect("# expect-error:\n# expect-exit: 1\n").is_err());
    assert!(parse_expect("# expect-error: x\n").is_err());
    assert!(parse_expect("# expect-error: x\n# expect-exit: 1\n# expect-error: y\n").is_err());
}

#[test]
fn mismatch_reports_status_and_stdout() {
    let case = Case {
        source: PathBuf::from("x.pyr"),
        stdout: "1\n".to_string(),
        expect: Expect {
            status: 0,
            deviation: None,
            check_error: None,
        },
    };
    let ok = Run {
        stdout: "1\n".to_string(),
        status: 0,
    };
    assert_eq!(mismatch(&case, &ok), None);
    let bad_status = Run {
        stdout: "1\n".to_string(),
        status: 1,
    };
    assert!(mismatch(&case, &bad_status).is_some());
    let bad_out = Run {
        stdout: "2\n".to_string(),
        status: 0,
    };
    assert!(mismatch(&case, &bad_out).is_some());
}

#[test]
fn run_reports_missing_binary() {
    let got = run(&mut Command::new("/nonexistent/pythrust-binary"), TIMEOUT);
    assert!(got.is_err());
}

/// A fresh, empty directory under the system temp dir, unique to `name`.
fn scratch(name: &str) -> Res<PathBuf> {
    let dir = std::env::temp_dir().join(format!("pythrust-conf-{}-{name}", std::process::id()));
    if dir.exists() {
        fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    }
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn cleanup(dir: &Path) -> Res<()> {
    fs::remove_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))
}

fn write(path: &Path, text: &str) -> Res<()> {
    fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

#[test]
fn missing_golden_fails_check() -> Res<()> {
    let dir = scratch("missing")?;
    write(&dir.join("p.pyr"), "print(1)\n")?;
    assert!(cpython_failures(&dir, false).is_err());
    cleanup(&dir)
}

#[test]
fn bless_creates_missing_golden() -> Res<()> {
    let dir = scratch("create")?;
    write(&dir.join("p.pyr"), "print(1)\n")?;
    assert_eq!(cpython_failures(&dir, true)?, Vec::<String>::new());
    assert_eq!(read(&dir.join("p.out"))?, "1\n");
    assert_eq!(cpython_failures(&dir, false)?, Vec::<String>::new());
    cleanup(&dir)
}

#[test]
fn bless_keeps_golden_on_unexpected_exit() -> Res<()> {
    let dir = scratch("keep")?;
    write(&dir.join("p.pyr"), "print(1)\nprint(1 // 0)\n")?;
    write(&dir.join("p.out"), "old\n")?;
    assert_eq!(cpython_failures(&dir, true)?.len(), 1);
    assert_eq!(read(&dir.join("p.out"))?, "old\n");
    cleanup(&dir)
}

#[test]
fn bless_skips_deviations() -> Res<()> {
    let dir = scratch("deviation")?;
    write(&dir.join("p.pyr"), "# deviation: test\nprint(2)\n")?;
    write(&dir.join("p.out"), "hand\n")?;
    assert_eq!(cpython_failures(&dir, true)?, Vec::<String>::new());
    assert_eq!(read(&dir.join("p.out"))?, "hand\n");
    cleanup(&dir)
}

#[test]
fn run_kills_on_timeout() {
    let mut cmd = Command::new("python3");
    cmd.args(["-I", "-c", "while True: pass"]);
    let got = run(&mut cmd, Duration::from_millis(200));
    assert!(got.is_err_and(|e| e.contains("timed out")));
}
