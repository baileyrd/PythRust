//! Lexer: source text to `Vec<Token>`. Thread A.
//!
//! Follows CPython 3.12 tokenization for the phase-2 subset
//! (`docs/grammar.md`, Lexical). Layout rules:
//!
//! - A logical line ends in `Newline`. Blank and comment-only lines emit
//!   nothing. Inside brackets, line breaks are ignored.
//! - Indentation is spaces only. A tab anywhere outside a string or
//!   comment is an error.
//! - `Indent`/`Dedent` track a stack of indentation widths; a dedent to a
//!   width not on the stack is an error. Whether an `Indent` is allowed
//!   (e.g. on the first line) is the parser's call.
//! - At end of input: a final `Newline` if the last line lacks one, a
//!   `Dedent` per open level, then `EndMarker`.

use crate::diag::{DiagKind, Diagnostic, Result};
use crate::span::Span;
use crate::token::{Token, TokenKind};

/// Tokenizes `src`.
///
/// Errors on the first invalid token: a tab, an inconsistent dedent, an
/// unmatched or unclosed bracket, a malformed literal, a reserved word, or
/// a character outside the grammar.
pub fn lex(src: &str) -> Result<Vec<Token>> {
    Lexer::new(src).run()
}

/// Words reserved for later phases. The lexer rejects them.
const RESERVED: &[&str] = &[
    "try", "except", "finally", "raise", "assert", "class", "import", "from", "as", "with", "is",
    "lambda", "yield", "async", "await", "global", "nonlocal", "del", "pause", "shared",
];

/// Python string prefixes, rejected with a clear message.
const STRING_PREFIXES: &[&str] = &["r", "u", "b", "f", "br", "rb", "fr", "rf"];

struct Lexer<'a> {
    src: &'a str,
    pos: usize,
    tokens: Vec<Token>,
    /// Open indentation widths. The base level `0` is implicit.
    indents: Vec<usize>,
    /// Open brackets: the opening byte and its span.
    brackets: Vec<(u8, Span)>,
    at_line_start: bool,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Lexer {
            src,
            pos: 0,
            tokens: Vec::new(),
            indents: Vec::new(),
            brackets: Vec::new(),
            at_line_start: true,
        }
    }

    fn run(mut self) -> Result<Vec<Token>> {
        while self.pos < self.src.len() {
            if self.at_line_start {
                self.line_start()?;
            } else {
                self.token()?;
            }
        }
        self.finish()
    }

    // ---- Layout ----

    /// Measures indentation at the start of a logical line. Skips blank
    /// and comment-only lines; otherwise emits `Indent`/`Dedent`.
    fn line_start(&mut self) -> Result<()> {
        let start = self.pos;
        let width = self.indent_width()?;
        let indent = Span {
            start,
            end: self.pos,
        };
        match self.peek() {
            None => Ok(()),
            Some(b'\t') => Err(self.tab_error()),
            Some(b'#') => {
                self.skip_comment();
                self.skip_blank_line_end()
            }
            Some(b'\n' | b'\r') => self.skip_blank_line_end(),
            Some(_) => {
                self.at_line_start = false;
                self.indent_to(width, indent)
            }
        }
    }

    /// Counts leading spaces, following `\` line joins as CPython does: the
    /// first join preceded by spaces fixes the width; otherwise the count
    /// runs on into the joined line.
    fn indent_width(&mut self) -> Result<usize> {
        let mut width = 0;
        let mut fixed = None;
        loop {
            match self.peek() {
                Some(b' ') => {
                    width += 1;
                    self.pos += 1;
                }
                Some(b'\\') => {
                    if width > 0 && fixed.is_none() {
                        fixed = Some(width);
                    }
                    self.continuation()?;
                }
                _ => return Ok(fixed.unwrap_or(width)),
            }
        }
    }

    fn skip_blank_line_end(&mut self) -> Result<()> {
        match self.newline_len() {
            Some(n) => {
                self.pos += n;
                Ok(())
            }
            None if self.peek().is_none() => Ok(()),
            None => Err(self.char_error()),
        }
    }

    /// Emits layout tokens for a line indented `width` spaces; `indent`
    /// spans the indentation, `\` joins included.
    fn indent_to(&mut self, width: usize, indent: Span) -> Result<()> {
        if width > self.top_indent() {
            self.indents.push(width);
            self.push(TokenKind::Indent, indent);
            return Ok(());
        }
        let at = Span {
            start: indent.end,
            end: indent.end,
        };
        while width < self.top_indent() {
            self.indents.pop();
            self.push(TokenKind::Dedent, at);
        }
        if width != self.top_indent() {
            return Err(error(
                indent,
                "unindent does not match any outer indentation level",
            ));
        }
        Ok(())
    }

    fn top_indent(&self) -> usize {
        self.indents.last().copied().unwrap_or(0)
    }

    fn finish(mut self) -> Result<Vec<Token>> {
        if let Some(&(open, span)) = self.brackets.last() {
            return Err(error(span, format!("'{}' was never closed", open as char)));
        }
        let end = Span {
            start: self.src.len(),
            end: self.src.len(),
        };
        if !self.at_line_start {
            self.push(TokenKind::Newline, end);
        }
        for _ in 0..self.indents.len() {
            self.push(TokenKind::Dedent, end);
        }
        self.push(TokenKind::EndMarker, end);
        Ok(self.tokens)
    }

    // ---- Tokens within a line ----

    fn token(&mut self) -> Result<()> {
        let Some(b) = self.peek() else {
            return Ok(());
        };
        match b {
            b' ' => {
                self.pos += 1;
                Ok(())
            }
            b'\t' => Err(self.tab_error()),
            b'#' => {
                self.skip_comment();
                Ok(())
            }
            b'\n' | b'\r' => self.line_end(),
            b'\\' => self.continuation(),
            b'0'..=b'9' => self.number(),
            b'.' if self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) => self.number(),
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.name(),
            b'\'' | b'"' => self.string(),
            _ => self.operator(),
        }
    }

    /// A line break: `Newline` at bracket depth 0, otherwise ignored.
    fn line_end(&mut self) -> Result<()> {
        let Some(n) = self.newline_len() else {
            return Err(self.char_error());
        };
        let span = Span {
            start: self.pos,
            end: self.pos + n,
        };
        self.pos += n;
        if self.brackets.is_empty() {
            self.push(TokenKind::Newline, span);
            self.at_line_start = true;
        }
        Ok(())
    }

    /// Explicit `\` line joining.
    fn continuation(&mut self) -> Result<()> {
        let start = self.pos;
        self.pos += 1;
        let Some(n) = self.newline_len() else {
            let span = Span {
                start,
                end: self.pos,
            };
            return Err(error(span, "unexpected character after line continuation"));
        };
        self.pos += n;
        if self.peek().is_none() {
            let span = Span {
                start,
                end: self.pos,
            };
            return Err(error(
                span,
                "unexpected end of file after line continuation",
            ));
        }
        Ok(())
    }

    fn name(&mut self) -> Result<()> {
        let start = self.pos;
        while self.peek().is_some_and(is_name_byte) {
            self.pos += 1;
        }
        let span = Span {
            start,
            end: self.pos,
        };
        let text = &self.src[start..self.pos];
        let followed_by_quote = matches!(self.peek(), Some(b'\'' | b'"'));
        if followed_by_quote && STRING_PREFIXES.contains(&text.to_ascii_lowercase().as_str()) {
            return Err(error(
                span,
                format!("string prefix '{text}' is not supported"),
            ));
        }
        if RESERVED.contains(&text) {
            return Err(error(
                span,
                format!("'{text}' is reserved for a future version"),
            ));
        }
        let kind = keyword(text).unwrap_or_else(|| TokenKind::Name(text.to_string()));
        self.push(kind, span);
        Ok(())
    }

    // ---- Numbers ----

    fn number(&mut self) -> Result<()> {
        let start = self.pos;
        self.digits(start)?;
        let mut is_float = false;
        if self.peek() == Some(b'.') {
            is_float = true;
            self.pos += 1;
            self.digits(start)?;
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            is_float = true;
            self.exponent(start)?;
        }
        let span = Span {
            start,
            end: self.pos,
        };
        if self.peek().is_some_and(is_name_byte) {
            return Err(error(span, "invalid number literal"));
        }
        let text = self.src[start..self.pos].replace('_', "");
        let kind = if is_float {
            TokenKind::Float(parse_float(&text, span)?)
        } else {
            TokenKind::Int(parse_int(&text, span)?)
        };
        self.push(kind, span);
        Ok(())
    }

    /// Consumes `digit (['_'] digit)*`, or nothing if no digit is next.
    /// `start` is the literal's start, for error spans.
    fn digits(&mut self, start: usize) -> Result<()> {
        let run = self.pos;
        while let Some(b) = self.peek() {
            match b {
                b'0'..=b'9' => self.pos += 1,
                b'_' if self.pos == run => break,
                b'_' if self.peek_at(1).is_some_and(|c| c.is_ascii_digit()) => self.pos += 1,
                b'_' => {
                    let span = Span {
                        start,
                        end: self.pos + 1,
                    };
                    return Err(error(span, "'_' in a number must sit between digits"));
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn exponent(&mut self, start: usize) -> Result<()> {
        self.pos += 1;
        if matches!(self.peek(), Some(b'+' | b'-')) {
            self.pos += 1;
        }
        if !self.peek().is_some_and(|c| c.is_ascii_digit()) {
            let span = Span {
                start,
                end: self.pos,
            };
            return Err(error(span, "exponent has no digits"));
        }
        self.digits(start)
    }

    // ---- Strings ----

    fn string(&mut self) -> Result<()> {
        let start = self.pos;
        let quote = self.src.as_bytes()[start];
        self.pos += 1;
        if self.peek() == Some(quote) && self.peek_at(1) == Some(quote) {
            let span = Span {
                start,
                end: start + 3,
            };
            return Err(error(span, "triple-quoted strings are not supported yet"));
        }
        let mut value = String::new();
        loop {
            let unterminated = Span {
                start,
                end: self.pos,
            };
            match self.peek_char() {
                None | Some('\n' | '\r') => return Err(error(unterminated, "unterminated string")),
                Some(c) if c == quote as char => break,
                Some('\\') => self.escape(&mut value, start)?,
                Some(c) => {
                    value.push(c);
                    self.pos += c.len_utf8();
                }
            }
        }
        self.pos += 1;
        self.push(
            TokenKind::Str(value),
            Span {
                start,
                end: self.pos,
            },
        );
        Ok(())
    }

    /// Decodes one `\` escape into `value`. `start` is the string's start,
    /// used when the escape runs into end of input.
    fn escape(&mut self, value: &mut String, start: usize) -> Result<()> {
        let esc = self.pos;
        self.pos += 1;
        let Some(c) = self.peek_char() else {
            return Err(error(
                Span {
                    start,
                    end: self.pos,
                },
                "unterminated string",
            ));
        };
        if let Some(n) = self.newline_len() {
            self.pos += n;
            return Ok(());
        }
        self.pos += c.len_utf8();
        let decoded = match c {
            '0'..='7' => self.octal_escape(c, esc)?,
            'x' => self.hex_escape(2, esc)?,
            'u' => self.hex_escape(4, esc)?,
            'U' => self.hex_escape(8, esc)?,
            _ => simple_escape(c).ok_or_else(|| {
                let span = Span {
                    start: esc,
                    end: self.pos,
                };
                error(span, format!("invalid escape sequence '\\{c}'"))
            })?,
        };
        value.push(decoded);
        Ok(())
    }

    /// `\o`, `\oo`, or `\ooo`; `first` is already consumed.
    fn octal_escape(&mut self, first: char, esc: usize) -> Result<char> {
        let mut code = first.to_digit(8).unwrap_or(0);
        for _ in 0..2 {
            let Some(d) = self.peek().and_then(|b| (b as char).to_digit(8)) else {
                break;
            };
            code = code * 8 + d;
            self.pos += 1;
        }
        code_point(
            code,
            Span {
                start: esc,
                end: self.pos,
            },
        )
    }

    /// `\x`, `\u`, `\U` followed by exactly `len` hex digits.
    fn hex_escape(&mut self, len: usize, esc: usize) -> Result<char> {
        let digits = self.src.get(self.pos..self.pos + len);
        let Some(digits) = digits.filter(|d| d.bytes().all(|b| b.is_ascii_hexdigit())) else {
            let span = Span {
                start: esc,
                end: self.pos,
            };
            return Err(error(span, format!("escape needs {len} hex digits")));
        };
        self.pos += len;
        let span = Span {
            start: esc,
            end: self.pos,
        };
        let code = u32::from_str_radix(digits, 16).map_err(|e| error(span, e.to_string()))?;
        code_point(code, span)
    }

    // ---- Operators and delimiters ----

    fn operator(&mut self) -> Result<()> {
        let start = self.pos;
        let rest = &self.src.as_bytes()[start..];
        let Some((kind, len)) = operator(rest) else {
            return Err(self.char_error());
        };
        let span = Span {
            start,
            end: start + len,
        };
        self.pos += len;
        self.track_bracket(rest[0], span)?;
        self.push(kind, span);
        Ok(())
    }

    /// Keeps the bracket stack balanced; `b` is the token's first byte.
    fn track_bracket(&mut self, b: u8, span: Span) -> Result<()> {
        let open = match b {
            b'(' | b'[' => {
                self.brackets.push((b, span));
                return Ok(());
            }
            b')' => b'(',
            b']' => b'[',
            _ => return Ok(()),
        };
        let close = b as char;
        match self.brackets.pop() {
            Some((o, _)) if o == open => Ok(()),
            Some((o, _)) => Err(error(
                span,
                format!("'{close}' does not match '{}'", o as char),
            )),
            None => Err(error(span, format!("unmatched '{close}'"))),
        }
    }

    // ---- Helpers ----

    fn push(&mut self, kind: TokenKind, span: Span) {
        self.tokens.push(Token { kind, span });
    }

    fn peek(&self) -> Option<u8> {
        self.peek_at(0)
    }

    fn peek_at(&self, offset: usize) -> Option<u8> {
        self.src.as_bytes().get(self.pos + offset).copied()
    }

    fn peek_char(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    /// Length of the line break at `pos`: 2 for `\r\n`, 1 for `\n` or a
    /// lone `\r` (CPython's universal newlines).
    fn newline_len(&self) -> Option<usize> {
        match (self.peek(), self.peek_at(1)) {
            (Some(b'\r'), Some(b'\n')) => Some(2),
            (Some(b'\n' | b'\r'), _) => Some(1),
            _ => None,
        }
    }

    fn skip_comment(&mut self) {
        while self.peek().is_some_and(|b| b != b'\n' && b != b'\r') {
            self.pos += 1;
        }
    }

    fn tab_error(&self) -> Diagnostic {
        let span = Span {
            start: self.pos,
            end: self.pos + 1,
        };
        error(span, "tab character; indent and separate with spaces only")
    }

    fn char_error(&self) -> Diagnostic {
        let c = self.peek_char().unwrap_or('\0');
        let span = Span {
            start: self.pos,
            end: self.pos + c.len_utf8(),
        };
        error(span, format!("unexpected character {c:?}"))
    }
}

// ---- Pure helpers ----

fn error(span: Span, message: impl Into<String>) -> Diagnostic {
    Diagnostic {
        span,
        message: message.into(),
        kind: DiagKind::Lex,
    }
}

fn is_name_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn keyword(text: &str) -> Option<TokenKind> {
    use TokenKind::*;
    let kind = match text {
        "def" => Def,
        "return" => Return,
        "if" => If,
        "elif" => Elif,
        "else" => Else,
        "while" => While,
        "for" => For,
        "in" => In,
        "break" => Break,
        "continue" => Continue,
        "pass" => Pass,
        "and" => And,
        "or" => Or,
        "not" => Not,
        "True" => True,
        "False" => False,
        "None" => None,
        "temper" => Temper,
        _ => return Option::None,
    };
    Some(kind)
}

/// The longest operator or delimiter at the start of `rest`, with its length.
fn operator(rest: &[u8]) -> Option<(TokenKind, usize)> {
    use TokenKind::*;
    let op = match rest {
        [b'*', b'*', b'=', ..] => (DoubleStarEq, 3),
        [b'/', b'/', b'=', ..] => (DoubleSlashEq, 3),
        [b'*', b'*', ..] => (DoubleStar, 2),
        [b'/', b'/', ..] => (DoubleSlash, 2),
        [b'=', b'=', ..] => (EqEq, 2),
        [b'!', b'=', ..] => (NotEq, 2),
        [b'<', b'=', ..] => (LtEq, 2),
        [b'>', b'=', ..] => (GtEq, 2),
        [b'+', b'=', ..] => (PlusEq, 2),
        [b'-', b'=', ..] => (MinusEq, 2),
        [b'*', b'=', ..] => (StarEq, 2),
        [b'/', b'=', ..] => (SlashEq, 2),
        [b'%', b'=', ..] => (PercentEq, 2),
        [b'-', b'>', ..] => (Arrow, 2),
        [b'+', ..] => (Plus, 1),
        [b'-', ..] => (Minus, 1),
        [b'*', ..] => (Star, 1),
        [b'/', ..] => (Slash, 1),
        [b'%', ..] => (Percent, 1),
        [b'<', ..] => (Lt, 1),
        [b'>', ..] => (Gt, 1),
        [b'=', ..] => (Assign, 1),
        [b'(', ..] => (LParen, 1),
        [b')', ..] => (RParen, 1),
        [b'[', ..] => (LBracket, 1),
        [b']', ..] => (RBracket, 1),
        [b',', ..] => (Comma, 1),
        [b':', ..] => (Colon, 1),
        [b'.', ..] => (Dot, 1),
        _ => return Option::None,
    };
    Some(op)
}

fn simple_escape(c: char) -> Option<char> {
    let decoded = match c {
        '\\' => '\\',
        '\'' => '\'',
        '"' => '"',
        'a' => '\x07',
        'b' => '\x08',
        'f' => '\x0c',
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        'v' => '\x0b',
        _ => return None,
    };
    Some(decoded)
}

fn code_point(code: u32, span: Span) -> Result<char> {
    char::from_u32(code).ok_or_else(|| error(span, format!("invalid code point U+{code:04X}")))
}

/// Parses a decimal integer with `_` already removed.
fn parse_int(text: &str, span: Span) -> Result<u64> {
    let leading_zero = text.len() > 1 && text.starts_with('0');
    if leading_zero && text.bytes().any(|b| b != b'0') {
        return Err(error(
            span,
            "leading zeros in integer literals are not permitted",
        ));
    }
    text.parse::<u64>()
        .map_err(|_| error(span, "integer literal is too large"))
}

/// Parses a float with `_` already removed.
fn parse_float(text: &str, span: Span) -> Result<f64> {
    text.parse::<f64>()
        .map_err(|e| error(span, format!("invalid float literal: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use TokenKind::*;

    fn kinds(src: &str) -> Vec<TokenKind> {
        match lex(src) {
            Ok(tokens) => tokens.into_iter().map(|t| t.kind).collect(),
            Err(d) => panic!("unexpected lex error for {src:?}: {d}"),
        }
    }

    /// Kinds with the trailing `Newline`/`EndMarker` of a one-line source
    /// stripped.
    fn line(src: &str) -> Vec<TokenKind> {
        let mut k = kinds(src);
        assert_eq!(k.split_off(k.len() - 2), vec![Newline, EndMarker]);
        k
    }

    fn lex_err(src: &str) -> Diagnostic {
        match lex(src) {
            Ok(tokens) => panic!("expected lex error for {src:?}, got {tokens:?}"),
            Err(d) => d,
        }
    }

    fn assert_err(src: &str, start: usize, end: usize, needle: &str) {
        let d = lex_err(src);
        assert_eq!(d.kind, DiagKind::Lex);
        assert_eq!(d.span, Span { start, end }, "span for {src:?}: {d}");
        assert!(d.message.contains(needle), "{d} lacks {needle:?}");
    }

    fn name(s: &str) -> TokenKind {
        Name(s.to_string())
    }

    fn string(s: &str) -> TokenKind {
        Str(s.to_string())
    }

    // ---- Layout ----

    #[test]
    fn empty_and_blank_sources_are_just_endmarker() {
        for src in ["", "\n", "   \n\n", "# only a comment", "# c\n   # c\n"] {
            assert_eq!(kinds(src), vec![EndMarker], "{src:?}");
        }
    }

    #[test]
    fn missing_final_newline_is_supplied() {
        assert_eq!(kinds("x"), vec![name("x"), Newline, EndMarker]);
        assert_eq!(kinds("x\n"), vec![name("x"), Newline, EndMarker]);
        assert_eq!(kinds("x  # trailing"), vec![name("x"), Newline, EndMarker]);
    }

    #[test]
    fn indent_and_dedent_around_a_block() {
        let src = "if x:\n    y\nz\n";
        let expected = vec![
            If,
            name("x"),
            Colon,
            Newline,
            Indent,
            name("y"),
            Newline,
            Dedent,
            name("z"),
            Newline,
            EndMarker,
        ];
        assert_eq!(kinds(src), expected);
    }

    #[test]
    fn nested_blocks_dedent_to_eof() {
        let src = "def f():\n  if x:\n    pass";
        let expected = vec![
            Def,
            name("f"),
            LParen,
            RParen,
            Colon,
            Newline,
            Indent,
            If,
            name("x"),
            Colon,
            Newline,
            Indent,
            Pass,
            Newline,
            Dedent,
            Dedent,
            EndMarker,
        ];
        assert_eq!(kinds(src), expected);
    }

    #[test]
    fn multi_level_dedent_in_one_line() {
        let src = "a:\n  b:\n    c\nd\n";
        let k = kinds(src);
        let tail = &k[k.len() - 5..];
        assert_eq!(tail, &[Dedent, Dedent, name("d"), Newline, EndMarker]);
    }

    #[test]
    fn blank_and_comment_lines_do_not_affect_indentation() {
        let src = "if x:\n\n    # note\n        \n    y\n";
        let expected = vec![
            If,
            name("x"),
            Colon,
            Newline,
            Indent,
            name("y"),
            Newline,
            Dedent,
            EndMarker,
        ];
        assert_eq!(kinds(src), expected);
    }

    #[test]
    fn crlf_line_endings() {
        let src = "if x:\r\n    y\r\n";
        let expected = vec![
            If,
            name("x"),
            Colon,
            Newline,
            Indent,
            name("y"),
            Newline,
            Dedent,
            EndMarker,
        ];
        assert_eq!(kinds(src), expected);
    }

    #[test]
    fn lone_cr_is_a_line_break() {
        assert_eq!(
            kinds("x\ry\r"),
            vec![name("x"), Newline, name("y"), Newline, EndMarker]
        );
        let blocks = "if x:\r    a\r\r    # c\r    b\r";
        let expected = vec![
            If,
            name("x"),
            Colon,
            Newline,
            Indent,
            name("a"),
            Newline,
            name("b"),
            Newline,
            Dedent,
            EndMarker,
        ];
        assert_eq!(kinds(blocks), expected);
        assert_eq!(line("1 + \\\r2"), vec![Int(1), Plus, Int(2)]);
        assert_eq!(line("'ab\\\rcd'"), vec![string("abcd")]);
        assert_err("'a\rb'", 0, 2, "unterminated");
        let spans: Vec<Span> = lex("x\ry").unwrap().iter().map(|t| t.span).collect();
        assert_eq!(spans[1], Span { start: 1, end: 2 });
    }

    #[test]
    fn brackets_join_lines_and_ignore_indentation() {
        let src = "f(1,\n        2,  # c\n\n  3)\n";
        let expected = vec![
            name("f"),
            LParen,
            Int(1),
            Comma,
            Int(2),
            Comma,
            Int(3),
            RParen,
            Newline,
            EndMarker,
        ];
        assert_eq!(kinds(src), expected);
    }

    #[test]
    fn backslash_joins_lines() {
        assert_eq!(
            line("x = 1 + \\\n        2"),
            vec![name("x"), Assign, Int(1), Plus, Int(2)]
        );
    }

    #[test]
    fn join_only_line_keeps_the_joined_lines_indentation() {
        let src = "if x:\n    pass\n\\\n    y\nz\n";
        let expected = vec![
            If,
            name("x"),
            Colon,
            Newline,
            Indent,
            Pass,
            Newline,
            name("y"),
            Newline,
            Dedent,
            name("z"),
            Newline,
            EndMarker,
        ];
        assert_eq!(kinds(src), expected);
        let top = "\\\n    pass\n";
        assert_eq!(kinds(top), vec![Indent, Pass, Newline, Dedent, EndMarker]);
    }

    #[test]
    fn spaces_before_the_first_join_fix_the_indentation() {
        let stays = "if x:\n    a\n    \\\nb\n";
        let expected = vec![
            If,
            name("x"),
            Colon,
            Newline,
            Indent,
            name("a"),
            Newline,
            name("b"),
            Newline,
            Dedent,
            EndMarker,
        ];
        assert_eq!(kinds(stays), expected);
        assert_err("if x:\n    a\n  \\\n    b\n", 12, 20, "unindent");
    }

    #[test]
    fn first_line_indent_is_left_to_the_parser() {
        assert_eq!(
            kinds("  x\n"),
            vec![Indent, name("x"), Newline, Dedent, EndMarker]
        );
    }

    #[test]
    fn layout_spans() {
        let tokens = lex("if x:\n  y").unwrap();
        let spans: Vec<(usize, usize)> =
            tokens.iter().map(|t| (t.span.start, t.span.end)).collect();
        // if x : NL INDENT y NL DEDENT END
        let expected = vec![
            (0, 2),
            (3, 4),
            (4, 5),
            (5, 6),
            (6, 8),
            (8, 9),
            (9, 9),
            (9, 9),
            (9, 9),
        ];
        assert_eq!(spans, expected);
    }

    #[test]
    fn tab_is_an_error_anywhere_outside_strings() {
        assert_err("if x:\n\ty\n", 6, 7, "tab");
        assert_err("x =\t1\n", 3, 4, "tab");
        assert_err("if x:\n  \ty\n", 8, 9, "tab");
    }

    #[test]
    fn tab_inside_string_or_comment_is_fine() {
        assert_eq!(line("'\t' # \t"), vec![string("\t")]);
    }

    #[test]
    fn inconsistent_dedent_is_an_error() {
        assert_err("if x:\n    y\n  z\n", 12, 14, "unindent");
    }

    #[test]
    fn bracket_errors() {
        assert_err("f(1\n", 1, 2, "never closed");
        assert_err("x)\n", 1, 2, "unmatched ')'");
        assert_err("[1)\n", 2, 3, "')' does not match '['");
    }

    #[test]
    fn bad_continuation() {
        assert_err("x \\ y\n", 2, 3, "after line continuation");
        assert_err("x \\\n", 2, 4, "end of file");
    }

    // ---- Names and keywords ----

    #[test]
    fn keywords_and_names() {
        let src = "def return if elif else while for in break continue pass and or not \
                   True False None temper";
        let expected = vec![
            Def, Return, If, Elif, Else, While, For, In, Break, Continue, Pass, And, Or, Not, True,
            False, None, Temper,
        ];
        assert_eq!(line(src), expected);
        assert_eq!(
            line("_x9 match true"),
            vec![name("_x9"), name("match"), name("true")]
        );
    }

    #[test]
    fn every_reserved_word_is_rejected() {
        for word in RESERVED {
            let d = lex_err(&format!("x = {word}\n"));
            assert_eq!(
                d.span,
                Span {
                    start: 4,
                    end: 4 + word.len()
                }
            );
            assert!(d.message.contains("reserved"), "{d}");
        }
    }

    #[test]
    fn reserved_prefix_is_just_a_name() {
        assert_eq!(line("classy is_ok"), vec![name("classy"), name("is_ok")]);
    }

    #[test]
    fn non_ascii_name_is_an_error() {
        assert_err("café = 1\n", 3, 5, "unexpected character 'é'");
    }

    // ---- Numbers ----

    #[test]
    fn integers() {
        assert_eq!(
            line("0 00 7 1_000 18446744073709551615"),
            vec![Int(0), Int(0), Int(7), Int(1000), Int(u64::MAX)]
        );
        assert_eq!(
            line("9223372036854775808"),
            vec![Int(9_223_372_036_854_775_808)]
        );
    }

    #[test]
    fn integer_errors() {
        assert_err("18446744073709551616\n", 0, 20, "too large");
        assert_err("012\n", 0, 3, "leading zeros");
        assert_err("1__0\n", 0, 2, "between digits");
        assert_err("10_\n", 0, 3, "between digits");
        assert_err("0x1F\n", 0, 1, "invalid number");
        assert_err("12abc\n", 0, 2, "invalid number");
    }

    #[test]
    fn floats() {
        let expected = vec![
            Float(1.5),
            Float(1.0),
            Float(0.5),
            Float(1e10),
            Float(2.5e-3),
            Float(1e5),
            Float(1000.25),
            Float(1e300),
        ];
        assert_eq!(
            line("1.5 1. .5 1e10 2.5E-3 1.e5 1_000.2_5 1e3_00"),
            expected
        );
    }

    #[test]
    fn float_errors() {
        assert_err("1e\n", 0, 2, "exponent");
        assert_err("1e+\n", 0, 3, "exponent");
        assert_err("1_.5\n", 0, 2, "between digits");
        assert_err("1._5\n", 0, 2, "invalid number");
        assert_err("1.5j\n", 0, 3, "invalid number");
    }

    #[test]
    fn dot_after_name_is_attribute() {
        assert_eq!(
            line("xs.append(1)"),
            vec![name("xs"), Dot, name("append"), LParen, Int(1), RParen]
        );
    }

    #[test]
    fn minus_is_separate_from_literal() {
        assert_eq!(
            line("-9223372036854775808"),
            vec![Minus, Int(9_223_372_036_854_775_808)]
        );
    }

    // ---- Strings ----

    #[test]
    fn strings_both_quotes() {
        assert_eq!(
            line(r#"'a' "b" '' "it's" 'say "hi"'"#),
            vec![
                string("a"),
                string("b"),
                string(""),
                string("it's"),
                string("say \"hi\"")
            ]
        );
    }

    #[test]
    fn string_escapes() {
        let src = r#"'\\ \' \" \a\b\f\n\r\t\v \0 \101 \x41 \u00e9 \U0001F600'"#;
        let expected = "\\ ' \" \x07\x08\x0c\n\r\t\x0b \0 A A é 😀";
        assert_eq!(line(src), vec![string(expected)]);
    }

    #[test]
    fn string_backslash_newline_joins() {
        assert_eq!(line("'ab\\\ncd'"), vec![string("abcd")]);
    }

    #[test]
    fn non_ascii_in_string_and_comment() {
        assert_eq!(line("'héllo' # ünïcode"), vec![string("héllo")]);
        let tokens = lex("'é' x").unwrap();
        assert_eq!(tokens[0].span, Span { start: 0, end: 4 });
        assert_eq!(tokens[1].span, Span { start: 5, end: 6 });
    }

    #[test]
    fn string_errors() {
        assert_err("'abc\n", 0, 4, "unterminated");
        assert_err("'abc", 0, 4, "unterminated");
        assert_err("'abc\\", 0, 5, "unterminated");
        assert_err(r"'\q'", 1, 3, r"invalid escape sequence '\q'");
        assert_err(r"'\N{DASH}'", 1, 3, r"'\N'");
        assert_err(r"'\x4'", 1, 3, "2 hex digits");
        assert_err(r"'\uD800'", 1, 7, "invalid code point U+D800");
        assert_err(r"'\U00110000'", 1, 11, "invalid code point");
        assert_err("'''doc'''", 0, 3, "triple-quoted");
        assert_err("f'x'", 0, 1, "string prefix 'f'");
        assert_err("RB'x'", 0, 2, "string prefix 'RB'");
    }

    #[test]
    fn keyword_directly_before_string_is_fine() {
        assert_eq!(line("x in'ab'"), vec![name("x"), In, string("ab")]);
    }

    // ---- Operators ----

    #[test]
    fn operators_longest_match() {
        let src = "+ - * / // % ** == != < <= > >= = += -= *= /= //= %= **= ( ) [ ] , : . ->";
        let expected = vec![
            Plus,
            Minus,
            Star,
            Slash,
            DoubleSlash,
            Percent,
            DoubleStar,
            EqEq,
            NotEq,
            Lt,
            LtEq,
            Gt,
            GtEq,
            Assign,
            PlusEq,
            MinusEq,
            StarEq,
            SlashEq,
            DoubleSlashEq,
            PercentEq,
            DoubleStarEq,
            LParen,
            RParen,
            LBracket,
            RBracket,
            Comma,
            Colon,
            Dot,
            Arrow,
        ];
        assert_eq!(line(src), expected);
        assert_eq!(line("a**-b"), vec![name("a"), DoubleStar, Minus, name("b")]);
    }

    #[test]
    fn not_in_is_two_tokens() {
        assert_eq!(line("a not in b"), vec![name("a"), Not, In, name("b")]);
    }

    #[test]
    fn characters_outside_the_grammar() {
        for (src, c) in [
            ("a ; b", ';'),
            ("a & b", '&'),
            ("{}", '{'),
            ("!x", '!'),
            ("@d", '@'),
        ] {
            let d = lex_err(src);
            assert!(d.message.contains(&format!("{c:?}")), "{d}");
        }
    }

    // ---- Whole program ----

    #[test]
    fn small_program() {
        let src = "\
temper def fib(n: int) -> int:
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a

print(fib(10))
";
        let k = kinds(src);
        assert_eq!(&k[..4], &[Temper, Def, name("fib"), LParen]);
        let indents = k.iter().filter(|t| **t == Indent).count();
        let dedents = k.iter().filter(|t| **t == Dedent).count();
        assert_eq!((indents, dedents), (2, 2));
        assert_eq!(k.last(), Some(&EndMarker));
    }
}
