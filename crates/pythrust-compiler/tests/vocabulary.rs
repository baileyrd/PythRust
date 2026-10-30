//! M5's exit: no diagnostic leaks Rust's borrow-checker vocabulary.
//!
//! Two nets. `corpus_diagnostics_speak_python` runs every corpus program
//! through `check` and `run` in process and reads each error and warning
//! it produces. `messages_speak_python` reads every string literal in the
//! compiler's and runtime's non-test sources, so a message no program
//! reaches yet is held to the rule too. Comments are not read: the code
//! may say "borrowed" to its readers, never to a Pythrust programmer.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use pythrust_compiler::{infer, interp, lexer, own, parser};

type Res<T> = std::result::Result<T, String>;

/// Rust words a Python programmer should never be asked to act on.
const BANNED: &[&str] = &["borrow", "lifetime", "move out of"];

/// The banned words in `text`, ignoring case.
fn banned_in(text: &str) -> Vec<&'static str> {
    let text = text.to_lowercase();
    BANNED
        .iter()
        .copied()
        .filter(|w| text.contains(w))
        .collect()
}

fn root() -> Res<PathBuf> {
    let crate_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)
        .ok_or_else(|| "no repository root above the crate".to_string())
}

/// Files under `dir` (recursively) with extension `ext`, sorted.
fn files(dir: &Path, ext: &str) -> Res<Vec<PathBuf>> {
    let mut out = Vec::new();
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries {
        let path = entry.map_err(|e| e.to_string())?.path();
        if path.is_dir() {
            out.extend(files(&path, ext)?);
        } else if path.extension().is_some_and(|e| e == ext) {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

fn read(path: &Path) -> Res<String> {
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

/// Every error and warning `check` then `run` report for `src`, in order;
/// the first error ends it, as in the CLI.
fn diagnostics(src: &str) -> Vec<String> {
    let tokens = match lexer::lex(src) {
        Ok(tokens) => tokens,
        Err(d) => return vec![d.message],
    };
    let module = match parser::parse(&tokens) {
        Ok(module) => module,
        Err(d) => return vec![d.message],
    };
    let types = match infer::infer(&module) {
        Ok(types) => types,
        Err(d) => return vec![d.message],
    };
    let mut out: Vec<String> = types.warnings().iter().map(|w| w.message.clone()).collect();
    let ran = own::check(&module, &own::analyze(&module, &types))
        .and_then(|()| interp::run(&module, &mut io::sink()));
    out.extend(ran.err().map(|d| d.message));
    out
}

#[test]
fn corpus_diagnostics_speak_python() -> Res<()> {
    let mut failures = Vec::new();
    let mut seen = 0;
    for source in files(&root()?.join("examples"), "pyr")? {
        for message in diagnostics(&read(&source)?) {
            seen += 1;
            let words = banned_in(&message);
            if !words.is_empty() {
                failures.push(format!("{}: {words:?} in {message:?}", source.display()));
            }
        }
    }
    assert!(seen > 0, "the corpus reports no diagnostic");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}

/// `src` without its inline `#[cfg(test)] mod tests { ... }`, which by
/// convention ends a file; `#[cfg(test)] mod tests;` lives in `tests.rs`.
fn non_test(src: &str) -> &str {
    src.find("#[cfg(test)]\nmod tests {")
        .map_or(src, |at| &src[..at])
}

/// The contents of the string literals in Rust source `src` (plain, byte
/// and raw), skipping comments and telling char literals from lifetimes.
fn literals(src: &str) -> Vec<String> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        i = match (chars[i], chars.get(i + 1).copied()) {
            ('/', Some('/')) => skip_past(&chars, i, "\n"),
            ('/', Some('*')) => skip_past(&chars, i + 2, "*/"),
            ('\'', _) => skip_char(&chars, i),
            ('"', _) => read_plain(&chars, i + 1, &mut out),
            ('r', Some('"' | '#')) if starts_token(&chars, i) => read_raw(&chars, i + 1, &mut out),
            _ => i + 1,
        };
    }
    out
}

/// Whether `chars[i]` starts a token rather than ending an identifier.
fn starts_token(chars: &[char], i: usize) -> bool {
    let prev = i.checked_sub(1).map(|p| chars[p]);
    !prev.is_some_and(|c| c.is_alphanumeric() || c == '_')
}

/// The index just past the next `end` at or after `from` (or the end).
fn skip_past(chars: &[char], from: usize, end: &str) -> usize {
    let end: Vec<char> = end.chars().collect();
    (from..chars.len())
        .find(|&j| chars[j..].starts_with(&end))
        .map_or(chars.len(), |j| j + end.len())
}

/// Past a char literal at `i` (`'x'`, `'\n'`, `'\u{..}'`), or one past a
/// lifetime's quote.
fn skip_char(chars: &[char], i: usize) -> usize {
    match (chars.get(i + 1), chars.get(i + 2)) {
        (Some('\\'), _) => skip_past(chars, i + 3, "'"),
        (Some(_), Some('\'')) => i + 3,
        _ => i + 1,
    }
}

/// Reads a `"..."` body starting at `i`; returns the index past its quote.
fn read_plain(chars: &[char], mut i: usize, out: &mut Vec<String>) -> usize {
    let mut text = String::new();
    while let Some(&c) = chars.get(i) {
        match c {
            '\\' => i += 2,
            '"' => break,
            _ => {
                text.push(c);
                i += 1;
            }
        }
    }
    out.push(text);
    i + 1
}

/// Reads a raw string whose `#`s or `"` start at `i`.
fn read_raw(chars: &[char], i: usize, out: &mut Vec<String>) -> usize {
    let hashes = chars[i..].iter().take_while(|&&c| c == '#').count();
    let open = i + hashes;
    if chars.get(open) != Some(&'"') {
        return i;
    }
    let close: String = std::iter::once('"')
        .chain("#".repeat(hashes).chars())
        .collect();
    let end = skip_past(chars, open + 1, &close);
    let body_end = end.saturating_sub(close.chars().count()).max(open + 1);
    out.push(chars[open + 1..body_end].iter().collect());
    end
}

#[test]
fn messages_speak_python() -> Res<()> {
    let root = root()?;
    let mut failures = Vec::new();
    for krate in ["pythrust-compiler", "pythrust-rt"] {
        for path in files(&root.join("crates").join(krate).join("src"), "rs")? {
            if path.file_name().is_some_and(|n| n == "tests.rs") {
                continue;
            }
            let src = read(&path)?;
            for lit in literals(non_test(&src)) {
                let words = banned_in(&lit);
                if !words.is_empty() {
                    failures.push(format!("{}: {words:?} in {lit:?}", path.display()));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
    Ok(())
}

#[test]
fn banned_words_ignore_case() {
    assert_eq!(banned_in("cannot Borrow `xs`"), vec!["borrow"]);
    assert_eq!(
        banned_in("its LIFETIME ends; move out of it"),
        vec!["lifetime", "move out of"]
    );
    assert!(banned_in("tempered `push` changes `xs` in place; write `mut xs`").is_empty());
}

#[test]
fn literals_skip_comments_and_lifetimes() {
    let src = "// \"borrow\"\nfn f<'a>(c: char) -> &'a str { if c == '\"' { \"one\" } else { r#\"t\"wo\"# } }\n/* \"x\" */ b\"three\"";
    assert_eq!(literals(src), vec!["one", "t\"wo", "three"]);
}

#[test]
fn literals_keep_escapes_out_of_the_way() {
    assert_eq!(literals(r#"("a\"b", '\'', "c\\")"#), vec!["ab", "c"]);
}

#[test]
fn non_test_drops_only_the_inline_test_module() {
    let src = "#[cfg(test)]\nmod tests;\nfn f() {}\n#[cfg(test)]\nmod tests {\n \"borrow\" }\n";
    assert_eq!(non_test(src), "#[cfg(test)]\nmod tests;\nfn f() {}\n");
}
