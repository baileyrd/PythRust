//! `pythrust` command line: `run`, `check` or `build` a source file.
//!
//! The only place that does I/O. Stages are pure; this file reads the
//! file, calls them, renders diagnostics as `path:line:col`, and for
//! `build` writes the object file and links it with the system linker:
//! `cc` on Unix, MSVC's `link.exe` on Windows.

#![forbid(unsafe_code)]

use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use pythrust_compiler::{codegen, infer, interp, lexer, lower, parser, DiagKind, Diagnostic, Span};

const USAGE: &str = "usage: pythrust <run|check|build> <file>";

/// The runtime library every native program links against. Found next
/// to this executable, where `cargo build` puts it, unless overridden.
#[cfg(not(windows))]
const RUNTIME_LIB: &str = "libpythrust_rt.a";
#[cfg(windows)]
const RUNTIME_LIB: &str = "pythrust_rt.lib";
const RUNTIME_ENV: &str = "PYTHRUST_RT";

/// The compiled program's object file, in the private build directory.
#[cfg(not(windows))]
const OBJECT: &str = "program.o";
#[cfg(windows)]
const OBJECT: &str = "program.obj";

/// System libraries a Rust static library needs (`rustc --print
/// native-static-libs`).
#[cfg(target_os = "macos")]
const NATIVE_LIBS: &[&str] = &["-lSystem", "-lc", "-lm"];
#[cfg(all(unix, not(target_os = "macos")))]
const NATIVE_LIBS: &[&str] = &[
    "-lgcc_s",
    "-lutil",
    "-lrt",
    "-lpthread",
    "-lm",
    "-ldl",
    "-lc",
];
/// `msvcrt.lib` is the dynamic C runtime, as Rust's MSVC targets use;
/// it also supplies `mainCRTStartup`, which calls the runtime's `main`.
#[cfg(windows)]
const NATIVE_LIBS: &[&str] = &[
    "kernel32.lib",
    "ntdll.lib",
    "userenv.lib",
    "ws2_32.lib",
    "dbghelp.lib",
    "msvcrt.lib",
];

/// Linker flags that keep only what the program reaches and drop debug
/// info, as Cargo's release profile does (`strip = "debuginfo"`). Without
/// them a binary carries the standard library's debug info and its unused
/// code: 5.5 to 6.3 MB instead of 0.44 to 0.52 MB on x86-64 Linux.
/// Symbols stay for backtraces. On macOS, ld64's `-S` is the analogue of
/// `--strip-debug`: it leaves out the debug map. MSVC writes debug info
/// only with `/DEBUG`, so there it only needs unreferenced code dropped
/// (`/OPT:REF`) and identical code folded (`/OPT:ICF`).
#[cfg(target_os = "macos")]
const LINK_FLAGS: &[&str] = &["-Wl,-dead_strip", "-Wl,-S"];
#[cfg(all(unix, not(target_os = "macos")))]
const LINK_FLAGS: &[&str] = &["-Wl,--gc-sections", "-Wl,--strip-debug"];
#[cfg(windows)]
const LINK_FLAGS: &[&str] = &["/NOLOGO", "/SUBSYSTEM:CONSOLE", "/OPT:REF", "/OPT:ICF"];

/// A subcommand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    /// Lex, parse, infer, and interpret. A compile error runs nothing.
    Run,
    /// Lex, parse, and infer. Never executes the program.
    Check,
    /// Compile to a native executable beside the source, named like it
    /// with the host's executable extension: `fib.pyr` builds `fib`, or
    /// `fib.exe` on Windows.
    Build,
}

/// A 1-based source position. `col` counts chars, not bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LineCol {
    line: usize,
    col: usize,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (command, path) = match parse_args(&args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("{message}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let src = match std::fs::read_to_string(path) {
        Ok(src) => src,
        Err(err) => {
            eprintln!("pythrust: cannot read {path}: {err}");
            return ExitCode::FAILURE;
        }
    };
    if command == Command::Build {
        return build(path, &src);
    }
    // Buffered for speed; flushed before any diagnostic so output already
    // printed precedes the error, as in CPython.
    let mut out = BufWriter::new(std::io::stdout());
    let result = execute(command, &src, &mut out).and_then(|()| flush(&mut out));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(diag) => {
            // The diagnostic is the error worth reporting; a flush failure
            // here would only repeat a broken stdout.
            let _ = out.flush();
            eprint!("{}", render(path, &src, &diag));
            ExitCode::FAILURE
        }
    }
}

/// Parses `<run|check|build> <file>`.
fn parse_args(args: &[String]) -> Result<(Command, &str), String> {
    let [command, path] = args else {
        return Err(format!("expected 2 arguments, got {}", args.len()));
    };
    let command = match command.as_str() {
        "run" => Command::Run,
        "check" => Command::Check,
        "build" => Command::Build,
        other => return Err(format!("unknown command `{other}`")),
    };
    Ok((command, path.as_str()))
}

/// Runs the pipeline for `command` over `src`, printing program output to
/// `out`. Every compile stage finishes before anything runs.
fn execute(
    command: Command,
    src: &str,
    out: &mut (dyn Write + Send),
) -> pythrust_compiler::Result<()> {
    let tokens = lexer::lex(src)?;
    let module = parser::parse(&tokens)?;
    infer::infer(&module)?;
    match command {
        Command::Check | Command::Build => Ok(()),
        Command::Run => interp::run(&module, out),
    }
}

/// Flushes program output; a failure (say, a closed pipe) is a runtime error.
fn flush(out: &mut impl Write) -> pythrust_compiler::Result<()> {
    out.flush().map_err(|e| Diagnostic {
        span: Span { start: 0, end: 0 },
        message: format!("cannot write output: {e}"),
        kind: DiagKind::Runtime,
    })
}

/// `pythrust build`: compiles and links, reporting any failure.
fn build(path: &str, src: &str) -> ExitCode {
    let built = compile(path, src)
        .map_err(|diag| render(path, src, &diag))
        .and_then(|object| link(path, &object));
    match built {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprint!("{message}");
            ExitCode::FAILURE
        }
    }
}

/// Compiles `src` to an object file. Runtime errors in it are rendered
/// now, against `path`, so the binary prints what `run` would.
fn compile(path: &str, src: &str) -> pythrust_compiler::Result<Vec<u8>> {
    let tokens = lexer::lex(src)?;
    let module = parser::parse(&tokens)?;
    let types = infer::infer(&module)?;
    let program = lower::lower(&module, &types)?;
    codegen::emit(&program, &|diag| render(path, src, diag))
}

/// Links `object` with the runtime into the executable for `path`.
fn link(path: &str, object: &[u8]) -> Result<(), String> {
    let out = output_path(path)?;
    let runtime = runtime_lib()?;
    let dir = private_dir()?;
    let linked = link_in(&dir, object, &runtime, &out);
    let removed = std::fs::remove_dir_all(&dir)
        .map_err(|e| format!("pythrust: cannot remove {}: {e}\n", dir.display()));
    linked.and(removed)
}

/// The executable `pythrust build` writes for `path`, refused if it would
/// be the source itself. Names alone cannot tell: Windows and macOS file
/// systems ignore case, so `foo.EXE` builds `foo.exe` over itself.
fn output_path(path: &str) -> Result<PathBuf, String> {
    let source = Path::new(path);
    let out = source.with_extension(std::env::consts::EXE_EXTENSION);
    if out == source || same_file(&out, source)? {
        return Err(format!(
            "pythrust: cannot build {path}: the executable would overwrite it\n"
        ));
    }
    Ok(out)
}

/// Whether existing `a` is the file at `b`, after the file system resolves
/// case, short names and symlinks. A missing `a` is no file at all.
fn same_file(a: &Path, b: &Path) -> Result<bool, String> {
    let exists = a
        .try_exists()
        .map_err(|e| format!("pythrust: cannot inspect {}: {e}\n", a.display()))?;
    if !exists {
        return Ok(false);
    }
    let canonical = |p: &Path| {
        std::fs::canonicalize(p)
            .map_err(|e| format!("pythrust: cannot resolve {}: {e}\n", p.display()))
    };
    Ok(canonical(a)? == canonical(b)?)
}

/// Writes `object` into `dir` and links it with `runtime` into `out`.
fn link_in(dir: &Path, object: &[u8], runtime: &Path, out: &Path) -> Result<(), String> {
    let obj = dir.join(OBJECT);
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&obj)
        .and_then(|mut f| f.write_all(object))
        .map_err(|e| format!("pythrust: cannot write {}: {e}\n", obj.display()))?;
    let mut linker = linker()?;
    let output = linker
        .arg(&obj)
        .arg(runtime)
        .args(NATIVE_LIBS)
        .args(LINK_FLAGS)
        .args(output_args(out))
        .output()
        .map_err(|e| format!("pythrust: cannot run {}: {e}\n", linker_name(&linker)))?;
    if output.status.success() {
        return Ok(());
    }
    // `link.exe` reports errors on stdout, `cc` on stderr; show both.
    Err(format!(
        "pythrust: linking {} failed: {} {}\n{}{}",
        out.display(),
        linker_name(&linker),
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    ))
}

/// The system C compiler driver, which runs the linker.
#[cfg(not(windows))]
fn linker() -> Result<std::process::Command, String> {
    Ok(std::process::Command::new("cc"))
}

/// MSVC's `link.exe`, from a developer prompt's `PATH` or the newest
/// installed Visual Studio, with its library paths set, as Cargo finds it.
#[cfg(windows)]
fn linker() -> Result<std::process::Command, String> {
    find_msvc_tools::find(std::env::consts::ARCH, "link.exe").ok_or_else(|| {
        "pythrust: cannot find the MSVC linker link.exe; install Visual Studio Build Tools with the C++ workload\n"
            .to_string()
    })
}

/// The linker's program name, for messages.
fn linker_name(linker: &std::process::Command) -> String {
    let program = Path::new(linker.get_program());
    program
        .file_name()
        .unwrap_or(program.as_os_str())
        .to_string_lossy()
        .into_owned()
}

/// The arguments naming the executable to write.
#[cfg(not(windows))]
fn output_args(out: &Path) -> Vec<std::ffi::OsString> {
    vec!["-o".into(), out.into()]
}

#[cfg(windows)]
fn output_args(out: &Path) -> Vec<std::ffi::OsString> {
    let mut arg = std::ffi::OsString::from("/OUT:");
    arg.push(out);
    vec![arg]
}

/// A fresh directory under the system temp dir that only this user can
/// read, so no other process can swap or read the object before the linker
/// does. On Unix it is created mode 0700; on Windows the temp dir is
/// already per-user (`%LOCALAPPDATA%\Temp`) and the new directory inherits
/// its owner-only ACL.
/// Created exclusively: an existing path is never reused.
fn private_dir() -> Result<PathBuf, String> {
    let base = std::env::temp_dir();
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let builder = owner_only_builder();
    for attempt in 0..100 {
        let dir = base.join(format!("pythrust-{}-{nanos}-{attempt}", std::process::id()));
        match builder.create(&dir) {
            Ok(()) => return Ok(dir),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("pythrust: cannot create {}: {e}\n", dir.display())),
        }
    }
    Err(format!(
        "pythrust: no free temporary directory in {}\n",
        base.display()
    ))
}

#[cfg(unix)]
fn owner_only_builder() -> std::fs::DirBuilder {
    let mut builder = std::fs::DirBuilder::new();
    std::os::unix::fs::DirBuilderExt::mode(&mut builder, 0o700);
    builder
}

/// Windows ACLs come from the per-user temp dir; see [`private_dir`].
#[cfg(not(unix))]
fn owner_only_builder() -> std::fs::DirBuilder {
    std::fs::DirBuilder::new()
}

/// The runtime library: `$PYTHRUST_RT`, or next to this executable.
fn runtime_lib() -> Result<PathBuf, String> {
    let path = match std::env::var_os(RUNTIME_ENV) {
        Some(p) => PathBuf::from(p),
        None => std::env::current_exe()
            .map_err(|e| format!("pythrust: cannot locate myself: {e}\n"))?
            .with_file_name(RUNTIME_LIB),
    };
    if !path.is_file() {
        return Err(format!(
            "pythrust: runtime library not found at {}; run `cargo build -p pythrust-rt` or set {RUNTIME_ENV}\n",
            path.display()
        ));
    }
    Ok(path)
}

/// Renders `diag` as a header, the offending line, and a caret underline.
fn render(path: &str, src: &str, diag: &Diagnostic) -> String {
    let start = line_col(src, diag.span.start);
    let text = line_text(src, diag.span.start);
    let width = caret_width(text, start.col, diag.span, src);
    format!(
        "{path}:{}:{}: {} error: {}\n  {text}\n  {}{}\n",
        start.line,
        start.col,
        diag.kind,
        diag.message,
        " ".repeat(start.col - 1),
        "^".repeat(width),
    )
}

/// Maps a byte offset to a line and column. Offsets past the end clamp to
/// the end; offsets inside a multi-byte char snap to that char.
fn line_col(src: &str, offset: usize) -> LineCol {
    let offset = floor_char_boundary(src, offset);
    let before = &src[..offset];
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    LineCol {
        line: before.matches('\n').count() + 1,
        col: before[line_start..].chars().count() + 1,
    }
}

/// The full line containing `offset`, without its line ending.
fn line_text(src: &str, offset: usize) -> &str {
    let offset = floor_char_boundary(src, offset);
    let line_start = src[..offset].rfind('\n').map_or(0, |i| i + 1);
    let rest = &src[line_start..];
    let line = rest.split('\n').next().unwrap_or(rest);
    line.strip_suffix('\r').unwrap_or(line)
}

/// Carets for the part of `span` on its first line; at least one.
fn caret_width(line: &str, col: usize, span: Span, src: &str) -> usize {
    let start = floor_char_boundary(src, span.start);
    let end = floor_char_boundary(src, span.end).max(start);
    let in_span = src[start..end].split('\n').next().unwrap_or("");
    let room = line.chars().count().saturating_sub(col - 1);
    in_span.chars().count().min(room).max(1)
}

/// The largest char boundary `<= offset`, clamped to `src.len()`.
fn floor_char_boundary(src: &str, offset: usize) -> usize {
    let mut offset = offset.min(src.len());
    while !src.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn at(line: usize, col: usize) -> LineCol {
        LineCol { line, col }
    }

    fn diag(start: usize, end: usize) -> Diagnostic {
        Diagnostic {
            span: Span { start, end },
            message: "bad".to_string(),
            kind: DiagKind::Parse,
        }
    }

    #[test]
    fn parse_args_accepts_each_command() {
        let run = args(&["run", "a.pyr"]);
        let check = args(&["check", "b.pyr"]);
        assert_eq!(parse_args(&run), Ok((Command::Run, "a.pyr")));
        assert_eq!(parse_args(&check), Ok((Command::Check, "b.pyr")));
        let build = args(&["build", "c.pyr"]);
        assert_eq!(parse_args(&build), Ok((Command::Build, "c.pyr")));
    }

    #[test]
    fn parse_args_rejects_unknown_command_and_wrong_arity() {
        assert!(parse_args(&args(&["bulid", "a"])).is_err());
        assert!(parse_args(&args(&[])).is_err());
        assert!(parse_args(&args(&["run"])).is_err());
        assert!(parse_args(&args(&["run", "a", "b"])).is_err());
    }

    fn exec(command: Command, src: &str) -> (pythrust_compiler::Result<()>, String) {
        let mut out = Vec::new();
        let result = execute(command, src, &mut out);
        (result, String::from_utf8_lossy(&out).into_owned())
    }

    #[test]
    fn execute_surfaces_the_first_stage_error() {
        let (err, _) = exec(Command::Check, "x = 1\ny = (2\n");
        assert_eq!(err.map_err(|d| d.kind), Err(DiagKind::Lex));
    }

    #[test]
    fn run_prints_program_output() {
        let (result, out) = exec(Command::Run, "print(1 + 2)\n");
        assert_eq!(result, Ok(()));
        assert_eq!(out, "3\n");
    }

    #[test]
    fn run_on_type_error_runs_nothing() {
        let (result, out) = exec(Command::Run, "print(1)\nx = 1 + \"a\"\n");
        assert_eq!(result.map_err(|d| d.kind), Err(DiagKind::Type));
        assert_eq!(out, "");
    }

    #[test]
    fn run_keeps_output_before_a_runtime_error() {
        let (result, out) = exec(Command::Run, "print(1)\nx = 0\nprint(1 // x)\n");
        assert_eq!(result.map_err(|d| d.kind), Err(DiagKind::Runtime));
        assert_eq!(out, "1\n");
    }

    #[test]
    fn check_never_runs_the_program() {
        let (result, out) = exec(Command::Check, "print(1)\n");
        assert_eq!(result, Ok(()));
        assert_eq!(out, "");
    }

    #[test]
    fn compile_reports_unsupported_code_as_a_backend_error() {
        let src = "x = print(7)\ndef print(n):\n    return 1\n";
        let err = compile("f.pyr", src).map(drop);
        assert_eq!(err.map_err(|d| d.kind), Err(DiagKind::Backend));
        assert!(compile("f.pyr", "print(1 + 2)\n").is_ok());
    }

    #[test]
    fn private_dirs_are_fresh_and_owner_only() {
        let (a, b) = (private_dir().unwrap(), private_dir().unwrap());
        assert_ne!(a, b);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&a).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o700);
        }
        std::fs::remove_dir(&a).unwrap();
        std::fs::remove_dir(&b).unwrap();
    }

    /// A source whose executable name is itself: no extension on Unix,
    /// `.EXE` in another case on Windows.
    #[cfg(windows)]
    const SELF_NAMED: &str = "p.EXE";
    #[cfg(not(windows))]
    const SELF_NAMED: &str = "p";

    #[test]
    fn build_refuses_to_overwrite_its_source() {
        let dir = private_dir().unwrap();
        let source = dir.join(SELF_NAMED);
        std::fs::write(&source, "print(1)\n").unwrap();
        let err = link(source.to_str().unwrap(), b"object").unwrap_err();
        assert!(err.contains("would overwrite"), "{err}");
        assert_eq!(std::fs::read(&source).unwrap(), b"print(1)\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Where the names differ but the file is the same.
    #[cfg(unix)]
    #[test]
    fn build_refuses_an_executable_name_linked_to_its_source() {
        let dir = private_dir().unwrap();
        let source = dir.join("p.pyr");
        std::fs::write(&source, "print(1)\n").unwrap();
        std::os::unix::fs::symlink(&source, dir.join("p")).unwrap();
        let err = link(source.to_str().unwrap(), b"object").unwrap_err();
        assert!(err.contains("would overwrite"), "{err}");
        assert_eq!(std::fs::read(&source).unwrap(), b"print(1)\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn output_path_allows_a_distinct_or_missing_executable() {
        let dir = private_dir().unwrap();
        let source = dir.join("p.pyr");
        std::fs::write(&source, "").unwrap();
        let path = source.to_str().unwrap();
        let want = source.with_extension(std::env::consts::EXE_EXTENSION);
        assert_eq!(output_path(path), Ok(want.clone()));
        std::fs::write(&want, "old build").unwrap();
        assert_eq!(output_path(path), Ok(want));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn line_col_is_one_based() {
        let src = "ab\ncd\n";
        assert_eq!(line_col(src, 0), at(1, 1));
        assert_eq!(line_col(src, 1), at(1, 2));
        assert_eq!(line_col(src, 2), at(1, 3));
        assert_eq!(line_col(src, 3), at(2, 1));
        assert_eq!(line_col(src, 6), at(3, 1));
    }

    #[test]
    fn line_col_counts_chars_and_clamps() {
        let src = "é = 1";
        assert_eq!(line_col(src, 2), at(1, 2));
        assert_eq!(line_col(src, 1), at(1, 1));
        assert_eq!(line_col(src, 99), at(1, 6));
        assert_eq!(line_col("", 0), at(1, 1));
    }

    #[test]
    fn line_text_strips_line_endings() {
        let src = "a\r\nbcd\r\n";
        assert_eq!(line_text(src, 0), "a");
        assert_eq!(line_text(src, 4), "bcd");
    }

    #[test]
    fn render_underlines_the_span() {
        let src = "x = 1\ny = (2\n";
        let out = render("f.pyr", src, &diag(10, 12));
        assert_eq!(out, "f.pyr:2:5: parse error: bad\n  y = (2\n      ^^\n");
    }

    #[test]
    fn render_cuts_multiline_span_and_marks_empty_span() {
        let src = "if x:\n  y\n";
        let multi = render("f", src, &diag(3, 9));
        assert!(multi.ends_with("  if x:\n     ^^\n"));
        let empty = render("f", src, &diag(0, 0));
        assert!(empty.ends_with("  if x:\n  ^\n"));
    }

    #[test]
    fn render_handles_span_at_end_of_file() {
        let out = render("f", "x", &diag(1, 1));
        assert_eq!(out, "f:1:2: parse error: bad\n  x\n   ^\n");
    }
}
