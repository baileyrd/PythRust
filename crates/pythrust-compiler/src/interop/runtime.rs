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
/// already has one.
pub fn standalone(manifest: &str, workspace: &str) -> Result<String, String> {
    let mut out = String::new();
    for line in manifest.lines() {
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
    if !manifest.lines().any(|l| l.trim() == "[workspace]") {
        out.push_str("\n[workspace]\n");
    }
    Ok(out)
}

/// The line of `[workspace.package]` in `workspace` that sets `key`.
fn inherited_line(key: &str, workspace: &str) -> Result<String, String> {
    let mut lines = workspace
        .lines()
        .skip_while(|l| l.trim() != "[workspace.package]");
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
