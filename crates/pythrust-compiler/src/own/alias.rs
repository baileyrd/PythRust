//! Aliasing in tempered bodies (grammar.md D3, D4).
//!
//! D4: two names for one list, with a change through either while the
//! other is still used, is an error; so is changing a list a `for` loop
//! is running over. D3: a tempered function may not return a parameter
//! or anything sharing a list with one.
//!
//! Non-lexical: a forward pass over the CFG gives each local, at each
//! point, the [`Loc`]s it may be, with strong updates; a backward pass
//! gives which locals are still used. A name only aliases what it may
//! be *the same list* as, so `r = rows[0]` and `rows` are not two names
//! for one list, while `b = a` and `b`, `a` are, and so are `pair[0]`
//! and `pair[1]` of a tuple from elsewhere, which may be `(xs, xs)`.
//! The heap is the settled, flow-insensitive [`Flow`] of the summaries.
//!
//! Passing one list twice to a call is A5's check (`own::calls`), not
//! this one.

use std::collections::{BTreeMap, BTreeSet};

use super::body::{self, Flow, Obj, Objs};
use super::{holds_list, Mode, Summary};
use crate::ast::{Mode as Temper, Module, StmtKind};
use crate::diag::{DiagKind, Diagnostic, Result};
use crate::ir::{BlockId, Func, FuncId, Inst, LocalId, Operand, Program, Rvalue, Term, Ty};
use crate::span::Span;

/// Where inside an abstract list a value is.
///
/// Only the depth: two slots of one list or tuple at the same depth may
/// hold the same list (`(xs, xs)`), and lists at different depths have
/// different types, so they never are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum At {
    /// This many steps inside the list (0: the list itself).
    Depth(usize),
    /// The list or any list inside it.
    Anywhere,
}

/// A list a local may be.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Loc {
    obj: Obj,
    /// For [`Obj::Fresh`]: one of the lists its earlier runs made, not
    /// the latest, so `row = []` in a loop is a new list.
    earlier: bool,
    at: At,
}

type Locs = BTreeSet<Loc>;

/// What a local may be.
#[derive(Debug, Clone, Default, PartialEq)]
struct Val {
    locs: Locs,
    /// For a tuple this body built, what each field may be: unlike a
    /// tuple from elsewhere, its fields are known apart.
    fields: Option<Vec<Locs>>,
}

/// Per local, what it may be.
type State = Vec<Val>;

/// How a change reaches lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Change {
    /// Only the list itself: `append`, item assignment.
    List,
    /// The list and every list inside it: a `mut` argument.
    Within,
}

/// The first D3 or D4 error of each tempered function `module` defines.
pub(super) fn tempered(
    module: &Module,
    program: &Program,
    summaries: &[Summary],
) -> BTreeMap<String, Diagnostic> {
    let names: BTreeSet<&str> = module
        .body
        .iter()
        .filter_map(|s| match &s.kind {
            StmtKind::FuncDef(f) if f.mode == Temper::Tempered => Some(f.name.name.as_str()),
            _ => None,
        })
        .collect();
    let mut out = BTreeMap::new();
    for func in &program.funcs {
        if !names.contains(func.name.as_str()) || out.contains_key(&func.name) {
            continue;
        }
        if let Err(d) = check(program, func, summaries) {
            out.insert(func.name.clone(), d);
        }
    }
    out
}

/// Checks tempered `func`'s body.
fn check(program: &Program, func: &Func, summaries: &[Summary]) -> Result<()> {
    let body = Body::new(program, func, summaries);
    let entries = body.entries();
    let live = body.live_out();
    for (b, entry) in entries.iter().enumerate() {
        let Some(entry) = entry else { continue };
        body.replay(&func.blocks[b].insts, entry.clone(), &live[b])?;
    }
    Ok(())
}

struct Body<'a> {
    program: &'a Program,
    func: &'a Func,
    summaries: &'a [Summary],
    flow: Flow<'a>,
    /// Per local, the source variables it was read from: itself for a
    /// variable, for a temporary the variables its value comes from.
    roots: Vec<BTreeSet<LocalId>>,
    /// The locals `return` assigns.
    results: BTreeSet<LocalId>,
}

impl<'a> Body<'a> {
    fn new(program: &'a Program, func: &'a Func, summaries: &'a [Summary]) -> Body<'a> {
        let results = func.blocks.iter().filter_map(|b| match b.term {
            Term::Return(Operand::Local(l)) => Some(l),
            _ => None,
        });
        let mut body = Body {
            program,
            func,
            summaries,
            flow: body::settle(program, func, summaries),
            roots: Vec::new(),
            results: results.collect(),
        };
        body.roots = body.roots();
        body
    }

    fn lists(&self, l: LocalId) -> bool {
        holds_list(self.program, self.func.locals[l.0])
    }

    fn named(&self, l: LocalId) -> bool {
        self.func.names.contains_key(&l)
    }

    fn roots(&self) -> Vec<BTreeSet<LocalId>> {
        let mut roots: Vec<BTreeSet<LocalId>> = (0..self.func.locals.len())
            .map(|i| match self.named(LocalId(i)) {
                true => BTreeSet::from([LocalId(i)]),
                false => BTreeSet::new(),
            })
            .collect();
        let mut changed = true;
        while changed {
            changed = false;
            for inst in self.func.blocks.iter().flat_map(|b| &b.insts) {
                let Inst::Assign { dest, value, .. } = inst else {
                    continue;
                };
                let Some(Operand::Local(src)) = projected(value) else {
                    continue;
                };
                if self.named(*dest) {
                    continue;
                }
                let from = roots[src.0].clone();
                let to = &mut roots[dest.0];
                let before = to.len();
                to.extend(from);
                changed |= to.len() > before;
            }
        }
        roots
    }

    /// Each block's entry state, `None` if unreachable.
    fn entries(&self) -> Vec<Option<State>> {
        let mut start: State = vec![Val::default(); self.func.locals.len()];
        for (i, p) in self.func.params.iter().enumerate() {
            if self.lists(*p) {
                start[p.0].locs.insert(root(Obj::Param(i)));
            }
        }
        let mut entries: Vec<Option<State>> = vec![None; self.func.blocks.len()];
        entries[0] = Some(start);
        let mut todo = vec![BlockId(0)];
        while let Some(b) = todo.pop() {
            let Some(mut state) = entries[b.0].clone() else {
                continue;
            };
            let block = &self.func.blocks[b.0];
            block
                .insts
                .iter()
                .for_each(|i| self.transfer(i, &mut state));
            for s in successors(&block.term) {
                if join(&mut entries[s.0], &state) {
                    todo.push(s);
                }
            }
        }
        entries
    }

    fn transfer(&self, inst: &Inst, state: &mut State) {
        match inst {
            Inst::Assign { dest, value, .. } if self.lists(*dest) => {
                if matches!(
                    value,
                    Rvalue::NewSeq(_) | Rvalue::CopyList(_) | Rvalue::Call(..)
                ) {
                    age(state, *dest);
                }
                state[dest.0] = self.value(*dest, value, state);
            }
            Inst::Release(l) | Inst::Forget(l) => state[l.0] = Val::default(),
            _ => {}
        }
    }

    /// What `rv` may give `dest`.
    fn value(&self, dest: LocalId, rv: &Rvalue, state: &State) -> Val {
        let fresh = Locs::from([root(Obj::Fresh(dest.0))]);
        let locs = match rv {
            Rvalue::Use(a) | Rvalue::Wrap(a) | Rvalue::Unwrap(a) => return val(state, *a),
            Rvalue::Field(a, k) => return self.field(&val(state, *a), *k),
            Rvalue::NewSeq(items) if self.is_tuple(dest) => {
                let fields = items.iter().map(|i| of(state, *i)).collect();
                return Val {
                    locs: fresh,
                    fields: Some(fields),
                };
            }
            Rvalue::Index(a, _) => self.step(&of(state, *a)),
            Rvalue::NewSeq(_) | Rvalue::CopyList(_) => fresh,
            Rvalue::Global(_) => Locs::from([root(Obj::World)]),
            Rvalue::Call(f, args) => self.call(*f, args, dest, state),
            _ => Locs::new(),
        };
        Val { locs, fields: None }
    }

    /// Field `k` of `tuple`: known apart when this body built it.
    fn field(&self, tuple: &Val, k: usize) -> Val {
        let locs = match tuple.fields.as_ref().and_then(|f| f.get(k)) {
            Some(locs) => locs.clone(),
            None => self.step(&tuple.locs),
        };
        Val { locs, fields: None }
    }

    fn is_tuple(&self, l: LocalId) -> bool {
        matches!(self.func.locals[l.0], Ty::Tuple(_))
    }

    /// A new list, or what the callee may return of its arguments and
    /// the module's values.
    fn call(&self, f: FuncId, args: &[Operand], dest: LocalId, state: &State) -> Locs {
        let summary = &self.summaries[f.0];
        let mut out = Locs::from([root(Obj::Fresh(dest.0))]);
        let aliased = args
            .iter()
            .zip(&summary.params)
            .filter(|(_, p)| p.result_aliases);
        for (a, _) in aliased {
            let objs: Objs = of(state, *a).iter().map(|l| l.obj).collect();
            out.extend(self.flow.reachable(&objs).into_iter().map(anywhere));
        }
        if summary.result_global {
            out.insert(anywhere(Obj::World));
        }
        out
    }

    /// The lists one step inside `locs` may be: a step further into
    /// each, or anything the body stored in it.
    fn step(&self, locs: &Locs) -> Locs {
        let mut out = Locs::new();
        for l in locs {
            let at = match l.at {
                At::Depth(d) => At::Depth(d + 1),
                At::Anywhere => At::Anywhere,
            };
            out.insert(Loc { at, ..l.clone() });
            let stored = self.flow.stored(l.obj).into_iter().flatten();
            out.extend(stored.map(|o| anywhere(*o)));
        }
        out
    }

    /// Each block's locals still used after it ends.
    fn live_out(&self) -> Vec<BTreeSet<LocalId>> {
        let blocks = &self.func.blocks;
        let mut out = vec![BTreeSet::new(); blocks.len()];
        let mut changed = true;
        while changed {
            changed = false;
            for (b, block) in blocks.iter().enumerate().rev() {
                let mut live = BTreeSet::new();
                for s in successors(&block.term) {
                    live.extend(self.live_in(s, &out[s.0]));
                }
                if live != out[b] {
                    out[b] = live;
                    changed = true;
                }
            }
        }
        out
    }

    /// The locals used at the start of block `b`, given those after it.
    fn live_in(&self, b: BlockId, after: &BTreeSet<LocalId>) -> BTreeSet<LocalId> {
        let block = &self.func.blocks[b.0];
        let mut live = after.clone();
        self.used(term_operands(&block.term), &mut live);
        for inst in block.insts.iter().rev() {
            self.step_back(inst, &mut live);
        }
        live
    }

    /// `live` before `inst`, given it after.
    fn step_back(&self, inst: &Inst, live: &mut BTreeSet<LocalId>) {
        match inst {
            Inst::Assign { dest, .. } | Inst::Release(dest) | Inst::Forget(dest) => {
                live.remove(dest);
            }
            _ => {}
        }
        self.used(inst_operands(inst), live);
    }

    fn used(&self, ops: Vec<Operand>, live: &mut BTreeSet<LocalId>) {
        for op in ops {
            if let Operand::Local(l) = op {
                if self.lists(l) {
                    live.insert(l);
                }
            }
        }
    }

    /// Checks `insts` from `state`, `live` being used after them.
    fn replay(&self, insts: &[Inst], mut state: State, live: &BTreeSet<LocalId>) -> Result<()> {
        let mut after = vec![live.clone(); insts.len()];
        let mut cur = live.clone();
        for (i, inst) in insts.iter().enumerate().rev() {
            after[i] = cur.clone();
            self.step_back(inst, &mut cur);
        }
        for (inst, live) in insts.iter().zip(&after) {
            self.inst(inst, &state, live)?;
            self.transfer(inst, &mut state);
        }
        Ok(())
    }

    fn inst(&self, inst: &Inst, state: &State, live: &BTreeSet<LocalId>) -> Result<()> {
        match inst {
            Inst::Append { list, span, .. } | Inst::SetItem { list, span, .. } => {
                self.change(*list, Change::List, state, live, *span)
            }
            Inst::Assign {
                dest, value, span, ..
            } => {
                if self.results.contains(dest) {
                    self.returned(&self.value(*dest, value, state).locs, *span)?;
                }
                let Rvalue::Call(f, args) = value else {
                    return Ok(());
                };
                let params = &self.summaries[f.0].params;
                let changed = args.iter().zip(params).filter(|(_, p)| p.mode >= Mode::Mut);
                for (a, _) in changed {
                    self.change(*a, Change::Within, state, live, *span)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// D4: changing what `op` is, at `span`, while another name for it
    /// is used later.
    fn change(
        &self,
        op: Operand,
        how: Change,
        state: &State,
        live: &BTreeSet<LocalId>,
        span: Span,
    ) -> Result<()> {
        let Operand::Local(w) = op else {
            return Ok(());
        };
        let changed = &state[w.0].locs;
        let clash = |a: &&LocalId| **a != w && self.overlaps(&state[a.0].locs, changed, how);
        let (named, loops): (Vec<&LocalId>, Vec<&LocalId>) =
            live.iter().filter(clash).partition(|a| self.named(**a));
        let other = named.into_iter().find(|a| !self.roots[w.0].contains(a));
        let message = match (other, loops.first()) {
            (Some(a), _) => self.aliased(w, *a),
            (None, Some(_)) => format!(
                "tempered `{}` changes a list while a `for` loop over it is running; loop over a copy: `for ... in list(...)`",
                self.func.name
            ),
            (None, None) => return Ok(()),
        };
        Err(error(span, message))
    }

    fn aliased(&self, w: LocalId, other: LocalId) -> String {
        let f = &self.func.name;
        let a = self.name(other);
        match self.func.names.get(&w) {
            Some(b) => format!(
                "tempered `{f}` changes `{b}` while `{a}` may be the same list and is used later; copy instead of sharing it: `{b} = list({a})`"
            ),
            None => format!(
                "tempered `{f}` changes a list while `{a}` may be the same list and is used later; copy instead of sharing it: `{a} = list(...)`"
            ),
        }
    }

    /// D3: a result sharing a list with a parameter.
    fn returned(&self, value: &Locs, span: Span) -> Result<()> {
        let objs: Objs = value.iter().map(|l| l.obj).collect();
        let reached = self.flow.reachable(&objs);
        let f = &self.func.name;
        for (i, p) in self.func.params.iter().enumerate() {
            let region = self.flow.region(Obj::Param(i));
            if !reached.iter().any(|o| region.contains(o)) {
                continue;
            }
            let xs = self.name(*p);
            let message = match value.contains(&root(Obj::Param(i))) {
                true => format!("tempered `{f}` returns its parameter `{xs}`; return a copy: `return list({xs})`"),
                false => format!(
                    "tempered `{f}` returns a value sharing a list with its parameter `{xs}`; return a copy made with `list(...)`"
                ),
            };
            return Err(error(span, message));
        }
        Ok(())
    }

    /// Whether a name that may be `seen` sees a change to `changed`.
    fn overlaps(&self, seen: &Locs, changed: &Locs, how: Change) -> bool {
        changed.iter().any(|c| {
            let inside = match how {
                Change::List => Objs::new(),
                Change::Within => self.flow.reachable(&Objs::from([c.obj])),
            };
            seen.iter()
                .any(|s| same(s, c, how) || (s.obj != c.obj && inside.contains(&s.obj)))
        })
    }

    fn name(&self, l: LocalId) -> &str {
        self.func.names.get(&l).map_or("?", String::as_str)
    }
}

/// Whether `seen` may be the list `changed` is, or for
/// [`Change::Within`], one inside it.
fn same(seen: &Loc, changed: &Loc, how: Change) -> bool {
    if seen.obj != changed.obj {
        return false;
    }
    let (At::Depth(s), At::Depth(c)) = (seen.at, changed.at) else {
        return true;
    };
    let reaches = match how {
        Change::List => s == c,
        Change::Within => s >= c,
    };
    reaches && seen.earlier == changed.earlier
}

/// The latest list `obj` stands for, itself.
fn root(obj: Obj) -> Loc {
    Loc {
        obj,
        earlier: false,
        at: At::Depth(0),
    }
}

fn anywhere(obj: Obj) -> Loc {
    Loc {
        obj,
        earlier: false,
        at: At::Anywhere,
    }
}

/// Marks the latest list allocated into `dest` as an earlier one: `dest`
/// is about to get a new one.
fn age(state: &mut State, dest: LocalId) {
    let latest = Obj::Fresh(dest.0);
    let age_all = |locs: &mut Locs| {
        let aged = std::mem::take(locs).into_iter().map(|l| Loc {
            earlier: l.earlier || (l.obj == latest && l.at != At::Anywhere),
            ..l
        });
        *locs = aged.collect();
    };
    for v in state.iter_mut() {
        age_all(&mut v.locs);
        v.fields.iter_mut().flatten().for_each(age_all);
    }
}

/// Joins `state` into `into`; whether it grew.
fn join(into: &mut Option<State>, state: &State) -> bool {
    let Some(into) = into else {
        *into = Some(state.clone());
        return true;
    };
    let mut grew = false;
    for (a, b) in into.iter_mut().zip(state) {
        let joined = join_val(a, b);
        grew |= joined != *a;
        *a = joined;
    }
    grew
}

/// What may be `a` or `b`: fields stay apart only if both keep them.
fn join_val(a: &Val, b: &Val) -> Val {
    if *b == Val::default() {
        return a.clone();
    }
    if *a == Val::default() {
        return b.clone();
    }
    let fields = match (&a.fields, &b.fields) {
        (Some(f), Some(g)) if f.len() == g.len() => {
            Some(f.iter().zip(g).map(|(x, y)| x | y).collect())
        }
        _ => None,
    };
    Val {
        locs: &a.locs | &b.locs,
        fields,
    }
}

fn val(state: &State, op: Operand) -> Val {
    match op {
        Operand::Local(l) => state[l.0].clone(),
        _ => Val::default(),
    }
}

fn of(state: &State, op: Operand) -> Locs {
    val(state, op).locs
}

/// The operand a value is the same list as, or a list inside of.
fn projected(rv: &Rvalue) -> Option<Operand> {
    match rv {
        Rvalue::Use(a) | Rvalue::Wrap(a) | Rvalue::Unwrap(a) => Some(*a),
        Rvalue::Index(a, _) | Rvalue::Field(a, _) => Some(*a),
        _ => None,
    }
}

fn successors(term: &Term) -> Vec<BlockId> {
    match term {
        Term::Goto(b) => vec![*b],
        Term::Branch { then, orelse, .. } => vec![*then, *orelse],
        _ => Vec::new(),
    }
}

fn term_operands(term: &Term) -> Vec<Operand> {
    match term {
        Term::Branch { cond, .. } => vec![*cond],
        Term::Return(op) => vec![*op],
        _ => Vec::new(),
    }
}

/// The operands `inst` reads, not counting reference counting.
fn inst_operands(inst: &Inst) -> Vec<Operand> {
    match inst {
        Inst::Assign { value, .. } => rvalue_operands(value),
        Inst::Print { args, sep, end } => args.iter().chain(sep).chain(end).copied().collect(),
        Inst::Append { list, value, .. } => vec![*list, *value],
        Inst::SetGlobal { value, .. } => vec![*value],
        Inst::SetItem {
            list, index, value, ..
        } => vec![*list, *index, *value],
        Inst::Raise { exc, .. } => vec![*exc],
        Inst::Retain(_) | Inst::Release(_) | Inst::Forget(_) | Inst::DropGlobal(_) => Vec::new(),
    }
}

fn rvalue_operands(rv: &Rvalue) -> Vec<Operand> {
    match rv {
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
        | Rvalue::Index(a, b)
        | Rvalue::Cmp(_, a, b)
        | Rvalue::AddOverflows(a, b)
        | Rvalue::Concat(a, b)
        | Rvalue::Repeat(a, b)
        | Rvalue::Float(_, a, b)
        | Rvalue::IntPow(a, b)
        | Rvalue::Contains { hay: a, needle: b } => vec![*a, *b],
        Rvalue::Call(_, args) | Rvalue::NewSeq(args) => args.clone(),
        Rvalue::NewExc(_, arg) => arg.iter().copied().collect(),
        Rvalue::StrLit(_)
        | Rvalue::Global(_)
        | Rvalue::Raised
        | Rvalue::PendingIs(_)
        | Rvalue::Catch => Vec::new(),
    }
}

fn error(span: Span, message: String) -> Diagnostic {
    Diagnostic {
        span,
        message,
        kind: DiagKind::Type,
    }
}
