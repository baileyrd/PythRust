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
    fn replace_matches_cpython_on_edges() {
        assert_eq!(replace("ab", "", "-"), "-a-b-");
        assert_eq!(replace("aaa", "aa", "b"), "ba");
        assert_eq!(replace("ÿüÿ", "ü", "u"), "ÿuÿ");
        assert_eq!(replace("", "", "x"), "x");
    }
}
