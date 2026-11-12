//! User exception classes (step 4-3, D138-D143): each derives from a
//! builtin exception class, and is called, raised and caught as one.

use std::collections::BTreeSet;

use crate::ast::{unqualified, ExcClassDef, FuncDef, Module, StmtKind};
use crate::diag::Result;
use crate::exc::{Classes, ExcKind};

use super::{builtins, error, Checker};

/// The builtin types a base may name, which are not exception classes.
const TYPES: [&str; 9] = [
    "int", "float", "str", "bool", "list", "dict", "set", "tuple", "object",
];

impl Checker {
    /// Checks every class's base, in the order the classes run, then
    /// declares them. Runs after the globals and records are declared,
    /// so a base a variable shadows is told apart.
    pub(super) fn declare_classes(&mut self, module: &Module, defs: &[&FuncDef]) -> Result<()> {
        let mut seen = BTreeSet::new();
        for s in &module.body {
            let StmtKind::ExcClass(c) = &s.kind else {
                continue;
            };
            self.check_base(c, &seen, defs)?;
            seen.insert(c.name.name.as_str());
        }
        self.classes = Classes::of(module);
        Ok(())
    }

    /// `c`'s base is a builtin exception class nothing shadows (D138,
    /// D139); `seen` holds the classes defined before `c`.
    fn check_base(&self, c: &ExcClassDef, seen: &BTreeSet<&str>, defs: &[&FuncDef]) -> Result<()> {
        let base = c.base.name.as_str();
        let def = defs.iter().any(|f| f.name.name == base);
        let shadowed = self.globals.contains_key(base) || def;
        if ExcKind::from_name(base).is_some() && !shadowed {
            return Ok(());
        }
        if seen.contains(base) {
            let msg = "deriving from a user exception class is not supported yet; \
                       derive from a builtin exception class such as `Exception`";
            return Err(error(c.base.span, msg.to_string()));
        }
        let known =
            shadowed || self.is_record(base) || builtins::is_builtin(base) || TYPES.contains(&base);
        let msg = match known {
            true => format!(
                "a class may derive only from a builtin exception class; `{}` is not one",
                unqualified(base)
            ),
            false => format!("name `{base}` is not defined"),
        };
        Err(error(c.base.span, msg))
    }
}
