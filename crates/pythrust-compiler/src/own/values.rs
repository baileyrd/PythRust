//! Calls through function values, for the analyses (D197).
//!
//! A call through a value may reach any function whose address the
//! program takes with the value's signature. Rather than teach every
//! analysis that, [`dispatch`] gives each such signature one synthetic
//! function that calls each candidate on its own path and returns what
//! that call returns, and calls it in place of each call through a value:
//! the summaries, effects and call sites of the candidates all join in it.

use std::collections::{BTreeMap, BTreeSet};

use crate::ir::{
    Block, BlockId, Compound, Func, FuncId, Inst, LocalId, Operand, Program, Rvalue, Term, Ty,
    TypeId,
};
use crate::span::Span;

const NOWHERE: Span = Span { start: 0, end: 0 };

/// Replaces every `CallValue` of `program` by a call of its signature's
/// dispatcher, which is appended to the functions. A call whose callee
/// is not typed as a signature stays as it is: lowering never makes one.
pub(super) fn dispatch(program: &mut Program) {
    let taken = address_taken(program);
    let mut made: BTreeMap<TypeId, FuncId> = BTreeMap::new();
    for i in 0..program.funcs.len() {
        for b in 0..program.funcs[i].blocks.len() {
            for k in 0..program.funcs[i].blocks[b].insts.len() {
                let Inst::Assign {
                    value: Rvalue::CallValue(f, args),
                    ..
                } = &program.funcs[i].blocks[b].insts[k]
                else {
                    continue;
                };
                let Ty::Fn(sig) = program.funcs[i].locals[local(*f).0] else {
                    continue;
                };
                let args = args.clone();
                let id = match made.get(&sig) {
                    Some(id) => *id,
                    None => match push_dispatcher(program, sig, &taken) {
                        Some(id) => *made.entry(sig).or_insert(id),
                        None => continue,
                    },
                };
                if let Inst::Assign { value, .. } = &mut program.funcs[i].blocks[b].insts[k] {
                    *value = Rvalue::Call(id, args);
                }
            }
        }
    }
}

fn local(op: Operand) -> LocalId {
    match op {
        Operand::Local(l) => l,
        _ => LocalId(usize::MAX),
    }
}

/// The functions used as values, by the signature of the value.
fn address_taken(program: &Program) -> BTreeMap<TypeId, BTreeSet<FuncId>> {
    let mut taken: BTreeMap<TypeId, BTreeSet<FuncId>> = BTreeMap::new();
    for f in &program.funcs {
        for inst in f.blocks.iter().flat_map(|b| &b.insts) {
            let Inst::Assign {
                dest,
                value: Rvalue::FuncRef(callee),
                ..
            } = inst
            else {
                continue;
            };
            if let Ty::Fn(sig) = f.locals[dest.0] {
                taken.entry(sig).or_default().insert(*callee);
            }
        }
    }
    taken
}

/// Appends the dispatcher of signature `sig` and gives its id.
fn push_dispatcher(
    program: &mut Program,
    sig: TypeId,
    taken: &BTreeMap<TypeId, BTreeSet<FuncId>>,
) -> Option<FuncId> {
    let Some(Compound::Fn(params, ret)) = program.types.get(sig.0) else {
        return None;
    };
    let (params, ret) = (params.clone(), *ret);
    let callees: Vec<FuncId> = taken.get(&sig).into_iter().flatten().copied().collect();
    let mut locals = params.clone();
    let args: Vec<Operand> = (0..params.len())
        .map(|i| Operand::Local(LocalId(i)))
        .collect();
    let result = LocalId(locals.len());
    locals.push(ret);
    let cond = LocalId(locals.len());
    locals.push(Ty::Bool);
    // Block `i` tests whether the call is of candidate `i`; the call of
    // candidate `i` is block `n + i`; the last block returns.
    let n = callees.len();
    let exit = BlockId(2 * n);
    let mut blocks = Vec::new();
    for i in 0..n {
        let test = Inst::Assign {
            dest: cond,
            value: Rvalue::Raised,
            span: NOWHERE,
        };
        let next = BlockId(i + 1);
        let term = match i + 1 < n {
            true => Term::Branch {
                cond: Operand::Local(cond),
                then: BlockId(n + i),
                orelse: next,
            },
            false => Term::Goto(BlockId(n + i)),
        };
        blocks.push(Block {
            insts: vec![test],
            term,
        });
    }
    for callee in &callees {
        let call = Inst::Assign {
            dest: result,
            value: Rvalue::Call(*callee, args.clone()),
            span: NOWHERE,
        };
        blocks.push(Block {
            insts: vec![call],
            term: Term::Goto(exit),
        });
    }
    let back = match ret {
        Ty::None => Operand::None,
        _ => Operand::Local(result),
    };
    blocks.push(Block {
        insts: Vec::new(),
        term: match n {
            0 => Term::Unreachable,
            _ => Term::Return(back),
        },
    });
    program.funcs.push(Func {
        name: format!("<call through a function value {}>", sig.0),
        params: (0..params.len()).map(LocalId).collect(),
        ret,
        locals,
        names: BTreeMap::new(),
        borrowed: BTreeSet::new(),
        blocks,
        regions: Vec::new(),
        tasks: Vec::new(),
    });
    Some(FuncId(program.funcs.len() - 1))
}
