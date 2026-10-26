//! `pythrust` command line: `run`, `check` or `build` a source file.
//!
//! The only place that does I/O. Stages are pure; this file reads the
//! file, calls them, renders diagnostics as `path:line:col`, and for
//! `build` writes the object file and links it with the system `cc`.

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use pythrust_compiler::{codegen, infer, lexer, lower, parser, DiagKind, Diagnostic, Span};

const USAGE: &str = "usage: pythrust <run|check|build> <file>";

/// The runtime library every native program links against. Found next
/// to this executable, where `cargo build` puts it, unless overridden.
const RUNTIME_LIB: &str = "libpythrust_rt.a";
const RUNTIME_ENV: &str = "PYTHRUST_RT";

/// System libraries a Rust static library needs (`rustc --print
/// native-static-libs`).
#[cfg(target_os = "macos")]
const NATIVE_LIBS: &[&str] = &["-lSystem", "-lc", "-lm"];
#[cfg(not(target_os = "macos"))]
const NATIVE_LIBS: &[&str] = &[
    "-lgcc_s",
    "-lutil",
    "-lrt",
    "-lpthread",
    "-lm",
    "-ldl",
    "-lc",
];

/// A subcommand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    /// Lex, parse, and interpret.
    Run,
    /// Lex, parse, and infer. Never executes the program.
    Check,
    /// Compile to a native executable beside the source, without its
    /// extension: `fib.pythrust` builds `fib`.
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
    let result = match command {
        Command::Build => compile(path, &src)
            .map_err(|diag| render(path, &src, &diag))
            .and_then(|object| link(path, &object)),
        _ => execute(command, &src).map_err(|diag| render(path, &src, &diag)),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprint!("{message}");
            ExitCode::FAILURE
        }
    }
}

/// Parses `<run|check> <file>`.
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

/// Runs the pipeline for `command` over `src`.
fn execute(command: Command, src: &str) -> pythrust_compiler::Result<()> {
    let tokens = lexer::lex(src)?;
    let module = parser::parse(&tokens)?;
    match command {
        Command::Check | Command::Build => infer::infer(&module).map(drop),
        // Thread D replaces this with the interpreter.
        Command::Run => Err(Diagnostic {
            span: module.span,
            message: "interpreter not implemented".to_string(),
            kind: DiagKind::Runtime,
        }),
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
    let out = Path::new(path).with_extension("");
    if out == Path::new(path) {
        return Err(format!(
            "pythrust: {path} has no extension to strip for the output name\n"
        ));
    }
    let runtime = runtime_lib()?;
    let obj = std::env::temp_dir().join(format!("pythrust-{}.o", std::process::id()));
    std::fs::write(&obj, object)
        .map_err(|e| format!("pythrust: cannot write {}: {e}\n", obj.display()))?;
    let linked = std::process::Command::new("cc")
        .arg(&obj)
        .arg(&runtime)
        .args(NATIVE_LIBS)
        .arg("-o")
        .arg(&out)
        .status();
    let removed = std::fs::remove_file(&obj);
    match linked {
        Ok(status) if status.success() => {}
        Ok(status) => {
            return Err(format!(
                "pythrust: linking {} failed: cc {status}\n",
                out.display()
            ))
        }
        Err(e) => return Err(format!("pythrust: cannot run cc: {e}\n")),
    }
    removed.map_err(|e| format!("pythrust: cannot remove {}: {e}\n", obj.display()))
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
        let run = args(&["run", "a.pythrust"]);
        let check = args(&["check", "b.pythrust"]);
        assert_eq!(parse_args(&run), Ok((Command::Run, "a.pythrust")));
        assert_eq!(parse_args(&check), Ok((Command::Check, "b.pythrust")));
        let build = args(&["build", "c.pythrust"]);
        assert_eq!(parse_args(&build), Ok((Command::Build, "c.pythrust")));
    }

    #[test]
    fn parse_args_rejects_unknown_command_and_wrong_arity() {
        assert!(parse_args(&args(&["bulid", "a"])).is_err());
        assert!(parse_args(&args(&[])).is_err());
        assert!(parse_args(&args(&["run"])).is_err());
        assert!(parse_args(&args(&["run", "a", "b"])).is_err());
    }

    #[test]
    fn execute_surfaces_the_first_stage_error() {
        let err = execute(Command::Check, "x = 1\ny = (2\n");
        assert_eq!(err.map_err(|d| d.kind), Err(DiagKind::Lex));
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
        let out = render("f.pythrust", src, &diag(10, 12));
        assert_eq!(
            out,
            "f.pythrust:2:5: parse error: bad\n  y = (2\n      ^^\n"
        );
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
