//! The standard library (M6a): the modules a program may `import` and
//! the names each exposes, and the builtin `open` with its file's
//! methods. A Python-compatible surface over the Rust
//! runtime (`docs/roadmap.md#phase-5-m6-plan`): each name behaves as in
//! CPython or is on grammar.md's deviations list.
//!
//! Stages see no modules. The parser resolves `module.attr` after
//! `import module` to one name with a dot in it (`sys.argv`), which no
//! user name can be, so a module variable is an ordinary global and a
//! module function a builtin. `import m` assigns each of `m`'s variables
//! from its initializer, a call to a [`StdFn`] only the parser can write.

use crate::exc::ExcKind;

/// A module and the dotted names it exposes.
pub struct ModuleDef {
    pub name: &'static str,
    /// Variables, each with the function that makes its value.
    pub vars: &'static [(&'static str, StdFn)],
    /// Functions, called by their [`StdFn::name`].
    pub funcs: &'static [StdFn],
}

/// The importable modules.
pub const MODULES: &[ModuleDef] = &[
    ModuleDef {
        name: "sys",
        vars: &[("sys.argv", StdFn::Argv)],
        funcs: &[],
    },
    ModuleDef {
        name: "os",
        vars: &[],
        funcs: &[
            StdFn::Listdir,
            StdFn::Join,
            StdFn::Isdir,
            StdFn::Isfile,
            StdFn::Islink,
            StdFn::Getsize,
        ],
    },
];

/// A runtime function behind a module name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StdFn {
    /// `sys.argv`'s value: a new `list[str]` of the program's arguments.
    Argv,
    /// `os.listdir(path='.')`: a new `list[str]` of the entry names.
    Listdir,
    /// `os.path.join(a, *p)`: POSIX rules (a listed deviation on Windows).
    Join,
    /// `os.path.isdir(p)`, following links; `False` on any error.
    Isdir,
    /// `os.path.isfile(p)`, following links; `False` on any error.
    Isfile,
    /// `os.path.islink(p)`; `False` on any error.
    Islink,
    /// `os.path.getsize(p)`, following links.
    Getsize,
    /// The builtin `open(p, "rb")`: a file for reading bytes.
    Open,
    /// `f.read()` of a file: the rest of it, as `bytes`.
    Read,
    /// `f.close()` of a file; closing it again does nothing.
    Close,
}

impl StdFn {
    pub const ALL: [StdFn; 10] = [
        StdFn::Argv,
        StdFn::Listdir,
        StdFn::Join,
        StdFn::Isdir,
        StdFn::Isfile,
        StdFn::Islink,
        StdFn::Getsize,
        StdFn::Open,
        StdFn::Read,
        StdFn::Close,
    ];

    /// The builtin name stages call it by: the dotted name a program
    /// writes. An initializer's name has a `__` part, so no program can
    /// write it.
    pub fn name(self) -> &'static str {
        match self {
            StdFn::Argv => "sys.__argv__",
            StdFn::Listdir => "os.listdir",
            StdFn::Join => "os.path.join",
            StdFn::Isdir => "os.path.isdir",
            StdFn::Isfile => "os.path.isfile",
            StdFn::Islink => "os.path.islink",
            StdFn::Getsize => "os.path.getsize",
            StdFn::Open => "open",
            StdFn::Read => "BufferedReader.read",
            StdFn::Close => "BufferedReader.close",
        }
    }

    /// The file method called `attr`: `read` or `close`. Each method
    /// name belongs to one type, so the name alone decides.
    pub fn method(attr: &str) -> Option<StdFn> {
        match attr {
            "read" => Some(StdFn::Read),
            "close" => Some(StdFn::Close),
            _ => None,
        }
    }

    /// The class of what it may raise, tracked in `raises` (D8). An
    /// `OSError` is raised as the subclass the error's kind names.
    /// Reading a closed file also raises a `ValueError`, untracked like
    /// the operator faults: it means a bug, as an index out of range does.
    pub fn raises(self) -> Option<ExcKind> {
        match self {
            StdFn::Listdir | StdFn::Getsize | StdFn::Open | StdFn::Read => Some(ExcKind::Os),
            StdFn::Argv
            | StdFn::Join
            | StdFn::Isdir
            | StdFn::Isfile
            | StdFn::Islink
            | StdFn::Close => None,
        }
    }

    /// The one parameter a call may pass by keyword, as CPython's
    /// signature names it: `os.listdir(path='.')`. The others take
    /// positional arguments only (grammar.md, omissions).
    pub fn keyword(self) -> Option<&'static str> {
        match self {
            StdFn::Listdir => Some("path"),
            _ => None,
        }
    }

    pub fn from_name(name: &str) -> Option<StdFn> {
        StdFn::ALL.into_iter().find(|f| f.name() == name)
    }
}

/// The module named `name`.
pub fn module(name: &str) -> Option<&'static ModuleDef> {
    MODULES.iter().find(|m| m.name == name)
}

impl ModuleDef {
    /// Every dotted name a program may write, sorted, for messages.
    pub fn names(&self) -> Vec<&'static str> {
        let vars = self.vars.iter().map(|(n, _)| *n);
        let mut names: Vec<&str> = vars.chain(self.funcs.iter().map(|f| f.name())).collect();
        names.sort_unstable();
        names
    }

    /// Whether `path` is a name a program may write.
    pub fn exposes(&self, path: &str) -> bool {
        self.names().contains(&path)
    }

    /// Whether some exposed name continues `path` (`os.path` for
    /// `os.path.join`).
    pub fn continues(&self, path: &str) -> bool {
        let prefix = format!("{path}.");
        self.names().iter().any(|n| n.starts_with(&prefix))
    }
}

/// The importable module names, sorted, for messages.
pub fn module_names() -> String {
    let mut names: Vec<&str> = MODULES.iter().map(|m| m.name).collect();
    names.sort_unstable();
    names.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_initializer_is_unwritable_and_found_by_name() {
        for f in StdFn::ALL {
            assert_eq!(StdFn::from_name(f.name()), Some(f));
        }
        for m in MODULES {
            for (var, init) in m.vars {
                assert!(var.starts_with(&format!("{}.", m.name)), "{var}");
                assert!(init.name().contains("__"), "{}", init.name());
                assert!(!m.exposes(init.name()));
            }
        }
    }

    #[test]
    fn functions_belong_to_their_module() {
        for m in MODULES {
            for f in m.funcs {
                assert!(f.name().starts_with(&format!("{}.", m.name)), "{f:?}");
                assert!(m.exposes(f.name()));
            }
        }
        let Some(os) = module("os") else {
            panic!("os is importable")
        };
        assert!(os.continues("os.path"));
        assert!(!os.exposes("os.path"));
        assert!(os.exposes("os.path.getsize"));
    }

    #[test]
    fn file_methods_are_unwritable_names() {
        for attr in ["read", "close"] {
            let Some(f) = StdFn::method(attr) else {
                panic!("{attr} is a file method")
            };
            assert!(f.name().ends_with(&format!(".{attr}")));
            assert!(MODULES.iter().all(|m| !m.exposes(f.name())));
        }
        assert_eq!(StdFn::method("write"), None);
        assert_eq!(StdFn::from_name("open"), Some(StdFn::Open));
    }

    #[test]
    fn sys_exposes_argv_only() {
        let sys = module("sys").map(ModuleDef::names);
        assert_eq!(sys, Some(vec!["sys.argv"]));
        assert!(module("os.path").is_none());
        let Some(sys) = module("sys") else {
            panic!("sys is importable")
        };
        assert!(!sys.continues("sys.argv"));
        assert!(!sys.exposes("sys.path"));
    }
}
