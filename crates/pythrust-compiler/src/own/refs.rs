//! What a function does with the references its parameters hold,
//! whatever their type (A8).
//!
//! The list analysis ([`super::body`]) answers for lists only: aliasing
//! anything else is unobservable, so a string parameter is always
//! [`Mode::Read`](super::Mode::Read). Whether a reference outlives the
//! call is still a fact about the value, and native code pays for it in
//! refcounts. This pass tracks, per local, which parameters' values (or
//! values inside them) it may hold a reference to, and per abstract list
//! of the settled points-to sets, which ones the body stores in it.
//! A parameter is [`Refs::kept`] when one of those references may reach
//! the caller's or the module's lists, a global, an exception, or a
//! callee that keeps it; [`Refs::returned`] when the result may hold one.
//! Flow-insensitive, like the summary it extends.

use std::collections::{BTreeMap, BTreeSet};

use super::body::{Flow, Obj, Objs};
use super::{Refs, Summary};
use crate::ir::{Func, Inst, LocalId, Operand, Rvalue, Term, Ty};

/// Parameter positions.
type Params = BTreeSet<usize>;

/// One [`Refs`] per parameter of `func`, its lists settled as `flow`.
pub(super) fn params(func: &Func, summaries: &[Summary], flow: &Flow) -> Vec<Refs> {
    let mut held = Held::new(func, summaries, flow);
    while held.propagate() {}
    let kept = held.kept();
    let returned = held.returned();
    (0..func.params.len())
        .map(|i| Refs {
            kept: kept.contains(&i),
            returned: returned.contains(&i),
        })
        .collect()
}

struct Held<'a> {
    func: &'a Func,
    summaries: &'a [Summary],
    flow: &'a Flow<'a>,
    /// Per local, the parameters whose references it may hold itself.
    has: Vec<Params>,
    /// Per abstract list, the parameters whose references the body may
    /// store in it.
    inside: BTreeMap<Obj, Params>,
    /// Parameters a reference of which went where the call cannot see
    /// its end: a global, an exception, a callee that keeps it.
    escaped: Params,
}

impl<'a> Held<'a> {
    fn new(func: &'a Func, summaries: &'a [Summary], flow: &'a Flow<'a>) -> Held<'a> {
        let mut has = vec![Params::new(); func.locals.len()];
        for (i, p) in func.params.iter().enumerate() {
            if func.locals[p.0].is_rc() {
                has[p.0].insert(i);
            }
        }
        Held {
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
                if lists.is_empty() {
                    return grow(&mut self.escaped, &value);
                }
                let mut changed = false;
                for o in lists {
                    changed |= grow(self.inside.entry(o).or_default(), &value);
                }
                changed
            }
            Inst::SetGlobal { value, .. } => {
                let value = self.holdings(*value);
                grow(&mut self.escaped, &value)
            }
            _ => false,
        }
    }

    fn assign(&mut self, dest: LocalId, value: &Rvalue) -> bool {
        if let Rvalue::Call(f, args) = value {
            return self.call(dest, f.0, args);
        }
        if let Rvalue::NewExc(_, Some(a)) = value {
            // An exception may outlive the call, raised or stored.
            let held = self.holdings(*a);
            return grow(&mut self.escaped, &held);
        }
        if !self.func.locals[dest.0].is_rc() {
            return false;
        }
        let gains = match value {
            Rvalue::Use(a) | Rvalue::Wrap(a) | Rvalue::Unwrap(a) | Rvalue::Field(a, _) => {
                self.holdings(*a)
            }
            // A character of a string is a new string.
            Rvalue::Index(a, _) if self.is_str(*a) => Params::new(),
            Rvalue::Index(a, _) | Rvalue::CopyList(a) => self.holdings(*a),
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
            if p.refs.returned {
                returned.extend(self.holdings(*a));
            }
            if p.refs.kept {
                kept.extend(self.holdings(*a));
            }
        }
        grow(&mut self.has[dest.0], &returned) | grow(&mut self.escaped, &kept)
    }

    /// The parameters whose references `op` may hold, itself or in a
    /// list it may reach.
    fn holdings(&self, op: Operand) -> Params {
        let Operand::Local(l) = op else {
            return Params::new();
        };
        let mut out = self.has[l.0].clone();
        out.extend(self.in_lists(&self.flow.reachable(&self.flow.points_to(op))));
        out
    }

    /// The parameters `lists` are, or that the body stored in them.
    fn in_lists(&self, lists: &Objs) -> Params {
        let mut out = Params::new();
        for o in lists {
            if let Obj::Param(i) = o {
                out.insert(*i);
            }
            if let Some(inside) = self.inside.get(o) {
                out.extend(inside.iter().copied());
            }
        }
        out
    }

    /// What escaped, and what the body stored in a list the caller or
    /// the module can see after the call.
    fn kept(&self) -> Params {
        let params = (0..self.func.params.len()).map(Obj::Param);
        let roots: Objs = params.chain([Obj::World]).collect();
        let visible = self.flow.reachable(&roots);
        let stored = visible.iter().filter_map(|o| self.inside.get(o)).flatten();
        let mut out = self.escaped.clone();
        out.extend(stored.copied());
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
}

/// Adds `more` to `set`; whether it grew.
fn grow(set: &mut Params, more: &Params) -> bool {
    let before = set.len();
    set.extend(more.iter().copied());
    set.len() > before
}
