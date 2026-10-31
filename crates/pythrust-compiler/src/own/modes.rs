//! Written parameter modes (grammar.md D2) against inferred ones.
//!
//! A written mode must cover what the body does: `mut` covers changing
//! the list in place, `owned` covers that and keeping it after the call.
//! Writing more than the body needs is allowed, as over-declaring
//! `raises` is: it reserves the right to change the body later. A
//! tempered function's unwritten mode is `read`; a loose function's is
//! inferred, so only the modes it writes are checked.
//!
//! A string parameter, or a tuple or optional holding one, is `owned`
//! when the body keeps or returns one of its strings ([`super::Strs`]);
//! it is never `mut`. Returning a list parameter is not a mode (it is
//! the summary's `result_aliases`); tempered code is barred from it by
//! D3, checked with the body (`alias`).
//!
//! What a mode may be written on depends only on the parameter's type:
//! `owned` on a heap type ([`Type::holds_heap`]), `mut` on one whose
//! operations reach a list ([`Type::reaches_mutable`]), neither on a
//! value type, which is copied.
//!
//! Diagnostics about the body come only from precise summaries: a
//! program that does not lower yet has its modes checked once it does.

use crate::ast::{FuncDef, Mode as Temper, Module, ParamMode, StmtKind};
use crate::diag::{DiagKind, Diagnostic, Result};
use crate::infer::Type;

use super::{Analysis, Mode, Param, Precision, Summary};

/// Checks every function's written modes against `analysis`, after a
/// tempered body's aliasing.
pub(super) fn check(module: &Module, analysis: &Analysis) -> Result<()> {
    for def in defs(module) {
        if let Some(types) = analysis.params.get(&def.name.name) {
            fits(def, types)?;
        }
    }
    if analysis.precision != Precision::Precise {
        return Ok(());
    }
    for def in defs(module) {
        if let Some(d) = analysis.aliasing.get(&def.name.name) {
            return Err(d.clone());
        }
        // A generic function no code calls has no summary to check.
        if let Some(summary) = analysis.funcs.get(&def.name.name) {
            func(def, summary)?;
        }
    }
    Ok(())
}

/// The functions `module` defines.
fn defs(module: &Module) -> impl Iterator<Item = &FuncDef> {
    module.body.iter().filter_map(|s| match &s.kind {
        StmtKind::FuncDef(def) => Some(def),
        _ => None,
    })
}

/// Rejects the first parameter of `def` whose written mode its type
/// cannot have. A generic type may have any.
fn fits(def: &FuncDef, types: &[Type]) -> Result<()> {
    for (p, ty) in def.params.iter().zip(types) {
        let (word, why) = match p.mode {
            ParamMode::Read => continue,
            ParamMode::Owned if ty.holds_heap() == Some(false) => ("owned", "are copied"),
            ParamMode::Mut if ty.reaches_mutable() != Some(false) => continue,
            ParamMode::Mut if ty.holds_heap() == Some(false) => ("mut", "are copied"),
            ParamMode::Mut => ("mut", "cannot change in place"),
            ParamMode::Owned => continue,
        };
        let message = format!(
            "`{}`: {} {why}; remove `{word}` from `{}`",
            def.name.name,
            plural(ty),
            p.name.name
        );
        return Err(Diagnostic {
            span: p.name.span,
            message,
            kind: DiagKind::Type,
        });
    }
    Ok(())
}

/// What a message calls values of type `ty`.
fn plural(ty: &Type) -> String {
    let word = match ty {
        Type::Int => "ints",
        Type::Float => "floats",
        Type::Bool => "bools",
        Type::None => "None values",
        Type::Range => "ranges",
        Type::Str => "strings",
        Type::Exception => "exceptions",
        Type::Bytes => "bytes values",
        Type::File => "files",
        _ => return format!("`{ty}` values"),
    };
    word.to_string()
}

/// Rejects the first parameter of `def` whose written mode its body outgrows.
fn func(def: &FuncDef, summary: &Summary) -> Result<()> {
    for (p, inferred) in def.params.iter().zip(&summary.params) {
        if def.mode == Temper::Loose && p.mode == ParamMode::Read {
            continue;
        }
        let (needed, verb, how) = needed(inferred);
        if needed <= written(p.mode) {
            continue;
        }
        let name = &p.name.name;
        let prefix = match def.mode {
            Temper::Tempered => "tempered ",
            Temper::Loose => "",
        };
        let word = match needed {
            Mode::Owned => "owned",
            Mode::Mut | Mode::Read => "mut",
        };
        let message = format!(
            "{prefix}`{}` {verb} `{name}`{how}; write `{word} {name}`",
            def.name.name
        );
        return Err(Diagnostic {
            span: p.name.span,
            message,
            kind: DiagKind::Type,
        });
    }
    Ok(())
}

/// The mode a parameter's body needs, and how it says why: its lists'
/// mode, or `owned` for a string it keeps or returns.
fn needed(p: &Param) -> (Mode, &'static str, &'static str) {
    let keeps = ("keeps", " after it returns");
    match p.mode {
        Mode::Owned => (Mode::Owned, keeps.0, keeps.1),
        _ if p.strs.kept => (Mode::Owned, keeps.0, keeps.1),
        _ if p.strs.returned => (Mode::Owned, "returns", ""),
        Mode::Mut => (Mode::Mut, "changes", " in place"),
        Mode::Read => (Mode::Read, "reads", ""),
    }
}

pub(super) fn written(mode: ParamMode) -> Mode {
    match mode {
        ParamMode::Read => Mode::Read,
        ParamMode::Mut => Mode::Mut,
        ParamMode::Owned => Mode::Owned,
    }
}
