//! One function's summary, given its callees' current summaries.
//!
//! A flow-insensitive points-to analysis over abstract lists: each
//! parameter, the module's values ([`Obj::World`]), and one list per
//! local that allocates. A parameter stands for itself and every list
//! inside it, so `xs[0]` of a parameter is the parameter. Points-to sets
//! are found to a fixpoint first, then read off for the summary. Being
//! flow-insensitive, it is sound for slots assigned more than once.

use std::collections::{BTreeMap, BTreeSet};

use super::{holds_list, Mode, Param, Summary};
use crate::ir::{Compound, Func, FuncId, Inst, LocalId, Operand, Program, Rvalue, Term};

/// An abstract list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Obj {
    /// Parameter `i`, and every list inside it.
    Param(usize),
    /// What the module's globals hold, and every list inside it.
    World,
    /// Every list the allocations into local `i` give.
    Fresh(usize),
}

type Objs = BTreeSet<Obj>;

struct Flow<'a> {
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
    let mut flow = Flow::new(program, func, summaries);
    while flow.propagate() {}
    flow.summary()
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
            Inst::Append { list, value } | Inst::SetItem { list, value, .. } => {
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
    /// values or in any list it may change.
    fn call_stores(&mut self, f: FuncId, args: &[Operand]) -> bool {
        let params = &self.summaries[f.0].params;
        let kept = self.passed(args, params, Mode::Owned);
        if kept.is_empty() {
            return false;
        }
        let mut targets = self.passed(args, params, Mode::Mut);
        targets.insert(Obj::World);
        self.store_all(&targets, &kept)
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
        // Stored in a list the caller or the module can see: kept.
        let visible = self
            .stored
            .iter()
            .filter(|(o, _)| !matches!(o, Obj::Fresh(_)));
        let visible: Objs = visible.flat_map(|(_, s)| s.iter().copied()).collect();
        kept.extend(self.reachable(&visible));
        let params = (0..self.func.params.len()).map(|i| {
            let p = Obj::Param(i);
            let mode = match (kept.contains(&p), mutated.contains(&p)) {
                (true, _) => Mode::Owned,
                (false, true) => Mode::Mut,
                (false, false) => Mode::Read,
            };
            Param {
                mode,
                result_aliases: returned.contains(&p),
            }
        });
        Summary {
            params: params.collect(),
            result_global: returned.contains(&Obj::World),
        }
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

    /// What `lists` may hold: what was stored in them, and a parameter
    /// or the module's values hold themselves.
    fn contents(&self, lists: &Objs) -> Objs {
        let mut out = Objs::new();
        for o in lists {
            if !matches!(o, Obj::Fresh(_)) {
                out.insert(*o);
            }
            if let Some(s) = self.stored.get(o) {
                out.extend(s.iter().copied());
            }
        }
        out
    }

    /// `lists` and everything inside them.
    fn reachable(&self, lists: &Objs) -> Objs {
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
