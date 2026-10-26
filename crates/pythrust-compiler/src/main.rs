//! `pythrust` command line: `run` and `check` a source file.
//!
//! The only place that does I/O. Stages are pure; this file reads the
//! file, calls them, and renders diagnostics as `path:line:col`.

#![forbid(unsafe_code)]

use std::io::{BufWriter, Write};
use std::process::ExitCode;

use pythrust_compiler::{infer, interp, lexer, parser, DiagKind, Diagnostic, Span};

const USAGE: &str = "usage: pythrust <run|check> <file>";

/// A subcommand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Command {
    /// Lex, parse, infer, and interpret. A compile error runs nothing.
    Run,
    /// Lex, parse, and infer. Never executes the program.
    Check,
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

/// Parses `<run|check> <file>`.
fn parse_args(args: &[String]) -> Result<(Command, &str), String> {
    let [command, path] = args else {
        return Err(format!("expected 2 arguments, got {}", args.len()));
    };
    let command = match command.as_str() {
        "run" => Command::Run,
        "check" => Command::Check,
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
        Command::Check => Ok(()),
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
    fn parse_args_accepts_run_and_check() {
        let run = args(&["run", "a.pythrust"]);
        let check = args(&["check", "b.pythrust"]);
        assert_eq!(parse_args(&run), Ok((Command::Run, "a.pythrust")));
        assert_eq!(parse_args(&check), Ok((Command::Check, "b.pythrust")));
    }

    #[test]
    fn parse_args_rejects_unknown_command_and_wrong_arity() {
        assert!(parse_args(&args(&["build", "a"])).is_err());
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
