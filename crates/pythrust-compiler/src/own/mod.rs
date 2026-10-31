//! Ownership summaries: how each function uses its parameters.
//!
//! Per function, a [`Mode`] for each parameter and whether the result
//! may alias it (`docs/research/ownership.md`); that document's `owned`
//! is [`Mode::Owned`] or an aliased result. Modes and aliasing track
//! only values that can hold a list: aliasing anything else (strings,
//! tuples of them) is unobservable, so such a parameter is always
//! [`Mode::Read`] there. What the call does with a parameter's strings
//! is [`Param::strs`] ([`strs`]): D2 asks tempered code to write `owned`
//! on a string it keeps or returns.
//!
//! The analysis runs on the IR, so it sees only what lowering covers.
//! [`analyze`] is the coverage gate: a program lowering cannot express
//! gets [`Precision::Conservative`] summaries, never an error, so a
//! lowering gap is never a new language restriction.
//!
//! [`check`] holds the modes a function writes (D2) to these summaries,
//! calls to tempered functions to their `mut` and `owned` parameters
//! (A5), and a tempered body to D3 and D4 ([`alias`]).

mod alias;
mod body;
mod calls;
mod modes;
mod strs;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};

use crate::ast::{Module, StmtKind};
use crate::diag::{Diagnostic, Result};
use crate::infer::{Type, Types};
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
    /// What the call does with its strings.
    pub strs: Strs,
}

/// What a call may do with a parameter's strings: the parameter if it is
/// a `str`, and the strings its tuple fields and optional's value hold,
/// but not past a list. A parameter whose language-level type holds no
/// string outside a list (an int, a list, a tuple of ints) has neither.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct Strs {
    /// Whether one may outlive the call other than in the result: in a
    /// list the caller or the module can see, a global, an exception, or
    /// a callee that keeps it.
    pub kept: bool,
    /// Whether the result may hold one.
    pub returned: bool,
    /// As `kept`, for the elements, at any depth, of the lists it holds
    /// as the caller passed them: no mode, since they are values of
    /// their own, but a caller follows its strings through them.
    pub elements_kept: bool,
    /// As `returned`, for those elements.
    pub elements_returned: bool,
}

/// A function's summary.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Summary {
    /// One per parameter, in order.
    pub params: Vec<Param>,
    /// Whether the result may be, or hold, a list a global holds.
    pub result_global: bool,
}

/// What an argument may reach, seen from its caller: a list of one of
/// the caller's parameters, or of the module's values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Peer {
    Param(usize),
    World,
}

/// One call, with what its arguments may share at that point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallSite {
    /// Indexes into [`Analysis::names`].
    pub caller: usize,
    pub callee: usize,
    /// Argument pairs, first lower, that may pass one list, or lists
    /// inside it.
    pub shared: Vec<(usize, usize)>,
    /// Per argument, the caller's parameters and module values it may
    /// reach.
    pub peers: Vec<BTreeSet<Peer>>,
    pub span: Span,
}

/// What a lowered function's heap effects look like to [`CallSite`]
/// checks: its parameters are analyzed as holding unrelated lists, which
/// holds only if no store through one shows through another.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Heap {
    /// The parameters that can hold a list.
    pub lists: Vec<usize>,
    /// Of those and the module's values, the ones the body may store a
    /// list into, anywhere inside.
    pub stored: BTreeSet<Peer>,
    /// Whether the body may get hold of a list the module's values hold:
    /// if not, a parameter that is one cannot show it anything.
    pub sees_world: bool,
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
    /// By tempered function name, the first aliasing its body may not
    /// do (D3, D4). Empty unless precise.
    pub aliasing: BTreeMap<String, Diagnostic>,
    /// Source name per lowered function (an instance of a generic
    /// function has its own), the module's code included; empty when
    /// conservative.
    pub names: Vec<String>,
    /// Per lowered function, as [`Analysis::names`]; empty when
    /// conservative.
    pub heaps: Vec<Heap>,
    /// Every call of the program; empty when conservative.
    pub calls: Vec<CallSite>,
    /// By function name, each parameter's type, from inference, so
    /// present even when conservative.
    pub params: BTreeMap<String, Vec<Type>>,
    pub precision: Precision,
}

/// Summarizes `module`, typed by `types`, including functions no code
/// calls when their signature is concrete. Never fails: see
/// [`Precision::Conservative`].
pub fn analyze(module: &Module, types: &Types) -> Analysis {
    match lower::lower_every(module, types) {
        Ok(program) => {
            let summaries = infer(&program);
            let calls = program.funcs.iter().enumerate();
            let calls = calls.flat_map(|(i, f)| body::calls(&program, i, f, &summaries));
            Analysis {
                funcs: by_name(&program, &summaries),
                aliasing: alias::tempered(module, &program, &summaries),
                names: program.funcs.iter().map(|f| f.name.clone()).collect(),
                heaps: program
                    .funcs
                    .iter()
                    .map(|f| body::heap(&program, f, &summaries))
                    .collect(),
                calls: calls.collect(),
                params: param_types(module, types),
                precision: Precision::Precise,
            }
        }
        Err(gap) => Analysis {
            funcs: conservative(module),
            aliasing: BTreeMap::new(),
            names: Vec::new(),
            heaps: Vec::new(),
            calls: Vec::new(),
            params: param_types(module, types),
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
            strs: Strs::default(),
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
            strs: Strs {
                kept: true,
                returned: true,
                elements_kept: true,
                elements_returned: true,
            },
        };
        Summary {
            params: vec![param; arity],
            result_global: true,
        }
    }

    /// The weakest summary covering both: per parameter, the stronger
    /// mode, aliased, kept or returned if either is.
    fn join(&mut self, other: &Summary) {
        for (p, q) in self.params.iter_mut().zip(&other.params) {
            p.mode = p.mode.max(q.mode);
            p.result_aliases |= q.result_aliases;
            p.holds_global |= q.holds_global;
            p.strs.kept |= q.strs.kept;
            p.strs.returned |= q.strs.returned;
            p.strs.elements_kept |= q.strs.elements_kept;
            p.strs.elements_returned |= q.strs.elements_returned;
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

/// The parameter types of each function `module` defines.
fn param_types(module: &Module, types: &Types) -> BTreeMap<String, Vec<Type>> {
    let defs = module.body.iter().filter_map(|s| match &s.kind {
        StmtKind::FuncDef(f) => types.func(&f.name.name).map(|t| (&f.name.name, t)),
        _ => None,
    });
    defs.map(|(name, t)| {
        (
            name.clone(),
            t.params.iter().map(|p| p.ty.clone()).collect(),
        )
    })
    .collect()
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

/// Whether a value of type `ty` is, or may hold, a list. A dict or set
/// is a list here: a container that changes in place.
fn holds_list(program: &Program, ty: Ty) -> bool {
    match program.compound(ty) {
        Some(Compound::List(_) | Compound::Dict(..) | Compound::Set(_)) => true,
        Some(Compound::Tuple(fields)) => fields.iter().any(|f| holds_list(program, *f)),
        Some(Compound::Opt(present) | Compound::View(_, present)) => holds_list(program, *present),
        None => false,
    }
}

/// The language-level type an IR type lowers.
fn source(program: &Program, ty: Ty) -> Type {
    match ty {
        Ty::Int => Type::Int,
        Ty::Float => Type::Float,
        Ty::Bool => Type::Bool,
        Ty::None => Type::None,
        Ty::Str => Type::Str,
        Ty::Range => Type::Range,
        Ty::Exc => Type::Exception,
        Ty::List(_) | Ty::Tuple(_) | Ty::Opt(_) | Ty::Dict(_) | Ty::Set(_) | Ty::View(_) => {
            compound_source(program, ty)
        }
    }
}

fn compound_source(program: &Program, ty: Ty) -> Type {
    let part = |t: &Ty| Box::new(source(program, *t));
    match program.compound(ty) {
        Some(Compound::List(elem)) => Type::List(part(elem)),
        Some(Compound::Tuple(fields)) => {
            Type::Tuple(fields.iter().map(|f| source(program, *f)).collect())
        }
        Some(Compound::Opt(present)) => Type::Optional(part(present)),
        Some(Compound::Dict(k, v)) => Type::Dict(part(k), part(v)),
        Some(Compound::Set(t)) => Type::Set(part(t)),
        Some(Compound::View(view, item)) => Type::View(view_source(*view), part(item)),
        None => Type::None,
    }
}

fn view_source(view: crate::ir::View) -> crate::infer::View {
    match view {
        crate::ir::View::Keys => crate::infer::View::Keys,
        crate::ir::View::Values => crate::infer::View::Values,
        crate::ir::View::Items => crate::infer::View::Items,
    }
}
