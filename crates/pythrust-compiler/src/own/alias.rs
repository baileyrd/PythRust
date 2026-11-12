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

use super::body::{self, Age, Flow, Obj, Objs};
use super::{holds_list, Mode, Summary};
use crate::ast::{Block, Expr, ExprKind, Mode as Temper, Module, StmtKind};
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
#[derive(Debug, Clone, PartialEq)]
struct State {
    values: Vec<Val>,
    /// A temporary's possible source names, each with its own value.
    /// None preserves alternatives made by calls or fresh expressions.
    origins: Vec<BTreeMap<Option<Key>, Val>>,
}

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
    /// The body, through a local and one possible source identity.
    Local(LocalId, Option<Key>),
    /// A callee, in a module variable's list: through that variable,
    /// if one.
    Callee(FuncId, Option<GlobalId>, Ty),
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
    let bodies: BTreeMap<&str, &Block> = module
        .body
        .iter()
        .filter_map(|s| match &s.kind {
            StmtKind::FuncDef(f) if f.mode == Temper::Tempered => {
                Some((f.name.name.as_str(), &f.body))
            }
            _ => None,
        })
        .collect();
    let effects = effects(program, summaries);
    let mut out = BTreeMap::new();
    for func in &program.funcs {
        let Some(source) = bodies.get(func.name.as_str()) else {
            continue;
        };
        if out.contains_key(&func.name) {
            continue;
        }
        if let Err(d) = check(program, func, summaries, &effects, source) {
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
fn check(
    program: &Program,
    func: &Func,
    summaries: &[Summary],
    effects: &Effects,
    source: &Block,
) -> Result<()> {
    let mut body = Body::new(program, func, summaries, effects);
    (body.loops, body.copy_sites) = source_facts(program, func, source);
    let entries = body.entries();
    let live = body.live_out();
    for (b, entry) in entries.iter().enumerate() {
        let Some(entry) = entry else { continue };
        body.replay(&func.blocks[b].insts, entry.clone(), &live[b])?;
    }
    Ok(())
}

/// A source loop input, with the source regions where its iteration is
/// active. Evaluating the iterable (including nested zip/enumerate inputs)
/// precedes both target binding and the body; its `else` follows iteration.
#[derive(Clone)]
struct LoopInput {
    source: Span,
    name: Option<String>,
    active: Vec<Span>,
}

impl LoopInput {
    fn running_at(&self, span: Span) -> bool {
        self.active
            .iter()
            .any(|s| s.start <= span.start && span.end <= s.end)
    }
}

struct Body<'a> {
    program: &'a Program,
    func: &'a Func,
    summaries: &'a [Summary],
    flow: Flow<'a>,
    effects: &'a Effects,
    /// Flow-insensitive source names, only for module write summaries.
    /// Mutation checks use the per-alternative origins in State.
    roots: Vec<Live>,
    /// The locals `return` assigns.
    results: BTreeSet<LocalId>,
    /// Only locals read by the lowering of a source `for` iterable.
    /// Ordinary call-argument temporaries are not loop references.
    loops: BTreeMap<LocalId, LoopInput>,
    /// Standalone mutations whose arguments cannot create new aliases.
    copy_sites: Vec<Span>,
    /// The lists this body allocates that it may store in the module's
    /// values, so that [`Obj::World`] may be them.
    held: Objs,
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
            loops: BTreeMap::new(),
            copy_sites: Vec::new(),
            held: Objs::new(),
        };
        body.roots = body.roots();
        let world = body.flow.reachable(&body.meeting(Obj::World));
        body.held = world
            .into_iter()
            .filter(|o| matches!(o, Obj::Fresh(..)))
            .collect();
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
        let mut start = State {
            values: vec![Val::default(); self.func.locals.len()],
            origins: vec![BTreeMap::new(); self.func.locals.len()],
        };
        for (i, p) in self.func.params.iter().enumerate() {
            if self.lists(*p) {
                let ty = self.ty(*p);
                start.values[p.0].locs.insert(root(Obj::Param(i), ty));
                start.origins[p.0].insert(Some(Key::Local(*p)), start.values[p.0].clone());
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
                    Rvalue::NewSeq(_)
                        | Rvalue::CopyList(_)
                        | Rvalue::Slice { .. }
                        | Rvalue::View(_)
                        | Rvalue::Call(..)
                        | Rvalue::Std(..)
                        | Rvalue::Rust(..)
                ) {
                    age(state, *dest);
                }
                let val = self.value(*dest, value, state);
                let origins = self.origins(*dest, value, &val, state);
                state.values[dest.0] = val;
                state.origins[dest.0] = origins;
            }
            Inst::Release(l) | Inst::Forget(l) => {
                state.values[l.0] = Val::default();
                state.origins[l.0].clear();
            }
            _ => {}
        }
    }

    /// Keep each alternative's source through copies, projections and CFG
    /// joins. A named assignment starts a new identity, even for an alias.
    fn origins(
        &self,
        dest: LocalId,
        rv: &Rvalue,
        value: &Val,
        state: &State,
    ) -> BTreeMap<Option<Key>, Val> {
        if self.named(dest) {
            return BTreeMap::from([(Some(Key::Local(dest)), value.clone())]);
        }
        if let Rvalue::Global(g) = rv {
            return BTreeMap::from([(Some(Key::Global(*g)), value.clone())]);
        }
        let Some(Operand::Local(src)) = projected(rv) else {
            return BTreeMap::from([(None, value.clone())]);
        };
        state.origins[src.0]
            .iter()
            .map(|(key, v)| {
                let projected = match rv {
                    Rvalue::Field(_, k) => self.field(v, *k, self.ty(dest)),
                    Rvalue::Index(..) | Rvalue::Entry(..) => Val {
                        locs: self.step(&v.locs, self.ty(dest)),
                        fields: None,
                    },
                    _ => v.clone(),
                };
                (*key, projected)
            })
            .collect()
    }

    /// What `rv` may give `dest`.
    fn value(&self, dest: LocalId, rv: &Rvalue, state: &State) -> Val {
        let ty = self.ty(dest);
        let fresh = Locs::from([root(Obj::Fresh(dest.0, Age::Latest), ty)]);
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
            Rvalue::Index(a, _) | Rvalue::Entry(a, ..) => self.step(&of(state, *a), ty),
            Rvalue::NewSeq(_)
            | Rvalue::CopyList(_)
            | Rvalue::Slice { .. }
            | Rvalue::View(_)
            | Rvalue::Std(..)
            | Rvalue::Rust(..) => fresh,
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
        matches!(self.func.locals[l.0], Ty::Tuple(_) | Ty::Record(_))
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
        self.within_in(outer, inner, &mut Vec::new())
    }

    /// [`Self::within`], past the records in `open`: one reached again
    /// inside itself (D162) adds nothing.
    fn within_in(&self, outer: Ty, inner: Ty, open: &mut Vec<Ty>) -> bool {
        if outer == inner {
            return true;
        }
        match self.program.compound(outer) {
            Some(
                Compound::List(t)
                | Compound::Opt(t)
                | Compound::Set(t)
                | Compound::Dict(_, t)
                | Compound::View(_, t),
            ) => self.within_in(*t, inner, open),
            Some(Compound::Tuple(fields)) => fields.iter().any(|t| self.within_in(*t, inner, open)),
            Some(Compound::Record { fields, .. } | Compound::Object { fields, .. })
                if !open.contains(&outer) =>
            {
                open.push(outer);
                let any = fields.iter().any(|t| self.within_in(*t, inner, open));
                open.pop();
                any
            }
            Some(Compound::Record { .. } | Compound::Object { .. }) | None => false,
        }
    }

    /// A new list, or what the callee may return of its arguments and
    /// the module's values.
    fn call(&self, f: FuncId, args: &[Operand], dest: LocalId, state: &State) -> Locs {
        let summary = &self.summaries[f.0];
        let mut out = Locs::from([root(Obj::Fresh(dest.0, Age::Latest), self.ty(dest))]);
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
            Key::Local(l) => state.values[l.0].locs.clone(),
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
                self.returned(&self.value(*dest, value, state).locs, *dest, value, *span)?;
            }
        }
        let Some(span) = inst_span(inst) else {
            return Ok(());
        };
        for (op, how) in self.changes(inst) {
            let Operand::Local(w) = op else {
                continue;
            };
            for (origin, value) in &state.origins[w.0] {
                self.change(
                    Writer::Local(w, *origin),
                    &value.locs,
                    how,
                    state,
                    live,
                    span,
                )?;
            }
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
            let writer = Writer::Callee(*f, w.global, w.ty);
            self.change(writer, &changed, w.how, state, live, span)?;
        }
        Ok(())
    }

    /// What `inst` changes in place, and how.
    fn changes(&self, inst: &Inst) -> Vec<(Operand, Change)> {
        match inst {
            Inst::Append { list, .. }
            | Inst::SetItem { list, .. }
            | Inst::SetField { obj: list, .. }
            | Inst::Extend { list, .. }
            | Inst::Scale { list, .. }
            | Inst::Insert { list, .. }
            | Inst::Remove { list, .. }
            | Inst::Sort { list, .. } => vec![(*list, Change::List)],
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
        let (itself, origin) = match writer {
            Writer::Local(w, origin) => (Some(Key::Local(w)), origin),
            Writer::Callee(_, g, _) => (None, g.map(Key::Global)),
        };
        let clash =
            |k: &&Key| Some(**k) != itself && self.overlaps(&self.locs(**k, state), changed, how);
        let (named, temporaries): (Vec<&Key>, Vec<&Key>) =
            live.iter().filter(clash).partition(|k| match k {
                Key::Local(l) => self.named(*l),
                Key::Global(_) => true,
            });
        let other = named.into_iter().find(|k| Some(**k) != origin);
        let message = match (other, temporaries.first()) {
            (Some(a), _) => self.aliased(writer, *a, span),
            (None, Some(_)) => {
                let looped = temporaries.iter().find_map(|k| match k {
                    Key::Local(l) => self
                        .loops
                        .get(l)
                        .filter(|input| input.running_at(span))
                        .map(|input| (*l, input)),
                    Key::Global(_) => None,
                });
                let (context, hint) = match looped {
                    Some((l, input)) => {
                        let hint = if temporaries.len() == 1 {
                            self.loop_hint(l, input.name.as_deref())
                        } else {
                            "make independent values for the loop and any other live references before changing the container".into()
                        };
                        ("a `for` loop over it is running", hint)
                    },
                    None => ("a temporary value may still refer to the changed container and is used later", "finish using that value before changing the container, or make an independent value before sharing it".into()),
                };
                format!(
                    "tempered `{}` changes {} while {context}; {hint}",
                    self.func.name,
                    self.subject(writer)
                )
            }
            (None, None) => return Ok(()),
        };
        Err(error(span, message))
    }

    /// Name the receiver's actual container type, without mistaking the
    /// outer source of a projection for the projected container.
    fn container(&self, ty: Ty) -> &'static str {
        match self.program.compound(ty) {
            Some(Compound::List(_)) => "list",
            Some(Compound::Set(_)) => "set",
            Some(Compound::Dict(..)) => "dict",
            Some(Compound::Opt(t)) => self.container(*t),
            _ => "mutable container",
        }
    }

    fn subject(&self, writer: Writer) -> String {
        match writer {
            Writer::Local(w, _) => self.func.names.get(&w).map_or_else(
                || format!("a {}", self.container(self.ty(w))),
                |name| format!("`{name}`"),
            ),
            Writer::Callee(f, _, ty) => format!(
                "a {} through `{}`",
                self.container(ty),
                self.program.funcs[f.0].name
            ),
        }
    }

    /// Copying a loop's outer container separates its iteration only when
    /// none of the elements can still carry the changed mutable value.
    fn loop_hint(&self, local: LocalId, name: Option<&str>) -> String {
        if let Some(name) = name.filter(|_| self.flat_container(self.func.locals[local.0])) {
            return format!(
                "loop over a copy, which costs one: replace the iterated `{name}` with `list({name})`"
            );
        }
        "iterate over an independent snapshot of the container before changing it; a shallow copy may still share nested lists, dicts or sets".into()
    }

    fn flat_container(&self, ty: Ty) -> bool {
        match self.program.compound(ty) {
            Some(Compound::List(t) | Compound::Set(t)) => self.copy_separates(*t),
            Some(Compound::Dict(k, v)) => self.copy_separates(*k) && self.copy_separates(*v),
            _ => false,
        }
    }

    /// Copy the current receiver immediately before mutation, not an
    /// earlier alias source: later aliases can invalidate a binding rewrite.
    /// A parameter must still communicate its mutation to the caller.
    fn alias_hint(&self, writer: Writer, span: Span) -> String {
        if let Writer::Local(w, _) = writer {
            let ty = self.func.locals[w.0];
            if self.copy_sites.contains(&span)
                && self.named(w)
                && !self.func.params.contains(&w)
                && matches!(ty, Ty::List(_))
                && self.flat_container(ty)
            {
                let name = self.name(w);
                return format!(
                    "copy the current value before this change: `{name} = list({name})`"
                );
            }
        }
        "make independent values before sharing them; a shallow copy may still share nested lists, dicts or sets".into()
    }

    fn aliased(&self, writer: Writer, other: Key, span: Span) -> String {
        let (name, ty) = match other {
            Key::Local(l) => (format!("`{}`", self.name(l)), self.func.locals[l.0]),
            Key::Global(g) => (
                format!(
                    "module variable `{}`",
                    self.program
                        .global_names
                        .get(&g)
                        .map_or("?", String::as_str)
                ),
                self.program.globals[g.0],
            ),
        };
        format!(
            "tempered `{}` changes {} while {name} may be the same {} and is used later; {}",
            self.func.name,
            self.subject(writer),
            self.container(ty),
            self.alias_hint(writer, span)
        )
    }

    /// D3: a result sharing a list with a parameter.
    fn returned(&self, value: &Locs, dest: LocalId, expression: &Rvalue, span: Span) -> Result<()> {
        let objs: Objs = value.iter().map(|l| l.obj).collect();
        let reached = self.flow.reachable(&objs);
        let f = &self.func.name;
        for (i, p) in self.func.params.iter().enumerate() {
            let region = self.flow.region(Obj::Param(i));
            let xs = self.name(*p);
            let message = if value.contains(&root(Obj::Param(i), self.ty(*p))) {
                format!("tempered `{f}` returns its parameter `{xs}`")
            } else if reached.iter().any(|o| region.contains(o)) {
                format!("tempered `{f}` returns a value sharing a list with its parameter `{xs}`")
            } else if self.lists(*p) && self.global_in(value, self.ty(*p)) {
                format!(
                    "tempered `{f}` returns a list a global holds, which its parameter `{xs}` may hold too"
                )
            } else {
                continue;
            };
            return Err(error(
                span,
                format!("{message}; {}", self.return_hint(dest, expression)),
            ));
        }
        Ok(())
    }

    /// A shallow copy fixes D3 only for a list with no mutable containers
    /// in its elements. Name the returned local, never a possible alias source.
    fn return_hint(&self, dest: LocalId, expression: &Rvalue) -> String {
        let Some(Compound::List(element)) = self.program.compound(self.func.locals[dest.0]) else {
            return "return an independent value that shares no lists, dicts or sets with the parameter".into();
        };
        if !self.copy_separates(*element) {
            return "return an independent value; a shallow copy still shares nested lists, dicts or sets".into();
        }
        if let Rvalue::Use(Operand::Local(local)) = expression {
            if let Some(name) = self.func.names.get(local) {
                return format!("return a copy: `return list({name})`");
            }
        }
        "return a copy made with `list(...)`".into()
    }

    /// Only immutable element shapes prove that copying the outer list
    /// separates it. An exception, for example, may carry a mutable value.
    fn copy_separates(&self, ty: Ty) -> bool {
        crate::ir::reachable(&self.program.types, ty)
            .into_iter()
            .all(|t| match self.program.compound(t) {
                Some(Compound::Tuple(_) | Compound::Record { .. } | Compound::Opt(_)) => true,
                Some(_) => false,
                None => matches!(
                    t,
                    Ty::Int | Ty::Float | Ty::Bool | Ty::None | Ty::Str | Ty::Bytes | Ty::Range
                ),
            })
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
        // A key holds no list, so reaching through keys finds no more.
        for t in crate::ir::reachable(&self.program.types, ty) {
            let compound = self.program.compound(t);
            if matches!(
                compound,
                Some(Compound::List(_) | Compound::Set(_) | Compound::Dict(..))
            ) {
                out.insert(t);
            }
        }
    }

    /// Whether a name that may be `seen` sees a change to `changed`.
    fn overlaps(&self, seen: &Locs, changed: &Locs, how: Change) -> bool {
        changed.iter().any(|c| {
            let inside = match how {
                Change::List => Objs::new(),
                Change::Within => self.flow.reachable(&self.meeting(c.obj)),
            };
            seen.iter().any(|s| {
                self.same(s, c, how) || (!self.meets(s.obj, c.obj) && inside.contains(&s.obj))
            })
        })
    }

    /// Whether `seen` may be the list `changed` is, or for
    /// [`Change::Within`], one inside it.
    fn same(&self, seen: &Loc, changed: &Loc, how: Change) -> bool {
        if !self.meets(seen.obj, changed.obj) {
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
            Obj::World | Obj::Global(_) => params.chain([Obj::World]).collect(),
            Obj::Fresh(..) => Objs::from([obj]),
        }
    }

    /// Whether abstract lists `a` and `b` may share a list: [`meets`], or
    /// the module's values and a list this body stored there.
    fn meets(&self, a: Obj, b: Obj) -> bool {
        meets(a, b)
            || (a == Obj::World && self.held.contains(&b))
            || (b == Obj::World && self.held.contains(&a))
    }

    fn name(&self, l: LocalId) -> &str {
        self.func.names.get(&l).map_or("?", String::as_str)
    }
}

/// Recover the loop reference from the existing source-span contract:
/// lowering emits `Len(iterable)` at the source iterable's span. Names
/// are advice only; membership never changes the alias rejection.
fn source_facts(
    program: &Program,
    func: &Func,
    source: &Block,
) -> (BTreeMap<LocalId, LoopInput>, Vec<Span>) {
    let mut expressions = Vec::new();
    let mut copies = Vec::new();
    for_iterables(program, source, &mut expressions, &mut copies);
    let loops = func
        .blocks
        .iter()
        .flat_map(|b| &b.insts)
        .filter_map(|i| {
            let Inst::Assign {
                value: Rvalue::Len(Operand::Local(l)),
                span,
                ..
            } = i
            else {
                return None;
            };
            let input = expressions.iter().find(|input| input.source == *span)?;
            Some((*l, input.clone()))
        })
        .collect();
    (loops, copies)
}

/// `zip` and `enumerate` keep references to their input iterables.
/// Other calls evaluate their arguments before the loop starts.
fn iterable_sources(program: &Program, expr: &Expr, active: &[Span], out: &mut Vec<LoopInput>) {
    let name = match &expr.kind {
        ExprKind::Name(name) => Some(name.clone()),
        _ => None,
    };
    out.push(LoopInput {
        source: expr.span,
        name,
        active: active.to_vec(),
    });
    let ExprKind::Call { func, args } = &expr.kind else {
        return;
    };
    let ExprKind::Name(name) = &func.kind else {
        return;
    };
    if !matches!(name.as_str(), "zip" | "enumerate")
        || program.funcs.iter().any(|f| f.name == *name)
    {
        return;
    }
    let count = if name == "enumerate" { 1 } else { args.len() };
    for arg in args.iter().take(count) {
        iterable_sources(program, &arg.value, active, out);
    }
}

/// A copy before an enclosing call can still alias an earlier argument;
/// an effectful argument can also share the new copy again. Only offer a
/// statement insertion for standalone append with an already simple value.
fn plain_append(stmt: &crate::ast::Stmt) -> Option<Span> {
    let StmtKind::Expr(expr) = &stmt.kind else {
        return None;
    };
    let ExprKind::Call { func, args } = &expr.kind else {
        return None;
    };
    let ExprKind::Attr { value, attr } = &func.kind else {
        return None;
    };
    let [arg] = args.as_slice() else { return None };
    let simple = matches!(
        arg.value.kind,
        ExprKind::Name(_)
            | ExprKind::Int(_)
            | ExprKind::Float(_)
            | ExprKind::Str(_)
            | ExprKind::Bool(_)
            | ExprKind::None
    );
    (attr.name == "append" && matches!(value.kind, ExprKind::Name(_)) && simple)
        .then_some(expr.span)
}

fn for_iterables(
    program: &Program,
    block: &Block,
    out: &mut Vec<LoopInput>,
    copies: &mut Vec<Span>,
) {
    for stmt in block {
        if let Some(span) = plain_append(stmt) {
            copies.push(span);
        }
        match &stmt.kind {
            StmtKind::For {
                target,
                iter,
                body,
                orelse,
            } => {
                let active: Vec<Span> = std::iter::once(target.span)
                    .chain(body.iter().map(|s| s.span))
                    .collect();
                iterable_sources(program, iter, &active, out);
                for_iterables(program, body, out, copies);
                for_iterables(program, orelse, out, copies);
            }
            StmtKind::If {
                branch,
                elifs,
                orelse,
            } => {
                for_iterables(program, &branch.body, out, copies);
                for branch in elifs {
                    for_iterables(program, &branch.body, out, copies);
                }
                for_iterables(program, orelse, out, copies);
            }
            StmtKind::While { branch, orelse } => {
                for_iterables(program, &branch.body, out, copies);
                for_iterables(program, orelse, out, copies);
            }
            StmtKind::Try {
                body,
                handlers,
                finally,
            } => {
                for_iterables(program, body, out, copies);
                for handler in handlers {
                    for_iterables(program, &handler.body, out, copies);
                }
                if let Some(f) = finally {
                    for_iterables(program, f, out, copies);
                }
            }
            StmtKind::With { body, .. } => for_iterables(program, body, out, copies),
            _ => {}
        }
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
    let latest = Obj::Fresh(dest.0, Age::Latest);
    let age_all = |locs: &mut Locs| {
        let aged = std::mem::take(locs).into_iter().map(|l| Loc {
            earlier: l.earlier || (l.obj == latest && l.ty.is_some()),
            ..l
        });
        *locs = aged.collect();
    };
    for v in state
        .values
        .iter_mut()
        .chain(state.origins.iter_mut().flat_map(|o| o.values_mut()))
    {
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
    for (a, b) in into.values.iter_mut().zip(&state.values) {
        let joined = join_val(a, b);
        grew |= joined != *a;
        *a = joined;
    }
    for (a, b) in into.origins.iter_mut().zip(&state.origins) {
        for (key, value) in b {
            let old = a.entry(*key).or_default();
            let joined = join_val(old, value);
            grew |= joined != *old;
            *old = joined;
        }
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
        Operand::Local(l) => state.values[l.0].clone(),
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
        Rvalue::Index(a, _) | Rvalue::Field(a, _) | Rvalue::Entry(a, ..) => Some(*a),
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
pub(super) fn inst_operands(inst: &Inst) -> Vec<Operand> {
    match inst {
        Inst::Assign { value, .. } => value.operands(),
        Inst::Print { args, sep, end } => args.iter().chain(sep).chain(end).copied().collect(),
        Inst::Append { list, value, .. }
        | Inst::Extend {
            list, other: value, ..
        } => {
            vec![*list, *value]
        }
        Inst::Scale { list, count, .. } => vec![*list, *count],
        Inst::SetGlobal { value, .. } => vec![*value],
        Inst::SetField { obj, value, .. } => vec![*obj, *value],
        Inst::SetItem {
            list, index, value, ..
        } => vec![*list, *index, *value],
        Inst::Raise { exc, .. } | Inst::Reverse(exc) => vec![*exc],
        Inst::Stash | Inst::Restore => vec![],
        Inst::Sort { list, reverse, .. } => vec![*list, *reverse],
        Inst::Insert {
            list, index, value, ..
        } => vec![*list, *index, *value],
        Inst::Remove { list, index, .. } => vec![*list, *index],
        Inst::Retain(_) | Inst::Release(_) | Inst::Forget(_) | Inst::DropGlobal(_) => Vec::new(),
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
pub(super) fn inst_span(inst: &Inst) -> Option<Span> {
    match inst {
        Inst::Append { span, .. }
        | Inst::SetItem { span, .. }
        | Inst::SetField { span, .. }
        | Inst::Extend { span, .. }
        | Inst::Scale { span, .. }
        | Inst::Insert { span, .. }
        | Inst::Remove { span, .. }
        | Inst::Sort { span, .. }
        | Inst::Assign { span, .. } => Some(*span),
        _ => None,
    }
}
