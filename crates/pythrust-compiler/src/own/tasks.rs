//! Rule 4 between tasks (M6c, `docs/research/concurrency.md#rule-4-in-full`).
//!
//! In each executor's `with` block, the parties are the submitter (its
//! code from a task's call to the block's end, the calls that start
//! tasks left out) and each task. Two parties conflict when one changes
//! or keeps a list the other touches; touching a value touches every
//! list inside it. A call that runs more than once (`map`, or a
//! `submit` in a loop) is a party against itself. A task's result
//! carries its function's aliasing ([`super::body`]'s call results).
//!
//! The module's values are one abstract list to [`body`], so they are
//! told apart here by variable: a party reads the list globals its code
//! and its callees load, and a conflict needs one that both read, while
//! one may change the module's values. Should a module value hold
//! another, or two variables name one list, every list global counts
//! as read by any party that reads one. A list of the function's own
//! that a module value or an exception may hold is the module's too:
//! touching it reads the globals the function loads, changing it
//! changes the module's values, and an exception reaches all of them.
//!
//! No file reaches a task: not in its arguments' types, nor in a module
//! variable or default it reads. `map` runs no function that changes or
//! keeps its parameter.

use std::collections::BTreeSet;

use super::alias::{inst_operands, inst_span};
use super::body::{self, Flow, Obj, Objs};
use super::{holds_list, Mode, Summary};
use crate::diag::{DiagKind, Diagnostic, Result};
use crate::ir::{
    BlockId, Compound, Func, GlobalId, Inst, LocalId, Operand, Program, Rvalue, Task, Term, Ty,
};
use crate::span::Span;

/// Checks every executor block of `program` against rule 4.
pub(super) fn check(program: &Program, summaries: &[Summary]) -> Result<()> {
    let world = World::new(program, summaries);
    for (i, func) in program.funcs.iter().enumerate() {
        if func.regions.is_empty() {
            continue;
        }
        let flow = body::settle(program, func, summaries);
        let mut held = flow.reachable(&Objs::from([Obj::World]));
        held.remove(&Obj::World);
        let cx = Cx {
            program,
            summaries,
            world: &world,
            func,
            index: i,
            flow: &flow,
            held,
        };
        func.regions.iter().try_for_each(|r| cx.region(r))?;
    }
    Ok(())
}

/// Per function, the module's values it reaches, with its callees'.
struct World {
    /// The globals holding a list it loads.
    lists: Vec<BTreeSet<GlobalId>>,
    /// The globals holding a file it loads.
    files: Vec<BTreeSet<GlobalId>>,
    /// Whether it may change or keep a list the module's values hold.
    changes: Vec<bool>,
}

impl World {
    fn new(program: &Program, summaries: &[Summary]) -> World {
        let n = program.funcs.len();
        let mut world = World {
            lists: vec![BTreeSet::new(); n],
            files: vec![BTreeSet::new(); n],
            changes: vec![false; n],
        };
        let mut callees = vec![BTreeSet::new(); n];
        let mut nested = false;
        for (i, func) in program.funcs.iter().enumerate() {
            let flow = body::settle(program, func, summaries);
            nested |= holds_world(&flow);
            for inst in func.blocks.iter().flat_map(|b| &b.insts) {
                world.direct(program, &flow, i, inst);
                if let Inst::Assign {
                    value: Rvalue::Call(f, _),
                    ..
                } = inst
                {
                    callees[i].insert(f.0);
                }
            }
        }
        world.close(&callees);
        if nested {
            world.widen(program);
        }
        world
    }

    /// What `inst` of function `i` reaches of the module's values itself.
    fn direct(&mut self, program: &Program, flow: &Flow, i: usize, inst: &Inst) {
        if let Inst::Assign {
            value: Rvalue::Global(g),
            ..
        } = inst
        {
            let ty = program.globals[g.0];
            if holds_list(program, ty) {
                self.lists[i].insert(*g);
            }
            if holds(program, ty, Ty::File) {
                self.files[i].insert(*g);
            }
        }
        self.changes[i] |=
            matches!(inst, Inst::SetGlobal { .. }) || changes(flow, inst).contains(&Obj::World);
    }

    /// Adds each function's callees' reach to its own, to a fixpoint.
    fn close(&mut self, callees: &[BTreeSet<usize>]) {
        let mut grew = true;
        while grew {
            grew = false;
            for (i, cs) in callees.iter().enumerate() {
                for c in cs {
                    grew |= extend(&mut self.lists, i, *c);
                    grew |= extend(&mut self.files, i, *c);
                    if self.changes[*c] && !self.changes[i] {
                        self.changes[i] = true;
                        grew = true;
                    }
                }
            }
        }
    }

    /// Every list global, for each function that loads one: the module's
    /// values may hold one another.
    fn widen(&mut self, program: &Program) {
        let all: BTreeSet<GlobalId> = (0..program.globals.len())
            .map(GlobalId)
            .filter(|g| holds_list(program, program.globals[g.0]))
            .collect();
        for lists in self.lists.iter_mut().filter(|l| !l.is_empty()) {
            *lists = all.clone();
        }
    }
}

/// Adds `sets[from]` to `sets[into]`; whether it grew.
fn extend(sets: &mut [BTreeSet<GlobalId>], into: usize, from: usize) -> bool {
    let add = sets[from].clone();
    let before = sets[into].len();
    sets[into].extend(add);
    sets[into].len() > before
}

/// Whether a body may store a module value's list in a module value.
fn holds_world(flow: &Flow) -> bool {
    let Some(inner) = flow.stored(Obj::World) else {
        return false;
    };
    flow.reachable(inner).contains(&Obj::World)
}

/// The lists `inst` changes or keeps.
fn changes(flow: &Flow, inst: &Inst) -> Objs {
    let (mut mutated, mut kept) = (Objs::new(), Objs::new());
    flow.effects(inst, &mut mutated, &mut kept);
    if let Inst::Sort(list) = inst {
        mutated.extend(flow.points_to(*list));
    }
    mutated.extend(kept);
    mutated
}

/// What one party does, in the submitter's lists.
struct Party {
    touched: Objs,
    changed: Objs,
    /// The list globals it loads.
    reads: BTreeSet<GlobalId>,
    changes_world: bool,
    span: Span,
}

/// A list two parties share, one changing it.
enum Shared {
    List(Obj),
    Global(GlobalId),
}

impl Party {
    /// What `self` and `other` share where one changes it, and whether
    /// `self` is the one.
    fn conflict(&self, other: &Party) -> Option<(Shared, bool)> {
        if let Some(o) = self.changed.iter().find(|o| other.touched.contains(o)) {
            return Some((Shared::List(*o), true));
        }
        if let Some(o) = other.changed.iter().find(|o| self.touched.contains(o)) {
            return Some((Shared::List(*o), false));
        }
        if !self.changes_world && !other.changes_world {
            return None;
        }
        let g = self.reads.iter().find(|g| other.reads.contains(g))?;
        Some((Shared::Global(*g), self.changes_world))
    }
}

/// One function's executor blocks being checked.
struct Cx<'a> {
    program: &'a Program,
    summaries: &'a [Summary],
    world: &'a World,
    func: &'a Func,
    index: usize,
    flow: &'a Flow<'a>,
    /// The function's lists a module value or an exception may hold.
    held: Objs,
}

/// A task's call: where it is, and what it calls with what.
struct Call<'a> {
    at: (BlockId, usize),
    task: Task,
    callee: usize,
    args: &'a [Operand],
    span: Span,
}

impl<'a> Cx<'a> {
    fn region(&self, blocks: &BTreeSet<BlockId>) -> Result<()> {
        let mut calls: Vec<Call> = self.calls(blocks);
        calls.sort_by_key(|c| c.span.start);
        calls.iter().try_for_each(|c| self.reaches_task(c))?;
        let parties: Vec<Party> = calls.iter().map(|c| self.task(c)).collect();
        for (k, (c, p)) in calls.iter().zip(&parties).enumerate() {
            if self.repeats(c, blocks) {
                if let Some((shared, _)) = p.conflict(p) {
                    let x = self.name(&shared);
                    return Err(error(
                        p.span,
                        format!("the tasks this call starts share {x}, and they change it"),
                    ));
                }
            }
            for q in &parties[..k] {
                if let Some((shared, _)) = p.conflict(q) {
                    let x = self.name(&shared);
                    return Err(error(
                        p.span,
                        format!("two tasks of this block share {x}, and one changes it"),
                    ));
                }
            }
        }
        self.submitter(&calls, &parties, blocks)
    }

    /// The calls in `blocks` that run tasks.
    fn calls(&self, blocks: &BTreeSet<BlockId>) -> Vec<Call<'a>> {
        let mut out = Vec::new();
        for b in blocks {
            for (k, inst) in self.func.blocks[b.0].insts.iter().enumerate() {
                let Inst::Assign {
                    dest,
                    value: Rvalue::Call(f, args),
                    span,
                } = inst
                else {
                    continue;
                };
                let Some(task) = self.func.tasks.iter().find(|t| t.dest == *dest) else {
                    continue;
                };
                out.push(Call {
                    at: (*b, k),
                    task: *task,
                    callee: f.0,
                    args,
                    span: *span,
                });
            }
        }
        out
    }

    /// Whether `c` may start more than one task: `map`, or a call on a
    /// cycle through the block.
    fn repeats(&self, c: &Call, blocks: &BTreeSet<BlockId>) -> bool {
        c.task.map || self.after(c.at.0, blocks).contains(&c.at.0)
    }

    /// The blocks of `blocks` reachable from `from`'s successors.
    fn after(&self, from: BlockId, blocks: &BTreeSet<BlockId>) -> BTreeSet<BlockId> {
        let mut seen = BTreeSet::new();
        let mut todo = successors(&self.func.blocks[from.0].term);
        while let Some(b) = todo.pop() {
            if !blocks.contains(&b) || !seen.insert(b) {
                continue;
            }
            todo.extend(successors(&self.func.blocks[b.0].term));
        }
        seen
    }

    /// No file reaches the task, and `map` runs no function that changes
    /// or keeps its parameter.
    fn reaches_task(&self, c: &Call) -> Result<()> {
        let callee = &self.program.funcs[c.callee];
        let name = &callee.name;
        let param = |k: usize| {
            callee
                .params
                .get(k)
                .and_then(|p| callee.names.get(p))
                .map_or("?", String::as_str)
        };
        for (k, p) in callee.params.iter().enumerate() {
            if holds(self.program, callee.locals[p.0], Ty::File) {
                let msg = format!(
                    "a file cannot reach a task: parameter `{}` of `{name}` holds one",
                    param(k)
                );
                return Err(error(c.span, msg));
            }
        }
        if let Some(g) = self.world.files[c.callee].first() {
            let g = self.global_name(*g);
            let msg = format!("a file cannot reach a task: `{name}` reads {g}, which holds one");
            return Err(error(c.span, msg));
        }
        let changes = self.summaries[c.callee]
            .params
            .first()
            .is_some_and(|p| p.mode >= Mode::Mut);
        if c.task.map && changes {
            let msg = format!(
                "`map` cannot run `{name}`, which changes or keeps its parameter `{}`",
                param(0)
            );
            return Err(error(c.span, msg));
        }
        Ok(())
    }

    /// The party a task is.
    fn task(&self, c: &Call) -> Party {
        let touched = self.reach(c.args);
        let changed = self
            .flow
            .passed(c.args, &self.summaries[c.callee].params, Mode::Mut);
        let mut reads = self.world.lists[c.callee].clone();
        if touched.contains(&Obj::World) {
            reads.extend(self.world.lists[self.index].iter().copied());
        }
        let changes_world = self.world.changes[c.callee] || self.changes_world(&changed);
        party(touched, changed, reads, changes_world, c.span)
    }

    /// Checks the submitter's code after each task's call, to the end
    /// of `blocks`, against that task.
    fn submitter(&self, calls: &[Call], tasks: &[Party], blocks: &BTreeSet<BlockId>) -> Result<()> {
        let starts: BTreeSet<LocalId> = calls.iter().map(|c| c.task.dest).collect();
        for (c, t) in calls.iter().zip(tasks) {
            for (b, from) in self.active(c, blocks) {
                // An instruction without a span reports the one before it.
                let mut span = c.span;
                for inst in &self.func.blocks[b.0].insts[from..] {
                    span = inst_span(inst).unwrap_or(span);
                    if matches!(inst, Inst::Assign { dest, .. } if starts.contains(dest)) {
                        continue;
                    }
                    let Some((shared, mine)) = self.inst(inst, span).conflict(t) else {
                        continue;
                    };
                    let x = self.name(&shared);
                    let how = if mine { "read" } else { "changed" };
                    let msg = format!("{x} is {how} by a task until the `with` block ends");
                    return Err(error(span, msg));
                }
            }
        }
        Ok(())
    }

    /// Where the submitter's code runs while `c`'s task may: the rest
    /// of the call's block, and each block of `blocks` after it.
    fn active(&self, c: &Call, blocks: &BTreeSet<BlockId>) -> Vec<(BlockId, usize)> {
        let (b, k) = c.at;
        let later = self.after(b, blocks);
        let rest = (!later.contains(&b)).then_some((b, k + 1));
        later.into_iter().map(|l| (l, 0)).chain(rest).collect()
    }

    /// The party one of the submitter's instructions is.
    fn inst(&self, inst: &Inst, span: Span) -> Party {
        let touched = self.reach(&inst_operands(inst));
        let changed = changes(self.flow, inst);
        let mut reads = BTreeSet::new();
        let mut changes_world = self.changes_world(&changed);
        match inst {
            Inst::Assign {
                value: Rvalue::Global(g),
                ..
            } if holds_list(self.program, self.program.globals[g.0]) => {
                reads.insert(*g);
            }
            Inst::Assign {
                value: Rvalue::Call(f, _),
                ..
            } => {
                reads.extend(self.world.lists[f.0].iter().copied());
                changes_world |= self.world.changes[f.0];
            }
            _ => {}
        }
        if touched.contains(&Obj::World) {
            reads.extend(self.world.lists[self.index].iter().copied());
        }
        party(touched, changed, reads, changes_world, span)
    }

    /// The lists `ops` reach, the module's values among them should
    /// one be held by a module value or an exception.
    fn reach(&self, ops: &[Operand]) -> Objs {
        let mut objs: Objs = ops.iter().flat_map(|a| self.flow.points_to(*a)).collect();
        if ops.iter().any(|a| self.holds_exc(*a)) {
            objs.insert(Obj::World);
        }
        let mut touched = self.flow.reachable(&objs);
        if touched.iter().any(|o| self.held.contains(o)) {
            touched.insert(Obj::World);
        }
        touched
    }

    /// Whether changing `changed` changes the module's values.
    fn changes_world(&self, changed: &Objs) -> bool {
        changed.contains(&Obj::World) || changed.iter().any(|o| self.held.contains(o))
    }

    fn holds_exc(&self, op: Operand) -> bool {
        let Operand::Local(l) = op else {
            return false;
        };
        holds(self.program, self.func.locals[l.0], Ty::Exc)
    }

    /// What a diagnostic calls `shared`.
    fn name(&self, shared: &Shared) -> String {
        match shared {
            Shared::Global(g) => self.global_name(*g),
            Shared::List(o) => self.list_name(*o),
        }
    }

    fn global_name(&self, g: GlobalId) -> String {
        let name = self.program.global_names.get(&g);
        name.map_or_else(|| "a parameter's default".to_string(), |n| format!("`{n}`"))
    }

    /// A source variable that may be `o`, preferring a parameter, else
    /// one that may hold it.
    fn list_name(&self, o: Obj) -> String {
        let func = self.func;
        if let Obj::Param(i) = o {
            if let Some(n) = func.params.get(i).and_then(|p| func.names.get(p)) {
                return format!("`{n}`");
            }
        }
        let is = |l: LocalId| self.flow.points_to(Operand::Local(l)).contains(&o);
        if let Some((_, n)) = func.names.iter().find(|(l, _)| is(**l)) {
            return format!("`{n}`");
        }
        let holds = |l: LocalId| {
            let objs = self.flow.points_to(Operand::Local(l));
            self.flow.reachable(&objs).contains(&o)
        };
        let holder = func.names.iter().find(|(l, _)| holds(**l));
        holder.map_or_else(|| "a list".to_string(), |(_, n)| format!("a list in `{n}`"))
    }
}

/// A party, the module's values left to `reads` and `changes_world`.
fn party(
    mut touched: Objs,
    mut changed: Objs,
    reads: BTreeSet<GlobalId>,
    changes_world: bool,
    span: Span,
) -> Party {
    touched.remove(&Obj::World);
    changed.remove(&Obj::World);
    Party {
        touched,
        changed,
        reads,
        changes_world,
        span,
    }
}

/// Whether a value of `ty` is or holds a `want` (a file, an exception).
fn holds(program: &Program, ty: Ty, want: Ty) -> bool {
    if ty == want {
        return true;
    }
    match program.compound(ty) {
        Some(Compound::List(t) | Compound::Set(t) | Compound::Opt(t) | Compound::View(_, t)) => {
            holds(program, *t, want)
        }
        Some(Compound::Dict(k, v)) => holds(program, *k, want) || holds(program, *v, want),
        Some(Compound::Tuple(fields)) => fields.iter().any(|f| holds(program, *f, want)),
        None => false,
    }
}

fn successors(term: &Term) -> Vec<BlockId> {
    match term {
        Term::Goto(b) => vec![*b],
        Term::Branch { then, orelse, .. } => vec![*then, *orelse],
        _ => Vec::new(),
    }
}

fn error(span: Span, message: String) -> Diagnostic {
    Diagnostic {
        span,
        message,
        kind: DiagKind::Type,
    }
}
