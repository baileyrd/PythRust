//! Written parameter modes (grammar.md D2) against inferred ones.
//!
//! A written mode must cover what the body does: `mut` covers changing
//! the list in place, `owned` covers that and keeping it after the call.
//! Writing more than the body needs is allowed, as over-declaring
//! `raises` is: it reserves the right to change the body later. A
//! tempered function's unwritten mode is `read`; a loose function's is
//! inferred, so only the modes it writes are checked.
//!
//! Diagnostics come only from precise summaries: a program that does
//! not lower yet has its modes checked once it does. Returning a
//! parameter is not a mode (it is the summary's `result_aliases`);
//! tempered code is barred from it by D3, checked with the body
//! (`alias`).

use crate::ast::{FuncDef, Mode as Temper, Module, ParamMode, StmtKind};
use crate::diag::{DiagKind, Diagnostic, Result};

use super::{Analysis, Mode, Precision, Summary};

/// Checks every function's written modes against `analysis`, after a
/// tempered body's aliasing.
pub(super) fn check(module: &Module, analysis: &Analysis) -> Result<()> {
    if analysis.precision != Precision::Precise {
        return Ok(());
    }
    for s in &module.body {
        let StmtKind::FuncDef(def) = &s.kind else {
            continue;
        };
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

/// Rejects the first parameter of `def` whose written mode its body outgrows.
fn func(def: &FuncDef, summary: &Summary) -> Result<()> {
    for (p, inferred) in def.params.iter().zip(&summary.params) {
        if def.mode == Temper::Loose && p.mode == ParamMode::Read {
            continue;
        }
        let written = written(p.mode);
        if inferred.mode <= written {
            continue;
        }
        let name = &p.name.name;
        let prefix = match def.mode {
            Temper::Tempered => "tempered ",
            Temper::Loose => "",
        };
        // `inferred` outgrows a written mode, so it is never `Read`.
        let (verb, how, word) = match inferred.mode {
            Mode::Owned => ("keeps", "after it returns", "owned"),
            Mode::Mut | Mode::Read => ("changes", "in place", "mut"),
        };
        let message = format!(
            "{prefix}`{}` {verb} `{name}` {how}; write `{word} {name}`",
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

pub(super) fn written(mode: ParamMode) -> Mode {
    match mode {
        ParamMode::Read => Mode::Read,
        ParamMode::Mut => Mode::Mut,
        ParamMode::Owned => Mode::Owned,
    }
}
