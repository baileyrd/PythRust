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
//! `temper def` and `raises` clauses are not Python, so CPython runs each
//! program loosened (every `temper ` prefix and `raises` clause removed).
//! `tempered_matches_loose` runs the loosened programs that `check` accepts
//! under Pythrust too: tempering and declaring never change meaning.
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
//! - `PYTHRUST_VALGRIND=1` also run every native program under valgrind's
//!   memcheck, failing on invalid reads, writes and frees (use-after-free)
//!   and on definitely lost blocks. CI sets it on x86-64 Linux. The instrumented run must also match the golden.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use pythrust_compiler::ir::{Func, Inst, LocalId, Program};
use pythrust_compiler::own::{self, Precision};
use pythrust_compiler::{infer, lexer, lower, parser, rc};

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

/// The oracle interpreter: `$PYTHON`, else `python3`, or `python` on
/// Windows, where `python3` may be the Microsoft Store stub.
fn python() -> String {
    let default = if cfg!(windows) { "python" } else { "python3" };
    std::env::var("PYTHON").unwrap_or_else(|_| default.to_string())
}

/// The CPython version the corpus's goldens and deviations are written
/// against (Nano, 2026-10-29): 3.14 accepts `except A, B:` (PEP 758),
/// which flips `except_tuple_unparenthesized`.
const ORACLE: &str = "3.13";

/// Fails unless the oracle is CPython [`ORACLE`].
fn check_oracle() -> Res<()> {
    let mut cmd = Command::new(python());
    cmd.args([
        "-I",
        "-c",
        "import sys; print('%d.%d' % sys.version_info[:2])",
    ]);
    let got = run(&mut cmd, TIMEOUT)?;
    oracle_mismatch(&python(), got.stdout.trim()).map_or(Ok(()), Err)
}

/// The error for an oracle reporting `version`, or `None` if it is [`ORACLE`].
fn oracle_mismatch(python: &str, version: &str) -> Option<String> {
    (version != ORACLE).then(|| {
        format!(
            "the oracle `{python}` is Python {version}, but the corpus is written against \
             {ORACLE}; set PYTHON to a CPython {ORACLE} interpreter"
        )
    })
}

fn run_cpython(source: &Path) -> Res<Run> {
    let mut cmd = Command::new(python());
    // -I: isolated mode, so nothing from the environment or cwd leaks in.
    // -X utf8: UTF-8 stdout even where the locale's code page is not.
    cmd.args(["-I", "-X", "utf8"]);
    match loosen(&read(source)?) {
        Some(loose) => cmd.arg("-c").arg(loose),
        None => cmd.arg(source),
    };
    let mut got = run(&mut cmd, TIMEOUT)?;
    got.stdout = universal_newlines(got.stdout);
    Ok(got)
}

/// Undoes text-mode stdout's newline translation: CPython on Windows
/// writes each `\n` as `\r\n`. Pythrust writes `\n` everywhere.
fn universal_newlines(stdout: String) -> String {
    match cfg!(windows) {
        true => stdout.replace("\r\n", "\n"),
        false => stdout,
    }
}

/// `src` as Python: each `temper ` prefix, `mut`/`owned` parameter mode
/// and `raises` clause removed, or `None` if it has none. `def` is
/// top-level only, so the prefix always starts a line, a mode follows
/// `(` or `, ` on its `def` line, and a clause ends that line.
fn loosen(src: &str) -> Option<String> {
    let lines: Vec<String> = src.lines().map(loosen_line).collect();
    let changed = lines.iter().zip(src.lines()).any(|(a, b)| a != b);
    changed.then(|| lines.join("\n") + "\n")
}

fn loosen_line(line: &str) -> String {
    let line = line.strip_prefix("temper ").unwrap_or(line);
    if !line.starts_with("def ") {
        return line.to_string();
    }
    let line = &strip_modes(line);
    let Some((head, names)) = line
        .strip_suffix(':')
        .and_then(|l| l.rsplit_once(" raises "))
    else {
        return line.to_string();
    };
    let is_list = names.split(',').all(|n| {
        let n = n.trim();
        !n.is_empty() && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    });
    if is_list {
        format!("{head}:")
    } else {
        line.to_string()
    }
}

/// `line` without the `mut `/`owned ` before each parameter name.
fn strip_modes(line: &str) -> String {
    ["mut ", "owned "].iter().fold(line.to_string(), |l, mode| {
        l.replace(&format!("({mode}"), "(")
            .replace(&format!(", {mode}"), ", ")
    })
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
    "annotations",
    "arithmetic",
    "bools_as_ints",
    "builtins",
    "call_before_def",
    "chained_compare",
    "collatz",
    "conditional_expr",
    "default_eval_error",
    "fibonacci",
    "empty_tuple_loop",
    "exception_hierarchy",
    "exception_hierarchy_uncaught",
    "exception_list",
    "exception_list_shapes",
    "exception_list_uncaught",
    "exception_tuple",
    "exception_tuple_uncaught",
    "exception_unbound",
    "exception_uncaught",
    "exceptions",
    "exceptions_flow",
    "fault_caught",
    "fault_caught_by_exception",
    "fault_uncaught_in_call",
    "fizzbuzz",
    "float_parse_error",
    "float_semantics",
    "float_to_int_nan",
    "float_to_int_range",
    "floats",
    "functions",
    "generic_diverging_branch",
    "generic_diverging_containers",
    "generic_diverging_exprs",
    "generic_diverging_global",
    "generic_diverging_logical",
    "generic_diverging_mutual",
    "generic_diverging_ops",
    "generic_diverging_paths",
    "generic_diverging_retype",
    "generic_diverging_suffix",
    "generic_never_returns",
    "generic_results",
    "generic_spin",
    "generic_undecided",
    "global_unbound",
    "globals_defaults",
    "hello",
    "index_error",
    "index_error_negative",
    "int_control",
    "int_parse_overflow",
    "int_pow_negative",
    "key_error_class",
    "keyword_args",
    "aliasing",
    "list_copy",
    "modes",
    "list_ownership",
    "list_semantics",
    "list_set_error",
    "lists",
    "loop_target_ownership",
    "nan_container_eq",
    "nested_control",
    "optional_basics",
    "optional_calls",
    "optional_loop_target",
    "optional_narrowing",
    "optional_ownership",
    "optional_shapes",
    "optional_short_circuit",
    "overflow_add",
    "overflow_factorial",
    "overflow_min_floordiv",
    "primes",
    "print_shadow_end_type",
    "print_shadow_keyword",
    "print_shadow_valid",
    "range_index_error",
    "range_values",
    "raises",
    "recursion_limit",
    "refcount_keeps",
    "refcount_moves",
    "shadow_builtin_late",
    "short_circuit",
    "sorting",
    "str_index_error",
    "str_loop_exits",
    "str_repeat_error",
    "strings",
    "tempered",
    "tuple_loops",
    "tuples",
    "unbound_augassign",
    "unbound_local",
    "unbound_loop_target",
    "unpack_list_error",
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
        (true, _) => Ok(Some(copy.with_extension(std::env::consts::EXE_EXTENSION))),
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
    let types = infer::infer(&module).map_err(|d| d.message)?;
    own::check(&module, &own::analyze(&module, &types)).map_err(|d| d.message)
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

/// M5's exit: a tempered function rejects what a loose one accepts. Every
/// program `check` rejects that tempers or declares something passes once
/// loosened, so tempering alone made the error.
#[test]
fn tempering_alone_rejects() -> Res<()> {
    let mut failures = Vec::new();
    let mut rejected = 0;
    for case in load_corpus(&corpus_dir()?)? {
        if case.expect.check_error.is_none() {
            continue;
        }
        let Some(loose) = loosen(&read(&case.source)?) else {
            continue;
        };
        rejected += 1;
        if let Err(msg) = check_source(&loose) {
            failures.push(format!(
                "{}: loosened, still {msg:?}",
                case.source.display()
            ));
        }
    }
    assert!(rejected > 0, "the corpus has no program tempering rejects");
    report(failures);
    Ok(())
}

/// The ownership analysis behind `check` and `run` is a coverage gate:
/// every checked program gets summaries, precise for each program the
/// native backend builds, and otherwise conservative only for a gap the
/// backend names, never an internal error.
#[test]
fn corpus_ownership_coverage() -> Res<()> {
    let mut failures = Vec::new();
    for case in load_corpus(&corpus_dir()?)? {
        if case.expect.check_error.is_some() {
            continue;
        }
        let name = stem(&case.source);
        let src = read(&case.source)?;
        let module =
            parser::parse(&lexer::lex(&src).map_err(|d| d.message)?).map_err(|d| d.message)?;
        let types = infer::infer(&module).map_err(|d| d.message)?;
        let failure = match own::analyze(&module, &types).precision {
            Precision::Precise => None,
            Precision::Conservative(_) if NATIVE.contains(&name.as_str()) => Some(format!(
                "{name}: builds natively, but its summaries are conservative"
            )),
            Precision::Conservative(gap) if gap.message.contains("does not support") => None,
            Precision::Conservative(gap) => Some(format!("{name}: {}", gap.message)),
        };
        failures.extend(failure);
    }
    report(failures);
    Ok(())
}

/// A7's IR fact against the summaries (A8): in every program the native
/// backend builds, a reference-counted parameter whose summary neither
/// keeps nor returns its lists or strings, and which its body never
/// reassigns, is
/// borrowed after `rc::elide`. The summaries check the fact but do not
/// feed it; each is per instance here, as `build` lowers.
#[test]
fn corpus_unkept_parameters_are_borrowed() -> Res<()> {
    let mut failures = Vec::new();
    for case in load_corpus(&corpus_dir()?)? {
        let name = stem(&case.source);
        if !NATIVE.contains(&name.as_str()) {
            continue;
        }
        let program = lowered(&read(&case.source)?)?;
        failures.extend(
            unkept_unborrowed(&program)
                .iter()
                .map(|m| format!("{name}: {m}")),
        );
    }
    report(failures);
    Ok(())
}

/// Each unkept, never reassigned parameter of `program` that
/// `rc::elide` does not borrow.
fn unkept_unborrowed(program: &Program) -> Vec<String> {
    let summaries = own::infer(program);
    let elided = rc::elide(program.clone());
    let funcs = program.funcs.iter().zip(summaries).zip(elided.funcs);
    let params = funcs.flat_map(|((func, summary), after)| {
        let checked: Vec<LocalId> = func
            .params
            .iter()
            .zip(&summary.params)
            .filter(|(p, s)| unkept(s) && func.locals[p.0].is_rc())
            .map(|(p, _)| *p)
            .filter(|p| !reassigns(func, *p))
            .collect();
        let missing = checked
            .into_iter()
            .filter(move |p| !after.borrowed.contains(p));
        missing.map(move |p| {
            format!(
                "`{}` parameter {} is unkept but not borrowed",
                func.name, p.0
            )
        })
    });
    params.collect()
}

/// Whether the summary says the call keeps no reference of the
/// parameter's: its lists are neither kept nor returned, nor its strings.
fn unkept(p: &own::Param) -> bool {
    p.mode != own::Mode::Owned && !p.result_aliases && !p.strs.kept && !p.strs.returned
}

/// `src` lowered as `build` lowers it, before elision.
fn lowered(src: &str) -> Res<Program> {
    let tokens = lexer::lex(src).map_err(|d| d.message)?;
    let module = parser::parse(&tokens).map_err(|d| d.message)?;
    let types = infer::infer(&module).map_err(|d| d.message)?;
    lower::lower(&module, &types).map_err(|d| d.message)
}

/// Whether `func`'s body assigns local `p`.
fn reassigns(func: &Func, p: LocalId) -> bool {
    let mut insts = func.blocks.iter().flat_map(|b| &b.insts);
    insts.any(|i| matches!(i, Inst::Assign { dest, .. } if *dest == p))
}

fn stem(source: &Path) -> String {
    let stem = source.file_stem().unwrap_or_default();
    stem.to_string_lossy().into_owned()
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
fn oracle_must_be_the_corpus_version() {
    assert_eq!(oracle_mismatch("python3", ORACLE), None);
    for version in ["3.12", "3.14", ""] {
        let err = oracle_mismatch("python3", version).unwrap_or_default();
        assert!(
            err.contains("set PYTHON to a CPython 3.13"),
            "{version}: {err}"
        );
    }
}

#[test]
fn corpus_matches_cpython() -> Res<()> {
    check_oracle()?;
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

#[test]
fn tempered_matches_loose() -> Res<()> {
    let dir = scratch("loose")?;
    let mut failures = Vec::new();
    let mut tempered = 0;
    for case in load_corpus(&corpus_dir()?)? {
        // A program tempering rejects means nothing tempered to compare.
        if case.expect.check_error.is_some() {
            continue;
        }
        let Some(loose) = loosen(&read(&case.source)?) else {
            continue;
        };
        tempered += 1;
        let copy = dir.join(case.source.file_name().ok_or("source has no file name")?);
        write(&copy, &loose)?;
        let got = run_pythrust(&copy)?;
        failures.extend(mismatch(&case, &got));
    }
    cleanup(&dir)?;
    assert!(tempered > 0, "the corpus has no tempered program");
    report(failures);
    Ok(())
}

#[test]
fn loosen_strips_only_the_prefix() {
    assert_eq!(loosen("def f():\n    pass\n"), None);
    let src = "temper def f() -> None:\n    print(\"temper \")\n";
    let want = "def f() -> None:\n    print(\"temper \")\n";
    assert_eq!(loosen(src).as_deref(), Some(want));
}

#[test]
fn loosen_strips_parameter_modes() {
    let src = "temper def f(mut xs: list[int], n: int, owned ys: list[int]) -> None:\n    pass\n";
    let want = "def f(xs: list[int], n: int, ys: list[int]) -> None:\n    pass\n";
    assert_eq!(loosen(src).as_deref(), Some(want));
    assert_eq!(loosen("def f(mut, owned):\n    pass\n"), None);
    assert_eq!(loosen("print(\"(mut x\")\n"), None);
}

#[test]
fn loosen_strips_raises_clauses() {
    let src = "temper def f(s: str) -> int raises ValueError, IndexError:\n    pass\n";
    let want = "def f(s: str) -> int:\n    pass\n";
    assert_eq!(loosen(src).as_deref(), Some(want));
    let loose = "def g(s) raises ValueError:\n    print(\" raises X:\")\n";
    let want = "def g(s):\n    print(\" raises X:\")\n";
    assert_eq!(loosen(loose).as_deref(), Some(want));
    assert_eq!(loosen("def raises(raises):\n    pass\n"), None);
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

/// Memcheck's exit status for a program it found errors in; corpus
/// programs exit 0 or 1, so it cannot be mistaken for theirs.
const MEMCHECK_EXIT: i32 = 97;

/// Whether `PYTHRUST_VALGRIND=1` asks for memcheck runs.
fn memcheck_requested() -> bool {
    std::env::var_os("PYTHRUST_VALGRIND").is_some_and(|v| v == "1")
}

/// A memcheck run is many times slower than a plain one.
const MEMCHECK_TIMEOUT: Duration = Duration::from_secs(120);

/// Runs `exe` under memcheck. The debug runtime already fails leaked
/// strings and sequences; memcheck adds use-after-free, double frees and
/// lost blocks outside that count. Runtime errors exit without releasing,
/// which leaves blocks reachable, not lost, so they pass.
fn memcheck(case: &Case, exe: &Path) -> Res<Option<String>> {
    let log = exe.with_extension("memcheck");
    let got = run(
        Command::new("valgrind")
            .args(["--leak-check=full", "--errors-for-leak-kinds=definite"])
            .arg(format!("--error-exitcode={MEMCHECK_EXIT}"))
            .arg(format!("--log-file={}", log.display()))
            .arg(exe),
        MEMCHECK_TIMEOUT,
    );
    let report = fs::read_to_string(&log).unwrap_or_else(|e| format!("no memcheck log: {e}"));
    Ok(memcheck_failure(case, got, &report))
}

/// Memcheck's closing line for a run it watched to the end with no errors.
const MEMCHECK_CLEAN: &str = "ERROR SUMMARY: 0 errors";

/// The report for a memcheck run, if it failed. It fails closed: the
/// instrumented run must finish exactly as the golden says (stdout and
/// status, so an expected runtime error still passes) and its log must end
/// clean, which proves memcheck ran the program to completion. A run that
/// could not start, timed out, died by a signal or left no log fails.
fn memcheck_failure(case: &Case, got: Res<Run>, report: &str) -> Option<String> {
    let why = match got {
        Err(e) => format!("memcheck run failed: {e}"),
        Ok(run) if run.status == MEMCHECK_EXIT => "memcheck errors".to_string(),
        Ok(run) => match mismatch(case, &run) {
            Some(m) => format!("memcheck run differs from the golden: {m}"),
            None if report.contains(MEMCHECK_CLEAN) => return None,
            None => format!("memcheck did not report a clean finish ({MEMCHECK_CLEAN})"),
        },
    };
    Some(format!(
        "{}\n  {why}\n  memcheck log:\n{report}",
        case.source.display()
    ))
}

/// A corpus program links to about 0.5 MB. Without dead-code removal
/// and debug-info stripping it is over 5 MB; this bound catches that.
const MAX_NATIVE_BYTES: u64 = 1 << 20;

/// Fails a native binary larger than [`MAX_NATIVE_BYTES`].
fn oversized(exe: &Path) -> Res<Option<String>> {
    let size = fs::metadata(exe).map_err(|e| e.to_string())?.len();
    Ok((size > MAX_NATIVE_BYTES)
        .then(|| format!("{}: {size} bytes exceeds {MAX_NATIVE_BYTES}", exe.display())))
}

/// `pythrust build` links in a private directory beside the executable
/// and must remove it, whether linking worked or not.
fn leftover_build_dirs(dir: &Path) -> Res<Vec<String>> {
    let mut left = Vec::new();
    for entry in fs::read_dir(dir).map_err(|e| e.to_string())? {
        let name = entry.map_err(|e| e.to_string())?.file_name();
        if name.to_string_lossy().starts_with(".pythrust-build-") {
            left.push(format!("build directory left behind: {name:?}"));
        }
    }
    Ok(left)
}

/// `build` rejects what `check` rejects, with the same diagnostic, so
/// no program a compile-time rule forbids builds natively.
#[test]
fn corpus_errors_fail_build() -> Res<()> {
    let dir = scratch("errors")?;
    let mut failures = Vec::new();
    for case in load_corpus(&corpus_dir()?)? {
        let Some(want) = case.expect.check_error.as_deref() else {
            continue;
        };
        let name = case.source.file_name().ok_or("source has no file name")?;
        let copy = dir.join(name);
        fs::copy(&case.source, &copy).map_err(|e| format!("{}: {e}", case.source.display()))?;
        let out = Command::new(pythrust_bin()?)
            .arg("build")
            .arg(&copy)
            .output()
            .map_err(|e| format!("cannot run pythrust build: {e}"))?;
        let stderr = String::from_utf8_lossy(&out.stderr);
        if out.status.success() || !stderr.contains(want) {
            failures.push(format!("{}: build gave:\n{stderr}", case.source.display()));
        }
    }
    cleanup(&dir)?;
    report(failures);
    Ok(())
}

#[test]
fn corpus_matches_native() -> Res<()> {
    build_runtime()?;
    let memchecked = memcheck_requested();
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
        failures.extend(oversized(&exe)?);
        let got = run(&mut Command::new(&exe), TIMEOUT)?;
        let failed = mismatch(&case, &got);
        let errored = failed.is_none() && got.status != 0;
        failures.extend(failed);
        if errored {
            failures.extend(stderr_mismatch(&exe)?);
        }
        if memchecked {
            failures.extend(memcheck(&case, &exe)?);
        }
        built.extend(
            case.source
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned()),
        );
    }
    failures.extend(leftover_build_dirs(&dir)?);
    fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
    for name in NATIVE {
        if !built.iter().any(|b| b == name) {
            failures.push(format!("{name}: must build natively"));
        }
    }
    report(failures);
    Ok(())
}

/// An exception holding a list that may hold exceptions could close a
/// reference cycle, so `build` refuses it at the construct, as a native
/// limitation, while `run` accepts it (`exception_list_cycle`).
#[test]
fn build_refuses_an_exception_that_may_close_a_cycle() -> Res<()> {
    let source = corpus_dir()?.join("exception_list_cycle.pyr");
    let out = Command::new(pythrust_bin()?)
        .arg("build")
        .arg(&source)
        .output()
        .map_err(|e| format!("cannot run pythrust build: {e}"))?;
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "{stderr}");
    assert!(
        stderr.contains("exception_list_cycle.pyr:5:5: backend error"),
        "{stderr}"
    );
    assert!(stderr.contains(NATIVE_UNSUPPORTED), "{stderr}");
    assert!(stderr.contains("no cycle collector"), "{stderr}");
    assert_eq!(run_pythrust(&source)?.status, 0);
    Ok(())
}

/// A warning (an unneeded `raises Exception`) goes to stderr once the
/// checks pass, before the program runs, and changes neither stdout nor
/// the exit status: both match the program with the suggested clause.
#[test]
fn warnings_print_and_leave_output_and_status_unchanged() -> Res<()> {
    let dir = scratch("warning")?;
    let program = |clause: &str| {
        format!(
            "def f(s: str) -> int{clause}:\n    return int(s)\nprint(f(\"1\"))\nprint(f(\"x\"))\n"
        )
    };
    let (warned, clean) = (dir.join("warned.pyr"), dir.join("clean.pyr"));
    write(&warned, &program(" raises Exception"))?;
    write(&clean, &program(" raises ValueError"))?;
    let mut failures = Vec::new();
    for command in ["check", "run"] {
        let got = pythrust(command, &warned)?;
        let want = pythrust(command, &clean)?;
        let stderr = String::from_utf8_lossy(&got.stderr);
        let warning = "warned.pyr:1:29: warning: `raises Exception`";
        if !stderr.contains(warning) {
            failures.push(format!("{command}: no warning in:\n{stderr}"));
        }
        if got.stdout != want.stdout || got.status.code() != want.status.code() {
            failures.push(format!("{command}: {got:?}\nwant {want:?}"));
        }
    }
    let run = pythrust("run", &warned)?;
    assert_eq!(String::from_utf8_lossy(&run.stdout), "1\n");
    assert_eq!(run.status.code(), Some(1));
    report(failures);
    cleanup(&dir)
}

fn pythrust(command: &str, source: &Path) -> Res<std::process::Output> {
    Command::new(pythrust_bin()?)
        .arg(command)
        .arg(source)
        .output()
        .map_err(|e| format!("cannot run pythrust {command}: {e}"))
}

/// Exceptions nest to any depth (`e = ValueError(e)`), so a native
/// `repr` of one must stop where `run`'s does, with the same error at the
/// same place and nothing printed by the failing `print`, and freeing a
/// long chain must not exhaust the stack. Not a corpus program: CPython's
/// limit differs.
#[test]
fn native_matches_run_at_the_repr_limit() -> Res<()> {
    build_runtime()?;
    let dir = scratch("repr-limit")?;
    let uses = [
        "print(\"before\")\nprint(\"x\", [e])\n",
        "s = str((e, 1))\nprint(len(s))\n",
        "print(ValueError((e,)))\n",
        "raise IndexError((e,))\n",
    ];
    let mut failures = Vec::new();
    // A list or tuple around a chain of `n + 1` exceptions: 10_000 nest.
    for (k, (n, fits)) in [(9998, true), (9999, false)].into_iter().enumerate() {
        for (u, tail) in uses.iter().enumerate() {
            let src =
                format!("e = ValueError()\nfor i in range({n}):\n    e = ValueError(e)\n{tail}");
            let exe = native_case(&dir, &format!("p{k}_{u}"), &src)?;
            let got = run(&mut Command::new(&exe), TIMEOUT)?;
            let want = run_pythrust(&exe.with_extension("pyr"))?;
            if got != want {
                failures.push(format!("{src}\n  run {want:?}\n  native {got:?}"));
            }
            failures.extend(stderr_mismatch(&exe)?);
            let raises = u == 3;
            if (got.status == 0) != (fits && !raises) {
                failures.push(format!("{src}\n  exit {} at depth {n}", got.status));
            }
        }
    }
    let long =
        "e = ValueError()\nfor i in range(200000):\n    e = ValueError(e)\nprint(str(e) == \"\")\n";
    let exe = native_case(&dir, "long", long)?;
    let got = run(&mut Command::new(&exe), TIMEOUT)?;
    if (got.status, got.stdout.as_str()) != (0, "True\n") {
        failures.push(format!("{long}\n  native {got:?}"));
    }
    cleanup(&dir)?;
    report(failures);
    Ok(())
}

/// Writes `src` as `<name>.pyr` under `dir/src` (building copies it
/// into `dir`) and builds it natively.
fn native_case(dir: &Path, name: &str, src: &str) -> Res<PathBuf> {
    let sources = dir.join("src");
    fs::create_dir_all(&sources).map_err(|e| e.to_string())?;
    let source = sources.join(format!("{name}.pyr"));
    write(&source, src)?;
    build_native(&source, dir)?.ok_or_else(|| format!("{name}: outside the native subset"))
}

/// A relative source under a directory named like a linker option (`-`)
/// or a response file (`@`, as in `@scope`) still builds and runs.
#[test]
fn build_takes_option_like_directories_as_paths() -> Res<()> {
    build_runtime()?;
    let root = scratch("option-dirs")?;
    let mut failures = Vec::new();
    for sub in ["-work", "@scope"] {
        fs::create_dir(root.join(sub)).map_err(|e| e.to_string())?;
        let source = format!("{sub}/hello.pyr");
        write(&root.join(&source), "print(\"hi\")\n")?;
        let out = Command::new(pythrust_bin()?)
            .arg("build")
            .arg(&source)
            .current_dir(&root)
            .output()
            .map_err(|e| format!("cannot run pythrust build: {e}"))?;
        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr);
            failures.push(format!("{source}: build failed:\n{stderr}"));
            continue;
        }
        let exe = root
            .join(&source)
            .with_extension(std::env::consts::EXE_EXTENSION);
        let got = run(&mut Command::new(&exe), TIMEOUT)?;
        if got.stdout != "hi\n" || got.status != 0 {
            failures.push(format!("{source}: ran as {got:?}"));
        }
    }
    report(failures);
    cleanup(&root)
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
fn memcheck_fails_closed() {
    let case = |stdout: &str, status: i32| Case {
        source: PathBuf::from("p.pyr"),
        stdout: stdout.to_string(),
        expect: Expect {
            status,
            deviation: None,
            check_error: None,
        },
    };
    let ran = |stdout: &str, status: i32| -> Res<Run> {
        Ok(Run {
            stdout: stdout.to_string(),
            status,
        })
    };
    let clean = "==1== ERROR SUMMARY: 0 errors from 0 contexts (suppressed: 0 from 0)\n";
    assert_eq!(
        memcheck_failure(&case("1\n", 0), ran("1\n", 0), clean),
        None
    );
    // An expected runtime error that memcheck saw through is not an error.
    assert_eq!(memcheck_failure(&case("", 1), ran("", 1), clean), None);
    // A program that should succeed but exits 1 under memcheck fails.
    assert!(memcheck_failure(&case("1\n", 0), ran("1\n", 1), clean).is_some());
    assert!(memcheck_failure(&case("1\n", 0), ran("", 0), clean).is_some());
    // A tool failure can look like an expected silent runtime error; only
    // the clean summary tells them apart.
    let no_log = "no memcheck log: No such file or directory (os error 2)";
    let tool_failed = "valgrind: failed to start tool 'memcheck' for platform 'amd64-linux'\n";
    for report in [no_log, tool_failed, ""] {
        let failed = memcheck_failure(&case("", 1), ran("", 1), report);
        assert!(failed.is_some_and(|f| f.contains("clean finish") && f.contains(report)));
    }
    let errors = memcheck_failure(
        &case("", 1),
        ran("", MEMCHECK_EXIT),
        "Invalid read of size 8",
    );
    assert!(errors.is_some_and(|f| f.contains("memcheck errors") && f.contains("Invalid read")));
    let killed = memcheck_failure(&case("", 0), Err("killed by a signal".to_string()), clean);
    assert!(killed.is_some_and(|f| f.contains("killed by a signal")));
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
    let mut cmd = Command::new(python());
    cmd.args(["-I", "-c", "while True: pass"]);
    let got = run(&mut cmd, Duration::from_millis(200));
    assert!(got.is_err_and(|e| e.contains("timed out")));
}
