//! `str` methods as CPython computes them, on `&str`: safe Rust with no
//! dependency, shared by the runtime's ABI (`str.rs`) and the
//! interpreter, which compiles this file too. Indices and lengths are
//! code points (D49).

/// Which ends `strip` trims.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Both,
    Left,
    Right,
}

/// CPython's `str.isspace` for one code point: Rust's whitespace plus
/// the separators U+001C to U+001F, which Rust leaves out (D49).
pub fn is_space(c: char) -> bool {
    c.is_whitespace() || ('\u{1c}'..='\u{1f}').contains(&c)
}

/// `s.split()`: the runs of non-space, with no empty parts.
pub fn split_space(s: &str) -> Vec<&str> {
    s.split(is_space).filter(|p| !p.is_empty()).collect()
}

/// `s.split(sep)`: every part between two `sep`s, empty ones too;
/// `None` for an empty `sep`, which CPython refuses with a `ValueError`.
pub fn split<'s>(s: &'s str, sep: &str) -> Option<Vec<&'s str>> {
    if sep.is_empty() {
        return None;
    }
    Some(s.split(sep).collect())
}

/// `s.strip(chars)` at `side`: drops the code points in `chars` from
/// those ends, or the spaces when `chars` is `None`.
pub fn strip<'s>(s: &'s str, chars: Option<&str>, side: Side) -> &'s str {
    let drop = |c: char| match chars {
        Some(set) => set.contains(c),
        None => is_space(c),
    };
    match side {
        Side::Both => s.trim_matches(drop),
        Side::Left => s.trim_start_matches(drop),
        Side::Right => s.trim_end_matches(drop),
    }
}

/// `s.replace(old, new)`: every `old`, left to right without overlap,
/// becomes `new`; an empty `old` puts `new` around every code point.
pub fn replace(s: &str, old: &str, new: &str) -> String {
    s.replace(old, new)
}

/// The code-point index where byte offset `at` of `s` starts.
fn chars_before(s: &str, at: usize) -> usize {
    s[..at].chars().count()
}

/// `s.find(sub)` (`right` false) or `s.rfind(sub)`: the code-point
/// index of the first or last `sub`, or `None`. An empty `sub` is found
/// at 0, or at the end from the right, as in CPython.
pub fn find(s: &str, sub: &str, right: bool) -> Option<usize> {
    let at = if right { s.rfind(sub) } else { s.find(sub) }?;
    Some(chars_before(s, at))
}

/// `s.count(sub)`: non-overlapping matches, left to right; an empty
/// `sub` matches between every two code points and at both ends.
pub fn count(s: &str, sub: &str) -> usize {
    if sub.is_empty() {
        return s.chars().count() + 1;
    }
    s.matches(sub).count()
}

/// `s.upper()` (`upper` true) or `s.lower()`: Rust's full Unicode case
/// mappings, special casing and final sigma included, as CPython's;
/// their Unicode version may be newer than CPython 3.13's 15.1 (D49).
pub fn case(s: &str, upper: bool) -> String {
    if upper {
        s.to_uppercase()
    } else {
        s.to_lowercase()
    }
}

/// The message of the `ValueError` `index` and `rindex` raise.
pub const NOT_FOUND: &str = "substring not found";

/// Python's `repr` of a string: single quotes unless the text holds a
/// single quote and no double quote; `\\`, `\n`, `\r`, `\t` and the
/// quote escaped, and every `escaped` code point as `\x`, `\u` or
/// `\U` (D54).
pub fn repr(s: &str) -> String {
    let quote = if s.contains('\'') && !s.contains('"') {
        '"'
    } else {
        '\''
    };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if escaped(c) => out.push_str(&hex_escape(c)),
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

/// Whether `repr` escapes `c`, as CPython does what `str.isprintable`
/// rejects: controls (Cc), separators other than space (Zs, Zl, Zp) and
/// private use (Co). Format characters (Cf) and unassigned code points
/// (Cn) need a category table, so they print raw, a listed deviation
/// (D54).
fn escaped(c: char) -> bool {
    let separator = c.is_whitespace() && !c.is_control() && c != ' ';
    let private = matches!(c, '\u{e000}'..='\u{f8ff}' | '\u{f0000}'..='\u{ffffd}' | '\u{100000}'..='\u{10fffd}');
    c.is_control() || separator || private
}

/// `\xhh` up to U+00FF, `\uhhhh` up to U+FFFF, `\Uhhhhhhhh` above.
fn hex_escape(c: char) -> String {
    let n = u32::from(c);
    match n {
        0..=0xff => format!("\\x{n:02x}"),
        0x100..=0xffff => format!("\\u{n:04x}"),
        _ => format!("\\U{n:08x}"),
    }
}

/// The message of the `ValueError` an empty separator raises.
pub const EMPTY_SEPARATOR: &str = "empty separator";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_space_is_cpythons_set() {
        // CPython 3.13: [c for c in range(0x110000) if chr(c).isspace()]
        let cpython: Vec<u32> = [
            0x9..=0xd,
            0x1c..=0x20,
            0x85..=0x85,
            0xa0..=0xa0,
            0x1680..=0x1680,
            0x2000..=0x200a,
            0x2028..=0x2029,
            0x202f..=0x202f,
            0x205f..=0x205f,
            0x3000..=0x3000,
        ]
        .into_iter()
        .flatten()
        .collect();
        let ours: Vec<u32> = (0..=0x10ffff_u32)
            .filter(|&c| char::from_u32(c).is_some_and(is_space))
            .collect();
        assert_eq!(ours, cpython);
    }

    #[test]
    fn split_space_drops_empty_parts() {
        assert_eq!(split_space("  a \u{3000}b\u{1f}c  "), ["a", "b", "c"]);
        assert!(split_space("").is_empty());
        assert!(split_space(" \t\n").is_empty());
    }

    #[test]
    fn split_keeps_empty_parts_and_refuses_an_empty_separator() {
        assert_eq!(split("a,,b,", ","), Some(vec!["a", "", "b", ""]));
        assert_eq!(split("", ","), Some(vec![""]));
        assert_eq!(split("αβγβ", "β"), Some(vec!["α", "γ", ""]));
        assert_eq!(split("a", ""), None);
    }

    #[test]
    fn strip_trims_the_given_ends() {
        assert_eq!(strip("\u{a0} a b \u{1c}", None, Side::Both), "a b");
        assert_eq!(strip("  a  ", None, Side::Left), "a  ");
        assert_eq!(strip("  a  ", None, Side::Right), "  a");
        assert_eq!(strip("xyaxy", Some("yx"), Side::Both), "a");
        assert_eq!(strip("ßaß", Some("ß"), Side::Left), "aß");
        assert_eq!(strip("aa", Some(""), Side::Both), "aa");
        assert_eq!(strip("", None, Side::Both), "");
    }

    #[test]
    fn repr_escapes_what_cpython_escapes_but_cf_and_cn() {
        assert_eq!(repr("a b"), "'a b'");
        assert_eq!(repr("it's"), "\"it's\"");
        assert_eq!(
            repr("\\\n\t\u{0}\u{7f}\u{85}"),
            "'\\\\\\n\\t\\x00\\x7f\\x85'"
        );
        assert_eq!(
            repr("\u{a0}\u{3000}\u{2028}\u{2029}"),
            "'\\xa0\\u3000\\u2028\\u2029'"
        );
        assert_eq!(
            repr("\u{e000}\u{f0000}\u{10fffd}"),
            "'\\ue000\\U000f0000\\U0010fffd'"
        );
        assert_eq!(repr("ßΣ中\u{200b}"), "'ßΣ中\u{200b}'");
    }

    #[test]
    fn find_and_count_count_code_points() {
        assert_eq!(find("αβγβ", "β", false), Some(1));
        assert_eq!(find("αβγβ", "β", true), Some(3));
        assert_eq!(find("αβ", "x", false), None);
        assert_eq!(find("αβ", "", false), Some(0));
        assert_eq!(find("αβ", "", true), Some(2));
        assert_eq!(count("aaaa", "aa"), 2);
        assert_eq!(count("αβ", ""), 3);
        assert_eq!(count("", ""), 1);
        assert_eq!(count("中文中", "中"), 2);
    }

    #[test]
    fn case_maps_in_full() {
        assert_eq!(case("ΣΑΣ ΑΣ.", false), "σας ας.");
        assert_eq!(case("ß ŉ ﬁ", true), "SS ʼN FI");
        assert_eq!(case("İ", false), "i\u{307}");
        assert_eq!(case("", true), "");
    }

    #[test]
    fn replace_matches_cpython_on_edges() {
        assert_eq!(replace("ab", "", "-"), "-a-b-");
        assert_eq!(replace("aaa", "aa", "b"), "ba");
        assert_eq!(replace("ÿüÿ", "ü", "u"), "ÿuÿ");
        assert_eq!(replace("", "", "x"), "x");
    }
}
