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
//! be *the same list* as. One list has one type, so inside an abstract
//! list a [`Loc`] is told apart only by type: `r = rows[0]` and `rows`
//! are not two names for one list, while `b = a` and `b`, `a` are, and
//! so are `pair[0]` and `pair[1][0]` of a `tuple[list[int],
//! list[list[int]]]` from elsewhere, which may be `(xs, [xs])`. A tuple
//! the body built keeps its fields apart. A parameter may be, or hold,
//! a list the module's globals hold; two parameters share none, which
//! A5 checks at every call to a tempered function. The heap is the
//! settled, flow-insensitive [`Flow`] of the summaries.
//!
//! Passing one list twice to a call is A5's check (`own::calls`), not
//! this one.

use std::collections::{BTreeMap, BTreeSet};

use super::body::{self, Flow, Obj, Objs};
use super::{holds_list, Mode, Summary};
use crate::ast::{Mode as Temper, Module, StmtKind};
use crate::diag::{DiagKind, Diagnostic, Result};
use crate::ir::{
    BlockId, Compound, Func, FuncId, GlobalId, Inst, LocalId, Operand, Program, Rvalue, Term, Ty,
};
use crate::span::Span;

/// A list a local may be.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Loc {
    obj: Obj,
    /// For [`Obj::Fresh`]: one of the lists its earlier runs made, not
    /// the latest, so `row = []` in a loop is a new list.
    earlier: bool,
    /// The value's type, optional unwrapped: `obj` itself or any value
    /// of this type inside it. `None`: `obj` or anything inside it.
    ty: Option<Ty>,
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

/// Something the body may use later: a local, or a module variable
/// it, or a function it calls, reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Local(LocalId),
    Global(GlobalId),
}

type Live = BTreeSet<Key>;

/// Who changes a list.
#[derive(Debug, Clone, Copy)]
enum Writer {
    /// The body, through a local.
    Local(LocalId),
    /// A callee, in a module variable's list: through that variable,
    /// if one.
    Callee(FuncId, Option<GlobalId>),
}

/// A change a function makes, itself or through a callee, to a list of
/// type `ty` the module's values hold: through module variable
/// `global`, if one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Write {
    global: Option<GlobalId>,
    ty: Ty,
    how: Change,
}

/// Per function, by `FuncId`, the module's lists it may read and change.
struct Effects {
    reads: Vec<BTreeSet<GlobalId>>,
    writes: Vec<BTreeSet<Write>>,
}

/// How a change reaches lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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
    let effects = effects(program, summaries);
    let mut out = BTreeMap::new();
    for func in &program.funcs {
        if !names.contains(func.name.as_str()) || out.contains_key(&func.name) {
            continue;
        }
        if let Err(d) = check(program, func, summaries, &effects) {
            out.insert(func.name.clone(), d);
        }
    }
    out
}

/// What every function may do to the module's lists, itself or
/// through its callees.
fn effects(program: &Program, summaries: &[Summary]) -> Effects {
    let mut effects = Effects {
        reads: reads(program),
        writes: Vec::new(),
    };
    let own: Vec<BTreeSet<Write>> = program
        .funcs
        .iter()
        .map(|f| Body::new(program, f, summaries, &effects).writes())
        .collect();
    let mut writes = own.clone();
    let mut changed = true;
    while changed {
        changed = false;
        for (i, func) in program.funcs.iter().enumerate() {
            let mut found = own[i].clone();
            for f in callees(func) {
                found.extend(writes[f.0].iter().copied());
            }
            if found.len() > writes[i].len() {
                writes[i] = found;
                changed = true;
            }
        }
    }
    effects.writes = writes;
    effects
}

fn callees(func: &Func) -> impl Iterator<Item = FuncId> + '_ {
    func.blocks
        .iter()
        .flat_map(|b| &b.insts)
        .filter_map(|i| match i {
            Inst::Assign {
                value: Rvalue::Call(f, _),
                ..
            } => Some(*f),
            _ => None,
        })
}

/// Per function, the globals it or a function it calls may read.
fn reads(program: &Program) -> Vec<BTreeSet<GlobalId>> {
    let mut reads = vec![BTreeSet::new(); program.funcs.len()];
    let mut changed = true;
    while changed {
        changed = false;
        for (i, func) in program.funcs.iter().enumerate() {
            let mut found = BTreeSet::new();
            for inst in func.blocks.iter().flat_map(|b| &b.insts) {
                let Inst::Assign { value, .. } = inst else {
                    continue;
                };
                match value {
                    Rvalue::Global(g) => {
                        found.insert(*g);
                    }
                    Rvalue::Call(f, _) => found.extend(reads[f.0].iter().copied()),
                    _ => {}
                }
            }
            if found.len() > reads[i].len() {
                reads[i] = found;
                changed = true;
            }
        }
    }
    reads
}

/// Checks tempered `func`'s body.
fn check(program: &Program, func: &Func, summaries: &[Summary], effects: &Effects) -> Result<()> {
    let body = Body::new(program, func, summaries, effects);
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
    effects: &'a Effects,
    /// Per local, the variables it was read from: itself for a variable,
    /// for a temporary the variables or module variables its value
    /// comes from.
    roots: Vec<Live>,
    /// The locals `return` assigns.
    results: BTreeSet<LocalId>,
}

impl<'a> Body<'a> {
    fn new(
        program: &'a Program,
        func: &'a Func,
        summaries: &'a [Summary],
        effects: &'a Effects,
    ) -> Body<'a> {
        let results = func.blocks.iter().filter_map(|b| match b.term {
            Term::Return(Operand::Local(l)) => Some(l),
            _ => None,
        });
        let mut body = Body {
            program,
            func,
            summaries,
            flow: body::settle(program, func, summaries),
            effects,
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

    fn roots(&self) -> Vec<Live> {
        let mut roots: Vec<Live> = (0..self.func.locals.len())
            .map(|i| match self.named(LocalId(i)) {
                true => Live::from([Key::Local(LocalId(i))]),
                false => Live::new(),
            })
            .collect();
        let mut changed = true;
        while changed {
            changed = false;
            for inst in self.func.blocks.iter().flat_map(|b| &b.insts) {
                let Inst::Assign { dest, value, .. } = inst else {
                    continue;
                };
                if self.named(*dest) {
                    continue;
                }
                let from = match (value, projected(value)) {
                    (Rvalue::Global(g), _) => Live::from([Key::Global(*g)]),
                    (_, Some(Operand::Local(src))) => roots[src.0].clone(),
                    _ => continue,
                };
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
                let ty = self.ty(*p);
                start[p.0].locs.insert(root(Obj::Param(i), ty));
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
        let ty = self.ty(dest);
        let fresh = Locs::from([root(Obj::Fresh(dest.0), ty)]);
        let locs = match rv {
            Rvalue::Use(a) | Rvalue::Wrap(a) | Rvalue::Unwrap(a) => return val(state, *a),
            Rvalue::Field(a, k) => return self.field(&val(state, *a), *k, ty),
            Rvalue::NewSeq(items) if self.is_tuple(dest) => {
                let fields = items.iter().map(|i| of(state, *i)).collect();
                return Val {
                    locs: fresh,
                    fields: Some(fields),
                };
            }
            Rvalue::Index(a, _) => self.step(&of(state, *a), ty),
            Rvalue::NewSeq(_) | Rvalue::CopyList(_) => fresh,
            Rvalue::Global(_) => Locs::from([root(Obj::World, ty)]),
            Rvalue::Call(f, args) => self.call(*f, args, dest, state),
            _ => Locs::new(),
        };
        Val { locs, fields: None }
    }

    /// Field `k`, of type `ty`, of `tuple`: known apart when this body
    /// built it.
    fn field(&self, tuple: &Val, k: usize, ty: Ty) -> Val {
        let locs = match tuple.fields.as_ref().and_then(|f| f.get(k)) {
            Some(locs) => locs.clone(),
            None => self.step(&tuple.locs, ty),
        };
        Val { locs, fields: None }
    }

    fn is_tuple(&self, l: LocalId) -> bool {
        matches!(self.func.locals[l.0], Ty::Tuple(_))
    }

    /// `l`'s type, optional unwrapped: a list is the same list either way.
    fn ty(&self, l: LocalId) -> Ty {
        let ty = self.func.locals[l.0];
        match self.program.compound(ty) {
            Some(Compound::Opt(present)) => *present,
            _ => ty,
        }
    }

    /// Whether a value of type `outer` is, or may hold, one of `inner`.
    fn within(&self, outer: Ty, inner: Ty) -> bool {
        if outer == inner {
            return true;
        }
        match self.program.compound(outer) {
            Some(Compound::List(t) | Compound::Opt(t)) => self.within(*t, inner),
            Some(Compound::Tuple(fields)) => fields.iter().any(|t| self.within(*t, inner)),
            None => false,
        }
    }

    /// A new list, or what the callee may return of its arguments and
    /// the module's values.
    fn call(&self, f: FuncId, args: &[Operand], dest: LocalId, state: &State) -> Locs {
        let summary = &self.summaries[f.0];
        let mut out = Locs::from([root(Obj::Fresh(dest.0), self.ty(dest))]);
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

    /// What a value of type `ty` one step inside `locs` may be: a value
    /// of that type inside each, or anything the body stored in it.
    fn step(&self, locs: &Locs, ty: Ty) -> Locs {
        let mut out = Locs::new();
        for l in locs {
            out.insert(Loc {
                ty: l.ty.map(|_| ty),
                ..l.clone()
            });
            // A parameter may be a global's list, which holds what the
            // body stored in the module's values.
            for o in self.meeting(l.obj) {
                let stored = self.flow.stored(o).into_iter().flatten();
                out.extend(stored.map(|s| anywhere(*s)));
            }
        }
        out
    }

    /// What each block's successors may still use.
    fn live_out(&self) -> Vec<Live> {
        let blocks = &self.func.blocks;
        let mut out = vec![Live::new(); blocks.len()];
        let mut changed = true;
        while changed {
            changed = false;
            for (b, block) in blocks.iter().enumerate().rev() {
                let mut live = Live::new();
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

    /// What is used from the start of block `b`, given what is after it.
    fn live_in(&self, b: BlockId, after: &Live) -> Live {
        let block = &self.func.blocks[b.0];
        let mut live = after.clone();
        self.used(term_operands(&block.term), &mut live);
        for inst in block.insts.iter().rev() {
            self.step_back(inst, &mut live);
        }
        live
    }

    /// `live` before `inst`, given it after.
    fn step_back(&self, inst: &Inst, live: &mut Live) {
        match inst {
            Inst::Assign { dest, .. } | Inst::Release(dest) | Inst::Forget(dest) => {
                live.remove(&Key::Local(*dest));
            }
            _ => {}
        }
        self.used(inst_operands(inst), live);
        let Inst::Assign { value, .. } = inst else {
            return;
        };
        let globals = match value {
            Rvalue::Global(g) => BTreeSet::from([*g]),
            Rvalue::Call(f, _) => self.effects.reads[f.0].clone(),
            _ => BTreeSet::new(),
        };
        let lists = globals.into_iter().filter(|g| self.global_lists(*g));
        live.extend(lists.map(Key::Global));
    }

    fn used(&self, ops: Vec<Operand>, live: &mut Live) {
        for op in ops {
            if let Operand::Local(l) = op {
                if self.lists(l) {
                    live.insert(Key::Local(l));
                }
            }
        }
    }

    fn global_lists(&self, g: GlobalId) -> bool {
        holds_list(self.program, self.program.globals[g.0])
    }

    /// The lists `key` may be.
    fn locs(&self, key: Key, state: &State) -> Locs {
        match key {
            Key::Local(l) => state[l.0].locs.clone(),
            Key::Global(g) => {
                let ty = self.program.globals[g.0];
                let ty = match self.program.compound(ty) {
                    Some(Compound::Opt(present)) => *present,
                    _ => ty,
                };
                Locs::from([root(Obj::World, ty)])
            }
        }
    }

    /// Checks `insts` from `state`, `live` being used after them.
    fn replay(&self, insts: &[Inst], mut state: State, live: &Live) -> Result<()> {
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

    fn inst(&self, inst: &Inst, state: &State, live: &Live) -> Result<()> {
        if let Inst::Assign {
            dest, value, span, ..
        } = inst
        {
            if self.results.contains(dest) {
                self.returned(&self.value(*dest, value, state).locs, *span)?;
            }
        }
        let Some(span) = inst_span(inst) else {
            return Ok(());
        };
        for (op, how) in self.changes(inst) {
            let Operand::Local(w) = op else {
                continue;
            };
            self.change(Writer::Local(w), &state[w.0].locs, how, state, live, span)?;
        }
        let Inst::Assign {
            value: Rvalue::Call(f, _),
            ..
        } = inst
        else {
            return Ok(());
        };
        for w in &self.effects.writes[f.0] {
            let changed = Locs::from([root(Obj::World, w.ty)]);
            let writer = Writer::Callee(*f, w.global);
            self.change(writer, &changed, w.how, state, live, span)?;
        }
        Ok(())
    }

    /// What `inst` changes in place, and how.
    fn changes(&self, inst: &Inst) -> Vec<(Operand, Change)> {
        match inst {
            Inst::Append { list, .. } | Inst::SetItem { list, .. } => vec![(*list, Change::List)],
            Inst::Assign {
                value: Rvalue::Call(f, args),
                ..
            } => {
                let params = &self.summaries[f.0].params;
                let changed = args.iter().zip(params).filter(|(_, p)| p.mode >= Mode::Mut);
                changed.map(|(a, _)| (*a, Change::Within)).collect()
            }
            _ => Vec::new(),
        }
    }

    /// The module's lists this body changes itself, not through a callee.
    fn writes(&self) -> BTreeSet<Write> {
        let mut out = BTreeSet::new();
        for inst in self.func.blocks.iter().flat_map(|b| &b.insts) {
            for (op, how) in self.changes(inst) {
                let Operand::Local(l) = op else {
                    continue;
                };
                if !self.flow.points_to(op).contains(&Obj::World) {
                    continue;
                }
                let mut roots = self.roots[l.0].iter();
                let global = match (roots.next(), roots.next()) {
                    (Some(Key::Global(g)), None) => Some(*g),
                    _ => None,
                };
                out.insert(Write {
                    global,
                    ty: self.ty(l),
                    how,
                });
            }
        }
        out
    }

    /// D4: `writer` changing `changed`, at `span`, while another name
    /// for it is used later.
    fn change(
        &self,
        writer: Writer,
        changed: &Locs,
        how: Change,
        state: &State,
        live: &Live,
        span: Span,
    ) -> Result<()> {
        let (itself, roots) = match writer {
            Writer::Local(w) => (Some(Key::Local(w)), self.roots[w.0].clone()),
            Writer::Callee(_, g) => (None, g.map(Key::Global).into_iter().collect()),
        };
        let clash =
            |k: &&Key| Some(**k) != itself && self.overlaps(&self.locs(**k, state), changed, how);
        let (named, loops): (Vec<&Key>, Vec<&Key>) =
            live.iter().filter(clash).partition(|k| match k {
                Key::Local(l) => self.named(*l),
                Key::Global(_) => true,
            });
        let other = named.into_iter().find(|k| !roots.contains(k));
        let message = match (other, loops.first()) {
            (Some(a), _) => self.aliased(writer, *a),
            (None, Some(_)) => format!(
                "tempered `{}` changes {} while a `for` loop over it is running; loop over a copy: `for ... in list(...)`",
                self.func.name,
                self.subject(writer)
            ),
            (None, None) => return Ok(()),
        };
        Err(error(span, message))
    }

    /// What `writer` changes, for a message: `` `b` ``, or a list.
    fn subject(&self, writer: Writer) -> String {
        match writer {
            Writer::Local(w) => self
                .func
                .names
                .get(&w)
                .map_or("a list".into(), |b| format!("`{b}`")),
            Writer::Callee(f, _) => format!("a list through `{}`", self.program.funcs[f.0].name),
        }
    }

    fn aliased(&self, writer: Writer, other: Key) -> String {
        let f = &self.func.name;
        let what = self.subject(writer);
        let a = match other {
            Key::Local(l) => self.name(l),
            Key::Global(g) => {
                let g = self
                    .program
                    .global_names
                    .get(&g)
                    .map_or("?", String::as_str);
                return format!(
                    "tempered `{f}` changes {what} while module variable `{g}` may be the same list and is used later; pass a copy instead, such as `list({g})`"
                );
            }
        };
        // A parameter is the caller's list: copy the other name instead.
        let copied = match writer {
            Writer::Local(w) if !self.func.params.contains(&w) => self.func.names.get(&w),
            _ => None,
        };
        match copied {
            Some(b) => format!(
                "tempered `{f}` changes {what} while `{a}` may be the same list and is used later; copy instead of sharing it: `{b} = list({a})`"
            ),
            None => format!(
                "tempered `{f}` changes {what} while `{a}` may be the same list and is used later; copy instead of sharing it: `{a} = list(...)`"
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
            let xs = self.name(*p);
            let message = if value.contains(&root(Obj::Param(i), self.ty(*p))) {
                format!("tempered `{f}` returns its parameter `{xs}`; return a copy: `return list({xs})`")
            } else if reached.iter().any(|o| region.contains(o)) {
                format!(
                    "tempered `{f}` returns a value sharing a list with its parameter `{xs}`; return a copy made with `list(...)`"
                )
            } else if self.lists(*p) && self.global_in(value, self.ty(*p)) {
                format!(
                    "tempered `{f}` returns a list a global holds, which its parameter `{xs}` may hold too; return a copy made with `list(...)`"
                )
            } else {
                continue;
            };
            return Err(error(span, message));
        }
        Ok(())
    }

    /// Whether `value` may hold a global's list, or one the body stored
    /// in a global, that a parameter of type `param` may hold too: the
    /// caller may pass that global.
    fn global_in(&self, value: &Locs, param: Ty) -> bool {
        let world = self.flow.region(Obj::World);
        value.iter().any(|l| {
            let typed = l.ty.is_none_or(|t| self.share_a_list_type(param, t));
            let mut inside = self.flow.reachable(&Objs::from([l.obj]));
            inside.remove(&l.obj);
            (world.contains(&l.obj) && typed) || inside.iter().any(|o| world.contains(o))
        })
    }

    /// Whether values of types `a` and `b` may hold lists of one type.
    fn share_a_list_type(&self, a: Ty, b: Ty) -> bool {
        let mut lists = BTreeSet::new();
        self.list_types(a, &mut lists);
        lists.into_iter().any(|t| self.within(b, t))
    }

    /// The list types a value of type `ty` is or may hold.
    fn list_types(&self, ty: Ty, out: &mut BTreeSet<Ty>) {
        let inner: Vec<Ty> = match self.program.compound(ty) {
            Some(Compound::List(t)) => {
                out.insert(ty);
                vec![*t]
            }
            Some(Compound::Opt(t)) => vec![*t],
            Some(Compound::Tuple(fields)) => fields.clone(),
            None => Vec::new(),
        };
        inner.into_iter().for_each(|t| self.list_types(t, out));
    }

    /// Whether a name that may be `seen` sees a change to `changed`.
    fn overlaps(&self, seen: &Locs, changed: &Locs, how: Change) -> bool {
        changed.iter().any(|c| {
            let inside = match how {
                Change::List => Objs::new(),
                Change::Within => self.flow.reachable(&self.meeting(c.obj)),
            };
            seen.iter()
                .any(|s| self.same(s, c, how) || (!meets(s.obj, c.obj) && inside.contains(&s.obj)))
        })
    }

    /// Whether `seen` may be the list `changed` is, or for
    /// [`Change::Within`], one inside it.
    fn same(&self, seen: &Loc, changed: &Loc, how: Change) -> bool {
        if !meets(seen.obj, changed.obj) {
            return false;
        }
        let (Some(s), Some(c)) = (seen.ty, changed.ty) else {
            return true;
        };
        let typed = match how {
            Change::List => s == c,
            Change::Within => self.within(c, s),
        };
        typed && (seen.obj != changed.obj || seen.earlier == changed.earlier)
    }

    /// The abstract lists that may share a list with `obj`.
    fn meeting(&self, obj: Obj) -> Objs {
        let params = (0..self.func.params.len()).map(Obj::Param);
        match obj {
            Obj::Param(_) => Objs::from([obj, Obj::World]),
            Obj::World => params.chain([Obj::World]).collect(),
            Obj::Fresh(_) => Objs::from([obj]),
        }
    }

    fn name(&self, l: LocalId) -> &str {
        self.func.names.get(&l).map_or("?", String::as_str)
    }
}

/// Whether abstract lists `a` and `b` may share a list: the same one,
/// or a parameter and the module's values (`f(saved)`).
fn meets(a: Obj, b: Obj) -> bool {
    a == b
        || matches!(
            (a, b),
            (Obj::Param(_), Obj::World) | (Obj::World, Obj::Param(_))
        )
}

/// The latest `obj`, itself of type `ty`.
fn root(obj: Obj, ty: Ty) -> Loc {
    Loc {
        obj,
        earlier: false,
        ty: Some(ty),
    }
}

fn anywhere(obj: Obj) -> Loc {
    Loc {
        obj,
        earlier: false,
        ty: None,
    }
}

/// Marks the latest list allocated into `dest` as an earlier one: `dest`
/// is about to get a new one.
fn age(state: &mut State, dest: LocalId) {
    let latest = Obj::Fresh(dest.0);
    let age_all = |locs: &mut Locs| {
        let aged = std::mem::take(locs).into_iter().map(|l| Loc {
            earlier: l.earlier || (l.obj == latest && l.ty.is_some()),
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

/// Where a change `inst` makes is reported.
fn inst_span(inst: &Inst) -> Option<Span> {
    match inst {
        Inst::Append { span, .. } | Inst::SetItem { span, .. } | Inst::Assign { span, .. } => {
            Some(*span)
        }
        _ => None,
    }
}
