//! Differential conformance harness. Thread E.
//!
//! Every `examples/<name>.pythrust` program has a golden `<name>.out`: the
//! stdout it must print. The phase-2 subset is Python-compatible, so CPython
//! is the oracle: `corpus_matches_cpython` checks each golden against
//! `python3`, and `corpus_matches_pythrust` checks it against `pythrust run`.
//! Stdout and exit status are compared; stderr is not, because diagnostics
//! and tracebacks differ by design.
//!
//! Directives in the leading comment block of a program:
//! - `# expect-exit: N` expected exit status (default 0).
//! - `# deviation: why` a documented deviation from Python (see
//!   `docs/grammar.md`). The golden holds Pythrust's output, and CPython is
//!   required to differ, so a stale deviation fails.
//!
//! Environment:
//! - `PYTHON` interpreter to use as the oracle (default `python3`).
//! - `PYTHRUST_BIN` the `pythrust` binary (default `target/debug/pythrust`).
//! - `PYTHRUST_BLESS=1` rewrite non-deviation goldens from CPython.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

type Res<T> = std::result::Result<T, String>;

/// What a program is expected to do, from its directives.
#[derive(Debug, PartialEq, Eq)]
struct Expect {
    status: i32,
    deviation: Option<String>,
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

/// Reads directives from the leading `#` comment block of `src`.
fn parse_expect(src: &str) -> Res<Expect> {
    let mut status: Option<i32> = None;
    let mut deviation: Option<String> = None;
    for line in src.lines().take_while(|l| l.starts_with('#')) {
        if let Some(v) = line.strip_prefix(EXIT) {
            let n = v
                .trim()
                .parse()
                .map_err(|e| format!("bad {EXIT} {v:?}: {e}"))?;
            set_once(&mut status, n, EXIT)?;
        } else if let Some(v) = line.strip_prefix(DEVIATION) {
            let why = v.trim();
            if why.is_empty() {
                return Err(format!("{DEVIATION} needs a reason"));
            }
            set_once(&mut deviation, why.to_string(), DEVIATION)?;
        }
    }
    Ok(Expect {
        status: status.unwrap_or(0),
        deviation,
    })
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

/// All `.pythrust` programs in `dir`, sorted, each with a golden.
fn load_corpus(dir: &Path) -> Res<Vec<Case>> {
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut sources: Vec<PathBuf> = entries
        .map(|e| e.map(|e| e.path()).map_err(|e| e.to_string()))
        .collect::<Res<_>>()?;
    sources.retain(|p| p.extension().is_some_and(|x| x == "pythrust"));
    sources.sort();
    sources.into_iter().map(load_case).collect()
}

fn load_case(source: PathBuf) -> Res<Case> {
    let expect = parse_expect(&read(&source)?).map_err(|e| format!("{}: {e}", source.display()))?;
    let stdout = read(&golden_path(&source))?;
    Ok(Case {
        source,
        stdout,
        expect,
    })
}

fn run(cmd: &mut Command) -> Res<Run> {
    let out = cmd
        .output()
        .map_err(|e| format!("cannot run {cmd:?}: {e}"))?;
    let status = out
        .status
        .code()
        .ok_or_else(|| format!("{cmd:?} killed by a signal"))?;
    let stdout = String::from_utf8(out.stdout).map_err(|e| format!("{cmd:?}: {e}"))?;
    Ok(Run { stdout, status })
}

fn run_cpython(source: &Path) -> Res<Run> {
    let python = std::env::var("PYTHON").unwrap_or_else(|_| "python3".to_string());
    // -I: isolated mode, so nothing from the environment or cwd leaks in.
    run(Command::new(python).arg("-I").arg(source))
}

fn run_pythrust(source: &Path) -> Res<Run> {
    let default = repo_root()?.join("target/debug/pythrust");
    let bin = std::env::var_os("PYTHRUST_BIN").map_or(default, PathBuf::from);
    run(Command::new(bin).arg("run").arg(source))
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

/// Checks one case against CPython, or blesses its golden.
fn check_cpython(case: &Case, bless: bool) -> Res<Option<String>> {
    let got = run_cpython(&case.source)?;
    if let Some(why) = &case.expect.deviation {
        let stale = got.stdout == case.stdout && got.status == case.expect.status;
        return Ok(stale.then(|| {
            format!(
                "{}: CPython agrees; stale deviation: {why}",
                case.source.display()
            )
        }));
    }
    if bless {
        fs::write(golden_path(&case.source), &got.stdout).map_err(|e| e.to_string())?;
        return Ok(None);
    }
    Ok(mismatch(case, &got))
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

#[test]
fn corpus_matches_cpython() -> Res<()> {
    let bless = std::env::var("PYTHRUST_BLESS").is_ok_and(|v| v == "1");
    let mut failures = Vec::new();
    for case in load_corpus(&corpus_dir()?)? {
        failures.extend(check_cpython(&case, bless)?);
    }
    report(failures);
    Ok(())
}

#[test]
#[ignore = "needs `pythrust run` end to end (threads A, B, D, F)"]
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
        let orphan = path.extension().is_some_and(|x| x == "out")
            && !path.with_extension("pythrust").exists();
        assert!(!orphan, "golden without a program: {}", path.display());
    }
    Ok(())
}

#[test]
fn expect_defaults() {
    let want = Expect {
        status: 0,
        deviation: None,
    };
    assert_eq!(parse_expect("print(1)\n"), Ok(want));
    assert_eq!(
        parse_expect(""),
        Ok(Expect {
            status: 0,
            deviation: None
        })
    );
}

#[test]
fn expect_reads_directives() {
    let src = "# a plain comment\n# expect-exit: 1\n# deviation: checked overflow\nx = 1\n";
    let want = Expect {
        status: 1,
        deviation: Some("checked overflow".to_string()),
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
}

#[test]
fn mismatch_reports_status_and_stdout() {
    let case = Case {
        source: PathBuf::from("x.pythrust"),
        stdout: "1\n".to_string(),
        expect: Expect {
            status: 0,
            deviation: None,
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
    let got = run(&mut Command::new("/nonexistent/pythrust-binary"));
    assert!(got.is_err());
}
