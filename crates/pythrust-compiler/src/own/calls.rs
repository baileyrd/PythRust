//! Calls that pass one list to two parameters of a tempered function
//! (A5, ownership.md "Decided" 2).
//!
//! A tempered signature is a contract: a `mut` or `owned` parameter is
//! the only way the call reaches its list. A call that may pass that
//! list, or one inside it, through another parameter too is an error
//! about the call, in loose code as well; the fix-it is a copy. Loose
//! callees keep Python aliasing, even when they write their modes.
//!
//! "May pass" is [`super::SharedArgs`], flow-sensitive, and errs toward
//! the error: it cannot tell two elements of one list apart, so
//! `f(rows[0], rows[1])` is rejected, and module variables a function
//! reads count as one list, since the module's code may have stored one
//! in another.

use std::collections::HashMap;

use crate::ast::{FuncDef, Mode as Temper, Module, StmtKind};
use crate::diag::{DiagKind, Diagnostic, Result};

use super::modes::written;
use super::{Analysis, Mode, Precision, SharedArgs};

/// Rejects the first call in source order whose shared arguments meet a
/// tempered callee's `mut` or `owned` parameter.
pub(super) fn check(module: &Module, analysis: &Analysis) -> Result<()> {
    if analysis.precision != Precision::Precise {
        return Ok(());
    }
    let defs: HashMap<&str, &FuncDef> = module
        .body
        .iter()
        .filter_map(|s| match &s.kind {
            StmtKind::FuncDef(f) => Some((f.name.name.as_str(), f)),
            _ => None,
        })
        .collect();
    let mut shared: Vec<&SharedArgs> = analysis.shared.iter().collect();
    shared.sort_by_key(|s| (s.span.start, s.params));
    for s in shared {
        let Some(def) = defs.get(s.callee.as_str()) else {
            continue;
        };
        call(def, s)?;
    }
    Ok(())
}

/// Rejects `shared` if it reaches a parameter `def` may change or keep.
fn call(def: &FuncDef, shared: &SharedArgs) -> Result<()> {
    if def.mode != Temper::Tempered {
        return Ok(());
    }
    let (i, j) = shared.params;
    let (Some(a), Some(b)) = (def.params.get(i), def.params.get(j)) else {
        return Ok(());
    };
    // The stronger parameter is the one the message is about.
    let (strong, other) = match written(a.mode) >= written(b.mode) {
        true => (a, b),
        false => (b, a),
    };
    let (verb, how) = match written(strong.mode) {
        Mode::Read => return Ok(()),
        Mode::Mut => ("changes", "in place"),
        Mode::Owned => ("keeps", "after it returns"),
    };
    let (callee, s, o) = (&def.name.name, &strong.name.name, &other.name.name);
    let message = format!(
        "tempered `{callee}` {verb} `{s}` {how}, and this call may also pass \
         that list as `{o}`; pass a copy (`list(...)`)"
    );
    Err(Diagnostic {
        span: shared.span,
        message,
        kind: DiagKind::Type,
    })
}
