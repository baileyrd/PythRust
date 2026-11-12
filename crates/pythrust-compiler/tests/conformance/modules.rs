//! Programs of several files (step 3, #1220). Each directory
//! `examples/modules/<name>/` is a program: its main file `main.pyr`,
//! which holds the directives, its golden `main.out`, and the modules
//! and directories beside them. CPython runs a copy of the tree with
//! every `.pyr` loosened and renamed `.py`, its root on `sys.path` (D78);
//! `run` and `check` take the tree in place, `build` a copy.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::{
    build_runtime, check_mismatch, check_oracle, cpython_verdict, load_case, loosen, memcheck,
    memcheck_requested, mismatch, native, python, pythrust_bin, read, report, run, run_pythrust,
    scratch, stderr_mismatch, universal_newlines, write, Case, Res, TIMEOUT,
};

/// The programs `run` accepts that `build` refuses, each a named backend
/// gap with why, as [`super::NATIVE_GAPS`]; a listed one that builds
/// fails, so a closed gap leaves the list.
const NATIVE_GAPS: &[(&str, &str)] = &[(
    "records_imported",
    "frozen records are not lowered yet (step 4-2); `run` is the reference (D59)",
)];

/// Every multi-file program, by its main file, sorted.
fn cases() -> Res<Vec<Case>> {
    let dir = super::corpus_dir()?.join("modules");
    let entries = fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let mut mains: Vec<PathBuf> = entries
        .map(|e| e.map(|e| e.path()).map_err(|e| e.to_string()))
        .collect::<Res<_>>()?;
    mains.retain(|p| p.is_dir());
    mains.sort();
    mains
        .into_iter()
        .map(|d| load_case(d.join("main.pyr")))
        .collect()
}

/// The program's name: its directory's.
fn name(case: &Case) -> String {
    let dir = case.source.parent().and_then(Path::file_name);
    dir.unwrap_or_default().to_string_lossy().into_owned()
}

/// Copies the tree `from` into `to`; with `python`, each `.pyr` loosened
/// as a `.py`.
fn stage(from: &Path, to: &Path, python: bool) -> Res<()> {
    fs::create_dir_all(to).map_err(|e| format!("{}: {e}", to.display()))?;
    let entries = fs::read_dir(from).map_err(|e| format!("{}: {e}", from.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        let file = path.file_name().ok_or("an entry has no name")?;
        let dest = to.join(file);
        if path.is_dir() {
            stage(&path, &dest, python)?;
        } else if python && path.extension().is_some_and(|x| x == "pyr") {
            let src = read(&path)?;
            write(&dest.with_extension("py"), &loosen(&src).unwrap_or(src))?;
        } else {
            fs::copy(&path, &dest).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    Ok(())
}

/// The tree of `case` copied into `dir`, for `python` or not; its main file.
fn staged(case: &Case, dir: &Path, python: bool) -> Res<PathBuf> {
    let from = case.source.parent().ok_or("a main file has no directory")?;
    let to = dir.join(name(case));
    stage(from, &to, python)?;
    Ok(to.join(if python { "main.py" } else { "main.pyr" }))
}

#[test]
fn corpus_modules_match_cpython() -> Res<()> {
    check_oracle()?;
    let dir = scratch("modules-cpython")?;
    let mut failures = Vec::new();
    for case in cases()? {
        let main = staged(&case, &dir, true)?;
        // -I would leave the root off `sys.path`: -E and -s keep the
        // environment and user site out, and -B the bytecode.
        let mut cmd = Command::new(python());
        cmd.args(["-B", "-E", "-s", "-X", "utf8"]).arg(&main);
        let mut got = run(&mut cmd, TIMEOUT)?;
        got.stdout = universal_newlines(got.stdout);
        failures.extend(cpython_verdict(&case, &got));
    }
    report(failures);
    dir.pass();
    Ok(())
}

#[test]
fn corpus_modules_match_pythrust() -> Res<()> {
    let mut failures = Vec::new();
    for case in cases()? {
        failures.extend(mismatch(&case, &run_pythrust(&case.source)?));
    }
    report(failures);
    Ok(())
}

/// `check` and `build` agree on each program: an `expect-error` fails
/// both with its diagnostic, and every other program passes `check`.
#[test]
fn corpus_modules_check_and_build_agree() -> Res<()> {
    let dir = scratch("modules-errors")?;
    let mut failures = Vec::new();
    for case in cases()? {
        let want = case.expect.check_error.as_deref();
        let checked = pythrust("check", &case.source)?;
        failures.extend(check_mismatch(&case.source, want, checked));
        let Some(want) = want else {
            continue;
        };
        let built = pythrust("build", &staged(&case, &dir, false)?)?;
        failures.extend(check_mismatch(&case.source, Some(want), built));
    }
    report(failures);
    dir.pass();
    Ok(())
}

/// Each program that passes `check` builds, and its executable matches
/// the golden, a failure printing `run`'s exact diagnostic; under
/// `PYTHRUST_VALGRIND=1` memcheck runs it too.
#[test]
fn corpus_matches_native_modules() -> Res<()> {
    build_runtime()?;
    let dir = scratch("modules-native")?;
    let mut failures = Vec::new();
    for case in cases()? {
        if case.expect.check_error.is_some() {
            continue;
        }
        let main = staged(&case, &dir, false)?;
        let gap = NATIVE_GAPS.iter().any(|(g, _)| *g == name(&case));
        match (pythrust("build", &main)?, gap) {
            (Err(stderr), true) if stderr.contains(super::NATIVE_UNSUPPORTED) => continue,
            (Err(stderr), _) => {
                failures.push(format!("{}: build failed:\n{stderr}", name(&case)));
                continue;
            }
            (Ok(()), true) => {
                let msg = "builds now; remove it from the modules' `NATIVE_GAPS`";
                failures.push(format!("{}: {msg}", name(&case)));
                continue;
            }
            (Ok(()), false) => {}
        }
        let exe = main.with_extension(std::env::consts::EXE_EXTENSION);
        let got = run(&mut native(&case, &exe, false), TIMEOUT)?;
        let failed = mismatch(&case, &got);
        let errored = failed.is_none() && got.status != 0;
        failures.extend(failed);
        if errored {
            failures.extend(stderr_mismatch(&exe, native(&case, &exe, false))?);
        }
        if memcheck_requested() {
            failures.extend(memcheck(&case, &exe)?);
        }
    }
    report(failures);
    dir.pass();
    Ok(())
}

/// `pythrust <command> main`: `Err` holds its stderr if it failed.
fn pythrust(command: &str, main: &Path) -> Res<Result<(), String>> {
    let out = Command::new(pythrust_bin()?)
        .arg(command)
        .arg(main)
        .output()
        .map_err(|e| format!("cannot run pythrust {command}: {e}"))?;
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    Ok(out.status.success().then_some(()).ok_or(stderr))
}
