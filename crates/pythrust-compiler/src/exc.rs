//! The builtin exception classes (grammar.md D6, B4) and Python's
//! hierarchy among them, which `except` and `raises` follow:
//!
//! ```text
//! Exception
//! ├── ArithmeticError: OverflowError, ZeroDivisionError
//! ├── LookupError: IndexError, KeyError
//! ├── RuntimeError: RecursionError
//! └── ValueError
//! ```

use std::cmp::Ordering;

/// A builtin exception class. The runtime knows each by its index in
/// [`ExcKind::ALL`], so new classes go at the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExcKind {
    Value,
    Index,
    ZeroDivision,
    Overflow,
    Recursion,
    Arithmetic,
    Lookup,
    Runtime,
    Exception,
    Key,
}

impl ExcKind {
    pub const ALL: [ExcKind; 10] = [
        ExcKind::Value,
        ExcKind::Index,
        ExcKind::ZeroDivision,
        ExcKind::Overflow,
        ExcKind::Recursion,
        ExcKind::Arithmetic,
        ExcKind::Lookup,
        ExcKind::Runtime,
        ExcKind::Exception,
        ExcKind::Key,
    ];

    /// The builtin exception class named `name`.
    pub fn from_name(name: &str) -> Option<ExcKind> {
        ExcKind::ALL.into_iter().find(|k| k.name() == name)
    }

    pub fn name(self) -> &'static str {
        match self {
            ExcKind::Value => "ValueError",
            ExcKind::Index => "IndexError",
            ExcKind::ZeroDivision => "ZeroDivisionError",
            ExcKind::Overflow => "OverflowError",
            ExcKind::Recursion => "RecursionError",
            ExcKind::Arithmetic => "ArithmeticError",
            ExcKind::Lookup => "LookupError",
            ExcKind::Runtime => "RuntimeError",
            ExcKind::Exception => "Exception",
            ExcKind::Key => "KeyError",
        }
    }

    /// The class this one derives from; `Exception` is the root.
    pub fn base(self) -> Option<ExcKind> {
        match self {
            ExcKind::Exception => None,
            ExcKind::ZeroDivision | ExcKind::Overflow => Some(ExcKind::Arithmetic),
            ExcKind::Index | ExcKind::Key => Some(ExcKind::Lookup),
            ExcKind::Recursion => Some(ExcKind::Runtime),
            ExcKind::Value | ExcKind::Arithmetic | ExcKind::Lookup | ExcKind::Runtime => {
                Some(ExcKind::Exception)
            }
        }
    }

    /// Whether an instance of this class is one of `class`: the class
    /// itself or one it derives from, as `except class` tests.
    pub fn is_a(self, class: ExcKind) -> bool {
        std::iter::successors(Some(self), |k| k.base()).any(|k| k == class)
    }

    /// The runtime's code for this class.
    pub fn code(self) -> u8 {
        self as u8
    }
}

/// By name, so a set of classes prints sorted.
impl Ord for ExcKind {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name().cmp(other.name())
    }
}

impl PartialOrd for ExcKind {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::ExcKind;

    #[test]
    fn names_round_trip_and_codes_are_positions() {
        for (i, k) in ExcKind::ALL.into_iter().enumerate() {
            assert_eq!(ExcKind::from_name(k.name()), Some(k));
            assert_eq!(usize::from(k.code()), i);
        }
        assert_eq!(ExcKind::from_name("BaseException"), None);
    }

    #[test]
    fn every_class_is_an_exception_and_itself() {
        for k in ExcKind::ALL {
            assert!(k.is_a(k) && k.is_a(ExcKind::Exception));
        }
    }

    #[test]
    fn subclasses_follow_python() {
        assert!(ExcKind::ZeroDivision.is_a(ExcKind::Arithmetic));
        assert!(ExcKind::Overflow.is_a(ExcKind::Arithmetic));
        assert!(ExcKind::Index.is_a(ExcKind::Lookup));
        assert!(ExcKind::Key.is_a(ExcKind::Lookup));
        assert!(!ExcKind::Key.is_a(ExcKind::Index));
        assert!(ExcKind::Recursion.is_a(ExcKind::Runtime));
        assert!(!ExcKind::Value.is_a(ExcKind::Arithmetic));
        assert!(!ExcKind::Arithmetic.is_a(ExcKind::ZeroDivision));
        assert!(!ExcKind::Exception.is_a(ExcKind::Value));
    }
}
