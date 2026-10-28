//! One function's summary, given its callees' current summaries.
//!
//! A flow-insensitive points-to analysis over abstract lists: each
//! parameter, the module's values ([`Obj::World`]), and one list per
//! local that allocates. Every abstract list stands for itself and every
//! list inside it that nothing stored there explicitly, so `xs[0]` of
//! any list may be the list itself: a callee's fresh result can be
//! nested to any depth. Points-to sets are found to a fixpoint first,
//! then read off for the summary. Being flow-insensitive, it is sound
//! for slots assigned more than once.
//!
//! A parameter's *region* is the lists it may hold that belong to it:
//! itself, and what the body stores in it, short of another parameter
//! or the module's values. Its mode and aliasing are about its region;
//! another parameter or a global it comes to hold answers for itself.

use std::collections::{BTreeMap, BTreeSet};

use super::{holds_list, Mode, Param, Summary};
use crate::ir::{Compound, Func, FuncId, Inst, LocalId, Operand, Program, Rvalue, Term};

/// An abstract list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum Obj {
    /// Parameter `i`, and every list inside it.
    Param(usize),
    /// What the module's globals hold, and every list inside it.
    World,
    /// Every list the allocations into local `i` give.
    Fresh(usize),
}

pub(super) type Objs = BTreeSet<Obj>;

pub(super) struct Flow<'a> {
    program: &'a Program,
    func: &'a Func,
    summaries: &'a [Summary],
    /// Per local, the lists it may be, or for an optional or a tuple,
    /// the lists it may hold.
    pts: Vec<Objs>,
    /// Per list, what the body may store in it.
    stored: BTreeMap<Obj, Objs>,
}

/// `func`'s summary, with callees summarized as `summaries` say.
pub(super) fn summarize(program: &Program, func: &Func, summaries: &[Summary]) -> Summary {
    settle(program, func, summaries).summary()
}

/// `func`'s points-to sets, settled.
pub(super) fn settle<'a>(
    program: &'a Program,
    func: &'a Func,
    summaries: &'a [Summary],
) -> Flow<'a> {
    let mut flow = Flow::new(program, func, summaries);
    while flow.propagate() {}
    flow
}

impl<'a> Flow<'a> {
    fn new(program: &'a Program, func: &'a Func, summaries: &'a [Summary]) -> Flow<'a> {
        let mut pts = vec![Objs::new(); func.locals.len()];
        for (i, p) in func.params.iter().enumerate() {
            if holds_list(program, func.locals[p.0]) {
                pts[p.0].insert(Obj::Param(i));
            }
        }
        Flow {
            program,
            func,
            summaries,
            pts,
            stored: BTreeMap::new(),
        }
    }

    /// One pass over the body; whether anything may point further.
    fn propagate(&mut self) -> bool {
        let func = self.func;
        let mut changed = false;
        for inst in func.blocks.iter().flat_map(|b| &b.insts) {
            changed |= self.flow(inst);
        }
        changed
    }

    fn flow(&mut self, inst: &Inst) -> bool {
        match inst {
            Inst::Assign { dest, value, .. } => self.assign(*dest, value),
            Inst::Append { list, value, .. } | Inst::SetItem { list, value, .. } => {
                let value = self.of(*value);
                let lists = self.of(*list);
                self.store_all(&lists, &value)
            }
            Inst::SetGlobal { value, .. } => {
                let value = self.of(*value);
                self.store(Obj::World, &value)
            }
            _ => false,
        }
    }

    fn assign(&mut self, dest: LocalId, value: &Rvalue) -> bool {
        let mut changed = match value {
            Rvalue::Call(f, args) => self.call_stores(*f, args),
            // An exception may outlive the call, raised or stored, and
            // render the list later: as good as keeping it in a global.
            Rvalue::NewExc(_, Some(a)) => {
                let held = self.of(*a);
                self.store(Obj::World, &held)
            }
            _ => false,
        };
        if !holds_list(self.program, self.func.locals[dest.0]) {
            return changed;
        }
        let fresh = Obj::Fresh(dest.0);
        match value {
            Rvalue::NewSeq(items) => {
                let items = self.union(items);
                changed |= self.store(fresh, &items);
            }
            // A shallow copy shares the elements, not the list.
            Rvalue::CopyList(a) if self.elements_hold_list(dest) => {
                let elements = self.contents(&self.of(*a));
                changed |= self.store(fresh, &elements);
            }
            _ => {}
        }
        let gains = self.value(dest, value);
        let pts = &mut self.pts[dest.0];
        let before = pts.len();
        pts.extend(gains);
        changed | (pts.len() > before)
    }

    /// The lists the value `rv` gives `dest` may be.
    fn value(&self, dest: LocalId, rv: &Rvalue) -> Objs {
        match rv {
            Rvalue::Use(a) | Rvalue::Wrap(a) | Rvalue::Unwrap(a) => self.of(*a),
            Rvalue::Index(a, _) | Rvalue::Field(a, _) => self.contents(&self.of(*a)),
            Rvalue::NewSeq(_) | Rvalue::CopyList(_) => Objs::from([Obj::Fresh(dest.0)]),
            Rvalue::Global(_) => Objs::from([Obj::World]),
            Rvalue::Call(f, args) => self.call_result(*f, args, dest),
            _ => Objs::new(),
        }
    }

    /// A new list, or what the callee may return of its arguments and
    /// the module's values.
    fn call_result(&self, f: FuncId, args: &[Operand], dest: LocalId) -> Objs {
        let summary = &self.summaries[f.0];
        let aliased = args.iter().zip(&summary.params);
        let aliased: Vec<Operand> = aliased
            .filter(|(_, p)| p.result_aliases)
            .map(|(a, _)| *a)
            .collect();
        let mut out = self.reachable(&self.union(&aliased));
        out.insert(Obj::Fresh(dest.0));
        if summary.result_global {
            out.insert(Obj::World);
        }
        out
    }

    /// A callee that keeps an argument may store it in the module's
    /// values or in any list it may change; one may leave a global's
    /// list inside an argument.
    fn call_stores(&mut self, f: FuncId, args: &[Operand]) -> bool {
        let params = &self.summaries[f.0].params;
        let mut changed = false;
        for (a, _) in args.iter().zip(params).filter(|(_, p)| p.holds_global) {
            let inside = self.reachable(&self.of(*a));
            changed |= self.store_all(&inside, &Objs::from([Obj::World]));
        }
        let kept = self.passed(args, params, Mode::Owned);
        if kept.is_empty() {
            return changed;
        }
        let mut targets = self.passed(args, params, Mode::Mut);
        targets.insert(Obj::World);
        self.store_all(&targets, &kept) | changed
    }

    /// Every list reachable from an argument whose mode is at least
    /// `min`.
    fn passed(&self, args: &[Operand], params: &[Param], min: Mode) -> Objs {
        let ops: Vec<Operand> = args
            .iter()
            .zip(params)
            .filter(|(_, p)| p.mode >= min)
            .map(|(a, _)| *a)
            .collect();
        self.reachable(&self.union(&ops))
    }

    /// The summary the settled points-to sets give.
    fn summary(&self) -> Summary {
        let mut mutated = Objs::new();
        let mut kept = Objs::new();
        let mut returned = Objs::new();
        for block in &self.func.blocks {
            for inst in &block.insts {
                self.effects(inst, &mut mutated, &mut kept);
            }
            if let Term::Return(op) = &block.term {
                returned.extend(self.reachable(&self.of(*op)));
            }
        }
        let arity = self.func.params.len();
        let regions: Vec<Objs> = (0..arity).map(|i| self.region(Obj::Param(i))).collect();
        let world = self.region(Obj::World);
        let params = regions.iter().enumerate().map(|(i, region)| {
            let others = regions.iter().enumerate().filter(|(j, _)| *j != i);
            let mut others = others.map(|(_, r)| r).chain([&world]);
            let stored_elsewhere = others.any(|r| self.stores_into(r, region));
            let mode = match (
                stored_elsewhere || meets(&kept, region),
                meets(&mutated, region),
            ) {
                (true, _) => Mode::Owned,
                (false, true) => Mode::Mut,
                (false, false) => Mode::Read,
            };
            let inside = self.reachable(&Objs::from([Obj::Param(i)]));
            Param {
                mode,
                result_aliases: meets(&returned, region),
                holds_global: meets(&inside, &world),
            }
        });
        Summary {
            params: params.collect(),
            result_global: meets(&returned, &world),
        }
    }

    /// `root` and what is stored in it, transitively, short of another
    /// parameter or the module's values.
    pub(super) fn region(&self, root: Obj) -> Objs {
        let mut out = Objs::from([root]);
        let mut todo = vec![root];
        while let Some(o) = todo.pop() {
            let Some(inner) = self.stored.get(&o) else {
                continue;
            };
            for i in inner.iter().filter(|i| matches!(i, Obj::Fresh(_))) {
                if out.insert(*i) {
                    todo.push(*i);
                }
            }
        }
        out
    }

    /// Whether a list in `from` may hold a list of `region`.
    fn stores_into(&self, from: &Objs, region: &Objs) -> bool {
        from.iter()
            .filter_map(|o| self.stored.get(o))
            .any(|s| meets(s, region))
    }

    fn effects(&self, inst: &Inst, mutated: &mut Objs, kept: &mut Objs) {
        match inst {
            Inst::Assign {
                value: Rvalue::Call(f, args),
                ..
            } => {
                let params = &self.summaries[f.0].params;
                mutated.extend(self.passed(args, params, Mode::Mut));
                kept.extend(self.passed(args, params, Mode::Owned));
            }
            Inst::Append { list, .. } | Inst::SetItem { list, .. } => {
                mutated.extend(self.of(*list));
            }
            _ => {}
        }
    }

    /// Stores `value` in each of `lists`; whether any grew.
    fn store_all(&mut self, lists: &Objs, value: &Objs) -> bool {
        let mut changed = false;
        for o in lists {
            changed |= self.store(*o, value);
        }
        changed
    }

    fn store(&mut self, list: Obj, value: &Objs) -> bool {
        let stored = self.stored.entry(list).or_default();
        let before = stored.len();
        stored.extend(value.iter().copied());
        stored.len() > before
    }

    /// What `lists` may hold: what was stored in them, and each list
    /// itself, standing for the lists inside it nothing stored.
    fn contents(&self, lists: &Objs) -> Objs {
        let mut out = lists.clone();
        for o in lists {
            if let Some(s) = self.stored.get(o) {
                out.extend(s.iter().copied());
            }
        }
        out
    }

    /// The lists `op` may be, or hold, anywhere in the body.
    pub(super) fn points_to(&self, op: Operand) -> Objs {
        self.of(op)
    }

    /// What the body may store in `list`.
    pub(super) fn stored(&self, list: Obj) -> Option<&Objs> {
        self.stored.get(&list)
    }

    /// `lists` and everything inside them.
    pub(super) fn reachable(&self, lists: &Objs) -> Objs {
        let mut out = lists.clone();
        let mut todo: Vec<Obj> = lists.iter().copied().collect();
        while let Some(o) = todo.pop() {
            let Some(inner) = self.stored.get(&o) else {
                continue;
            };
            for i in inner {
                if out.insert(*i) {
                    todo.push(*i);
                }
            }
        }
        out
    }

    /// Whether the list `dest` holds elements that can hold a list.
    fn elements_hold_list(&self, dest: LocalId) -> bool {
        match self.program.compound(self.func.locals[dest.0]) {
            Some(Compound::List(elem)) => holds_list(self.program, *elem),
            _ => false,
        }
    }

    fn union(&self, ops: &[Operand]) -> Objs {
        ops.iter().flat_map(|op| self.of(*op)).collect()
    }

    fn of(&self, op: Operand) -> Objs {
        match op {
            Operand::Local(l) => self.pts[l.0].clone(),
            _ => Objs::new(),
        }
    }
}

/// Whether `a` and `b` share a list.
fn meets(a: &Objs, b: &Objs) -> bool {
    a.iter().any(|o| b.contains(o))
}
