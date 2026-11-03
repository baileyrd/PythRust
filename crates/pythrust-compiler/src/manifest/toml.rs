//! The strict TOML subset `pythrust.toml` is written in
//! (`docs/grammar.md#manifest-pythrusttoml`): tables of bare keys whose
//! values are basic strings or one-line inline tables of them. Every
//! accepted form parses as CPython 3.13's `tomllib` parses it; every
//! other form is a named error saying what to write instead.

use crate::span::Span;

/// A bare key or table name and where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Key {
    pub name: String,
    pub span: Span,
}

/// A basic string's value and where its literal is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Str {
    pub text: String,
    pub span: Span,
}

/// A value: a string, or an inline table of strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Str(Str),
    Inline { span: Span, pairs: Vec<(Key, Str)> },
}

/// One `key = value` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair {
    pub key: Key,
    pub value: Value,
}

/// A `[name]` table and its pairs, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub name: Key,
    pub pairs: Vec<Pair>,
}

/// An error at a byte span of the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub span: Span,
    pub message: String,
}

type Result<T> = std::result::Result<T, Error>;

fn error<T>(at: usize, message: impl Into<String>) -> Result<T> {
    Err(Error {
        span: Span { start: at, end: at },
        message: message.into(),
    })
}

/// Reads the tables of `bytes`, in order. A table or key defined twice
/// is an error, as in TOML; so is a key before the first table.
pub fn parse(bytes: &[u8]) -> Result<Vec<Table>> {
    if bytes.starts_with(b"\xEF\xBB\xBF") {
        return error(
            0,
            "a byte-order mark is not allowed; save the file as UTF-8 without one",
        );
    }
    let text = std::str::from_utf8(bytes)
        .or_else(|e| error(e.valid_up_to(), "the file is not valid UTF-8"))?;
    let mut tables: Vec<Table> = Vec::new();
    let mut start = 0;
    for raw in text.split_inclusive('\n') {
        let line = match raw.strip_suffix('\n') {
            Some(line) => line.strip_suffix('\r').unwrap_or(line),
            None => raw,
        };
        if let Some(cr) = line.find('\r') {
            return error(
                start + cr,
                "a carriage return must be followed by a line feed",
            );
        }
        match (Line {
            text: line,
            base: start,
            pos: 0,
        })
        .item()?
        {
            Item::Blank => {}
            Item::Header(name) => open(&mut tables, name)?,
            Item::Pair(pair) => add(&mut tables, pair)?,
        }
        start += raw.len();
    }
    Ok(tables)
}

/// Starts the table `name`, unless one of that name exists.
fn open(tables: &mut Vec<Table>, name: Key) -> Result<()> {
    if tables.iter().any(|t| t.name.name == name.name) {
        let message = format!("table `[{}]` is defined twice", name.name);
        return error(name.span.start, message);
    }
    tables.push(Table {
        name,
        pairs: Vec::new(),
    });
    Ok(())
}

/// Adds `pair` to the last table, unless its key is there already.
fn add(tables: &mut [Table], pair: Pair) -> Result<()> {
    let Some(table) = tables.last_mut() else {
        let message = "a key must come after a table header such as `[package]`";
        return error(pair.key.span.start, message);
    };
    if table.pairs.iter().any(|p| p.key.name == pair.key.name) {
        let message = format!("key `{}` is defined twice", pair.key.name);
        return error(pair.key.span.start, message);
    }
    table.pairs.push(pair);
    Ok(())
}

/// What one line holds.
enum Item {
    Blank,
    Header(Key),
    Pair(Pair),
}

/// A cursor over one line, without its line ending; `base` is where
/// the line starts in the file.
struct Line<'a> {
    text: &'a str,
    base: usize,
    pos: usize,
}

impl Line<'_> {
    fn item(&mut self) -> Result<Item> {
        self.ws();
        let item = match self.peek() {
            None | Some('#') => Item::Blank,
            Some('[') => Item::Header(self.header()?),
            Some(_) => Item::Pair(self.pair()?),
        };
        self.ws();
        self.end()?;
        Ok(item)
    }

    /// `[name]`.
    fn header(&mut self) -> Result<Key> {
        self.bump();
        if self.peek() == Some('[') {
            return self.fail("arrays of tables (`[[...]]`) are not supported");
        }
        self.ws();
        let name = self.key("a table name")?;
        self.ws();
        if self.peek() == Some('.') {
            return self.fail("dotted table headers are not supported; write `[name]`");
        }
        self.expect(']', "`]` after the table name")?;
        Ok(name)
    }

    /// `key = value`, where the value is a string or an inline table.
    fn pair(&mut self) -> Result<Pair> {
        let key = self.key("a key")?;
        self.after_key()?;
        let value = match self.peek() {
            Some('{') => self.inline()?,
            _ => Value::Str(self.string()?),
        };
        Ok(Pair { key, value })
    }

    /// The ` = ` after a key.
    fn after_key(&mut self) -> Result<()> {
        self.ws();
        if self.peek() == Some('.') {
            return self.fail("dotted keys are not supported; write a table header and a bare key");
        }
        self.expect('=', "`=` after the key")?;
        self.ws();
        Ok(())
    }

    /// `{ key = "...", ... }`, on one line, without a trailing comma.
    fn inline(&mut self) -> Result<Value> {
        let start = self.at();
        self.bump();
        self.ws();
        let mut pairs: Vec<(Key, Str)> = Vec::new();
        while self.peek() != Some('}') {
            let key = self.key("a key")?;
            if pairs.iter().any(|(k, _)| k.name == key.name) {
                let message = format!("key `{}` is defined twice", key.name);
                return error(key.span.start, message);
            }
            self.after_key()?;
            if self.peek() == Some('{') {
                return self.fail("an inline table inside an inline table is not supported");
            }
            pairs.push((key, self.string()?));
            self.ws();
            self.inline_sep()?;
        }
        self.bump();
        let span = Span {
            start,
            end: self.at(),
        };
        Ok(Value::Inline { span, pairs })
    }

    /// The `,` between an inline table's pairs, or its closing `}`.
    fn inline_sep(&mut self) -> Result<()> {
        match self.peek() {
            Some('}') => Ok(()),
            Some(',') => {
                self.bump();
                self.ws();
                match self.peek() {
                    Some('}') => self.fail("a trailing comma is not allowed in an inline table"),
                    None => self.fail("an inline table must close with `}` on the same line"),
                    _ => Ok(()),
                }
            }
            None => self.fail("an inline table must close with `}` on the same line"),
            Some(_) => self.fail("expected `,` or `}` in the inline table"),
        }
    }

    /// A bare key: ASCII letters, digits, `_` and `-`.
    fn key(&mut self, what: &str) -> Result<Key> {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if is_bare(c)) {
            self.bump();
        }
        if self.pos > start {
            let span = Span {
                start: self.base + start,
                end: self.at(),
            };
            let name = self.text[start..self.pos].to_string();
            return Ok(Key { name, span });
        }
        match self.peek() {
            Some('"' | '\'') => self.fail(
                "quoted keys are not supported; write a bare key of letters, digits, `_` and `-`",
            ),
            _ => self.fail(format!("expected {what}")),
        }
    }

    /// A basic string, or the named error for any other value.
    fn string(&mut self) -> Result<Str> {
        match self.peek() {
            Some('"') if self.rest().starts_with("\"\"\"") => {
                self.fail("multi-line strings are not supported; write a one-line \"...\" string")
            }
            Some('"') => self.basic(),
            Some('\'') => self.fail("literal strings ('...') are not supported; write \"...\""),
            Some('[') => self.fail("arrays are not supported"),
            Some('t' | 'f') if self.boolean() => {
                self.fail("booleans are not supported; write a string")
            }
            Some(c) if c.is_ascii_digit() || matches!(c, '+' | '-') || self.special_float() => {
                self.fail("numbers and dates are not supported; write a string")
            }
            _ => self.fail("expected a string \"...\""),
        }
    }

    /// Whether the value is `true` or `false`.
    fn boolean(&self) -> bool {
        self.rest().starts_with("true") || self.rest().starts_with("false")
    }

    /// Whether the value is `inf` or `nan`, TOML's special floats.
    fn special_float(&self) -> bool {
        self.rest().starts_with("inf") || self.rest().starts_with("nan")
    }

    /// `"..."` with TOML's escapes.
    fn basic(&mut self) -> Result<Str> {
        let start = self.at();
        self.bump();
        let mut text = String::new();
        loop {
            match self.peek() {
                None => return self.fail("the string must close with `\"` on the same line"),
                Some('"') => break,
                Some('\\') => text.push(self.escape()?),
                Some(c) if is_control(c) => {
                    return self
                        .fail("a control character in a string must be written as an escape")
                }
                Some(c) => {
                    text.push(c);
                    self.bump();
                }
            }
        }
        self.bump();
        let span = Span {
            start,
            end: self.at(),
        };
        Ok(Str { text, span })
    }

    /// One escape, after its `\`.
    fn escape(&mut self) -> Result<char> {
        let at = self.at();
        self.bump();
        let c = self.peek();
        self.bump();
        match c {
            Some('"') => Ok('"'),
            Some('\\') => Ok('\\'),
            Some('b') => Ok('\u{8}'),
            Some('t') => Ok('\t'),
            Some('n') => Ok('\n'),
            Some('f') => Ok('\u{c}'),
            Some('r') => Ok('\r'),
            Some('u') => self.unicode(4, at),
            Some('U') => self.unicode(8, at),
            _ => error(
                at,
                "unknown escape; the escapes are \\\" \\\\ \\b \\t \\n \\f \\r \\uXXXX \\UXXXXXXXX",
            ),
        }
    }

    /// The `digits` hex digits of a `\u` or `\U` escape starting at `at`.
    fn unicode(&mut self, digits: usize, at: usize) -> Result<char> {
        let hex: String = self.rest().chars().take(digits).collect();
        let valid = hex.len() == digits && hex.chars().all(|c| c.is_ascii_hexdigit());
        let scalar = valid
            .then(|| u32::from_str_radix(&hex, 16).ok())
            .flatten()
            .and_then(char::from_u32);
        let Some(c) = scalar else {
            return error(
                at,
                format!(
                    "`\\{}` needs {digits} hex digits naming a Unicode scalar value",
                    if digits == 4 { 'u' } else { 'U' }
                ),
            );
        };
        self.pos += digits;
        Ok(c)
    }

    /// The end of the line, after an optional comment.
    fn end(&mut self) -> Result<()> {
        match self.peek() {
            None => Ok(()),
            Some('#') => {
                let comment = &self.text[self.pos..];
                match comment.char_indices().find(|&(_, c)| is_control(c)) {
                    Some((i, _)) => error(
                        self.at() + i,
                        "a control character is not allowed in a comment",
                    ),
                    None => Ok(()),
                }
            }
            Some(_) => self.fail("expected the end of the line"),
        }
    }

    fn ws(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.bump();
        }
    }

    fn expect(&mut self, c: char, what: &str) -> Result<()> {
        if self.peek() != Some(c) {
            return self.fail(format!("expected {what}"));
        }
        self.bump();
        Ok(())
    }

    fn rest(&self) -> &str {
        &self.text[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn bump(&mut self) {
        self.pos += self.peek().map_or(0, char::len_utf8);
    }

    /// The cursor's offset in the file.
    fn at(&self) -> usize {
        self.base + self.pos
    }

    fn fail<T>(&self, message: impl Into<String>) -> Result<T> {
        error(self.at(), message)
    }
}

fn is_bare(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// TOML's control characters other than tab: U+0000 to U+001F and U+007F.
fn is_control(c: char) -> bool {
    (c < ' ' && c != '\t') || c == '\u{7f}'
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tables(src: &str) -> Vec<Table> {
        match parse(src.as_bytes()) {
            Ok(t) => t,
            Err(e) => panic!("{src:?}: {e:?}"),
        }
    }

    fn err(src: &[u8]) -> Error {
        match parse(src) {
            Ok(t) => panic!("{src:?} parsed: {t:?}"),
            Err(e) => e,
        }
    }

    #[test]
    fn tables_hold_strings_and_inline_tables() {
        let src = "# top\n[package]\nname = \"x\" # trailing\n\n[rust]\nfastfs = { path = \"fast fs\", b = \"\" }\n";
        let got = tables(src);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].name.name, "package");
        let Value::Str(name) = &got[0].pairs[0].value else {
            panic!("{got:?}");
        };
        assert_eq!(name.text, "x");
        assert_eq!(&src[name.span.start..name.span.end], "\"x\"");
        let Value::Inline { pairs, .. } = &got[1].pairs[0].value else {
            panic!("{got:?}");
        };
        let flat: Vec<(&str, &str)> = pairs
            .iter()
            .map(|(k, v)| (k.name.as_str(), v.text.as_str()))
            .collect();
        assert_eq!(flat, [("path", "fast fs"), ("b", "")]);
    }

    #[test]
    fn crlf_tabs_empty_inline_and_no_final_newline_are_accepted() {
        let got = tables("[a]\r\n\tk\t=\t{}\r\n[b]");
        assert_eq!(got.len(), 2);
        assert!(matches!(&got[0].pairs[0].value, Value::Inline { pairs, .. } if pairs.is_empty()));
    }

    #[test]
    fn escapes_decode() {
        let got = tables("[a]\nk = \"q\\\" b\\\\ \\t \\u00e9 \\U0001F600 é\"\n");
        let Value::Str(s) = &got[0].pairs[0].value else {
            panic!("{got:?}");
        };
        assert_eq!(s.text, "q\" b\\ \t é 😀 é");
    }

    #[test]
    fn unsupported_forms_get_named_errors() {
        let cases: &[(&str, &str)] = &[
            ("[a]\nk = 'x'\n", "literal strings"),
            ("[a]\nk = \"\"\"x\"\"\"\n", "multi-line strings"),
            ("[a]\n\"k\" = \"x\"\n", "quoted keys"),
            ("[a]\nk.j = \"x\"\n", "dotted keys"),
            ("[a.b]\n", "dotted table headers"),
            ("[[a]]\n", "arrays of tables"),
            ("[a]\nk = [\"x\"]\n", "arrays are not supported"),
            ("[a]\nk = 1\n", "numbers and dates"),
            ("[a]\nk = -1.5\n", "numbers and dates"),
            ("[a]\nk = 1979-05-27\n", "numbers and dates"),
            ("[a]\nk = inf\n", "numbers and dates"),
            ("[a]\nk = true\n", "booleans"),
            (
                "[a]\nk = { j = { i = \"x\" } }\n",
                "inline table inside an inline table",
            ),
            ("[a]\nk = { j = \"x\", }\n", "trailing comma"),
            ("[a]\nk = { j = \"x\",\n i = \"y\" }\n", "same line"),
            ("[a]\nk = \"x\n", "close with `\"`"),
            ("[a]\nk = \"\\e\"\n", "unknown escape"),
            ("[a]\nk = \"\\uD800\"\n", "Unicode scalar value"),
            ("[a]\nk = \"\\u12\"\n", "Unicode scalar value"),
            ("[a]\nk = \"a\u{1}b\"\n", "control character in a string"),
            ("[a]\nk = \"a\u{7f}b\"\n", "control character in a string"),
            (
                "[a]\n# a\u{7f}\n",
                "control character is not allowed in a comment",
            ),
            ("[a]\nk = \"x\" y\n", "end of the line"),
            ("[a]\nk = \"x\"\nk = \"y\"\n", "key `k` is defined twice"),
            (
                "[a]\nk = { j = \"x\", j = \"y\" }\n",
                "key `j` is defined twice",
            ),
            ("[a]\n[a]\n", "table `[a]` is defined twice"),
            ("k = \"x\"\n", "after a table header"),
            ("[a]\nk = \"x\"\r\r\n", "carriage return"),
            ("[a]\nk = \"x\"\r", "carriage return"),
            ("[a]\r", "carriage return"),
            ("[a]\n# c\r", "carriage return"),
            ("[a]\nk = fastfs\n", "expected a string"),
            ("[a]\nk\n", "`=` after the key"),
            ("[a\n", "`]` after the table name"),
            ("[a]\n= \"x\"\n", "expected a key"),
        ];
        for (src, want) in cases {
            let e = err(src.as_bytes());
            assert!(e.message.contains(want), "{src:?}: {}", e.message);
        }
    }

    #[test]
    fn encoding_errors_are_named() {
        assert!(err(b"\xEF\xBB\xBF[a]\n")
            .message
            .contains("byte-order mark"));
        let bad = err(b"[a]\nk = \"\xFF\"\n");
        assert!(bad.message.contains("not valid UTF-8"));
        assert_eq!(bad.span.start, 9);
    }

    #[test]
    fn errors_point_at_their_place() {
        let src = "[a]\nk = 'x'\n";
        assert_eq!(err(src.as_bytes()).span.start, src.find('\'').unwrap_or(0));
        for esc in [
            "[a]\nk = \"ab\\e\"\n",
            "[a]\nk = \"ab\\u12\"\n",
            "[a]\nk = \"ab\\UD800\"\n",
        ] {
            assert_eq!(
                err(esc.as_bytes()).span.start,
                esc.find('\\').unwrap_or(0),
                "{esc:?}"
            );
        }
        let dup = "[a]\nk = \"x\"\nk = \"y\"\n";
        assert_eq!(err(dup.as_bytes()).span.start, dup.rfind('k').unwrap_or(0));
    }

    #[test]
    fn u0085_is_allowed_as_tomllib_allows_it() {
        let got = tables("[a]\nk = \"a\u{85}b\" # c\u{85}\n");
        assert_eq!(got[0].pairs.len(), 1);
    }
}
