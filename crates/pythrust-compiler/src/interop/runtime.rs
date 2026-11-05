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
        "src/fsum.rs",
        include_str!("../../../pythrust-rt/src/fsum.rs"),
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
    (
        "src/text.rs",
        include_str!("../../../pythrust-rt/src/text.rs"),
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
    for stmt in &ws {
        plain::quoted(&stmt.code)?;
    }
    let mut out = String::new();
    let mut table = String::new();
    for stmt in &stmts {
        if let Some(name) = header(&stmt.code) {
            table = name;
        }
        plain::check(&table, &stmt.code)?;
        let metadata = in_metadata(&table, &stmt.code);
        if sets_path(&table, &stmt.code) {
            return Err(format!(
                "`{}` is a path dependency, which would resolve from the extracted copy; a runtime override cannot have one",
                stmt.code.trim()
            ));
        }
        let inherits = !metadata && inherited(&table, &stmt.code);
        let rewritable = !own && table == "package";
        match (inherits, supported(&stmt.code).filter(|_| rewritable)) {
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
/// `workspace` table, as a header or a root-table dotted key.
pub fn is_workspace_root(text: &str) -> Result<bool, String> {
    let stmts = statements(text);
    for stmt in &stmts {
        plain::quoted(&stmt.code)?;
    }
    Ok(is_root(&stmts))
}

fn is_root(stmts: &[Statement]) -> bool {
    let mut table = String::new();
    stmts.iter().any(|stmt| {
        if let Some(name) = header(&stmt.code) {
            table = name;
        }
        path_parts(&table, &stmt.code).first() == Some(&"workspace")
    })
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
            out.push(finished(std::mem::take(&mut current)));
        }
    }
    if !current.lines.is_empty() {
        out.push(finished(current));
    }
    out
}

/// `stmt` with the unicode escapes in its code decoded.
fn finished(mut stmt: Statement<'_>) -> Statement<'_> {
    stmt.code = decoded(&stmt.code);
    stmt
}

/// `code` with each `\\uXXXX` or `\\UXXXXXXXX` escape in a basic string
/// replaced by its character, so a quoted key reads as cargo decodes it.
/// An escape that would make a quote, a backslash or a control
/// character stays as written, so string bounds never move.
fn decoded(code: &str) -> String {
    let mut out = String::with_capacity(code.len());
    let mut close: Option<&'static str> = None;
    let mut i = 0;
    while let Some(c) = code[i..].chars().next() {
        let rest = &code[i..];
        let step = match close {
            None => QUOTES.into_iter().find(|q| rest.starts_with(q)).map(|q| {
                close = Some(q);
                out.push_str(q);
                q.len()
            }),
            Some(q) if q.starts_with('"') && c == '\\' => Some(push_escape(rest, &mut out)),
            Some(q) if rest.starts_with(q) => {
                close = None;
                let run = rest.bytes().take_while(|&b| b == q.as_bytes()[0]).count();
                let len = if q.len() == 3 { run.min(5) } else { 1 };
                out.push_str(&rest[..len]);
                Some(len)
            }
            Some(_) => None,
        };
        i += step.unwrap_or_else(|| {
            out.push(c);
            c.len_utf8()
        });
    }
    out
}

/// Pushes the escape at the start of `rest` onto `out`, decoded when it
/// is a safe unicode escape, and returns its length.
fn push_escape(rest: &str, out: &mut String) -> usize {
    let digits = match rest.as_bytes().get(1) {
        Some(b'u') => 4,
        Some(b'U') => 8,
        _ => 0,
    };
    let decoded = rest
        .get(2..2 + digits)
        .filter(|hex| digits > 0 && hex.bytes().all(|b| b.is_ascii_hexdigit()))
        .and_then(|hex| u32::from_str_radix(hex, 16).ok())
        .and_then(char::from_u32)
        .filter(|ch| !ch.is_control() && !matches!(ch, '"' | '\'' | '\\'));
    if let Some(ch) = decoded {
        out.push(ch);
        return 2 + digits;
    }
    let len = 1 + rest[1..].chars().next().map_or(0, char::len_utf8);
    out.push_str(&rest[..len]);
    len
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

/// Whether the statement `code` in `table` sets a dependency's `path`:
/// a key whose last part is `path`, bare or quoted, below a part naming
/// a dependency table, judged by the full path of its key and of each
/// key in an inline-table value, outside `package.metadata`. A
/// heuristic, not a TOML parser: it errs towards refusing, and a
/// refusal costs only a named error.
fn sets_path(table: &str, code: &str) -> bool {
    full_keys(table, code).iter().any(|full| {
        let Some((last, above)) = full.split_last() else {
            return false;
        };
        last == "path"
            && !matches!(full.as_slice(), [a, b, ..] if a == "package" && b == "metadata")
            && names_dependency(above)
    })
}

/// Whether the key or table path `parts` reaches a dependency table:
/// a part naming one (`dependencies`, `dev-dependencies`, ...), or a
/// `[patch]` or `[replace]` table, whose entries are dependencies too.
fn names_dependency<S: AsRef<str>>(parts: &[S]) -> bool {
    let first = parts.first().map(AsRef::as_ref);
    matches!(first, Some("patch" | "replace"))
        || parts.iter().any(|p| p.as_ref().ends_with("dependencies"))
}

/// The full key path, as unquoted parts, of the statement `code` in
/// `table` and of every key in its inline-table value.
fn full_keys(table: &str, code: &str) -> Vec<Vec<String>> {
    let Some((key, value)) = assignment(code) else {
        return Vec::new();
    };
    let mut keys = vec![key.trim().to_string()];
    inline_keys(key.trim(), value, &mut keys);
    keys.iter()
        .map(|key| {
            parts(table)
                .into_iter()
                .chain(parts(key))
                .map(str::to_string)
                .collect()
        })
        .collect()
}

/// Whether the statement `code` in `table` inherits from a workspace
/// where cargo reads inheritance, in any spelling: a key part named
/// `workspace`, bare or quoted, in its key or in the keys of an
/// inline-table value, judged by each key's full path so a
/// `package.metadata` subtree is skipped. A dependency named `workspace`
/// (`workspace = "0.1"` in a dependency table) is not inheritance; its
/// inline keys still are checked. Recognized only to refuse
/// what `supported` does not rewrite.
fn inherited(table: &str, code: &str) -> bool {
    let Some((key, value)) = assignment(code) else {
        return false;
    };
    let base = parts(table);
    let mut keys = vec![key.trim().to_string()];
    inline_keys(key.trim(), value, &mut keys);
    let named = names_dependency(&base) && parts(key.trim()) == ["workspace"];
    let dependency = named && value.trim() != "true";
    keys.iter().skip(usize::from(dependency)).any(|key| {
        let own = parts(key);
        let full: Vec<&str> = base.iter().chain(&own).copied().collect();
        own.contains(&"workspace")
            && !full.starts_with(&["package", "metadata"])
            && inheritable(&full)
    })
}

/// Pushes onto `out` the full dotted key, under `key`, of every key in
/// the inline-table `value`, nested tables included. Array contents and
/// string contents are skipped.
fn inline_keys(key: &str, value: &str, out: &mut Vec<String>) {
    let mut open: Vec<String> = Vec::new();
    let mut last = key.to_string();
    let mut name = String::new();
    let mut naming = false;
    let mut quote: Option<&'static str> = None;
    let mut arrays = 0usize;
    let mut i = 0;
    while let Some(c) = value[i..].chars().next() {
        let rest = &value[i..];
        let opener = QUOTES.into_iter().find(|q| rest.starts_with(q));
        let len = match (quote, opener) {
            (Some(q), _) => string_step(rest, q, &mut quote),
            (None, Some(q)) => {
                quote = Some(q);
                q.len()
            }
            (None, None) => c.len_utf8(),
        };
        if naming && (quote.is_some() || opener.is_some()) {
            name.push_str(&rest[..len]);
        }
        i += len;
        if quote.is_some() || opener.is_some() {
            continue;
        }
        match c {
            '[' => arrays += 1,
            ']' => arrays = arrays.saturating_sub(1),
            _ if arrays > 0 => {}
            '{' => {
                open.push(std::mem::take(&mut last));
                naming = true;
            }
            '}' => {
                open.pop();
                naming = false;
            }
            ',' => naming = !open.is_empty(),
            '=' if naming => {
                let prefix = open.last().map_or("", String::as_str);
                last = format!("{prefix}.{}", name.trim());
                out.push(last.clone());
                name.clear();
                naming = false;
            }
            _ if naming => name.push(c),
            _ => {}
        }
    }
}

/// Steps over the start of `rest` inside a string closed by `q`: an
/// escape in a basic string, the closing quote run (clearing `quote`),
/// or one character. Returns how far it stepped.
fn string_step(rest: &str, q: &'static str, quote: &mut Option<&'static str>) -> usize {
    let one = |s: &str| s.chars().next().map_or(0, char::len_utf8);
    if q.starts_with('"') && rest.starts_with('\\') {
        return 1 + one(&rest[1..]);
    }
    if !rest.starts_with(q) {
        return one(rest);
    }
    *quote = None;
    let run = rest.bytes().take_while(|&b| b == q.as_bytes()[0]).count();
    if q.len() == 3 {
        run.min(5)
    } else {
        1
    }
}

/// The key and value of the key-value statement `code`, split at the
/// first `=` outside a quoted key, so `'cfg(a = "b")'` stays whole.
fn assignment(code: &str) -> Option<(&str, &str)> {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (i, c) in code.char_indices() {
        if let Some(q) = quote {
            let closes = c == q && !escaped;
            escaped = q == '"' && c == '\\' && !escaped;
            quote = quote.filter(|_| !closes);
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '=' => return Some((&code[..i], &code[i + 1..])),
            _ => {}
        }
    }
    None
}

/// The unquoted, non-empty parts of the dotted key or table name `name`.
fn parts(name: &str) -> Vec<&str> {
    name.split('.')
        .map(unquoted)
        .filter(|part| !part.is_empty())
        .collect()
}

/// Whether the statement `code` in `table` sets a key under
/// `package.metadata`, which Cargo leaves to other tools.
fn in_metadata(table: &str, code: &str) -> bool {
    path_parts(table, code).starts_with(&["package", "metadata"])
}

/// Whether the full key path `parts` is somewhere cargo reads workspace
/// inheritance: a package field (a root-table `package.x` or
/// `package = { ... }` included), a dependency, or `lints`.
/// `package.metadata` is excluded by the caller.
fn inheritable(parts: &[&str]) -> bool {
    match parts.first() {
        Some(&"workspace") | None => false,
        Some(&"lints") | Some(&"package") => true,
        Some(_) => names_dependency(parts),
    }
}

/// The unquoted parts of the full key the statement `code` sets in
/// `table`; for a header, the table's own name.
fn path_parts<'a>(table: &'a str, code: &'a str) -> Vec<&'a str> {
    let key = assignment(code).map_or("", |(key, _)| key);
    let mut all = parts(table);
    all.extend(parts(key));
    all
}

/// The key of a `key.workspace = true` line with a bare key, the one
/// inheritance form `standalone` rewrites.
fn supported(code: &str) -> Option<&str> {
    let (key, value) = assignment(code)?;
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
        let sets = assignment(&stmt.code).is_some_and(|(k, _)| k.trim() == key);
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

mod plain;
