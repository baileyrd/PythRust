//! Refcount elision over the IR, between [`lower`](crate::lower) and
//! [`codegen`](crate::codegen) (phase 4, A7).
//!
//! Lowering spells every reference out conservatively (`docs/native.md`):
//! a call takes each reference argument owned, so the caller copies a
//! variable it still uses with a `Retain` and the callee drops it at its
//! exit; every temporary is released where its statement ends, moved out
//! or not. [`elide`] removes what it proves unneeded, in three passes,
//! each keeping the IR's ownership invariant:
//!
//! 1. `borrow`: a parameter its function does not `retains` (never
//!    moves, reassigns or releases before its exit, a per-function fact)
//!    is *borrowed* ([`Func::borrowed`]): the callee no longer drops it,
//!    and every call releases the argument itself after the call instead
//!    of moving it in.
//! 2. `copies`: a copy `a = x; Retain(a)` needs no reference of its own
//!    when, on every path, it is released and only read while `x` still
//!    holds one (it borrows `x`'s: both operations go, the `for`
//!    snapshot of a variable among them), or when `x` is never read
//!    again (it takes `x`'s: the `Retain` becomes `Forget(x)`).
//! 3. `skip_null`: a `Retain` or `Release` of a local that is null on
//!    every path to it (never assigned, moved out or released) does
//!    nothing, so it goes.
//!
//! Nothing here reads the ownership summaries of [`own`](crate::own)
//! (Nano, 2026-10-29): those track list aliasing (a string parameter is
//! always `read`), while what decides a refcount is whether the callee
//! keeps the reference itself, which the IR states exactly. Phase 4
//! checks this fact against the summaries once they cover every
//! reference-counted type (`docs/roadmap.md`, A8).

#[cfg(test)]
mod tests;

use std::collections::{BTreeSet, HashSet};

use crate::ir::{Block, Func, FuncId, Inst, LocalId, Operand, Program, Rvalue, Term};

/// `program` with the refcount operations it does not need removed.
pub fn elide(mut program: Program) -> Program {
    let borrowed = borrow(&mut program);
    for f in &mut program.funcs {
        copies(f, &borrowed);
        skip_null(f);
    }
    program
}

/// Makes every parameter that can be borrowed so, rewriting its calls;
/// gives each function's borrowed parameter positions.
fn borrow(program: &mut Program) -> Vec<BTreeSet<usize>> {
    let funcs = &program.funcs;
    let retained: Vec<BTreeSet<usize>> = funcs.iter().map(retains).collect();
    let borrowed: Vec<BTreeSet<usize>> = (0..funcs.len())
        .map(|id| borrowable(funcs, FuncId(id), &retained[id]))
        .collect();
    for (f, params) in program.funcs.iter_mut().zip(&borrowed) {
        let locals: BTreeSet<LocalId> = params.iter().map(|&i| f.params[i]).collect();
        for b in f.blocks.iter_mut().filter(|b| is_exit(b)) {
            b.insts
                .retain(|i| !matches!(i, Inst::Release(l) if locals.contains(l)));
        }
        f.blocks
            .iter_mut()
            .for_each(|b| release_after_calls(b, &borrowed));
        f.borrowed = locals;
    }
    borrowed
}

/// The positions of `f`'s reference-counted parameters whose reference
/// `f` may keep past, or give up before, its exit: one it moves,
/// reassigns or releases anywhere but its exit, or names in an
/// instruction [`keeps`] cannot see through. Computed once per function
/// from its own body: lowering passes a variable to a call as a copy
/// with its own `Retain`, so whatever a callee keeps is that copy's
/// reference, never the caller's parameter, and the fact needs no
/// callee's, call-graph order or fixpoint.
fn retains(f: &Func) -> BTreeSet<usize> {
    let kept = |i: &usize| {
        let p = f.params[*i];
        let keeps_in = |b: &Block| b.insts.iter().any(|inst| keeps(inst, p, is_exit(b)));
        f.locals[p.0].is_rc() && f.blocks.iter().any(keeps_in)
    };
    (0..f.params.len()).filter(kept).collect()
}

/// The positions of `f`'s parameters it can borrow: reference-counted,
/// not [`retains`]-ed, and moved in by every call as lowering emits it,
/// so [`release_after_calls`] can rewrite each.
fn borrowable(funcs: &[Func], id: FuncId, retained: &BTreeSet<usize>) -> BTreeSet<usize> {
    let f = &funcs[id.0];
    let kept = |i: &usize| {
        f.locals[f.params[*i].0].is_rc() && !retained.contains(i) && moved_in(funcs, id, *i)
    };
    (0..f.params.len()).filter(kept).collect()
}

/// Whether `inst` may keep or give up local `p`'s reference: anything
/// but reading it or the exit's `Release`. Every instruction and rvalue
/// is named, so a new one (a channel send, a call into Rust or Python)
/// counts as keeping until it is classified here.
fn keeps(inst: &Inst, p: LocalId, at_exit: bool) -> bool {
    match inst {
        Inst::Assign { dest, value, .. } => *dest == p || takes(value, p),
        Inst::Release(l) => *l == p && !at_exit,
        Inst::Forget(l) => *l == p,
        // Each adds a reference of its own, or none.
        Inst::Retain(_)
        | Inst::Print { .. }
        | Inst::Append { .. }
        | Inst::SetGlobal { .. }
        | Inst::DropGlobal(_)
        | Inst::SetItem { .. }
        | Inst::Raise { .. } => false,
    }
}

/// Whether `value` takes over local `p`'s reference: a call's argument
/// is owned. Every other rvalue borrows its operands and gives a new
/// reference.
fn takes(value: &Rvalue, p: LocalId) -> bool {
    match value {
        Rvalue::Call(_, args) => args.contains(&Operand::Local(p)),
        Rvalue::Use(_)
        | Rvalue::Int(..)
        | Rvalue::Neg(_)
        | Rvalue::Cmp(..)
        | Rvalue::AddOverflows(..)
        | Rvalue::StrLit(_)
        | Rvalue::Concat(..)
        | Rvalue::Repeat(..)
        | Rvalue::Index(..)
        | Rvalue::Len(_)
        | Rvalue::Contains { .. }
        | Rvalue::ToStr(_)
        | Rvalue::CopyList(_)
        | Rvalue::NewSeq(_)
        | Rvalue::Field(..)
        | Rvalue::Global(_)
        | Rvalue::Wrap(_)
        | Rvalue::Unwrap(_)
        | Rvalue::IsNone(_)
        | Rvalue::Float(..)
        | Rvalue::IntPow(..)
        | Rvalue::ToFloat(_)
        | Rvalue::BoolToInt(_)
        | Rvalue::FloatToInt(_)
        | Rvalue::ParseFloat(_)
        | Rvalue::ParseInt(_)
        | Rvalue::NewExc(..)
        | Rvalue::Raised
        | Rvalue::PendingIs(_)
        | Rvalue::Catch => false,
    }
}

/// Whether every call to `callee` passes argument `i` as a constant or
/// as a local the next mention of which is a `Forget` in the same block.
fn moved_in(funcs: &[Func], callee: FuncId, i: usize) -> bool {
    funcs.iter().flat_map(|f| &f.blocks).all(|b| {
        b.insts
            .iter()
            .enumerate()
            .all(|(k, inst)| match call_args(inst) {
                Some((id, args)) if id == callee => match args.get(i) {
                    Some(Operand::Local(a)) => {
                        next_mention(&b.insts[k + 1..], *a) == Some(&Inst::Forget(*a))
                    }
                    _ => true,
                },
                _ => true,
            })
    })
}

/// Turns the `Forget` that moves an argument into a borrowing callee
/// into a `Release`: the caller keeps the reference, so it drops it.
fn release_after_calls(block: &mut Block, borrowed: &[BTreeSet<usize>]) {
    for k in 0..block.insts.len() {
        let Some((id, args)) = call_args(&block.insts[k]) else {
            continue;
        };
        let locals: Vec<LocalId> = borrowed[id.0]
            .iter()
            .filter_map(|&i| match args.get(i) {
                Some(Operand::Local(a)) => Some(*a),
                _ => None,
            })
            .collect();
        for a in locals {
            let rest = &mut block.insts[k + 1..];
            if let Some(i) = rest.iter_mut().find(|i| mentions(i, a)) {
                *i = Inst::Release(a);
            }
        }
    }
}

/// Removes the `Retain` of each copy `a = x` that does not need its own
/// reference, wherever its uses run in the CFG: one released while
/// still read only as `x` lives borrows `x`'s reference ([`borrows`]),
/// and one whose source is not read again takes `x`'s reference
/// ([`last_use`]).
fn copies(f: &mut Func, borrowed: &[BTreeSet<usize>]) {
    let sites: Vec<(usize, usize)> = f
        .blocks
        .iter()
        .enumerate()
        .flat_map(|(b, block)| {
            let retains = block.insts.iter().enumerate();
            retains.filter_map(move |(k, i)| matches!(i, Inst::Retain(_)).then_some((b, k)))
        })
        .collect();
    // Back to front, so removing a `Retain` moves no site still to come.
    let mut lent = Lent::default();
    for (b, k) in sites.into_iter().rev() {
        copy_at(f, b, k, borrowed, &mut lent);
    }
}

/// The locals made to borrow so far, and those they borrow from: neither
/// may give its reference away, since it is not, or not only, its own.
#[derive(Default)]
struct Lent {
    borrowers: HashSet<LocalId>,
    sources: HashSet<LocalId>,
}

/// Elides the `Retain(a)` at block `b`, index `k`, if [`borrows`] or
/// [`last_use`] allows.
fn copy_at(f: &mut Func, b: usize, k: usize, borrowed: &[BTreeSet<usize>], lent: &mut Lent) {
    let insts = &f.blocks[b].insts;
    let Inst::Retain(a) = insts[k] else {
        return;
    };
    let Some((def, x)) = copied_from(&insts[..k], a) else {
        return;
    };
    if let Some(sites) = borrows(f, a, x, (b, def + 1), borrowed) {
        sites
            .into_iter()
            .for_each(|(sb, si)| f.blocks[sb].insts[si] = Inst::Forget(a));
        f.blocks[b].insts.remove(k);
        lent.borrowers.insert(a);
        lent.sources.insert(x);
        return;
    }
    let owns = !lent.borrowers.contains(&x) && !lent.sources.contains(&x);
    if owns && last_use(f, x, (b, def + 1)) {
        f.blocks[b].insts[k] = Inst::Forget(x);
    }
}

/// Where a copy `a` of `x`, from `at` on, is released: the `Release(a)`
/// ending every path, if `a` is defined and retained only once, is
/// never moved or reassigned on the way, and is read only while nothing
/// has ended `x`. Then `a` can borrow `x`'s reference: after `x` ends,
/// `a` is only nulled, never read. A copy made of `a` could be read
/// later through that copy, so then `x` must outlive `a` outright. A
/// path out of the program (`Term::Fail`, `Term::Unreachable`) needs no
/// release.
fn borrows(
    f: &Func,
    a: LocalId,
    x: LocalId,
    at: (usize, usize),
    borrowed: &[BTreeSet<usize>],
) -> Option<Vec<(usize, usize)>> {
    let all = f.blocks.iter().flat_map(|b| &b.insts);
    let defs = all
        .clone()
        .filter(|i| matches!(i, Inst::Assign { dest, .. } if *dest == a));
    let retains = all.filter(|i| **i == Inst::Retain(a));
    if defs.count() != 1 || retains.count() != 1 {
        return None;
    }
    let copied = f.blocks.iter().flat_map(|b| &b.insts).any(
        |i| matches!(i, Inst::Assign { value: Rvalue::Use(Operand::Local(l)), .. } if *l == a),
    );
    let mut sites = Vec::new();
    let mut seen: HashSet<(usize, bool)> = HashSet::new();
    let mut todo = vec![(at.0, at.1, true)];
    while let Some((b, from, mut alive)) = todo.pop() {
        let block = &f.blocks[b];
        let mut released = false;
        for (k, i) in block.insts.iter().enumerate().skip(from) {
            if *i == Inst::Release(a) {
                sites.push((b, k));
                released = true;
                break;
            }
            let read = mentions(i, a);
            if read && (gives_up(i, a, borrowed) || !alive) {
                return None;
            }
            alive = alive && !gives_up(i, x, borrowed);
            if !alive && copied {
                return None;
            }
        }
        if released {
            continue;
        }
        if matches!(block.term, Term::Return(_) | Term::Uncaught) {
            return None;
        }
        let next = successors(&block.term)
            .into_iter()
            .filter(|s| seen.insert((*s, alive)));
        todo.extend(next.map(|s| (s, 0, alive)).collect::<Vec<_>>());
    }
    Some(sites)
}

/// Whether `x`, from `at` on, is never read again: every path drops,
/// moves or reassigns it first, or leaves the program. Then a copy of
/// it can take its reference instead of adding one. A borrowed
/// parameter's reference is its caller's, so it is never taken; nor is
/// one [`Lent`] (the caller checks).
fn last_use(f: &Func, x: LocalId, at: (usize, usize)) -> bool {
    if f.borrowed.contains(&x) {
        return false;
    }
    let mut seen: HashSet<usize> = HashSet::new();
    let mut todo = vec![at];
    while let Some((b, from)) = todo.pop() {
        let block = &f.blocks[b];
        let mut ended = false;
        for i in block.insts.iter().skip(from) {
            if matches!(i, Inst::Release(l) | Inst::Forget(l) if *l == x) {
                ended = true;
                break;
            }
            if let Inst::Assign { dest, value, .. } = i {
                if *dest == x && !operands(value).contains(&Operand::Local(x)) {
                    ended = true;
                    break;
                }
            }
            if mentions(i, x) {
                return false;
            }
        }
        if ended {
            continue;
        }
        if matches!(block.term, Term::Return(_) | Term::Uncaught) {
            return false;
        }
        let next = successors(&block.term)
            .into_iter()
            .filter(|s| seen.insert(*s));
        todo.extend(next.map(|s| (s, 0)).collect::<Vec<_>>());
    }
    true
}

/// The index of the assignment that last made `a` a plain copy of a
/// local, and that local, if `a`'s last mention before is it.
fn copied_from(before: &[Inst], a: LocalId) -> Option<(usize, LocalId)> {
    let k = before.iter().rposition(|i| mentions(i, a))?;
    match &before[k] {
        Inst::Assign {
            dest,
            value: Rvalue::Use(Operand::Local(x)),
            ..
        } if *dest == a && *x != a => Some((k, *x)),
        _ => None,
    }
}

/// Removes `Retain`s and `Release`s of locals null on every path to
/// them. Locals other than parameters start null; an assignment may set
/// one, a `Release` or `Forget` nulls it.
fn skip_null(f: &mut Func) {
    let entry: BTreeSet<LocalId> = (0..f.locals.len())
        .map(LocalId)
        .filter(|l| !f.params.contains(l))
        .collect();
    let at_entry = null_at_entry(f, entry);
    for (b, null) in f.blocks.iter_mut().zip(at_entry) {
        let Some(mut null) = null else {
            continue;
        };
        b.insts.retain(|i| {
            let dead = matches!(i, Inst::Retain(l) | Inst::Release(l) if null.contains(l));
            step(&mut null, i);
            !dead
        });
    }
}

/// Per block, the locals null on every path into it; `None` for a block
/// no path reaches.
fn null_at_entry(f: &Func, entry: BTreeSet<LocalId>) -> Vec<Option<BTreeSet<LocalId>>> {
    let mut at: Vec<Option<BTreeSet<LocalId>>> = vec![None; f.blocks.len()];
    at[0] = Some(entry);
    let mut todo = vec![0];
    let mut queued: HashSet<usize> = HashSet::from([0]);
    while let Some(b) = todo.pop() {
        queued.remove(&b);
        let Some(mut null) = at[b].clone() else {
            continue;
        };
        f.blocks[b].insts.iter().for_each(|i| step(&mut null, i));
        for s in successors(&f.blocks[b].term) {
            let joined = match &at[s] {
                Some(old) => old.intersection(&null).copied().collect(),
                None => null.clone(),
            };
            if at[s].as_ref() != Some(&joined) {
                at[s] = Some(joined);
                if queued.insert(s) {
                    todo.push(s);
                }
            }
        }
    }
    at
}

/// The null locals after `inst`, given those before.
fn step(null: &mut BTreeSet<LocalId>, inst: &Inst) {
    match inst {
        Inst::Assign { dest, .. } => {
            null.remove(dest);
        }
        Inst::Release(l) | Inst::Forget(l) => {
            null.insert(*l);
        }
        _ => {}
    }
}

fn successors(term: &Term) -> Vec<usize> {
    match term {
        Term::Goto(t) => vec![t.0],
        Term::Branch { then, orelse, .. } => vec![then.0, orelse.0],
        _ => Vec::new(),
    }
}

/// Whether `b` is a function's exit, the one block that returns.
fn is_exit(b: &Block) -> bool {
    matches!(b.term, Term::Return(_))
}

fn call_args(inst: &Inst) -> Option<(FuncId, &[Operand])> {
    match inst {
        Inst::Assign {
            value: Rvalue::Call(id, args),
            ..
        } => Some((*id, args)),
        _ => None,
    }
}

/// Whether `inst` changes what local `l` owns: assigns, releases or
/// forgets it.
fn ends(inst: &Inst, l: LocalId) -> bool {
    match inst {
        // A call moving `l` in is followed by its `Forget`; one that
        // borrows it leaves it as it was.
        Inst::Assign { dest, .. } => *dest == l,
        Inst::Release(x) | Inst::Forget(x) => *x == l,
        Inst::Retain(_)
        | Inst::Print { .. }
        | Inst::Append { .. }
        | Inst::SetGlobal { .. }
        | Inst::DropGlobal(_)
        | Inst::SetItem { .. }
        | Inst::Raise { .. } => false,
    }
}

/// Whether `inst` ends local `l`'s reference: [`ends`], or a call
/// taking it at a parameter its callee does not borrow (`borrowed`, by
/// function), or any other rvalue [`takes`] it.
fn gives_up(inst: &Inst, l: LocalId, borrowed: &[BTreeSet<usize>]) -> bool {
    if ends(inst, l) {
        return true;
    }
    match (call_args(inst), inst) {
        (Some((id, args)), _) => args
            .iter()
            .enumerate()
            .any(|(p, op)| *op == Operand::Local(l) && !borrowed[id.0].contains(&p)),
        (None, Inst::Assign { value, .. }) => takes(value, l),
        (None, _) => false,
    }
}

/// The first instruction in `insts` that names `l`.
fn next_mention(insts: &[Inst], l: LocalId) -> Option<&Inst> {
    insts.iter().find(|i| mentions(i, l))
}

/// Whether `inst` names local `l` anywhere.
fn mentions(inst: &Inst, l: LocalId) -> bool {
    let local = |op: &Operand| *op == Operand::Local(l);
    match inst {
        Inst::Assign { dest, value, .. } => *dest == l || operands(value).iter().any(local),
        Inst::Print { args, sep, end } => args.iter().chain(sep).chain(end).any(local),
        Inst::Retain(x) | Inst::Release(x) | Inst::Forget(x) => *x == l,
        Inst::Append { list, value, .. } => local(list) || local(value),
        Inst::SetGlobal { value, .. } => local(value),
        Inst::DropGlobal(_) => false,
        Inst::SetItem {
            list, index, value, ..
        } => [list, index, value].into_iter().any(local),
        Inst::Raise { exc, .. } => local(exc),
    }
}

/// The operands an rvalue reads.
fn operands(value: &Rvalue) -> Vec<Operand> {
    match value {
        Rvalue::Use(a)
        | Rvalue::Neg(a)
        | Rvalue::Len(a)
        | Rvalue::ToStr(a)
        | Rvalue::CopyList(a)
        | Rvalue::Field(a, _)
        | Rvalue::Wrap(a)
        | Rvalue::Unwrap(a)
        | Rvalue::IsNone(a)
        | Rvalue::ToFloat(a)
        | Rvalue::BoolToInt(a)
        | Rvalue::FloatToInt(a)
        | Rvalue::ParseFloat(a)
        | Rvalue::ParseInt(a) => vec![*a],
        Rvalue::Int(_, a, b)
        | Rvalue::Cmp(_, a, b)
        | Rvalue::AddOverflows(a, b)
        | Rvalue::Concat(a, b)
        | Rvalue::Repeat(a, b)
        | Rvalue::Index(a, b)
        | Rvalue::Float(_, a, b)
        | Rvalue::IntPow(a, b) => vec![*a, *b],
        Rvalue::Contains { hay, needle } => vec![*hay, *needle],
        Rvalue::Call(_, args) | Rvalue::NewSeq(args) => args.clone(),
        Rvalue::NewExc(_, arg) => arg.iter().copied().collect(),
        Rvalue::StrLit(_)
        | Rvalue::Global(_)
        | Rvalue::Raised
        | Rvalue::PendingIs(_)
        | Rvalue::Catch => Vec::new(),
    }
}
