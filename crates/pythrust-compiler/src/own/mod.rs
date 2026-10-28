//! Ownership summaries: how each function uses its parameters.
//!
//! Per function, a [`Mode`] for each parameter and whether the result
//! may alias it (`docs/research/ownership.md`); that document's `owned`
//! is [`Mode::Owned`] or an aliased result. Only values that can hold
//! a list are tracked: aliasing anything else (ints, strings, tuples of
//! them) is unobservable, so such a parameter is always [`Mode::Read`].
//!
//! The analysis runs on the IR, so it sees only what lowering covers.
//! [`analyze`] is the coverage gate: a program lowering cannot express
//! gets [`Precision::Conservative`] summaries, never an error, so a
//! lowering gap is never a new language restriction.
//!
//! [`check`] holds the modes a function writes (D2) to these summaries,
//! and calls to tempered functions to their `mut` and `owned`
//! parameters (A5).

mod body;
mod calls;
mod modes;

#[cfg(test)]
mod tests;

use std::collections::BTreeMap;

use crate::ast::{Module, StmtKind};
use crate::diag::{Diagnostic, Result};
use crate::infer::Types;
use crate::ir::{Compound, Program, Ty};
use crate::lower;
use crate::span::Span;

/// How a function uses a parameter, weakest first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Mode {
    /// Only reads it.
    Read,
    /// Changes it, or a list inside it, in place.
    Mut,
    /// Keeps it, or a list inside it, past the call other than as the
    /// result: stores it where the caller or the module can see it, or
    /// passes it to a callee that does. Returning it is
    /// [`Param::result_aliases`], which the caller accounts for.
    Owned,
}

/// One parameter's summary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Param {
    pub mode: Mode,
    /// Whether the result may be, or hold, the parameter or a list
    /// inside it, including one the body stored there.
    pub result_aliases: bool,
    /// Whether the call may leave a list a global holds inside it.
    pub holds_global: bool,
}

/// A function's summary.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Summary {
    /// One per parameter, in order.
    pub params: Vec<Param>,
    /// Whether the result may be, or hold, a list a global holds.
    pub result_global: bool,
}

/// A call that may pass one list, or lists inside it, to two parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedArgs {
    /// The callee's source name.
    pub callee: String,
    /// The two parameters' indexes, first lower.
    pub params: (usize, usize),
    /// The call.
    pub span: Span,
}

/// How far [`Analysis::funcs`] can be trusted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Precision {
    /// Inferred from every lowered body.
    Precise,
    /// The program does not lower, for this reason: every parameter is
    /// [`Mode::Owned`], may alias the result and may come to hold a
    /// global's list, and the result may alias the module's values. Sound, but nothing may
    /// be diagnosed from it.
    Conservative(Diagnostic),
}

/// Summaries for a whole program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Analysis {
    /// By function name. A generic function's instances are joined; a
    /// generic function no code calls has none when precise.
    pub funcs: BTreeMap<String, Summary>,
    /// Every call of the program that may share arguments, in no
    /// particular order; empty when conservative.
    pub shared: Vec<SharedArgs>,
    pub precision: Precision,
}

/// Summarizes `module`, typed by `types`, including functions no code
/// calls when their signature is concrete. Never fails: see
/// [`Precision::Conservative`].
pub fn analyze(module: &Module, types: &Types) -> Analysis {
    match lower::lower_every(module, types) {
        Ok(program) => {
            let summaries = infer(&program);
            let shared = program.funcs.iter();
            let shared = shared.flat_map(|f| body::shared_args(&program, f, &summaries));
            Analysis {
                funcs: by_name(&program, &summaries),
                shared: shared.collect(),
                precision: Precision::Precise,
            }
        }
        Err(gap) => Analysis {
            funcs: conservative(module),
            shared: Vec::new(),
            precision: Precision::Conservative(gap),
        },
    }
}

/// Holds `module` to `analysis`: written modes (D2), then calls to
/// tempered functions (A5). Only precise analyses are checked.
pub fn check(module: &Module, analysis: &Analysis) -> Result<()> {
    modes::check(module, analysis)?;
    calls::check(module, analysis)
}

/// One summary per function of `program`, indexed by `FuncId`: the
/// least fixpoint over the call graph, so mutual recursion settles.
/// Every round only grows summaries, which form a finite lattice, so it
/// ends.
pub fn infer(program: &Program) -> Vec<Summary> {
    let mut summaries: Vec<Summary> = program
        .funcs
        .iter()
        .map(|f| Summary::read_only(f.params.len()))
        .collect();
    loop {
        let mut changed = false;
        for (i, func) in program.funcs.iter().enumerate() {
            let next = body::summarize(program, func, &summaries);
            if next != summaries[i] {
                summaries[i] = next;
                changed = true;
            }
        }
        if !changed {
            return summaries;
        }
    }
}

impl Summary {
    /// Every parameter only read, none aliased: the fixpoint's start.
    fn read_only(arity: usize) -> Summary {
        let param = Param {
            mode: Mode::Read,
            result_aliases: false,
            holds_global: false,
        };
        Summary {
            params: vec![param; arity],
            result_global: false,
        }
    }

    /// What lowering cannot see into may do anything.
    fn conservative(arity: usize) -> Summary {
        let param = Param {
            mode: Mode::Owned,
            result_aliases: true,
            holds_global: true,
        };
        Summary {
            params: vec![param; arity],
            result_global: true,
        }
    }

    /// The weakest summary covering both: per parameter, the stronger
    /// mode, aliased if either is.
    fn join(&mut self, other: &Summary) {
        for (p, q) in self.params.iter_mut().zip(&other.params) {
            p.mode = p.mode.max(q.mode);
            p.result_aliases |= q.result_aliases;
            p.holds_global |= q.holds_global;
        }
        self.result_global |= other.result_global;
    }
}

/// Instance summaries joined by source function, leaving out the module.
fn by_name(program: &Program, summaries: &[Summary]) -> BTreeMap<String, Summary> {
    let mut funcs: BTreeMap<String, Summary> = BTreeMap::new();
    for (i, (func, summary)) in program.funcs.iter().zip(summaries).enumerate() {
        if i == program.main.0 {
            continue;
        }
        funcs
            .entry(func.name.clone())
            .and_modify(|joined| joined.join(summary))
            .or_insert_with(|| summary.clone());
    }
    funcs
}

/// The conservative summary of every function `module` defines.
fn conservative(module: &Module) -> BTreeMap<String, Summary> {
    let defs = module.body.iter().filter_map(|s| match &s.kind {
        StmtKind::FuncDef(f) => Some(f),
        _ => None,
    });
    defs.map(|f| (f.name.name.clone(), Summary::conservative(f.params.len())))
        .collect()
}

/// Whether a value of type `ty` is, or may hold, a list.
fn holds_list(program: &Program, ty: Ty) -> bool {
    match program.compound(ty) {
        Some(Compound::List(_)) => true,
        Some(Compound::Tuple(fields)) => fields.iter().any(|f| holds_list(program, *f)),
        Some(Compound::Opt(present)) => holds_list(program, *present),
        None => false,
    }
}
