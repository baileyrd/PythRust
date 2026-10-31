//! `pythrust` command line: `run`, `check` or `build` a source file.
//!
//! The only place that does I/O. Stages are pure; this file reads the
//! file, calls them, renders diagnostics and warnings as
//! `path:line:col` (warnings once every check passes, before anything
//! runs; they never change the exit status), and for
//! `build` writes the object file and links it with the system linker:
//! `cc` on Unix, MSVC's `link.exe` on Windows. A program calling Rust
//! bindings first has cargo build them with the runtime ([`cargo`]).

#![forbid(unsafe_code)]

mod bindings;
mod cargo;

use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use bindings::Sources;
use pythrust_compiler::ast::{Binding, Module};
use pythrust_compiler::infer::Types;
use pythrust_compiler::interop::gen;
use pythrust_compiler::{
    codegen, infer, interp, lexer, lower, own, parser, rc, stdlib, DiagKind, Diagnostic, Span,
    Warning,
};

const USAGE: &str = "usage: pythrust <check|build> [--rust name=path ...] <file> \
                     | pythrust run [--rust name=path ...] <file> [arg ...]";

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
    /// Lex, parse, infer, summarize ownership, and interpret. A compile
    /// error runs nothing.
    Run,
    /// Lex, parse, infer, and summarize ownership. Never executes the
    /// program.
    Check,
    /// Compile to a native executable beside the source, named like it
    /// with the host's executable extension: `fib.pyr` builds `fib`, or
    /// `fib.exe` on Windows.
    Build,
}

/// A parsed command line.
#[derive(Debug, PartialEq, Eq)]
struct Invocation<'a> {
    command: Command,
    /// Each `--rust name=path` (M6b, interim until M6d), in order.
    rust: Vec<(&'a str, &'a str)>,
    path: &'a str,
    /// The program's arguments, after the file: `run` only.
    rest: &'a [String],
}

/// A 1-based source position. `col` counts chars, not bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LineCol {
    line: usize,
    col: usize,
}

fn main() -> ExitCode {
    // Lossy, as a built program's `sys.argv` is (a listed deviation).
    let args: Vec<String> = std::env::args_os()
        .skip(1)
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    let inv = match parse_args(&args) {
        Ok(parsed) => parsed,
        Err(message) => {
            eprintln!("{message}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let path = inv.path;
    let src = match std::fs::read_to_string(path) {
        Ok(src) => src,
        Err(err) => {
            eprintln!("pythrust: cannot read {path}: {err}");
            return ExitCode::FAILURE;
        }
    };
    let mut sources = Sources::new(path, &src);
    let rust = match bindings::load(&inv.rust, &mut sources) {
        Ok(rust) => rust,
        Err(message) => {
            eprint!("{message}");
            return ExitCode::FAILURE;
        }
    };
    let (module, types) = match front(&src, &rust) {
        Ok(checked) => checked,
        Err(diag) => {
            eprint!("{}", sources.render(&diag));
            return ExitCode::FAILURE;
        }
    };
    for w in types.warnings() {
        eprint!("{}", sources.render_warning(w));
    }
    match inv.command {
        Command::Check => ExitCode::SUCCESS,
        Command::Build => build(&sources, &module, &types, &inv.rust),
        Command::Run if !module.stubs.is_empty() => {
            eprintln!("pythrust: {}", stdlib::RUST_RUN_GAP);
            ExitCode::FAILURE
        }
        Command::Run => {
            let argv: Vec<String> = std::iter::once(path.to_string())
                .chain(inv.rest.iter().cloned())
                .collect();
            run(&sources, &module, &types, &argv)
        }
    }
}

/// `pythrust run`: interprets the checked `module` with `argv` as
/// `sys.argv`.
fn run(sources: &Sources, module: &Module, types: &Types, argv: &[String]) -> ExitCode {
    // Buffered for speed; flushed before any diagnostic so output already
    // printed precedes the error, as in CPython.
    let mut out = BufWriter::new(std::io::stdout());
    let result = interp::run(module, types, argv, &mut out).and_then(|()| flush(&mut out));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(diag) => {
            // The diagnostic is the error worth reporting; a flush failure
            // here would only repeat a broken stdout.
            let _ = out.flush();
            eprint!("{}", sources.render(&diag));
            ExitCode::FAILURE
        }
    }
}

/// Parses `<check|build> [--rust name=path ...] <file>` or
/// `run [--rust name=path ...] <file> [arg ...]`: the command, the
/// bindings, the file, and the program's arguments after it, as
/// `python` takes them.
fn parse_args(args: &[String]) -> Result<Invocation<'_>, String> {
    let [command, more @ ..] = args else {
        return Err("expected a command".to_string());
    };
    let command = match command.as_str() {
        "run" => Command::Run,
        "check" => Command::Check,
        "build" => Command::Build,
        other => return Err(format!("unknown command `{other}`")),
    };
    let mut rust = Vec::new();
    let mut more = more;
    while let [flag, value, after @ ..] = more {
        if flag != "--rust" {
            break;
        }
        rust.push(bindings::split(value)?);
        more = after;
    }
    let [path, rest @ ..] = more else {
        return Err("expected a file".to_string());
    };
    if path == "--rust" {
        return Err("`--rust` takes `name=path`".to_string());
    }
    if command != Command::Run && !rest.is_empty() {
        return Err(format!("expected one file, got {}", rest.len() + 1));
    }
    Ok(Invocation {
        command,
        rust,
        path: path.as_str(),
        rest,
    })
}

/// The checks `run`, `check` and `build` share: lex, parse, infer, and
/// check ownership over every function. All finish before anything runs.
/// `import` may name one of `rust`.
fn front(src: &str, rust: &[Binding]) -> pythrust_compiler::Result<(Module, Types)> {
    let tokens = lexer::lex(src)?;
    let module = parser::parse_with(&tokens, rust)?;
    let types = infer::infer(&module)?;
    own::check(&module, &own::analyze(&module, &types))?;
    Ok((module, types))
}

/// Flushes program output; a failure (say, a closed pipe) is a runtime error.
fn flush(out: &mut impl Write) -> pythrust_compiler::Result<()> {
    out.flush().map_err(|e| Diagnostic {
        span: Span { start: 0, end: 0 },
        message: format!("cannot write output: {e}"),
        kind: DiagKind::Runtime,
    })
}

/// `pythrust build`: compiles the checked `module` and links it,
/// reporting any failure.
fn build(sources: &Sources, module: &Module, types: &Types, rust: &[(&str, &str)]) -> ExitCode {
    match native(sources, module, types, rust) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprint!("{message}");
            ExitCode::FAILURE
        }
    }
}

/// What a program links against: a runtime staticlib, and the system
/// libraries it needs.
struct Runtime {
    lib: PathBuf,
    native: Vec<String>,
}

/// Compiles and links `module`. A program calling Rust bindings first
/// has its shims generated, which names any stub this step cannot call
/// before anything is compiled; cargo then builds them with the runtime.
fn native(
    sources: &Sources,
    module: &Module,
    types: &Types,
    rust: &[(&str, &str)],
) -> Result<(), String> {
    let out = absolute(&output_path(sources.path())?)?;
    let render = |diag: &Diagnostic| sources.render(diag);
    let shims = match module.stubs.is_empty() {
        true => None,
        false => {
            Some(gen::shims(&module.stubs, &|span| sources.place(span)).map_err(|d| render(&d))?)
        }
    };
    let object = compile(sources, module, types).map_err(|d| render(&d))?;
    let runtime = match shims {
        None => plain_runtime()?,
        Some(lib) => cargo::build(&lib, &called(module, rust)?, &out)?,
    };
    link(&out, &object, &runtime)
}

/// The bindings `module` calls, with absolute directories.
fn called(module: &Module, rust: &[(&str, &str)]) -> Result<Vec<cargo::Crate>, String> {
    let calls = |name: &str| {
        let prefix = format!("{name}.");
        module
            .stubs
            .iter()
            .any(|s| s.name.name.starts_with(&prefix))
    };
    rust.iter()
        .filter(|(name, _)| calls(name))
        .map(|(name, dir)| {
            Ok(cargo::Crate {
                name: name.to_string(),
                dir: absolute(Path::new(dir))?,
            })
        })
        .collect()
}

/// Compiles the checked `module` to an object file, lowering only what
/// runs. Runtime errors in it are rendered now, against their source
/// file, so the binary prints what `run` would.
fn compile(
    sources: &Sources,
    module: &Module,
    types: &Types,
) -> pythrust_compiler::Result<Vec<u8>> {
    let program = rc::elide(lower::lower(module, types)?);
    codegen::emit(&program, &|diag| sources.render(diag))
}

/// The plain runtime, `pythrust-rt`, and the system libraries a Rust
/// staticlib needs.
fn plain_runtime() -> Result<Runtime, String> {
    Ok(Runtime {
        lib: absolute(&runtime_lib()?)?,
        native: NATIVE_LIBS.iter().map(ToString::to_string).collect(),
    })
}

/// Links `object` with `runtime` into the executable `out`. Every path
/// handed to the linker is absolute, so none can read as an option
/// (`-work/...`) or a response file (`@scope/...`).
fn link(out: &Path, object: &[u8], runtime: &Runtime) -> Result<(), String> {
    let dir = private_dir(output_dir(out))?;
    let linked = link_in(&dir, object, runtime, out);
    let removed = std::fs::remove_dir_all(&dir)
        .map_err(|e| format!("pythrust: cannot remove {}: {e}\n", dir.display()));
    linked.and(removed)
}

/// `path` made absolute against the current directory, without touching
/// the file system.
fn absolute(path: &Path) -> Result<PathBuf, String> {
    std::path::absolute(path)
        .map_err(|e| format!("pythrust: cannot resolve {}: {e}\n", path.display()))
}

/// The directory `out` is written to: `.` for a bare file name.
fn output_dir(out: &Path) -> &Path {
    match out.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    }
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
fn link_in(dir: &Path, object: &[u8], runtime: &Runtime, out: &Path) -> Result<(), String> {
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
        .arg(&runtime.lib)
        .args(&runtime.native)
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

/// A fresh build directory in `base`, the directory the executable goes
/// to. The object becomes that executable, so it is kept exactly as safe:
/// whoever could swap or read it there could swap or read the output too.
/// The system temp dir would promise more than it holds, since `TMP` may
/// name a shared directory and Windows offers no owner-only mode without
/// writing an ACL. On Unix the directory is also mode 0700. Created
/// exclusively: an existing path is never reused.
fn private_dir(base: &Path) -> Result<PathBuf, String> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let builder = owner_only_builder();
    for attempt in 0..100 {
        let dir = base.join(format!(
            ".pythrust-build-{}-{nanos}-{attempt}",
            std::process::id()
        ));
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

/// Windows ACLs are inherited from the output directory; see [`private_dir`].
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
    let label = format!("{} error", diag.kind);
    render_at(path, src, diag.span, &label, &diag.message)
}

/// Renders `w` as [`render`] does an error.
fn render_warning(path: &str, src: &str, w: &Warning) -> String {
    render_at(path, src, w.span, "warning", &w.message)
}

fn render_at(path: &str, src: &str, span: Span, label: &str, message: &str) -> String {
    let start = line_col(src, span.start);
    let text = line_text(src, span.start);
    let width = caret_width(text, start.col, span, src);
    format!(
        "{path}:{}:{}: {label}: {message}\n  {text}\n  {}{}\n",
        start.line,
        start.col,
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

    fn invocation<'a>(command: Command, path: &'a str, rest: &'a [String]) -> Invocation<'a> {
        Invocation {
            command,
            rust: Vec::new(),
            path,
            rest,
        }
    }

    #[test]
    fn parse_args_accepts_each_command() {
        let run = args(&["run", "a.pyr"]);
        let check = args(&["check", "b.pyr"]);
        let none: &[String] = &[];
        assert_eq!(
            parse_args(&run),
            Ok(invocation(Command::Run, "a.pyr", none))
        );
        assert_eq!(
            parse_args(&check),
            Ok(invocation(Command::Check, "b.pyr", none))
        );
        let build = args(&["build", "c.pyr"]);
        assert_eq!(
            parse_args(&build),
            Ok(invocation(Command::Build, "c.pyr", none))
        );
    }

    #[test]
    fn parse_args_passes_what_follows_the_file_to_run() {
        let run = args(&["run", "a.pyr", "x", "--y"]);
        let rest = args(&["x", "--y"]);
        assert_eq!(
            parse_args(&run),
            Ok(invocation(Command::Run, "a.pyr", &rest))
        );
    }

    #[test]
    fn parse_args_takes_rust_bindings_before_the_file() {
        let given = args(&["check", "--rust", "a=x", "--rust", "b=../y z", "f.pyr"]);
        let none: &[String] = &[];
        let want = Invocation {
            rust: vec![("a", "x"), ("b", "../y z")],
            ..invocation(Command::Check, "f.pyr", none)
        };
        assert_eq!(parse_args(&given), Ok(want));
        let run = args(&["run", "--rust", "a=x", "f.pyr", "--rust", "b=y"]);
        let rest = args(&["--rust", "b=y"]);
        let got = parse_args(&run).map(|i| (i.rust, i.rest.to_vec()));
        assert_eq!(got, Ok((vec![("a", "x")], rest)));
        assert!(parse_args(&args(&["check", "--rust", "f.pyr"])).is_err());
        assert!(parse_args(&args(&["check", "--rust"])).is_err());
        assert!(parse_args(&args(&["check", "--rust", "=x", "f.pyr"])).is_err());
        assert!(parse_args(&args(&["check", "f.pyr", "--rust", "a=x"])).is_err());
    }

    #[test]
    fn parse_args_rejects_unknown_command_and_wrong_arity() {
        assert!(parse_args(&args(&["bulid", "a"])).is_err());
        assert!(parse_args(&args(&[])).is_err());
        assert!(parse_args(&args(&["run"])).is_err());
        assert!(parse_args(&args(&["check", "a", "b"])).is_err());
        assert!(parse_args(&args(&["build", "a", "b"])).is_err());
    }

    /// What `main` does for `command` short of rendering: the checks,
    /// then the program for `run`.
    fn exec(command: Command, src: &str) -> (pythrust_compiler::Result<()>, String) {
        let mut out = Vec::new();
        let result = front(src, &[]).and_then(|(module, types)| match command {
            Command::Run => interp::run(&module, &types, &["f.pyr".to_string()], &mut out),
            Command::Check | Command::Build => Ok(()),
        });
        (result, String::from_utf8_lossy(&out).into_owned())
    }

    fn compiled(src: &str) -> pythrust_compiler::Result<Vec<u8>> {
        let (module, types) = front(src, &[])?;
        compile(&Sources::new("f.pyr", src), &module, &types)
    }

    #[test]
    fn front_surfaces_the_first_stage_error() {
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
        let err = compiled(src).map(drop);
        assert_eq!(err.map_err(|d| d.kind), Err(DiagKind::Backend));
        assert!(compiled("print(1 + 2)\n").is_ok());
    }

    #[test]
    fn checks_pass_with_warnings_and_render_them_at_the_clause() {
        let src = "def f(n: int) -> int raises Exception:\n    return n\n";
        let (_, types) = front(src, &[]).unwrap();
        let [w] = types.warnings() else {
            panic!("{:?}", types.warnings());
        };
        let out = render_warning("f.pyr", src, w);
        let head = "f.pyr:1:29: warning: `raises Exception` admits any exception";
        assert!(out.starts_with(head), "{out}");
        assert!(out.ends_with("remove the `raises` clause\n  def f(n: int) -> int raises Exception:\n                              ^^^^^^^^^\n"), "{out}");
    }

    #[test]
    fn build_dirs_go_beside_the_output() {
        assert_eq!(output_dir(Path::new("fib.exe")), Path::new("."));
        assert_eq!(output_dir(Path::new("a/fib")), Path::new("a"));
        let base = private_dir(&std::env::temp_dir()).unwrap();
        let dir = private_dir(&base).unwrap();
        assert_eq!(dir.parent(), Some(base.as_path()));
        std::fs::remove_dir_all(&base).unwrap();
    }

    #[test]
    fn private_dirs_are_fresh_and_owner_only() {
        let (a, b) = (
            private_dir(&std::env::temp_dir()).unwrap(),
            private_dir(&std::env::temp_dir()).unwrap(),
        );
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
        let dir = private_dir(&std::env::temp_dir()).unwrap();
        let source = dir.join(SELF_NAMED);
        std::fs::write(&source, "print(1)\n").unwrap();
        let err = output_path(source.to_str().unwrap()).unwrap_err();
        assert!(err.contains("would overwrite"), "{err}");
        assert_eq!(std::fs::read(&source).unwrap(), b"print(1)\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    /// Where the names differ but the file is the same.
    #[cfg(unix)]
    #[test]
    fn build_refuses_an_executable_name_linked_to_its_source() {
        let dir = private_dir(&std::env::temp_dir()).unwrap();
        let source = dir.join("p.pyr");
        std::fs::write(&source, "print(1)\n").unwrap();
        std::os::unix::fs::symlink(&source, dir.join("p")).unwrap();
        let err = output_path(source.to_str().unwrap()).unwrap_err();
        assert!(err.contains("would overwrite"), "{err}");
        assert_eq!(std::fs::read(&source).unwrap(), b"print(1)\n");
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn output_path_allows_a_distinct_or_missing_executable() {
        let dir = private_dir(&std::env::temp_dir()).unwrap();
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
