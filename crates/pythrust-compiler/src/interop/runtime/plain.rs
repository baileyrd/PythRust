//! The fail-closed reading of a runtime override's manifest (D42): in
//! `[package]`, `[workspace.package]` and dependency tables, only the
//! spellings the checks plainly read are accepted. Anything else is a
//! named error, so a spelling the scanner misses can only over-refuse.

use super::{assignment, inline_keys, parts, unquoted};

/// Checks that the statement `code`, in the table named `table` (its
/// parts unquoted; empty for the root table), is plainly written.
pub(super) fn check(table: &str, code: &str) -> Result<(), String> {
    let ok = match assignment(code) {
        _ if code.trim().is_empty() => true,
        None => plain_header(code),
        Some((key, value)) => plain_key(table, key, value),
    };
    ok.then_some(()).ok_or_else(|| refused(code))
}

/// Whether the header `code` is plain: outside the guarded tables any
/// spelling, inside them bare parts, with a quoted part allowed only
/// right after `target` (a cfg or a target triple).
fn plain_header(code: &str) -> bool {
    let name = code.trim().trim_start_matches('[').trim_end_matches(']');
    let raw = dotted(name);
    let names: Vec<&str> = raw.iter().map(|part| unquoted(part)).collect();
    if !guarded(&names) {
        return true;
    }
    raw.iter()
        .enumerate()
        .all(|(i, part)| bare(part) || i > 0 && names[i - 1] == "target")
}

/// Whether the statement `key = value` in `table` is plain. A statement
/// reaching a guarded table from the root table, by a dotted key or an
/// inline table, is never plain; inside a guarded table the key is one
/// bare part (or `key.workspace = true` in `[package]`), and only a
/// dependency may be a flat inline table of bare keys.
fn plain_key(table: &str, key: &str, value: &str) -> bool {
    let mut keys = vec![key.trim().to_string()];
    inline_keys(key.trim(), value, &mut keys);
    let full = |key: &str| -> Vec<String> {
        parts(table)
            .into_iter()
            .chain(parts(key))
            .map(str::to_string)
            .collect()
    };
    let reached: Vec<bool> = keys.iter().map(|k| guarded_owned(&full(k))).collect();
    if !reached.iter().any(|&g| g) {
        return true;
    }
    if table.is_empty() || !guarded_owned(&full(key)) {
        return false;
    }
    plain_assignment(table, key.trim(), value) && flat_inline(table, key.trim(), &keys[1..])
}

/// Whether `key = value` in the guarded `table` has a plain key.
fn plain_assignment(table: &str, key: &str, value: &str) -> bool {
    match dotted(key).as_slice() {
        [one] => bare(one),
        [field, workspace] => {
            table == "package" && bare(field) && *workspace == "workspace" && value.trim() == "true"
        }
        _ => false,
    }
}

/// Whether the inline-table keys `inner` of `key` in `table` are plain:
/// none, or, in a dependency table, one level of bare keys.
fn flat_inline(table: &str, key: &str, inner: &[String]) -> bool {
    if inner.is_empty() {
        return true;
    }
    let dependency = parts(table).iter().any(|p| p.ends_with("dependencies"));
    let depth = dotted(key).len() + 1;
    dependency
        && inner.iter().all(|k| {
            let raw = dotted(k);
            raw.len() == depth && raw.last().is_some_and(|last| bare(last))
        })
}

/// Whether the full key or table path `parts` is guarded: in
/// `[package]` (not `package.metadata`), `[workspace.package]`, or a
/// dependency table.
fn guarded(parts: &[&str]) -> bool {
    match parts {
        ["package", "metadata", ..] => false,
        ["package", ..] | ["workspace", "package", ..] => true,
        _ => parts.iter().any(|p| p.ends_with("dependencies")),
    }
}

fn guarded_owned(parts: &[String]) -> bool {
    guarded(&parts.iter().map(String::as_str).collect::<Vec<_>>())
}

/// Whether `part` is a bare TOML key.
fn bare(part: &str) -> bool {
    let part = part.trim();
    !part.is_empty()
        && part
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// The parts of the dotted key or table name `name`, as written, split
/// only at dots outside quotes.
fn dotted(name: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (i, c) in name.char_indices() {
        if let Some(q) = quote {
            let closes = c == q && !escaped;
            escaped = q == '"' && c == '\\' && !escaped;
            quote = quote.filter(|_| !closes);
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '.' => {
                out.push(name[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(name[start..].trim());
    out
}

fn refused(code: &str) -> String {
    format!(
        "`{}` is not a spelling pythrust checks in a runtime override's [package], [workspace.package] or dependency tables (bare keys, one header per table, flat inline dependencies only); write the override's Cargo.toml as a plainly written standalone manifest",
        code.trim()
    )
}
