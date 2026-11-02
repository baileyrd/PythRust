//! Calls that pass one list to two parameters of a tempered function
//! (A5, ownership.md "Decided" 2).
//!
//! A tempered signature is a contract: a `mut` or `owned` parameter is
//! the only way the call reaches its list. A call that may pass that
//! list, or one inside it, through another parameter too is an error
//! about the call, in loose code as well; the fix-it is a copy. Loose
//! callees keep Python aliasing, even when they write their modes,
//! except where they pass their parameters on to such a contract: then
//! the contract becomes theirs, as a *need* that two of their
//! parameters, or one and the module's values, share no list. Needs
//! flow up the call graph to a fixpoint, so `forward(a, a)` fails when
//! `forward(xs, ys)` calls `push(xs, ys)`. A body is analyzed as if its
//! parameters held unrelated lists; where it stores through one, a
//! function whose body is checked for a contract (it has a need, or
//! calls a function that does) also needs that one unshared
//! ([`entry`]), so a caller cannot make the store show through another
//! unseen.
//!
//! "May share" is [`super::CallSite`], flow-sensitive, and errs toward
//! the error: it cannot tell two elements of one list apart, so
//! `f(rows[0], rows[1])` is rejected, and module variables a function
//! reads count as one list, since the module's code may have stored one
//! in another.

use std::collections::btree_map::Entry;
use std::collections::{BTreeMap, HashMap};

use crate::ast::{FuncDef, Mode as Temper, Module, StmtKind};
use crate::diag::{DiagKind, Diagnostic, Result};

use super::modes::written;
use super::{Analysis, CallSite, Heap, Mode, Peer, Precision};

/// A parameter, and the parameter or module values it may share no list
/// with; a [`Peer::Param`] is always the higher index.
type Key = (usize, Peer);

/// Why a function needs a [`Key`].
#[derive(Debug, Clone)]
struct Need {
    /// The tempered contract it comes from: "tempered `push` changes
    /// `xs` in place".
    root: String,
    /// For the contract itself, the other parameter's name.
    other: Option<String>,
}

type Needs = BTreeMap<Key, Need>;

/// Rejects the first call in source order that may share a list its
/// callee needs unshared.
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
    let needs = settle(analysis, &defs);
    let mut sites: Vec<&CallSite> = analysis.calls.iter().collect();
    sites.sort_by_key(|s| s.span.start);
    for site in sites {
        if analysis.opaque.contains(&analysis.names[site.caller]) {
            continue;
        }
        let Some(def) = defs.get(analysis.names[site.callee].as_str()) else {
            continue;
        };
        let broken = needs[site.callee].iter().find(|(k, _)| breaks(site, **k));
        if let Some((key, need)) = broken {
            return Err(diagnostic(site, def, *key, need));
        }
    }
    Ok(())
}

/// Every function's needs: tempered contracts, then what calls pass on,
/// to a fixpoint. Needs only grow, over finitely many keys, so it ends.
fn settle(analysis: &Analysis, defs: &HashMap<&str, &FuncDef>) -> Vec<Needs> {
    let mut needs: Vec<Needs> = analysis
        .names
        .iter()
        .map(|n| {
            defs.get(n.as_str())
                .map_or_else(Needs::new, |d| contract(d))
        })
        .collect();
    loop {
        let mut changed = false;
        let roots = relied_on(analysis, &needs);
        for ((needs, heap), root) in needs.iter_mut().zip(&analysis.heaps).zip(roots) {
            changed |= root.is_some_and(|root| entry(needs, heap, &root));
        }
        for site in &analysis.calls {
            for (key, need) in passed_on(site, &needs[site.callee]) {
                if let Entry::Vacant(slot) = needs[site.caller].entry(key) {
                    slot.insert(need);
                    changed = true;
                }
            }
        }
        if !changed {
            return needs;
        }
    }
}

/// Per function, a contract its body's analysis is relied on for, if
/// any: one of its own needs, or one of a callee it calls.
fn relied_on(analysis: &Analysis, needs: &[Needs]) -> Vec<Option<String>> {
    let first = |n: &Needs| n.values().next().map(|n| n.root.clone());
    let mut roots: Vec<Option<String>> = needs.iter().map(first).collect();
    for site in &analysis.calls {
        if roots[site.caller].is_none() {
            roots[site.caller] = first(&needs[site.callee]);
        }
    }
    roots
}

/// A body checked for a contract (`root`) is analyzed as if its
/// parameters held unrelated lists, which a caller breaks by passing one
/// list as two when the body stores through one of them: what it stores,
/// a parameter's list or one it made, shows through the other unseen.
/// So it also needs each parameter it stores into, and the module's
/// values if it stores there, unshared with every other; the module's
/// values count only if the body can see them. Whether any key was
/// added.
fn entry(needs: &mut Needs, heap: &Heap, root: &str) -> bool {
    let peers = heap.lists.iter().map(|i| Peer::Param(*i));
    let world = heap.sees_world.then_some(Peer::World);
    let peers: Vec<Peer> = peers.chain(world).collect();
    let mut changed = false;
    for (n, p) in peers.iter().enumerate() {
        for q in &peers[n + 1..] {
            let touched = heap.stored.contains(p) || heap.stored.contains(q);
            let Some(key) = key(*p, *q).filter(|_| touched) else {
                continue;
            };
            if let Entry::Vacant(slot) = needs.entry(key) {
                slot.insert(Need {
                    root: root.to_string(),
                    other: None,
                });
                changed = true;
            }
        }
    }
    changed
}

/// A tempered function's own needs: no list shared between a `mut` or
/// `owned` parameter and another.
fn contract(def: &FuncDef) -> Needs {
    let mut needs = Needs::new();
    if def.mode != Temper::Tempered {
        return needs;
    }
    for (i, a) in def.params.iter().enumerate() {
        for (j, b) in def.params.iter().enumerate().skip(i + 1) {
            // The stronger parameter is the one the message is about.
            let (strong, other) = match written(a.mode) >= written(b.mode) {
                true => (a, b),
                false => (b, a),
            };
            let (verb, how) = match written(strong.mode) {
                Mode::Read => continue,
                Mode::Mut => ("changes", "in place"),
                Mode::Owned => ("keeps", "after it returns"),
            };
            let root = format!(
                "tempered `{}` {verb} `{}` {how}",
                def.name.name, strong.name.name
            );
            let other = Some(other.name.name.clone());
            needs.insert((i, Peer::Param(j)), Need { root, other });
        }
    }
    needs
}

/// What the caller of `site` needs because its callee needs `needs`:
/// the caller's parameters and module values the arguments reach.
fn passed_on(site: &CallSite, needs: &Needs) -> Vec<(Key, Need)> {
    let mut out = Vec::new();
    for ((k, peer), need) in needs {
        let mine = &site.peers[*k];
        let theirs: Vec<Peer> = match peer {
            Peer::Param(l) => site.peers[*l].iter().copied().collect(),
            Peer::World => vec![Peer::World],
        };
        for (p, q) in mine
            .iter()
            .flat_map(|p| theirs.iter().map(move |q| (*p, *q)))
        {
            let Some(key) = key(p, q) else {
                continue;
            };
            let root = need.root.clone();
            out.push((key, Need { root, other: None }));
        }
    }
    out
}

/// The need that `p` and `q` share no list, unless that is not about
/// the caller's parameters: the same one, or module values both, is
/// checked at the call itself.
fn key(p: Peer, q: Peer) -> Option<Key> {
    match (p, q) {
        (Peer::Param(a), Peer::Param(b)) if a != b => Some((a.min(b), Peer::Param(a.max(b)))),
        (Peer::Param(a), Peer::World) | (Peer::World, Peer::Param(a)) => Some((a, Peer::World)),
        _ => None,
    }
}

/// Whether `site` may share what its callee needs unshared under `key`.
fn breaks(site: &CallSite, (k, peer): Key) -> bool {
    match peer {
        Peer::Param(l) => site.shared.contains(&(k, l)),
        Peer::World => site.peers[k].contains(&Peer::World),
    }
}

fn diagnostic(site: &CallSite, def: &FuncDef, (k, peer): Key, need: &Need) -> Diagnostic {
    let root = &need.root;
    let name = |i: usize| def.params.get(i).map_or("?", |p| p.name.name.as_str());
    let callee = &def.name.name;
    let fix = "pass a copy (`list(...)`)";
    let message = match (peer, &need.other) {
        (Peer::Param(_), Some(other)) => {
            format!("{root}, and this call may also pass that list as `{other}`; {fix}")
        }
        (Peer::Param(l), None) => format!(
            "this call may pass one list as `{}` and `{}` of `{callee}`, and from there {root}; {fix}",
            name(k),
            name(l)
        ),
        (Peer::World, _) => format!(
            "this call may pass a list a module variable holds as `{}` of `{callee}`, and from there {root}; {fix}",
            name(k)
        ),
    };
    Diagnostic {
        span: site.span,
        message,
        kind: DiagKind::Type,
    }
}
