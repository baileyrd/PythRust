//! The standard library (M6a): the modules a program may `import` and
//! the names each exposes. A Python-compatible surface over the Rust
//! runtime (`docs/roadmap.md#phase-5-m6-plan`): each name behaves as in
//! CPython or is on grammar.md's deviations list.
//!
//! Stages see no modules. The parser resolves `module.attr` after
//! `import module` to one name with a dot in it (`sys.argv`), which no
//! user name can be, so a module variable is an ordinary global and a
//! module function a builtin. `import m` assigns each of `m`'s variables
//! from its initializer, a call to a [`StdFn`] only the parser can write.

/// A module and the dotted names it exposes.
pub struct ModuleDef {
    pub name: &'static str,
    /// Variables, each with the function that makes its value.
    pub vars: &'static [(&'static str, StdFn)],
}

/// The importable modules.
pub const MODULES: &[ModuleDef] = &[ModuleDef {
    name: "sys",
    vars: &[("sys.argv", StdFn::Argv)],
}];

/// A runtime function behind a module name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StdFn {
    /// `sys.argv`'s value: a new `list[str]` of the program's arguments.
    Argv,
}

impl StdFn {
    pub const ALL: [StdFn; 1] = [StdFn::Argv];

    /// The builtin name stages call it by. An initializer's name has a
    /// `__` part, so no program can write it.
    pub fn name(self) -> &'static str {
        match self {
            StdFn::Argv => "sys.__argv__",
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
        let mut names: Vec<&str> = self.vars.iter().map(|(n, _)| *n).collect();
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
