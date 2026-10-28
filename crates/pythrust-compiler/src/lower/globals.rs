//! Module-level storage functions read: module variables they refer to,
//! and parameter defaults.
//!
//! A module variable some function reads stays a local of the module's
//! code, and every assignment to it is mirrored into a global, with a
//! `Bool` global saying it has been bound. Functions cannot assign
//! module variables (Python would need `global`), so they only read the
//! mirror. A default is evaluated when its `def` runs, as in Python, and
//! kept in a global of its own.

use std::collections::{BTreeSet, HashMap};

use crate::ast::{FuncDef, Module, Stmt};
use crate::diag::Result;
use crate::infer::scope;
use crate::ir::{CmpOp, GlobalId, Inst, Operand, Rvalue, Term, Ty};
use crate::span::Span;

use super::vars::runtime;
use super::{unsupported, Cx, FnBuilder};

/// A mirrored module variable.
#[derive(Debug, Clone, Copy)]
pub(super) struct Mirror {
    value: GlobalId,
    bound: GlobalId,
}

/// Program-wide globals.
#[derive(Debug, Default)]
pub(super) struct Globals {
    /// Module variables some function refers to.
    shared: BTreeSet<String>,
    mirrors: HashMap<String, Mirror>,
    /// Shared module variables an assignment that never completes (it
    /// calls a function that never returns) assigns.
    dead: BTreeSet<String>,
    /// Of those, the ones a function read while no completing assignment
    /// had typed them: the read always fails, so none may follow.
    failing: BTreeSet<String>,
    /// By function name and parameter index.
    defaults: HashMap<(String, usize), GlobalId>,
    tys: Vec<Ty>,
}

impl Globals {
    /// The module variables `module`'s functions may read.
    pub(super) fn new(module: &Module, defs: &HashMap<&str, &FuncDef>) -> Self {
        let assigned = scope::assigned(&module.body);
        let shared = defs
            .values()
            .flat_map(|f| scope::references(f))
            .filter(|n| assigned.contains(n))
            .collect();
        Globals {
            shared,
            ..Globals::default()
        }
    }

    pub(super) fn into_vec(self) -> Vec<Ty> {
        self.tys
    }

    fn add(&mut self, ty: Ty) -> GlobalId {
        self.tys.push(ty);
        GlobalId(self.tys.len() - 1)
    }

    /// The mirror of `name`, created on first use with type `ty`.
    fn mirror(&mut self, name: &str, ty: Ty, span: Span) -> Result<Mirror> {
        if let Some(m) = self.mirrors.get(name) {
            if self.tys[m.value.0] != ty {
                let what = format!("module variable `{name}` read with another type");
                return Err(unsupported(span, &what));
            }
            return Ok(*m);
        }
        let m = Mirror {
            value: self.add(ty),
            bound: self.add(Ty::Bool),
        };
        self.mirrors.insert(name.to_string(), m);
        Ok(m)
    }

    /// The reference-counted globals, which the module's end drops.
    pub(super) fn owning(&self) -> Vec<GlobalId> {
        let rc = self.tys.iter().enumerate().filter(|(_, t)| t.is_rc());
        rc.map(|(i, _)| GlobalId(i)).collect()
    }
}

impl Cx<'_> {
    /// Whether `name` is a module variable some function reads.
    pub(super) fn is_shared(&self, name: &str) -> bool {
        self.globals.shared.contains(name)
    }
}

impl FnBuilder<'_, '_> {
    /// After module code assigns `name` the `value` now in its local:
    /// mirrors it if a function may read it.
    pub(super) fn mirror_write(&mut self, name: &str, value: Operand, span: Span) -> Result<()> {
        if self.defined.is_none() || !self.cx.is_shared(name) {
            return Ok(());
        }
        if self.cx.globals.failing.contains(name) {
            let what = format!("assigning `{name}` after a function read it as never assigned");
            return Err(unsupported(span, &what));
        }
        let m = self.cx.globals.mirror(name, self.ty(value), span)?;
        self.emit(Inst::SetGlobal {
            global: m.value,
            value,
        });
        self.emit(Inst::SetGlobal {
            global: m.bound,
            value: Operand::Bool(true),
        });
        Ok(())
    }

    /// After module code unbinds `name` (an `except ... as name` ends):
    /// functions reading its mirror find it unbound.
    pub(super) fn mirror_unbind(&mut self, name: &str) {
        if self.defined.is_none() {
            return;
        }
        let Some(m) = self.cx.globals.mirrors.get(name).copied() else {
            return;
        };
        self.emit(Inst::SetGlobal {
            global: m.bound,
            value: Operand::Bool(false),
        });
    }

    /// In module code, notes the shared variables `dead` would have
    /// assigned: the statements from one cut short by a call that never
    /// returns to the end of its block, never lowered, at any depth and
    /// in any target.
    pub(super) fn dead_assigns(&mut self, dead: &[Stmt]) {
        if self.defined.is_none() {
            return;
        }
        let globals = &mut self.cx.globals;
        let assigned = scope::assigned(dead);
        let shared = assigned.into_iter().filter(|n| globals.shared.contains(n));
        globals.dead.extend(shared.collect::<Vec<String>>());
    }

    /// A function's read of module variable `name`: `NameError` until
    /// the module has assigned it.
    pub(super) fn read_global(&mut self, name: &str, span: Span) -> Result<Operand> {
        // The module's own type: inference may have narrowed this read.
        let message = format!("NameError: name '{name}' is not defined");
        let ty = match self.cx.globals.mirrors.get(name) {
            Some(m) => self.cx.globals.tys[m.value.0],
            None => match self.ty_at(span) {
                Ok(ty) => ty,
                // Only an assignment that never completes has assigned
                // it, so the read fails, and ends its path.
                Err(_) if self.cx.globals.dead.contains(name) => {
                    self.cx.globals.failing.insert(name.to_string());
                    self.jump_away(Term::Fail(runtime(span, &message)));
                    return Err(self.diverge(span));
                }
                Err(e) => return Err(e),
            },
        };
        let m = self.cx.globals.mirror(name, ty, span)?;
        let bound = self.temp(Ty::Bool, Rvalue::Global(m.bound), span);
        let unbound = Rvalue::Cmp(CmpOp::Eq, bound, Operand::Bool(false));
        self.fail_if(unbound, span, &message);
        let v = self.temp(ty, Rvalue::Global(m.value), span);
        self.narrow(v, span)
    }

    /// Evaluates `f`'s defaults, as its `def` statement does.
    pub(super) fn store_defaults(&mut self, f: &FuncDef) -> Result<()> {
        let with_default = f.params.iter().enumerate();
        for (k, default) in with_default.filter_map(|(k, p)| Some((k, p.default.as_ref()?))) {
            let v = self.expr(default)?;
            let global = self.cx.globals.add(self.ty(v));
            self.cx
                .globals
                .defaults
                .insert((f.name.name.clone(), k), global);
            self.emit(Inst::SetGlobal { global, value: v });
        }
        Ok(())
    }

    /// The default of parameter `k` of `f`, a new temporary.
    pub(super) fn default_arg(&mut self, f: &FuncDef, k: usize, span: Span) -> Result<Operand> {
        let key = (f.name.name.clone(), k);
        let Some(global) = self.cx.globals.defaults.get(&key).copied() else {
            return Err(unsupported(span, "a default its `def` has not stored"));
        };
        let ty = self.cx.globals.tys[global.0];
        Ok(self.temp(ty, Rvalue::Global(global), span))
    }
}
