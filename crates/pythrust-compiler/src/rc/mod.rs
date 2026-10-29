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
//! 1. `borrow`: a parameter the callee never moves, reassigns or
//!    releases before its exit is *borrowed* ([`Func::borrowed`]): the
//!    callee no longer drops it, and every call releases the argument
//!    itself after the call instead of moving it in.
//! 2. `cancel`: a copy `a = x; Retain(a)` released again in the same
//!    block, while `x` still holds its reference, borrows from `x`:
//!    both refcount operations go.
//! 3. `skip_null`: a `Retain` or `Release` of a local that is null on
//!    every path to it (never assigned, moved out or released) does
//!    nothing, so it goes.
//!
//! Nothing here reads the ownership summaries of [`own`](crate::own):
//! those track list aliasing (a string parameter is always `read`),
//! while what decides a refcount is whether the callee keeps the
//! reference itself, which the IR states exactly.

#[cfg(test)]
mod tests;

use std::collections::{BTreeSet, HashSet};

use crate::ir::{Block, Func, FuncId, Inst, LocalId, Operand, Program, Rvalue, Term};

/// `program` with the refcount operations it does not need removed.
pub fn elide(mut program: Program) -> Program {
    borrow(&mut program);
    for f in &mut program.funcs {
        f.blocks.iter_mut().for_each(cancel);
        skip_null(f);
    }
    program
}

/// Makes every parameter that can be borrowed so, rewriting its calls.
fn borrow(program: &mut Program) {
    let funcs = &program.funcs;
    let borrowed: Vec<BTreeSet<usize>> = (0..funcs.len())
        .map(|id| borrowable(funcs, FuncId(id)))
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
}

/// The positions of `f`'s parameters it can borrow: reference-counted,
/// dropped only by its exit, and moved in by every call as lowering
/// emits it, so [`release_after_calls`] can rewrite each.
fn borrowable(funcs: &[Func], id: FuncId) -> BTreeSet<usize> {
    let f = &funcs[id.0];
    let kept = |i: &usize| {
        let p = f.params[*i];
        f.locals[p.0].is_rc() && only_exit_drops(f, p) && moved_in(funcs, id, *i)
    };
    (0..f.params.len()).filter(kept).collect()
}

/// Whether `f` drops, moves or reassigns `p` only in a `Release` of its
/// exit block.
fn only_exit_drops(f: &Func, p: LocalId) -> bool {
    f.blocks.iter().all(|b| {
        b.insts.iter().all(|i| match i {
            Inst::Release(l) => *l != p || is_exit(b),
            _ => !ends(i, p),
        })
    })
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

/// Removes each `a = x; Retain(a)` copy whose next end, in the same
/// block, is a `Release(a)` reached before anything ends `x`: `a` then
/// borrows `x`'s reference, and is nulled where it was released.
fn cancel(block: &mut Block) {
    let mut k = 0;
    while k < block.insts.len() {
        let Some((a, r)) = cancelled_by(&block.insts, k) else {
            k += 1;
            continue;
        };
        block.insts[r] = Inst::Forget(a);
        block.insts.remove(k);
    }
}

/// For a `Retain(a)` at `k` copying a local `x`, `a` and the index of
/// the `Release(a)` that cancels it, if one does.
fn cancelled_by(insts: &[Inst], k: usize) -> Option<(LocalId, usize)> {
    let Inst::Retain(a) = insts[k] else {
        return None;
    };
    let x = copied_from(&insts[..k], a)?;
    let r = k
        + 1
        + insts[k + 1..]
            .iter()
            .position(|i| ends(i, a) || ends(i, x))?;
    (insts[r] == Inst::Release(a)).then_some((a, r))
}

/// The local `a` was last assigned a plain copy of, if its last
/// mention before is that assignment.
fn copied_from(before: &[Inst], a: LocalId) -> Option<LocalId> {
    let last = before.iter().rev().find(|i| mentions(i, a))?;
    match last {
        Inst::Assign {
            dest,
            value: Rvalue::Use(Operand::Local(x)),
            ..
        } if *dest == a && *x != a => Some(*x),
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
        Inst::Assign { dest, .. } => *dest == l,
        Inst::Release(x) | Inst::Forget(x) => *x == l,
        _ => false,
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
