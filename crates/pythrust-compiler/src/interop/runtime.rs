//! The runtime source `pythrust` ships (M6d,
//! `docs/research/package-tool.md`): each file of `crates/pythrust-rt`,
//! embedded, with its `Cargo.toml` made standalone, and the hash that
//! names its extracted copy. Writing the copy stays in the driver.

/// One file of a runtime source: its path in the crate, `/`-separated,
/// and its bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct File {
    pub path: String,
    pub text: Vec<u8>,
}

/// Each file of `crates/pythrust-rt` as this `pythrust` was compiled,
/// sorted by path; `Cargo.toml` as the repository writes it.
pub const EMBEDDED: &[(&str, &str)] = &[
    (
        "Cargo.toml",
        include_str!("../../../pythrust-rt/Cargo.toml"),
    ),
    (
        "src/abi.rs",
        include_str!("../../../pythrust-rt/src/abi.rs"),
    ),
    (
        "src/exc.rs",
        include_str!("../../../pythrust-rt/src/exc.rs"),
    ),
    (
        "src/lib.rs",
        include_str!("../../../pythrust-rt/src/lib.rs"),
    ),
    (
        "src/num.rs",
        include_str!("../../../pythrust-rt/src/num.rs"),
    ),
    (
        "src/obj.rs",
        include_str!("../../../pythrust-rt/src/obj.rs"),
    ),
    ("src/os.rs", include_str!("../../../pythrust-rt/src/os.rs")),
    (
        "src/range.rs",
        include_str!("../../../pythrust-rt/src/range.rs"),
    ),
    (
        "src/seq.rs",
        include_str!("../../../pythrust-rt/src/seq.rs"),
    ),
    (
        "src/sha256.rs",
        include_str!("../../../pythrust-rt/src/sha256.rs"),
    ),
    (
        "src/str.rs",
        include_str!("../../../pythrust-rt/src/str.rs"),
    ),
    (
        "src/sync.rs",
        include_str!("../../../pythrust-rt/src/sync.rs"),
    ),
    (
        "src/sys.rs",
        include_str!("../../../pythrust-rt/src/sys.rs"),
    ),
    (
        "src/table.rs",
        include_str!("../../../pythrust-rt/src/table.rs"),
    ),
    (
        "src/task.rs",
        include_str!("../../../pythrust-rt/src/task.rs"),
    ),
];

/// The workspace manifest whose `[workspace.package]` the runtime's
/// `Cargo.toml` inherits from.
const WORKSPACE: &str = include_str!("../../../../Cargo.toml");

/// The embedded runtime, its `Cargo.toml` standalone.
pub fn embedded() -> Result<Vec<File>, String> {
    EMBEDDED
        .iter()
        .map(|(path, text)| {
            let text = match *path {
                "Cargo.toml" => standalone(text, WORKSPACE)?,
                _ => text.to_string(),
            };
            Ok(File {
                path: path.to_string(),
                text: text.into_bytes(),
            })
        })
        .collect()
}

/// `manifest` as a crate outside its workspace reads it (D39): each
/// `key.workspace = true` line in `[package]` becomes that key's value
/// in `workspace`'s `[workspace.package]`, and an empty `[workspace]`
/// ends it unless it has one. That is the only inheritance the repo's
/// own runtime uses; any other form, or inheritance in a manifest that
/// is its own workspace, is a named error asking for a standalone
/// manifest. A dependency with a `path` is refused, since it would
/// resolve from wherever the manifest is copied.
pub fn standalone(manifest: &str, workspace: &str) -> Result<String, String> {
    let stmts = statements(manifest);
    let own = is_root(&stmts);
    let ws = statements(workspace);
    let mut out = String::new();
    let mut table = String::new();
    for stmt in &stmts {
        if let Some(name) = header(&stmt.code) {
            table = name;
        }
        if sets_path(&stmt.code, names_dependencies(&table)) {
            return Err(format!(
                "`{}` is a path dependency, which would resolve from the extracted copy; a runtime override cannot have one",
                stmt.code.trim()
            ));
        }
        let inherits = table == "package" && inherited(&stmt.code).is_some();
        match (inherits, supported(&stmt.code).filter(|_| !own)) {
            (false, _) => out.push_str(&stmt.lines.join("\n")),
            (true, Some(key)) => out.push_str(&inherited_value(key, &ws)?),
            (true, None) => return Err(unsupported(&stmt.code)),
        }
        out.push('\n');
    }
    if !own {
        out.push_str("\n[workspace]\n");
    }
    Ok(out)
}

/// Whether the manifest `text` is a workspace root: it has a
/// `[workspace]` table.
pub fn is_workspace_root(text: &str) -> bool {
    is_root(&statements(text))
}

fn is_root(stmts: &[Statement]) -> bool {
    stmts
        .iter()
        .any(|s| header(&s.code).as_deref() == Some("workspace"))
}

/// One TOML statement: a header, or a key and its whole value, which may
/// span lines (a multi-line array or string).
#[derive(Default)]
struct Statement<'a> {
    lines: Vec<&'a str>,
    /// Its lines without their comments, joined by newlines.
    code: String,
}

/// The statements of `text`, read with the string, array and inline
/// table context carried across lines.
fn statements(text: &str) -> Vec<Statement<'_>> {
    let mut out = Vec::new();
    let mut scan = Scan::default();
    let mut current = Statement::default();
    for line in text.lines() {
        let end = scan.line(line);
        if !current.lines.is_empty() {
            current.code.push('\n');
        }
        current.lines.push(line);
        current.code.push_str(&line[..end]);
        if !scan.open() {
            out.push(std::mem::take(&mut current));
        }
    }
    if !current.lines.is_empty() {
        out.push(current);
    }
    out
}

/// TOML's string openers, longest first.
const QUOTES: [&str; 4] = ["\"\"\"", "'''", "\"", "'"];

/// Where a scan of TOML is: inside which kind of string, and how deep in
/// brackets and braces.
#[derive(Default)]
struct Scan {
    string: Option<&'static str>,
    depth: i32,
}

impl Scan {
    /// Whether a statement continues on the next line.
    fn open(&self) -> bool {
        self.string.is_some() || self.depth > 0
    }

    /// Scans `line`, returning where its comment starts (its length when
    /// it has none).
    fn line(&mut self, line: &str) -> usize {
        let b = line.as_bytes();
        let mut i = 0;
        while i < b.len() {
            if let Some(close) = self.string {
                i = self.in_string(b, i, close);
                continue;
            }
            if let Some(open) = QUOTES
                .into_iter()
                .find(|q| b[i..].starts_with(q.as_bytes()))
            {
                self.string = Some(open);
                i += open.len();
                continue;
            }
            match b[i] {
                b'#' => return i,
                b'[' | b'{' => self.depth += 1,
                b']' | b'}' => self.depth -= 1,
                _ => {}
            }
            i += 1;
        }
        if matches!(self.string, Some("\"" | "'")) {
            self.string = None;
        }
        line.len()
    }

    /// Steps over one character of a string closed by `close`, at `i` in
    /// `b`, and returns where the scan goes on.
    fn in_string(&mut self, b: &[u8], i: usize, close: &'static str) -> usize {
        if b[i] == b'\\' && close.starts_with('"') {
            return i + 2;
        }
        if !b[i..].starts_with(close.as_bytes()) {
            return i + 1;
        }
        self.string = None;
        // A multi-line string may end with up to two quotes of its own
        // before its closing three: `"""a""""` is `a"`.
        let run = b[i..].iter().take_while(|&&c| c == b[i]).count();
        i + if close.len() == 3 { run.min(5) } else { 1 }
    }
}

/// Whether `line` sets a dependency's `path`: a key whose last dotted
/// part is `path`, bare or quoted, on the line or in its inline tables,
/// where the table (`in_dependencies`) or the line's own dotted key names
/// a dependency table. A line heuristic, not a TOML parser: it errs
/// towards refusing, and a refusal costs only a named error.
fn sets_path(line: &str, in_dependencies: bool) -> bool {
    let Some((key, value)) = line.split_once('=') else {
        return false;
    };
    if !in_dependencies && !names_dependencies(key) {
        return false;
    }
    let keys = value
        .split(['{', ','])
        .filter_map(|part| part.split_once('=').map(|(k, _)| k));
    std::iter::once(key)
        .chain(keys)
        .any(|k| last_part(k) == "path")
}

/// The key a comment-free line inherits from the workspace, written as
/// `key.workspace = true` or `key = { workspace = true }`.
fn inherited(code: &str) -> Option<&str> {
    let (key, value) = code.split_once('=')?;
    let value: String = value.chars().filter(|c| !c.is_whitespace()).collect();
    if value == "{workspace=true}" {
        return Some(key.trim());
    }
    (value == "true").then_some(())?;
    key.trim().strip_suffix(".workspace").map(str::trim_end)
}

/// The key of a `key.workspace = true` line with a bare key, the one
/// inheritance form `standalone` rewrites.
fn supported(code: &str) -> Option<&str> {
    let (key, value) = code.split_once('=')?;
    (value.trim() == "true").then_some(())?;
    let key = key.trim().strip_suffix(".workspace")?.trim_end();
    let bare = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
    (!key.is_empty() && key.chars().all(bare)).then_some(key)
}

fn unsupported(code: &str) -> String {
    format!(
        "`{}` inherits from a workspace in a form pythrust does not rewrite: only `key.workspace = true` in [package], from a workspace above; make the override's Cargo.toml standalone",
        code.trim()
    )
}

/// Whether the dotted key or table name `name` has a part naming a
/// dependency table (`dependencies`, `dev-dependencies`, ...).
fn names_dependencies(name: &str) -> bool {
    name.split('.')
        .any(|p| unquoted(p).ends_with("dependencies"))
}

/// The last part of the dotted key `key`, unquoted.
fn last_part(key: &str) -> &str {
    unquoted(key.rsplit('.').next().unwrap_or_default())
}

fn unquoted(part: &str) -> &str {
    part.trim().trim_matches(['"', '\''])
}

/// The table name of the header `code`, such as `workspace` for
/// `[workspace]`; `None` when `code` is not a header.
fn header(code: &str) -> Option<String> {
    let rest = code.trim().strip_prefix('[')?;
    let name = rest.trim_end_matches(']').trim_start_matches('[');
    Some(name.split('.').map(unquoted).collect::<Vec<_>>().join("."))
}

/// The statement of `[workspace.package]` in `workspace` that sets
/// `key`, as written.
fn inherited_value(key: &str, workspace: &[Statement]) -> Result<String, String> {
    let mut table = String::new();
    for stmt in workspace {
        if let Some(name) = header(&stmt.code) {
            table = name;
            continue;
        }
        let sets = stmt
            .code
            .split_once('=')
            .is_some_and(|(k, _)| unquoted(k) == unquoted(key));
        if table == "workspace.package" && sets {
            return Ok(stmt.lines.join("\n"));
        }
    }
    Err(format!(
        "the workspace's [workspace.package] sets no `{key}`"
    ))
}

/// The hash of `files` that names their extracted copy: FNV-1a over each
/// path and text, each length-prefixed, in hex. The copy is still
/// compared byte for byte, so a collision can only cost a rewrite.
pub fn hash(files: &[File]) -> String {
    let mut bytes = Vec::new();
    for f in files {
        for part in [f.path.as_bytes(), &f.text] {
            bytes.extend_from_slice(&(part.len() as u64).to_le_bytes());
            bytes.extend_from_slice(part);
        }
    }
    digest(&bytes)
}

/// 64-bit FNV-1a of `bytes`, as 16 hex digits.
pub fn digest(bytes: &[u8]) -> String {
    let h = bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{h:016x}")
}

#[cfg(test)]
mod tests;
