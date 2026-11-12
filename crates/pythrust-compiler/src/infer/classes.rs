//! User exception classes (step 4-3, D138-D143, D156): each derives
//! from a builtin exception class or one the program defines before it,
//! and is called, raised and caught as one.

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
        let all: Vec<&ExcClassDef> = module
            .body
            .iter()
            .filter_map(|s| match &s.kind {
                StmtKind::ExcClass(c) => Some(c),
                _ => None,
            })
            .collect();
        let mut seen = BTreeSet::new();
        for c in &all {
            self.check_base(c, &seen, &all, defs)?;
            seen.insert(c.name.name.as_str());
        }
        self.classes = Classes::of(module);
        Ok(())
    }

    /// `c`'s base is a builtin exception class nothing shadows or a
    /// class defined before `c`, which `seen` holds (D139, D156); one of
    /// `all` defined later is not defined yet, as CPython's `NameError`.
    fn check_base(
        &self,
        c: &ExcClassDef,
        seen: &BTreeSet<&str>,
        all: &[&ExcClassDef],
        defs: &[&FuncDef],
    ) -> Result<()> {
        let base = c.base.name.as_str();
        if seen.contains(base) {
            return Ok(());
        }
        if all.iter().any(|d| d.name.name == base) {
            let msg = format!("`{}` is not defined yet", unqualified(base));
            return Err(error(c.base.span, msg));
        }
        let def = defs.iter().any(|f| f.name.name == base);
        let shadowed = self.globals.contains_key(base) || def;
        if ExcKind::from_name(base).is_some() && !shadowed {
            return Ok(());
        }
        let known =
            shadowed || self.is_record(base) || builtins::is_builtin(base) || TYPES.contains(&base);
        let msg = match known {
            true => format!(
                "a class may derive only from an exception class; `{}` is not one",
                unqualified(base)
            ),
            false => format!("name `{base}` is not defined"),
        };
        Err(error(c.base.span, msg))
    }
}
