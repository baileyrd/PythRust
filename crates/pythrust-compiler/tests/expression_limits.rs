//! Flat syntax must not hide a tree deep enough to abort a compiler pass or drop.
use std::fs;
use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

mod common;

fn compiler(command: &str, dir: &Path, src: &str) -> Output {
    let path = dir.join("input.pyr");
    fs::write(&path, src).unwrap();
    let stdout = fs::File::create(dir.join("stdout")).unwrap();
    let stderr = fs::File::create(dir.join("stderr")).unwrap();
    let bin =
        std::env::var_os("PYTHRUST_BIN").unwrap_or_else(|| env!("CARGO_BIN_EXE_pythrust").into());
    let mut child = Command::new(bin)
        .arg(command)
        .arg(path)
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .unwrap();
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if start.elapsed() > Duration::from_secs(20) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("{command} exceeded 20 seconds");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    Output {
        status,
        stdout: fs::read(dir.join("stdout")).unwrap(),
        stderr: fs::read(dir.join("stderr")).unwrap(),
    }
}

fn chain(term: &str, op: &str, count: usize) -> String {
    vec![term; count].join(op)
}

fn rejected(dir: &Path, src: &str) {
    for command in ["check", "run", "build"] {
        let out = compiler(command, dir, src);
        let err = String::from_utf8_lossy(&out.stderr);
        assert_eq!(out.status.code(), Some(1), "{command}: {err}");
        assert!(out.stdout.is_empty(), "{out:?}");
        assert!(err.contains("too deeply nested"), "{command}: {err}");
        assert!(err.contains("input.pyr:1:"), "{err}");
        assert!(
            !err.contains("panicked") && !err.contains("stack overflow"),
            "{err}"
        );
    }
}

#[test]
fn long_flat_expressions_return_source_diagnostics() {
    let dir = common::scratch("expression_limits", "long_flat");
    for count in [10_000, 5_000, 100_000] {
        rejected(&dir, &format!("x = {}\n", chain("1", " + ", count)));
        // A later parse error used to drop the entire left-folded tree.
        rejected(&dir, &format!("x = {} +\n", chain("1", " + ", count)));
    }
    for expr in [
        chain("1", " * ", 5_000),
        chain("True", " and ", 5_000),
        chain("False", " or ", 5_000),
        format!("x{}", "[0]".repeat(5_000)),
        format!("x{}", ".a".repeat(5_000)),
        format!("f{}", "()".repeat(5_000)),
        // Each individual fold is short; their combined tree is too deep.
        format!("{}({})", "not ".repeat(60), chain("True", " and ", 60)),
        format!("{} + [{}]", chain("1", " + ", 60), chain("1", " + ", 100)),
    ] {
        rejected(&dir, &format!("x = {expr}\n"));
    }
}

#[test]
fn expression_depth_boundary_and_wide_controls() {
    let dir = common::scratch("expression_limits", "boundary");
    for count in [99, 100] {
        let src = format!("x = {}\nprint(x)\n", chain("1", " + ", count));
        for command in ["check", "run"] {
            let out = compiler(command, &dir, &src);
            assert!(out.status.success(), "{command}: {out:?}");
            if command == "run" {
                assert_eq!(out.stdout, format!("{count}\n").as_bytes());
            }
        }
    }
    rejected(&dir, &format!("x = {}\n", chain("1", " + ", 101)));
    // Width is not depth: a large display and comparison stay accepted.
    let src = format!(
        "x = [{}]\ny = {}\nprint(len(x), y)\n",
        chain("1", ", ", 1_000),
        chain("1", " == ", 1_000)
    );
    let out = compiler("run", &dir, &src);
    assert!(out.status.success(), "{out:?}");
    assert_eq!(out.stdout, b"1000 True\n");
}

#[test]
fn recursive_wrappers_cannot_hide_flat_depth() {
    let dir = common::scratch("expression_limits", "wrappers");
    let flat = chain("1", " + ", 100);
    for expr in [
        format!("({flat},)"),
        format!("[{flat}]"),
        format!("{{{flat}: 1}}"),
        format!("{{1: {flat}}}"),
        format!("{{{flat}}}"),
        format!("+({flat})"),
        format!("not ({flat})"),
        format!("({flat}) and 1"),
        format!("({flat}) == 1"),
        format!("1 == ({flat})"),
        format!("({flat}) if True else 1"),
        format!("1 if ({flat}) else 1"),
        format!("1 if True else ({flat})"),
        format!("({flat}) ** 1"),
        format!("1 ** ({flat})"),
        format!("f({flat})"),
        format!("({flat})[0]"),
        format!("x[{flat}]"),
        format!("({flat}).a"),
    ] {
        rejected(&dir, &format!("x = {expr}\n"));
    }
}
