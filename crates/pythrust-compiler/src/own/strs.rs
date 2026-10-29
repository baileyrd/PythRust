//! What a function does with the strings its parameters hold (A8).
//!
//! The list analysis ([`super::body`]) answers for lists. A string is a
//! value, but D2 still asks tempered code to write `owned` on a string
//! parameter it keeps, so this pass answers for a parameter's *strings*:
//! the parameter itself if it is a `str`, and the strings in its tuple
//! fields and its optional's value, recursively, but not past a list (a
//! list's elements are values of their own, A6). An exception counts as
//! a string. Which parameters have strings is a property of the
//! language-level type ([`Type::holds_heap`], [`Type::reaches_mutable`]), so a tuple or optional of
//! scalars never has any.
//!
//! It tracks, per local, which parameters' strings it may hold, and per
//! abstract list of the settled points-to sets, which ones the body
//! stores in it. A parameter's strings are [`Strs::kept`] when one may
//! reach a list the caller or the module can see, a global, an
//! exception (or is one raised), or a callee that keeps it; [`Strs::returned`] when the
//! result may hold one. Flow-insensitive, like the summary it extends.

use std::collections::{BTreeMap, BTreeSet};

use super::body::{Flow, Obj, Objs};
use super::{source, Strs, Summary};
use crate::infer::Type;
use crate::ir::{Compound, Func, Inst, LocalId, Operand, Program, Rvalue, Term, Ty};

/// Parameter positions.
type Params = BTreeSet<usize>;

/// One [`Strs`] per parameter of `func`, its lists settled as `flow`.
pub(super) fn params(
    program: &Program,
    func: &Func,
    summaries: &[Summary],
    flow: &Flow,
) -> Vec<Strs> {
    let mut held = Held::new(program, func, summaries, flow);
    while held.propagate() {}
    let kept = held.kept();
    let returned = held.returned();
    (0..func.params.len())
        .map(|i| Strs {
            kept: kept.contains(&i),
            returned: returned.contains(&i),
        })
        .collect()
}

/// Whether a value of type `ty` is, or holds outside any list, a heap
/// value that cannot change in place: a `str` or an exception.
fn holds_str(ty: &Type) -> bool {
    match ty {
        Type::Tuple(fields) => fields.iter().any(holds_str),
        Type::Optional(present) => holds_str(present),
        Type::List(_) => false,
        _ => ty.holds_heap() == Some(true) && ty.reaches_mutable() == Some(false),
    }
}

struct Held<'a> {
    program: &'a Program,
    func: &'a Func,
    summaries: &'a [Summary],
    flow: &'a Flow<'a>,
    /// Per local, the parameters whose strings it may hold outside any
    /// list.
    has: Vec<Params>,
    /// Per abstract list, the parameters whose strings the body may
    /// store in it.
    inside: BTreeMap<Obj, Params>,
    /// Parameters a string of which went where the call cannot see its
    /// end: a global, an exception, a callee that keeps it.
    escaped: Params,
}

impl<'a> Held<'a> {
    fn new(
        program: &'a Program,
        func: &'a Func,
        summaries: &'a [Summary],
        flow: &'a Flow<'a>,
    ) -> Held<'a> {
        let mut has = vec![Params::new(); func.locals.len()];
        for (i, p) in func.params.iter().enumerate() {
            if holds_str(&source(program, func.locals[p.0])) {
                has[p.0].insert(i);
            }
        }
        Held {
            program,
            func,
            summaries,
            flow,
            has,
            inside: BTreeMap::new(),
            escaped: Params::new(),
        }
    }

    /// One pass over the body; whether anything may hold more.
    fn propagate(&mut self) -> bool {
        let func = self.func;
        let mut changed = false;
        for inst in func.blocks.iter().flat_map(|b| &b.insts) {
            changed |= self.flow_inst(inst);
        }
        changed
    }

    fn flow_inst(&mut self, inst: &Inst) -> bool {
        match inst {
            Inst::Assign { dest, value, .. } => self.assign(*dest, value),
            Inst::Append { list, value, .. } | Inst::SetItem { list, value, .. } => {
                let value = self.holdings(*value);
                let lists = self.flow.points_to(*list);
                self.store(&lists, &value)
            }
            Inst::SetGlobal { value, .. } | Inst::Raise { exc: value, .. } => {
                let value = self.holdings(*value);
                grow(&mut self.escaped, &value)
            }
            _ => false,
        }
    }

    fn assign(&mut self, dest: LocalId, value: &Rvalue) -> bool {
        match value {
            Rvalue::Call(f, args) => return self.call(dest, f.0, args),
            // An exception may outlive the call, raised or stored.
            Rvalue::NewExc(_, Some(a)) => {
                let held = self.holdings(*a);
                return grow(&mut self.escaped, &held);
            }
            Rvalue::NewSeq(items) if self.is_list(dest) => {
                let held = items.iter().flat_map(|a| self.holdings(*a)).collect();
                return self.store_fresh(dest, &held);
            }
            Rvalue::CopyList(a) => {
                let held = self.holdings(*a);
                return self.store_fresh(dest, &held);
            }
            _ => {}
        }
        if !holds_str(&source(self.program, self.func.locals[dest.0])) {
            return false;
        }
        let gains = match value {
            // A character of a string is a new string.
            Rvalue::Index(a, _) if self.is_str(*a) => Params::new(),
            Rvalue::Use(a)
            | Rvalue::Wrap(a)
            | Rvalue::Unwrap(a)
            | Rvalue::Field(a, _)
            | Rvalue::Index(a, _) => self.holdings(*a),
            Rvalue::NewSeq(items) => items.iter().flat_map(|a| self.holdings(*a)).collect(),
            _ => Params::new(),
        };
        grow(&mut self.has[dest.0], &gains)
    }

    /// A callee returns what it says it may, and keeps what it says it
    /// does.
    fn call(&mut self, dest: LocalId, callee: usize, args: &[Operand]) -> bool {
        let params = &self.summaries[callee].params;
        let mut returned = Params::new();
        let mut kept = Params::new();
        for (a, p) in args.iter().zip(params) {
            if p.strs.returned {
                returned.extend(self.holdings(*a));
            }
            if p.strs.kept {
                kept.extend(self.holdings(*a));
            }
        }
        grow(&mut self.has[dest.0], &returned) | grow(&mut self.escaped, &kept)
    }

    /// Stores `value` in each of `lists`, or where the call cannot see
    /// its end if it knows none; whether anything grew.
    fn store(&mut self, lists: &Objs, value: &Params) -> bool {
        if lists.is_empty() {
            return grow(&mut self.escaped, value);
        }
        let mut changed = false;
        for o in lists {
            changed |= grow(self.inside.entry(*o).or_default(), value);
        }
        changed
    }

    /// Stores `value` in the new list `dest` holds.
    fn store_fresh(&mut self, dest: LocalId, value: &Params) -> bool {
        let fresh = self.flow.points_to(Operand::Local(dest));
        self.store(&fresh, value)
    }

    /// The parameters whose strings `op` may hold, itself or in a list
    /// it may reach.
    fn holdings(&self, op: Operand) -> Params {
        let Operand::Local(l) = op else {
            return Params::new();
        };
        let mut out = self.has[l.0].clone();
        out.extend(self.in_lists(&self.flow.reachable(&self.flow.points_to(op))));
        out
    }

    /// The parameters whose strings the body stored in `lists`.
    fn in_lists(&self, lists: &Objs) -> Params {
        let inside = lists.iter().filter_map(|o| self.inside.get(o));
        inside.flatten().copied().collect()
    }

    /// What escaped, and what the body stored in a list the caller or
    /// the module can see after the call.
    fn kept(&self) -> Params {
        let params = (0..self.func.params.len()).map(Obj::Param);
        let roots: Objs = params.chain([Obj::World]).collect();
        let mut out = self.escaped.clone();
        out.extend(self.in_lists(&self.flow.reachable(&roots)));
        out
    }

    /// What any `return` may hold.
    fn returned(&self) -> Params {
        let returns = self.func.blocks.iter().filter_map(|b| match &b.term {
            Term::Return(op) => Some(*op),
            _ => None,
        });
        returns.flat_map(|op| self.holdings(op)).collect()
    }

    fn is_str(&self, op: Operand) -> bool {
        matches!(op, Operand::Local(l) if self.func.locals[l.0] == Ty::Str)
    }

    fn is_list(&self, local: LocalId) -> bool {
        matches!(
            self.program.compound(self.func.locals[local.0]),
            Some(Compound::List(_))
        )
    }
}

/// Adds `more` to `set`; whether it grew.
fn grow(set: &mut Params, more: &Params) -> bool {
    let before = set.len();
    set.extend(more.iter().copied());
    set.len() > before
}
