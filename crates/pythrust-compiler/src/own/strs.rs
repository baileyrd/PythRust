//! What a function does with the strings its parameters hold (A8).
//!
//! The list analysis ([`super::body`]) answers for lists. A string is a
//! value, but D2 still asks tempered code to write `owned` on a string
//! parameter it keeps, so this pass answers for a parameter's *strings*:
//! the parameter itself if it is a `str`, and the strings in its tuple
//! fields and its optional's value, recursively, but not past a list (a
//! list's elements are values of their own, A6). An exception counts as
//! a string. Which parameters have strings is a property of the
//! language-level type ([`Type::holds_heap`], [`Type::reaches_mutable`]),
//! so a tuple or optional of scalars never has any.
//!
//! It tracks, per local, which [`Origin`]s it may hold outside any list,
//! and per abstract list of the settled points-to sets, which ones the
//! body stores in it: a parameter's strings, or the elements of its
//! lists, which a caller needs to follow its own strings through a
//! callee (`first([s])` returns `s`). A parameter's strings are
//! [`Strs::kept`] when one may reach a list the caller or the module can
//! see, a global, an exception (or is one raised), or a callee that keeps
//! it; [`Strs::returned`] when the result may hold one; the elements of
//! its lists likewise. Flow-insensitive, like the summary it extends.

use std::collections::{BTreeMap, BTreeSet};

use super::body::{Flow, Obj, Objs};
use super::{source, Strs, Summary};
use crate::infer::Type;
use crate::ir::{Compound, Func, Inst, LocalId, Operand, Program, Rvalue, Term, Ty};

/// Where a string held in the body came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Origin {
    /// Parameter `i`'s strings.
    Part(usize),
    /// The elements of parameter `i`'s lists, as the caller passed them.
    Element(usize),
}

type Origins = BTreeSet<Origin>;

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
            kept: kept.contains(&Origin::Part(i)),
            returned: returned.contains(&Origin::Part(i)),
            elements_kept: kept.contains(&Origin::Element(i)),
            elements_returned: returned.contains(&Origin::Element(i)),
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
    /// Per local, what it may hold outside any list.
    has: Vec<Origins>,
    /// Per abstract list, what the body may store in it.
    inside: BTreeMap<Obj, Origins>,
    /// What went where the call cannot see its end: a global, an
    /// exception, a callee that keeps it.
    escaped: Origins,
}

impl<'a> Held<'a> {
    fn new(
        program: &'a Program,
        func: &'a Func,
        summaries: &'a [Summary],
        flow: &'a Flow<'a>,
    ) -> Held<'a> {
        let mut has = vec![Origins::new(); func.locals.len()];
        for (i, p) in func.params.iter().enumerate() {
            if holds_str(&source(program, func.locals[p.0])) {
                has[p.0].insert(Origin::Part(i));
            }
        }
        Held {
            program,
            func,
            summaries,
            flow,
            has,
            inside: BTreeMap::new(),
            escaped: Origins::new(),
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
        // A list's own strings are its abstract lists' (`inside`).
        let ty = source(self.program, self.func.locals[dest.0]);
        if self.is_list(dest) || ty.holds_heap() == Some(false) {
            return false;
        }
        let gains = match value {
            // A character of a string is a new string.
            Rvalue::Index(a, _) if self.is_str(*a) => Origins::new(),
            Rvalue::Use(a)
            | Rvalue::Wrap(a)
            | Rvalue::Unwrap(a)
            | Rvalue::Field(a, _)
            | Rvalue::Index(a, _) => self.holdings(*a),
            Rvalue::NewSeq(items) => items.iter().flat_map(|a| self.holdings(*a)).collect(),
            _ => Origins::new(),
        };
        grow(&mut self.has[dest.0], &gains)
    }

    /// A callee returns what it says it may of an argument's strings or
    /// its lists' elements, and keeps what it says it does. A list result
    /// holds what is returned in its abstract lists.
    fn call(&mut self, dest: LocalId, callee: usize, args: &[Operand]) -> bool {
        let params = &self.summaries[callee].params;
        let mut returned = Origins::new();
        let mut kept = Origins::new();
        for (a, p) in args.iter().zip(params) {
            if p.strs.returned || p.strs.elements_returned {
                returned.extend(self.holdings(*a));
            }
            if p.strs.kept || p.strs.elements_kept {
                kept.extend(self.holdings(*a));
            }
        }
        let changed = grow(&mut self.escaped, &kept);
        if self.is_list(dest) {
            return self.store_fresh(dest, &returned) | changed;
        }
        grow(&mut self.has[dest.0], &returned) | changed
    }

    /// Stores `value` in each of `lists`, or where the call cannot see
    /// its end if it knows none; whether anything grew.
    fn store(&mut self, lists: &Objs, value: &Origins) -> bool {
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
    fn store_fresh(&mut self, dest: LocalId, value: &Origins) -> bool {
        let fresh = self.flow.points_to(Operand::Local(dest));
        self.store(&fresh, value)
    }

    /// What `op` may hold, itself or in a list it may reach.
    fn holdings(&self, op: Operand) -> Origins {
        let Operand::Local(l) = op else {
            return Origins::new();
        };
        let lists = self.flow.reachable(&self.flow.points_to(op));
        let elements = lists.iter().filter_map(|o| match o {
            Obj::Param(i) => Some(Origin::Element(*i)),
            _ => None,
        });
        let mut out = self.has[l.0].clone();
        out.extend(elements);
        out.extend(self.stored(&lists));
        out
    }

    /// What the body stored in `lists`.
    fn stored(&self, lists: &Objs) -> Origins {
        let inside = lists.iter().filter_map(|o| self.inside.get(o));
        inside.flatten().copied().collect()
    }

    /// What escaped, and what the body stored in a list the caller or
    /// the module can see after the call.
    fn kept(&self) -> Origins {
        let params = (0..self.func.params.len()).map(Obj::Param);
        let roots: Objs = params.chain([Obj::World]).collect();
        let mut out = self.escaped.clone();
        out.extend(self.stored(&self.flow.reachable(&roots)));
        out
    }

    /// What any `return` may hold.
    fn returned(&self) -> Origins {
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
fn grow(set: &mut Origins, more: &Origins) -> bool {
    let before = set.len();
    set.extend(more.iter().copied());
    set.len() > before
}
