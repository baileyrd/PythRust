//! The builtin exception classes (grammar.md D6, B4) and Python's
//! hierarchy among them, which `except` and `raises` follow:
//!
//! ```text
//! Exception
//! ├── ArithmeticError: OverflowError, ZeroDivisionError
//! ├── LookupError: IndexError, KeyError
//! ├── OSError: FileNotFoundError, IsADirectoryError, NotADirectoryError,
//! │   PermissionError
//! ├── RuntimeError: RecursionError
//! └── ValueError
//!     └── UnicodeError: UnicodeDecodeError
//! ```
//!
//! A program's own exception classes (step 4-3, D138-D143) each derive
//! from one of these: a [`Class`] is either.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::ast::{unqualified, Module, StmtKind};

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
    Os,
    FileNotFound,
    Permission,
    IsADirectory,
    NotADirectory,
    Unicode,
    UnicodeDecode,
}

impl ExcKind {
    pub const ALL: [ExcKind; 17] = [
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
        ExcKind::Os,
        ExcKind::FileNotFound,
        ExcKind::Permission,
        ExcKind::IsADirectory,
        ExcKind::NotADirectory,
        ExcKind::Unicode,
        ExcKind::UnicodeDecode,
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
            ExcKind::Os => "OSError",
            ExcKind::FileNotFound => "FileNotFoundError",
            ExcKind::Permission => "PermissionError",
            ExcKind::IsADirectory => "IsADirectoryError",
            ExcKind::NotADirectory => "NotADirectoryError",
            ExcKind::Unicode => "UnicodeError",
            ExcKind::UnicodeDecode => "UnicodeDecodeError",
        }
    }

    /// The class this one derives from; `Exception` is the root.
    pub fn base(self) -> Option<ExcKind> {
        match self {
            ExcKind::Exception => None,
            ExcKind::ZeroDivision | ExcKind::Overflow => Some(ExcKind::Arithmetic),
            ExcKind::Index | ExcKind::Key => Some(ExcKind::Lookup),
            ExcKind::Recursion => Some(ExcKind::Runtime),
            ExcKind::FileNotFound
            | ExcKind::Permission
            | ExcKind::IsADirectory
            | ExcKind::NotADirectory => Some(ExcKind::Os),
            ExcKind::Unicode => Some(ExcKind::Value),
            ExcKind::UnicodeDecode => Some(ExcKind::Unicode),
            ExcKind::Value
            | ExcKind::Arithmetic
            | ExcKind::Lookup
            | ExcKind::Runtime
            | ExcKind::Os => Some(ExcKind::Exception),
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

    /// The code as one byte of a runtime descriptor: `'0'` plus the
    /// code, so `KeyError` is `'9'` and `OSError` `':'`.
    pub fn code_char(self) -> char {
        char::from(b'0' + self.code())
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

/// An exception class: a builtin one, or a program's own (D138).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Class {
    Builtin(ExcKind),
    User(Arc<UserClass>),
}

/// A program's own exception class, which derives from a builtin one
/// and adds nothing to it (D139).
#[derive(Debug, PartialEq, Eq, Hash)]
pub struct UserClass {
    /// Its global name, qualified as any module's global is (D127).
    pub name: String,
    pub base: ExcKind,
    /// The runtime's code: the next after the builtins', in the order
    /// the program defines its classes (D143).
    pub code: u16,
}

impl Class {
    /// The name `raises` sets, messages and `repr` show: the class's
    /// own, `E` even when module `a` defines it (D140).
    pub fn name(&self) -> &str {
        match self {
            Class::Builtin(k) => k.name(),
            Class::User(u) => unqualified(&u.name),
        }
    }

    /// The name an uncaught exception's last line shows, as CPython's
    /// traceback: `E` from the main file, `a.E` from module `a` (D141).
    pub fn shown(&self) -> &str {
        match self {
            Class::Builtin(k) => k.name(),
            Class::User(u) => u.name.strip_prefix("__main__.").unwrap_or(&u.name),
        }
    }

    /// The builtin class this one is or derives from.
    pub fn builtin(&self) -> ExcKind {
        match self {
            Class::Builtin(k) => *k,
            Class::User(u) => u.base,
        }
    }

    /// Whether an instance of this class is one of `class`, as `except
    /// class` tests: a user class is only itself or one of its base's.
    pub fn is_a(&self, class: &Class) -> bool {
        match class {
            Class::User(_) => self == class,
            Class::Builtin(k) => self.builtin().is_a(*k),
        }
    }

    /// The runtime's code for this class.
    pub fn code(&self) -> u16 {
        match self {
            Class::Builtin(k) => u16::from(k.code()),
            Class::User(u) => u.code,
        }
    }
}

impl From<ExcKind> for Class {
    fn from(k: ExcKind) -> Self {
        Class::Builtin(k)
    }
}

/// By shown name, so a set of classes prints sorted.
impl Ord for Class {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.name(), self.shown()).cmp(&(other.name(), other.shown()))
    }
}

impl PartialOrd for Class {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A program's exception classes, by name.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Classes {
    user: BTreeMap<String, Class>,
}

impl Classes {
    /// The classes `module` defines, coded in the order it runs them.
    /// Each base names a builtin class, as inference checks (D138); one
    /// that does not makes no class here.
    pub fn of(module: &Module) -> Classes {
        let defs = module.body.iter().filter_map(|s| match &s.kind {
            StmtKind::ExcClass(c) => Some(c),
            _ => None,
        });
        let mut user = BTreeMap::new();
        let first = u16::try_from(ExcKind::ALL.len()).unwrap_or(u16::MAX);
        for (code, def) in (first..).zip(defs) {
            let Some(base) = ExcKind::from_name(&def.base.name) else {
                continue;
            };
            let class = UserClass {
                name: def.name.name.clone(),
                base,
                code,
            };
            user.insert(def.name.name.clone(), Class::User(Arc::new(class)));
        }
        Classes { user }
    }

    /// The user class named `name`, by its qualified name.
    pub fn user(&self, name: &str) -> Option<&Class> {
        self.user.get(name)
    }

    /// The class `name` names: a user class, else a builtin one. The
    /// caller checks that no variable shadows a builtin's name.
    pub fn get(&self, name: &str) -> Option<Class> {
        self.user
            .get(name)
            .cloned()
            .or_else(|| ExcKind::from_name(name).map(Class::Builtin))
    }

    /// Every user class, in no particular order.
    pub fn users(&self) -> impl Iterator<Item = &Class> {
        self.user.values()
    }
}

#[cfg(test)]
mod tests {
    use super::{Class, Classes, ExcKind, UserClass};

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
        assert!(ExcKind::FileNotFound.is_a(ExcKind::Os));
        assert!(ExcKind::Permission.is_a(ExcKind::Os));
        assert!(ExcKind::NotADirectory.is_a(ExcKind::Os));
        assert!(ExcKind::IsADirectory.is_a(ExcKind::Os));
        assert!(!ExcKind::Os.is_a(ExcKind::FileNotFound));
    }

    fn module(src: &str) -> crate::ast::Module {
        let tokens = crate::lexer::lex(src).unwrap_or_else(|d| panic!("{}", d.message));
        crate::parser::parse(&tokens).unwrap_or_else(|d| panic!("{}", d.message))
    }

    #[test]
    fn user_classes_follow_the_builtins_in_definition_order() {
        let m = module("class B(KeyError):\n    pass\n\n\nclass A(Exception):\n    pass\n");
        let classes = Classes::of(&m);
        let (a, b) = (classes.get("A"), classes.get("B"));
        let codes = (a.as_ref().map(Class::code), b.as_ref().map(Class::code));
        assert_eq!(codes, (Some(18), Some(17)));
        assert_eq!(classes.get("KeyError"), Some(Class::Builtin(ExcKind::Key)));
        assert_eq!(classes.get("C"), None);
        assert_eq!(classes.users().count(), 2);
    }

    #[test]
    fn a_user_class_is_itself_and_its_bases_only() {
        let m = module("class B(KeyError):\n    pass\n\n\nclass A(Exception):\n    pass\n");
        let classes = Classes::of(&m);
        let (Some(a), Some(b)) = (classes.get("A"), classes.get("B")) else {
            panic!("both declared");
        };
        let lookup = Class::from(ExcKind::Lookup);
        assert!(b.is_a(&b) && b.is_a(&lookup) && b.is_a(&ExcKind::Exception.into()));
        assert!(!b.is_a(&a) && !a.is_a(&b) && !a.is_a(&lookup));
        assert!(!lookup.is_a(&b), "a base is not its subclass");
        assert_eq!(b.builtin(), ExcKind::Key);
    }

    #[test]
    fn a_class_shows_its_module_but_names_itself() {
        let class = |name: &str| {
            let user = UserClass {
                name: name.to_string(),
                base: ExcKind::Value,
                code: 17,
            };
            Class::User(std::sync::Arc::new(user))
        };
        let names = |c: Class| (c.name().to_string(), c.shown().to_string());
        assert_eq!(names(class("E")), ("E".into(), "E".into()));
        assert_eq!(names(class("pkg.a.E")), ("E".into(), "pkg.a.E".into()));
        assert_eq!(names(class("__main__.E")), ("E".into(), "E".into()));
    }

    #[test]
    fn descriptor_codes_stay_ascii() {
        for k in ExcKind::ALL {
            assert!(k.code_char().is_ascii_graphic(), "{}", k.name());
        }
    }
}
