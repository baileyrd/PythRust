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

/// `manifest` as a crate outside its workspace reads it: each
/// `key.workspace = true` line becomes that key's line in `workspace`'s
/// `[workspace.package]`, and an empty `[workspace]` ends it unless it
/// already has one. A dependency with a `path` is refused, since it
/// would resolve from wherever the manifest is copied.
pub fn standalone(manifest: &str, workspace: &str) -> Result<String, String> {
    let own = manifest.lines().any(|l| is_header(l, "workspace"));
    let workspace = if own { manifest } else { workspace };
    let mut out = String::new();
    let mut in_dependencies = false;
    for line in manifest.lines() {
        if let Some(name) = header(line) {
            in_dependencies = names_dependencies(name);
        }
        if sets_path(line, in_dependencies) {
            return Err(format!(
                "`{}` is a path dependency, which would resolve from the extracted copy; a runtime override cannot have one",
                line.trim()
            ));
        }
        let inherited = line
            .split_once('=')
            .filter(|(_, v)| v.trim() == "true")
            .and_then(|(k, _)| k.trim().strip_suffix(".workspace"));
        match inherited {
            Some(key) => out.push_str(&inherited_line(key, workspace)?),
            None => out.push_str(line),
        }
        out.push('\n');
    }
    if !own {
        out.push_str("\n[workspace]\n");
    }
    Ok(out)
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

/// The table name of the header `line`, such as `workspace` for
/// `[workspace] # root`; `None` when `line` is not a header.
fn header(line: &str) -> Option<&str> {
    let rest = line.trim().strip_prefix('[')?;
    let rest = rest.split('#').next().unwrap_or_default().trim_end();
    Some(rest.trim_end_matches(']').trim_start_matches('[').trim())
}

/// Whether `line` is the header of the table `name`.
pub fn is_header(line: &str, name: &str) -> bool {
    header(line) == Some(name)
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

/// The line of `[workspace.package]` in `workspace` that sets `key`.
fn inherited_line(key: &str, workspace: &str) -> Result<String, String> {
    let mut lines = workspace
        .lines()
        .skip_while(|l| !is_header(l, "workspace.package"));
    lines.next();
    lines
        .take_while(|l| !l.trim_start().starts_with('['))
        .find(|l| l.split_once('=').is_some_and(|(k, _)| k.trim() == key))
        .map(|l| l.trim().to_string())
        .ok_or_else(|| format!("the workspace's [workspace.package] sets no `{key}`"))
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
