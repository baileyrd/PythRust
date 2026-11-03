//! A project's manifest, `pythrust.toml` (M6d,
//! `docs/research/package-tool.md`): `[package]` and the Rust bindings
//! of `[rust]`, read from the TOML subset of [`toml`]. Pure: finding
//! and reading the file stay in the driver.

pub mod toml;

use crate::span::Span;
use toml::{Error, Key, Pair, Str, Table, Value};

/// The file name a project's manifest has.
pub const FILE: &str = "pythrust.toml";

/// A checked manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    /// The bindings of `[rust]`, in order.
    pub rust: Vec<RustDep>,
}

/// One `name = { path = "..." }` of `[rust]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustDep {
    pub name: Key,
    /// Relative to the manifest's directory.
    pub path: Str,
}

type Result<T> = std::result::Result<T, Error>;

fn error<T>(span: Span, message: impl Into<String>) -> Result<T> {
    Err(Error {
        span,
        message: message.into(),
    })
}

/// Reads and checks the manifest `bytes`: the whole file, whichever
/// bindings a program uses.
pub fn parse(bytes: &[u8]) -> Result<Manifest> {
    let tables = toml::parse(bytes)?;
    let mut package = None;
    let mut rust = Vec::new();
    for table in &tables {
        match table.name.name.as_str() {
            "package" => package = Some(table),
            "rust" => rust = rust_deps(table)?,
            "python" => {
                return error(
                    table.name.span,
                    "`[python]` is reserved: Python packages arrive with M6e",
                )
            }
            "pythrust" => {
                let message =
                    "`[pythrust]` is reserved: Pythrust packages arrive with multi-file programs";
                return error(table.name.span, message);
            }
            other => {
                let message =
                    format!("unknown table `[{other}]`; the tables are `[package]` and `[rust]`");
                return error(table.name.span, message);
            }
        }
    }
    let Some(package) = package else {
        let start = Span { start: 0, end: 0 };
        return error(
            start,
            "a `[package]` table with `name` and `version` is required",
        );
    };
    let (name, version) = package_fields(package)?;
    Ok(Manifest {
        name,
        version,
        rust,
    })
}

/// `[package]`'s `name` and `version`.
fn package_fields(table: &Table) -> Result<(String, String)> {
    let mut name = None;
    let mut version = None;
    for Pair { key, value } in &table.pairs {
        let slot = match key.name.as_str() {
            "name" => &mut name,
            "version" => &mut version,
            other => {
                let message = format!(
                    "unknown key `{other}` in `[package]`; the keys are `name` and `version`"
                );
                return error(key.span, message);
            }
        };
        let Value::Str(s) = value else {
            return error(key.span, format!("`{}` must be a string", key.name));
        };
        *slot = Some(s);
    }
    let missing = |what: &str| error(table.name.span, format!("`[package]` needs `{what}`"));
    let Some(name) = name else {
        return missing("name");
    };
    let Some(version) = version else {
        return missing("version");
    };
    if !is_package_name(&name.text) {
        let message =
            "a package name is ASCII letters, digits, `-` and `_`, starting with a letter";
        return error(name.span, message);
    }
    Ok((name.text.clone(), version.text.clone()))
}

fn is_package_name(name: &str) -> bool {
    let mut chars = name.chars();
    let first = chars.next().is_some_and(|c| c.is_ascii_alphabetic());
    first && chars.all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// `[rust]`'s bindings.
fn rust_deps(table: &Table) -> Result<Vec<RustDep>> {
    table.pairs.iter().map(rust_dep).collect()
}

/// One binding: `name = { path = "..." }`.
fn rust_dep(pair: &Pair) -> Result<RustDep> {
    let shape = format!("write `{} = {{ path = \"...\" }}`", pair.key.name);
    let Value::Inline { span, pairs } = &pair.value else {
        return error(
            pair.key.span,
            format!("a binding is an inline table: {shape}"),
        );
    };
    let mut path = None;
    for (key, value) in pairs {
        if key.name != "path" {
            let message = format!("unknown key `{}` in a binding; the key is `path`", key.name);
            return error(key.span, message);
        }
        path = Some(value);
    }
    let Some(path) = path else {
        return error(*span, format!("a binding needs `path`: {shape}"));
    };
    check_path(path)?;
    Ok(RustDep {
        name: pair.key.clone(),
        path: path.clone(),
    })
}

/// A binding's path: non-empty and relative to the manifest, so a
/// project moves as one; `..` may leave its directory.
fn check_path(path: &Str) -> Result<()> {
    let text = path.text.as_str();
    if text.is_empty() {
        return error(path.span, "a binding's path must not be empty");
    }
    if text.contains('\\') {
        return error(path.span, "write `/` in a binding's path, not `\\`");
    }
    // A `:` anywhere could name a Windows drive once the path is split.
    if text.starts_with('/') || text.contains(':') {
        let message = "a binding's path must be relative to the directory of pythrust.toml";
        return error(path.span, message);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PACKAGE: &str = "[package]\nname = \"fclone-lite\"\nversion = \"0.1.0\"\n";

    fn ok(src: &str) -> Manifest {
        match parse(src.as_bytes()) {
            Ok(m) => m,
            Err(e) => panic!("{src:?}: {e:?}"),
        }
    }

    fn err(src: &str) -> String {
        match parse(src.as_bytes()) {
            Ok(m) => panic!("{src:?} parsed: {m:?}"),
            Err(e) => e.message,
        }
    }

    #[test]
    fn a_manifest_names_its_package_and_bindings() {
        let m = ok(&format!(
            "{PACKAGE}\n[rust]\nfastfs = {{ path = \"fastfs\" }}\nb = {{ path = \"../b\" }}\n"
        ));
        assert_eq!(
            (m.name.as_str(), m.version.as_str()),
            ("fclone-lite", "0.1.0")
        );
        let deps: Vec<(&str, &str)> = m
            .rust
            .iter()
            .map(|d| (d.name.name.as_str(), d.path.text.as_str()))
            .collect();
        assert_eq!(deps, [("fastfs", "fastfs"), ("b", "../b")]);
    }

    #[test]
    fn rust_is_optional_and_may_come_first() {
        assert!(ok(PACKAGE).rust.is_empty());
        let m = ok(&format!("[rust]\na = {{ path = \"a\" }}\n{PACKAGE}"));
        assert_eq!(m.rust.len(), 1);
    }

    #[test]
    fn reserved_tables_are_errors_even_when_empty() {
        let python = err(&format!("{PACKAGE}[python]\n"));
        assert_eq!(
            python,
            "`[python]` is reserved: Python packages arrive with M6e"
        );
        let pythrust = err(&format!("{PACKAGE}[pythrust]\n"));
        assert!(pythrust.contains("multi-file programs"), "{pythrust}");
    }

    #[test]
    fn schema_errors_are_named() {
        let cases: &[(&str, &str)] = &[
            (
                "",
                "`[package]` table with `name` and `version` is required",
            ),
            ("[package]\nname = \"a\"\n", "`[package]` needs `version`"),
            ("[package]\nversion = \"1\"\n", "`[package]` needs `name`"),
            (
                "[package]\nname = \"a\"\nversion = \"1\"\nedition = \"x\"\n",
                "unknown key `edition`",
            ),
            (
                "[package]\nname = { a = \"b\" }\nversion = \"1\"\n",
                "`name` must be a string",
            ),
            (
                "[package]\nname = \"1a\"\nversion = \"1\"\n",
                "a package name is ASCII",
            ),
            (
                "[package]\nname = \"\"\nversion = \"1\"\n",
                "a package name is ASCII",
            ),
            (
                "[package]\nname = \"fäst\"\nversion = \"1\"\n",
                "a package name is ASCII",
            ),
            ("[dependencies]\n", "unknown table `[dependencies]`"),
        ];
        for (src, want) in cases {
            let got = err(src);
            assert!(got.contains(want), "{src:?}: {got}");
        }
    }

    #[test]
    fn binding_errors_are_named() {
        let cases: &[(&str, &str)] = &[
            (
                "a = \"a\"",
                "a binding is an inline table: write `a = { path = \"...\" }`",
            ),
            ("a = {}", "a binding needs `path`"),
            (
                "a = { path = \"a\", git = \"x\" }",
                "unknown key `git` in a binding",
            ),
            ("a = { path = \"\" }", "must not be empty"),
            ("a = { path = \"/abs\" }", "must be relative"),
            ("a = { path = \"\\\\server\" }", "write `/`"),
            ("a = { path = \"sub\\\\x\" }", "write `/`"),
            ("a = { path = \"C:/x\" }", "must be relative"),
            ("a = { path = \"sub/C:/x\" }", "must be relative"),
        ];
        for (line, want) in cases {
            let got = err(&format!("{PACKAGE}[rust]\n{line}\n"));
            assert!(got.contains(want), "{line}: {got}");
        }
    }

    #[test]
    fn subset_errors_pass_through() {
        assert!(err("[package]\nname = 'a'\n").contains("literal strings"));
    }
}
