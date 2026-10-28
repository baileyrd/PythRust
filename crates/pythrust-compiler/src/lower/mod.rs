//! Lowering: a typed `ast::Module` to an [`ir::Program`].
//!
//! Generic functions are monomorphized: each call site's concrete
//! argument and result types pick (or create) one instance, whose type
//! variables are bound by matching the generic signature against them.
//! Instances are lowered from a worklist, so only reachable code is
//! compiled. Anything outside the subset the IR covers is a
//! `DiagKind::Backend` error naming the construct, never a guess.

mod exc;
mod expr;
mod globals;
mod opt;
mod seq;
mod stmt;
mod types;
mod vars;

#[cfg(test)]
mod tests;

use std::collections::{BTreeSet, HashMap, HashSet};

use crate::ast::{FuncDef, Module, StmtKind};
use crate::diag::{DiagKind, Diagnostic, Result};
use crate::infer::{scope, Type, TypeVar, Types};
use crate::ir::{
    self, Block, BlockId, Func, FuncId, Inst, LocalId, Operand, Program, Rvalue, Term, Ty,
};
use crate::span::Span;

/// Concrete types for one instance's type variables.
type Subst = HashMap<TypeVar, Ty>;

/// An instance lowered with an open result type, and that type.
type OpenInstance = (FuncId, Ty);

/// Where an instance whose result type its body decides stands.
#[derive(Clone, Copy)]
enum Open {
    /// Being lowered, this many open instances deep: a call back into it
    /// cannot know its result yet.
    Lowering(usize),
    /// Lowered with this result type.
    Done(OpenInstance),
    /// Every `return` needed a call into an instance still being
    /// lowered beneath it; lowered again, under the same id, when called
    /// again.
    Undecided(FuncId),
}

/// What lowering an instance's body gave.
enum Body {
    /// The function, and whether a statement was skipped for a call
    /// whose result was unknown.
    Lowered(Func, bool),
    /// No `return` decided the open result type; the variables the body
    /// bound on the way, and whether it may return at all.
    Undecided(Subst, bool),
}

/// What later code says of a variable first met untyped.
#[derive(Clone, Copy)]
enum Hint {
    /// The type a live assignment gave it.
    Typed(Ty),
    /// No live assignment: only code that never completes assigns it,
    /// so every read of it fails as unbound.
    Unbound,
}

/// Hints by variable name ([`with_hints`]).
type Hints = HashMap<String, Hint>;

/// One try at lowering a body.
enum Attempt<T> {
    Done(T),
    /// Lower again, with these variables typed so.
    Retype(Hints),
}

/// Lowers a body with `attempt` until every variable is typed before
/// its first use. A variable's type is fixed only once the whole body
/// has been seen, so a read that comes first (in a branch, or in a loop
/// before the assignment) never types it from code that never runs.
/// Each retry hints at least one more variable, so it ends.
fn with_hints<T>(mut attempt: impl FnMut(&Hints) -> Result<Attempt<T>>) -> Result<T> {
    let mut hints = Hints::new();
    loop {
        match attempt(&hints)? {
            Attempt::Done(t) => return Ok(t),
            Attempt::Retype(more) => hints.extend(more),
        }
    }
}

/// What [`Cx::open_instance`] knows of a call's result.
enum Found {
    Known(OpenInstance),
    /// Not yet: the callee waits on an instance still being lowered.
    /// Whether it may return anyway (a `None`, or by falling off).
    Unknown(bool),
}

/// Which functions [`lower_reaching`] lowers.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Reach {
    /// Only what the module's code may call: what `build` compiles.
    Called,
    /// Also every function no code calls whose signature is concrete,
    /// so analysis sees each tempered body.
    Every,
}

/// Lowers `module`, typed by `types`, to IR: the module and every
/// instance its code may call.
pub fn lower(module: &Module, types: &Types) -> Result<Program> {
    lower_reaching(module, types, Reach::Called)
}

/// As [`lower`], but also lowers each function no code calls when its
/// signature names no type variable: every tempered function and every
/// non-generic loose one. For analysis, which must see bodies the
/// program never runs.
pub fn lower_every(module: &Module, types: &Types) -> Result<Program> {
    lower_reaching(module, types, Reach::Every)
}

fn lower_reaching(module: &Module, types: &Types, reach: Reach) -> Result<Program> {
    let mut defs = HashMap::new();
    for s in &module.body {
        if let StmtKind::FuncDef(f) = &s.kind {
            defs.insert(f.name.name.as_str(), f);
        }
    }
    let mut cx = Cx {
        types,
        globals: globals::Globals::new(module, &defs),
        defs,
        instances: HashMap::new(),
        funcs: Vec::new(),
        queue: Vec::new(),
        seqs: types::SeqTypes::default(),
        open: HashMap::new(),
        lowering: 0,
        waits: usize::MAX,
        diverges: HashSet::new(),
    };
    let main = cx.reserve();
    let func = with_hints(|hints| {
        let scope = scope::assigned(&module.body);
        let mut b = FnBuilder::new(&mut cx, Subst::new(), scope, Ty::None);
        b.defined = Some(HashSet::new());
        b.hints = hints.clone();
        b.caught = scope::caught(&module.body);
        let lowered = b.block(&module.body);
        if let Some(more) = b.pending_hints(lowered.is_ok()) {
            return Ok(Attempt::Retype(more));
        }
        lowered?;
        Ok(Attempt::Done(b.finish("<module>", Vec::new())))
    })?;
    cx.funcs[main.0] = Some(func);
    if reach == Reach::Every {
        cx.root_concrete(module)?;
    }
    while let Some(job) = cx.queue.pop() {
        let Body::Lowered(func, _) = cx.instance_body(&job)? else {
            return Err(internal(module.span, "a queued instance undecided"));
        };
        cx.funcs[job.id.0] = Some(func);
    }
    let funcs = std::mem::take(&mut cx.funcs);
    let funcs = funcs.into_iter().collect::<Option<Vec<Func>>>();
    let mut funcs = funcs.ok_or_else(|| internal(module.span, "an instance was never lowered"))?;
    // The module's exit, which ends the program, drops what globals own.
    let exit = &mut funcs[main.0].blocks[1].insts;
    exit.extend(cx.globals.owning().into_iter().map(Inst::DropGlobal));
    Ok(Program {
        funcs,
        main,
        types: cx.seqs.into_vec(),
        globals: cx.globals.into_vec(),
    })
}

/// Program-wide lowering state.
struct Cx<'a> {
    types: &'a Types,
    defs: HashMap<&'a str, &'a FuncDef>,
    /// Instances by function name, argument types and result type.
    instances: HashMap<(String, Vec<Ty>, Ty), FuncId>,
    /// `None` until the instance is lowered.
    funcs: Vec<Option<Func>>,
    queue: Vec<Job<'a>>,
    seqs: types::SeqTypes,
    globals: globals::Globals,
    /// Instances lowered by [`Cx::open_instance`], by function name and
    /// argument types.
    open: HashMap<(String, Vec<Ty>), Open>,
    /// How many open instances are being lowered, one inside another.
    lowering: usize,
    /// The depth of the shallowest instance still being lowered whose
    /// result a body needed, since the innermost open instance began.
    waits: usize,
    /// Settled instances that never return: every path through them
    /// needs their own result first.
    diverges: HashSet<FuncId>,
}

/// An instance waiting to be lowered.
struct Job<'a> {
    id: FuncId,
    def: &'a FuncDef,
    subst: Subst,
    args: Vec<Ty>,
    /// `None`: decided by the body's first `return`.
    ret: Option<Ty>,
}

impl<'a> Cx<'a> {
    fn reserve(&mut self) -> FuncId {
        self.funcs.push(None);
        FuncId(self.funcs.len() - 1)
    }

    /// Queues the one instance of each function in `module` whose
    /// signature is concrete. One the code calls is already queued with
    /// these types, so only the uncalled are new.
    fn root_concrete(&mut self, module: &'a Module) -> Result<()> {
        let closed = Subst::new();
        for s in &module.body {
            let StmtKind::FuncDef(def) = &s.kind else {
                continue;
            };
            let Some(sig) = self.types.func(&def.name.name) else {
                continue;
            };
            let types = sig.params.iter().map(|p| &p.ty).chain([&sig.ret]);
            if !types.clone().all(|t| is_closed(t, &closed)) {
                continue;
            }
            let mut tys = types
                .map(|t| concrete(t, &closed, &mut self.seqs, def.span))
                .collect::<Result<Vec<Ty>>>()?;
            let ret = tys
                .pop()
                .ok_or_else(|| internal(def.span, "no result type"))?;
            self.instance(def, tys, ret, def.span)?;
        }
        Ok(())
    }

    /// The instance of `def` for `args` and `ret`, queued if new.
    fn instance(&mut self, def: &'a FuncDef, args: Vec<Ty>, ret: Ty, span: Span) -> Result<FuncId> {
        let key = (def.name.name.clone(), args.clone(), ret);
        if let Some(id) = self.instances.get(&key) {
            return Ok(*id);
        }
        let name = &def.name.name;
        let sig = self
            .types
            .func(name)
            .ok_or_else(|| internal(span, &format!("`{name}` has no signature")))?;
        let mut subst = Subst::new();
        let params = sig.params.iter().map(|p| &p.ty);
        let bound = params.zip(&args).chain([(&sig.ret, &ret)]);
        if !bound
            .into_iter()
            .all(|(t, ty)| bind(t, *ty, &self.seqs, &mut subst))
        {
            return Err(internal(span, &format!("call does not match `{name}`")));
        }
        let id = self.take_undecided(name, &args);
        self.instances.insert(key, id);
        self.queue.push(Job {
            id,
            def,
            subst,
            args,
            ret: Some(ret),
        });
        Ok(id)
    }

    /// The instance of `def` for `args` when the call's result type is
    /// open: in generic code, a call's node type is in the callee's
    /// variables, and the signature may not fix it (`n * n` gives a
    /// type of its own). The instance is lowered now, and its first
    /// `return` decides the result type; inference made every `return`
    /// agree.
    ///
    /// `None` while the result type is unknown: for a call back into an
    /// instance still being lowered, or into one whose every `return`
    /// needed a call into an instance beneath it. The caller then skips
    /// the statement if it can ([`FnBuilder::block`]); a body that
    /// skipped one is lowered again once its result type is decided.
    ///
    /// An instance no `return` decides on its own (every `return` needs
    /// its own result, or returns `None` into an open optional, or there
    /// is none) never returns anything else, so its open variables are
    /// settled ([`settle`]) and it is lowered again with that type. If
    /// no path returns at all, it [`diverges`](Cx::diverges).
    fn open_instance(&mut self, def: &'a FuncDef, args: Vec<Ty>, span: Span) -> Result<Found> {
        let name = &def.name.name;
        let key = (name.clone(), args.clone());
        let id = match self.open.get(&key) {
            Some(Open::Done(found)) => return Ok(Found::Known(*found)),
            // Assumed not to return: a body that returns only through
            // this call never returns.
            Some(Open::Lowering(depth)) => {
                self.waits = self.waits.min(*depth);
                return Ok(Found::Unknown(false));
            }
            Some(Open::Undecided(id)) => *id,
            None => self.reserve(),
        };
        let sig = self
            .types
            .func(name)
            .ok_or_else(|| internal(span, &format!("`{name}` has no signature")))?;
        let mut subst = Subst::new();
        let params = sig.params.iter().map(|p| &p.ty);
        if !params
            .zip(&args)
            .all(|(t, ty)| bind(t, *ty, &self.seqs, &mut subst))
        {
            return Err(internal(span, &format!("call does not match `{name}`")));
        }
        let depth = self.lowering;
        self.open.insert(key.clone(), Open::Lowering(depth));
        let mut job = Job {
            id,
            def,
            subst,
            args: args.clone(),
            ret: None,
        };
        let (body, waits) = self.nested_body(&job);
        let ret = match body? {
            Body::Lowered(func, false) => {
                let ret = func.ret;
                self.decided(key, id, ret);
                self.funcs[id.0] = Some(func);
                return Ok(Found::Known((id, ret)));
            }
            Body::Lowered(func, true) => func.ret,
            Body::Undecided(_, returns) if waits < depth => {
                self.open.insert(key, Open::Undecided(id));
                return Ok(Found::Unknown(returns));
            }
            Body::Undecided(bound, returns) => {
                if !returns {
                    self.diverges.insert(id);
                }
                settle(&sig.ret, &job.subst, &bound, &mut self.seqs)
            }
        };
        // Decided before lowering again, so recursive calls find it.
        self.decided(key, id, ret);
        bind(&sig.ret, ret, &self.seqs, &mut job.subst);
        job.ret = Some(ret);
        let Body::Lowered(func, _) = self.instance_body(&job)? else {
            return Err(internal(
                span,
                "an instance undecided with its result known",
            ));
        };
        self.funcs[id.0] = Some(func);
        Ok(Found::Known((id, ret)))
    }

    /// Lowers `job`'s body one open instance deeper, and the depth of
    /// the shallowest instance still being lowered whose result it needed.
    fn nested_body(&mut self, job: &Job<'a>) -> (Result<Body>, usize) {
        let outer = std::mem::replace(&mut self.waits, usize::MAX);
        self.lowering += 1;
        let body = self.instance_body(job);
        self.lowering -= 1;
        let waits = std::mem::replace(&mut self.waits, outer);
        // Waiting on this instance, or on one inside it, ends here.
        if waits < self.lowering {
            self.waits = self.waits.min(waits);
        }
        (body, waits)
    }

    /// Records that the open instance `key` is `id`, returning `ret`.
    fn decided(&mut self, key: (String, Vec<Ty>), id: FuncId, ret: Ty) {
        self.instances
            .insert((key.0.clone(), key.1.clone(), ret), id);
        self.open.insert(key, Open::Done((id, ret)));
    }

    /// The id an instance of `name` for `args` lowered undecided holds,
    /// so it is not left empty, or a new one.
    fn take_undecided(&mut self, name: &str, args: &[Ty]) -> FuncId {
        let key = (name.to_string(), args.to_vec());
        match self.open.get(&key) {
            Some(Open::Undecided(id)) => {
                let id = *id;
                self.open.remove(&key);
                id
            }
            _ => self.reserve(),
        }
    }

    /// The function `job` asks for, unless no `return` decided its open
    /// result type.
    fn instance_body(&mut self, job: &Job<'a>) -> Result<Body> {
        with_hints(|hints| self.instance_attempt(job, hints))
    }

    fn instance_attempt(&mut self, job: &Job<'a>, hints: &Hints) -> Result<Attempt<Body>> {
        let f = job.def;
        let mut scope = scope::assigned(&f.body);
        scope.extend(f.params.iter().map(|p| p.name.name.clone()));
        let mut b = FnBuilder::new(self, job.subst.clone(), scope, job.ret.unwrap_or(Ty::None));
        b.hints = hints.clone();
        b.caught = scope::caught(&f.body);
        if job.ret.is_none() {
            let sig = b.cx.types.func(&f.name.name);
            b.open_ret = Some(sig.map_or(Type::None, |sig| sig.ret.clone()));
        }
        let mut params = Vec::new();
        for (p, ty) in f.params.iter().zip(&job.args) {
            let id = b.new_local(*ty);
            let name = &p.name.name;
            // Bound on entry, but a handler may unbind it.
            let flag = b.caught.contains(name).then(|| b.flag(true, p.span));
            b.names.insert(name.clone(), vars::Var { local: id, flag });
            params.push(id);
        }
        let lowered = b.block(&f.body);
        if let Some(more) = b.pending_hints(lowered.is_ok()) {
            return Ok(Attempt::Retype(more));
        }
        lowered?;
        if b.open_ret.is_some() {
            let returns = b.may_return || b.reachable(b.cur);
            return Ok(Attempt::Done(Body::Undecided(b.subst, returns)));
        }
        let skipped = b.skipped;
        let func = b.finish(&f.name.name, params);
        Ok(Attempt::Done(Body::Lowered(func, skipped)))
    }
}

/// Whether `t` has no variables `subst` leaves unbound.
fn is_closed(t: &Type, subst: &Subst) -> bool {
    match t {
        Type::Var(v) => subst.contains_key(v),
        Type::List(t) | Type::Optional(t) => is_closed(t, subst),
        Type::Tuple(ts) => ts.iter().all(|t| is_closed(t, subst)),
        Type::Int | Type::Float | Type::Str | Type::Bool | Type::None | Type::Range => true,
        Type::Exception => true,
    }
}

/// The IR type of a result type no `return` decided: a variable `subst`
/// leaves open takes what the body bound it to, else `None`. The instance
/// never returns a value of that type (only `None`, or nothing), so
/// any type is sound; this one matches what the body built.
fn settle(t: &Type, subst: &Subst, bound: &Subst, seqs: &mut types::SeqTypes) -> Ty {
    match t {
        Type::Var(v) => subst.get(v).or(bound.get(v)).copied().unwrap_or(Ty::None),
        Type::List(t) => {
            let elem = settle(t, subst, bound, seqs);
            seqs.list(elem)
        }
        Type::Optional(t) => {
            let present = settle(t, subst, bound, seqs);
            seqs.optional(present)
        }
        Type::Tuple(ts) => {
            let fields = ts.iter().map(|t| settle(t, subst, bound, seqs)).collect();
            seqs.tuple(fields)
        }
        Type::Int => Ty::Int,
        Type::Float => Ty::Float,
        Type::Bool => Ty::Bool,
        Type::None => Ty::None,
        Type::Str => Ty::Str,
        Type::Range => Ty::Range,
        Type::Exception => Ty::Exc,
    }
}

/// Every top-level function `name` may call, directly or not, and itself.
fn reach<'a>(defs: &HashMap<&'a str, &'a FuncDef>, name: &'a str) -> BTreeSet<&'a str> {
    let mut seen = BTreeSet::new();
    let mut todo = vec![name];
    while let Some(n) = todo.pop() {
        let Some(def) = defs.get(n) else {
            continue;
        };
        if !seen.insert(def.name.name.as_str()) {
            continue;
        }
        for r in scope::references(def) {
            if let Some((k, _)) = defs.get_key_value(r.as_str()) {
                todo.push(k);
            }
        }
    }
    seen
}

/// Binds the variables of `t` so that it becomes `ty`. False on a clash.
fn bind(t: &Type, ty: Ty, seqs: &types::SeqTypes, subst: &mut Subst) -> bool {
    match (t, ty) {
        (Type::Var(v), _) => *subst.entry(*v).or_insert(ty) == ty,
        (Type::List(t), Ty::List(_)) => seqs.elem(ty).is_some_and(|e| bind(t, e, seqs, subst)),
        (Type::Optional(t), Ty::Opt(_)) => {
            seqs.present(ty).is_some_and(|p| bind(t, p, seqs, subst))
        }
        (Type::Tuple(ts), Ty::Tuple(_)) => {
            let fields = seqs.fields(ty).map(<[Ty]>::to_vec).unwrap_or_default();
            ts.len() == fields.len() && ts.iter().zip(fields).all(|(t, f)| bind(t, f, seqs, subst))
        }
        (Type::Int, Ty::Int)
        | (Type::Float, Ty::Float)
        | (Type::Bool, Ty::Bool)
        | (Type::None, Ty::None)
        | (Type::Str, Ty::Str)
        | (Type::Range, Ty::Range)
        | (Type::Exception, Ty::Exc) => true,
        _ => false,
    }
}

/// A block under construction; `term` is set once.
struct Pending {
    insts: Vec<Inst>,
    term: Option<Term>,
}

/// Where `break` and `continue` go in the innermost loop.
#[derive(Clone, Copy)]
struct Loop {
    next: BlockId,
    exit: BlockId,
    /// How many statement pools were open outside the body: a jump out
    /// of the body releases the ones above.
    pools: usize,
}

/// The handlers of the innermost `try` whose body is being lowered.
#[derive(Clone, Copy)]
struct Try {
    /// Tests the pending exception against each handler in turn.
    dispatch: BlockId,
    /// How many statement pools were open outside the body: raising in
    /// it releases the ones above.
    pools: usize,
}

/// An `except ... as name` whose body is being lowered: leaving it any
/// way unbinds `name`, as Python does.
#[derive(Clone)]
struct Unbind {
    name: String,
    span: Span,
    /// How many loops and `try` bodies enclosed the handler.
    loops: usize,
    tries: usize,
}

/// Builds one function's IR.
struct FnBuilder<'c, 'a> {
    cx: &'c mut Cx<'a>,
    subst: Subst,
    /// What calls that never return bind: types for values that never
    /// exist (a variable only such a call assigns), used where `subst`
    /// has none, and never evidence against what a live value decides.
    dead: Subst,
    /// Variable types to use before any other ([`with_hints`]).
    hints: Hints,
    /// Variables first met before live code typed them: the statement
    /// is skipped and the body lowered again once the rest has had its
    /// say ([`with_hints`]).
    pending: Vec<String>,
    /// Names local to this function, as Python decides them.
    scope: BTreeSet<String>,
    names: HashMap<String, vars::Var>,
    /// Locals known to be bound wherever control is now: assigned in the
    /// entry block, which runs before every other block.
    always_bound: HashSet<LocalId>,
    /// Locals assigned earlier in the current block.
    block_bound: HashSet<LocalId>,
    /// In the module's code: the functions whose `def` has run so far.
    defined: Option<HashSet<String>>,
    locals: Vec<Ty>,
    blocks: Vec<Pending>,
    cur: BlockId,
    loops: Vec<Loop>,
    ret: Ty,
    /// When the first `return` decides `ret`: the signature's result
    /// type, whose variables it binds, so later recursive calls resolve.
    open_ret: Option<Type>,
    /// Set by a recursive call met while `open_ret` is undecided; the
    /// statement holding it is then skipped and `skipped` set.
    stuck: bool,
    skipped: bool,
    /// Set by a call into an instance that [diverges](Cx::diverges): the
    /// rest of the path is unreachable, so the expression holding it is
    /// abandoned up to the statement or branch it ends.
    diverged: bool,
    /// While `open_ret` is undecided: a skipped statement may return
    /// (it returns `None`, or calls an instance that may).
    may_return: bool,
    /// Where `return` leaves its value, created by the first `return`.
    ret_slot: Option<LocalId>,
    /// Where every `return` goes: it releases the function's strings.
    exit: BlockId,
    /// Per statement (innermost last), the `Str` temporaries it owns,
    /// released when it ends.
    pools: Vec<Vec<LocalId>>,
    /// Enclosing `try` bodies, innermost last.
    tries: Vec<Try>,
    /// Enclosing handlers that bind a name, innermost last.
    unbinds: Vec<Unbind>,
    /// Names some handler of this body binds, and so unbinds: never
    /// known to be bound everywhere.
    caught: BTreeSet<String>,
    /// Where an exception no `try` here catches goes, created on first
    /// use: the exit block, or in the module's code [`Term::Uncaught`].
    unwind: Option<BlockId>,
}

impl<'c, 'a> FnBuilder<'c, 'a> {
    fn new(cx: &'c mut Cx<'a>, subst: Subst, scope: BTreeSet<String>, ret: Ty) -> Self {
        let mut b = FnBuilder {
            cx,
            subst,
            dead: Subst::new(),
            hints: Hints::new(),
            pending: Vec::new(),
            scope,
            names: HashMap::new(),
            always_bound: HashSet::new(),
            block_bound: HashSet::new(),
            defined: None,
            locals: Vec::new(),
            blocks: Vec::new(),
            cur: BlockId(0),
            loops: Vec::new(),
            ret,
            open_ret: None,
            stuck: false,
            skipped: false,
            diverged: false,
            may_return: false,
            ret_slot: None,
            exit: BlockId(1),
            pools: vec![Vec::new()],
            tries: Vec::new(),
            unbinds: Vec::new(),
            caught: BTreeSet::new(),
            unwind: None,
        };
        b.cur = b.new_block();
        b.exit = b.new_block();
        b
    }

    /// Closes the current block (falling off the end returns `None`) and
    /// any unreachable ones, fills in the exit block, and assembles the
    /// function.
    fn finish(mut self, name: &str, params: Vec<LocalId>) -> Func {
        let end = match self.ret {
            Ty::None => Term::Goto(self.exit),
            // Falling off returns `None`: the slot, never assigned on
            // that path, is null.
            Ty::Opt(_) => {
                self.return_slot();
                Term::Goto(self.exit)
            }
            _ => Term::Unreachable,
        };
        self.terminate(end);
        let ret = self.ret;
        // A function that never returns has no `return` to create the
        // slot; the exit, unreachable, still returns its type.
        let value = match ret {
            Ty::None => Operand::None,
            _ => Operand::Local(self.return_slot()),
        };
        let strs = (0..self.locals.len()).map(LocalId);
        let owned = strs.filter(|l| self.locals[l.0].is_rc() && Some(*l) != self.ret_slot);
        let releases: Vec<Inst> = owned.map(Inst::Release).collect();
        // The module's code ends on an uncaught exception releasing what
        // it owns too, so nothing it held is lost.
        if let (Some(unwind), Some(_)) = (self.unwind, &self.defined) {
            self.blocks[unwind.0].insts = releases.clone();
        }
        let exit = &mut self.blocks[self.exit.0];
        exit.insts = releases;
        exit.term = Some(Term::Return(value));
        let blocks = self.blocks.into_iter().map(|p| Block {
            insts: p.insts,
            term: p.term.unwrap_or(Term::Unreachable),
        });
        Func {
            name: name.to_string(),
            params,
            ret,
            locals: self.locals,
            names: self.names.into_iter().map(|(n, v)| (v.local, n)).collect(),
            blocks: blocks.collect(),
        }
    }

    /// Where `return` leaves its value, created on first use.
    fn return_slot(&mut self) -> LocalId {
        let ret = self.ret;
        *self.ret_slot.get_or_insert_with(|| {
            self.locals.push(ret);
            LocalId(self.locals.len() - 1)
        })
    }

    fn new_block(&mut self) -> BlockId {
        self.blocks.push(Pending {
            insts: Vec::new(),
            term: None,
        });
        BlockId(self.blocks.len() - 1)
    }

    /// Whether `target` is reachable from the entry block.
    fn reachable(&self, target: BlockId) -> bool {
        let mut seen = HashSet::from([BlockId(0)]);
        let mut todo = vec![BlockId(0)];
        while let Some(b) = todo.pop() {
            if b == target {
                return true;
            }
            let next = match &self.blocks[b.0].term {
                Some(Term::Goto(t)) => vec![*t],
                Some(Term::Branch { then, orelse, .. }) => vec![*then, *orelse],
                _ => Vec::new(),
            };
            todo.extend(next.into_iter().filter(|t| seen.insert(*t)));
        }
        false
    }

    fn new_local(&mut self, ty: Ty) -> LocalId {
        self.locals.push(ty);
        LocalId(self.locals.len() - 1)
    }

    fn switch_to(&mut self, block: BlockId) {
        self.cur = block;
        self.block_bound.clear();
    }

    /// Ends the current block. A block already ended (code after a
    /// `return`) keeps its first terminator.
    fn terminate(&mut self, term: Term) {
        let pending = &mut self.blocks[self.cur.0];
        pending.term.get_or_insert(term);
    }

    /// Ends the current block and continues in a fresh, unreachable one:
    /// for `return`, `break` and `continue` mid-block.
    fn jump_away(&mut self, term: Term) {
        self.terminate(term);
        let next = self.new_block();
        self.switch_to(next);
    }

    fn emit(&mut self, inst: Inst) {
        self.blocks[self.cur.0].insts.push(inst);
    }

    fn assign(&mut self, dest: LocalId, value: Rvalue, span: Span) {
        self.emit(Inst::Assign { dest, value, span });
    }

    /// A fresh temporary holding `value`. A `Str` one owns its value,
    /// so `value` must make a new reference, and the statement releases
    /// it.
    fn temp(&mut self, ty: Ty, value: Rvalue, span: Span) -> Operand {
        let dest = self.pooled(ty);
        self.assign(dest, value, span);
        Operand::Local(dest)
    }

    /// A fresh local, owned by the current statement if a `Str`.
    fn pooled(&mut self, ty: Ty) -> LocalId {
        let dest = self.new_local(ty);
        if ty.is_rc() {
            if let Some(pool) = self.pools.last_mut() {
                pool.push(dest);
            }
        }
        dest
    }

    /// Runs `f` as one statement: the `Str` temporaries it creates are
    /// released where it ends.
    fn pooled_stmt<T>(&mut self, f: impl FnOnce(&mut Self) -> Result<T>) -> Result<T> {
        self.pools.push(Vec::new());
        let out = f(self);
        let pool = self.pools.pop().unwrap_or_default();
        pool.into_iter().for_each(|t| self.emit(Inst::Release(t)));
        out
    }

    /// Whether `op` is a temporary whose reference may be moved out.
    fn is_temp(&self, op: Operand) -> bool {
        let Operand::Local(l) = op else {
            return false;
        };
        self.pools.iter().any(|p| p.contains(&l))
    }

    /// A temporary owning a reference to `op`: `op` itself if it is one.
    fn own(&mut self, op: Operand, span: Span) -> Operand {
        let ty = self.ty(op);
        if !ty.is_rc() || self.is_temp(op) {
            return op;
        }
        let t = self.temp(ty, Rvalue::Use(op), span);
        if let Operand::Local(l) = t {
            self.emit(Inst::Retain(l));
        }
        t
    }

    /// `dest = v`, where `dest` holds no reference: moves `v`'s
    /// reference if `v` is a temporary, else adds one.
    fn move_into(&mut self, dest: LocalId, v: Operand, span: Span) {
        self.assign(dest, Rvalue::Use(v), span);
        if !self.ty(v).is_rc() {
            return;
        }
        match v {
            Operand::Local(l) if self.is_temp(v) => self.emit(Inst::Forget(l)),
            _ => self.emit(Inst::Retain(dest)),
        }
    }

    fn ty(&self, op: Operand) -> Ty {
        ir::operand_ty(&self.locals, op)
    }

    /// The concrete type of the node at `span`.
    fn ty_at(&mut self, span: Span) -> Result<Ty> {
        let t = self.node_type(span)?;
        self.concrete(t, span)
    }

    /// The inferred type of the node at `span`.
    fn node_type(&self, span: Span) -> Result<&'a Type> {
        self.cx
            .types
            .at(span)
            .ok_or_else(|| internal(span, "node has no type"))
    }

    /// `t` under this instance's bindings, else those of calls that
    /// never return.
    fn concrete(&mut self, t: &Type, span: Span) -> Result<Ty> {
        let live = concrete(t, &self.subst, &mut self.cx.seqs, span);
        if live.is_ok() || self.dead.is_empty() {
            return live;
        }
        let mut both = self.dead.clone();
        both.extend(self.subst.iter().map(|(v, ty)| (*v, *ty)));
        concrete(t, &both, &mut self.cx.seqs, span)
    }

    /// The node type at `span` if it has settled: no variable is left
    /// anywhere in it under live bindings. In generic code an open one
    /// (`[x * x]`'s element) waits for a live value to decide it; the
    /// `None` defaulting [`concrete`] does for an unconstrained element
    /// (`not []`) is no such decision.
    fn settled_ty_at(&mut self, span: Span) -> Result<Option<Ty>> {
        let t = self.node_type(span)?;
        if !is_closed(t, &self.subst) {
            return Ok(None);
        }
        concrete(t, &self.subst, &mut self.cx.seqs, span).map(Some)
    }
}

/// The IR type of `t`, its type variables bound by `subst`.
fn concrete(t: &Type, subst: &Subst, seqs: &mut types::SeqTypes, span: Span) -> Result<Ty> {
    match t {
        // An element type nothing constrains, as in `not []`: the list
        // stays empty, so any slot type will do.
        Type::List(elem) if matches!(**elem, Type::Var(v) if !subst.contains_key(&v)) => {
            Ok(seqs.list(Ty::None))
        }
        Type::List(elem) => {
            let elem = concrete(elem, subst, seqs, span)?;
            Ok(seqs.list(elem))
        }
        Type::Tuple(ts) => {
            let fields = ts
                .iter()
                .map(|t| concrete(t, subst, seqs, span))
                .collect::<Result<Vec<Ty>>>()?;
            Ok(seqs.tuple(fields))
        }
        Type::Optional(present) => {
            let present = concrete(present, subst, seqs, span)?;
            Ok(seqs.optional(present))
        }
        Type::Int => Ok(Ty::Int),
        Type::Float => Ok(Ty::Float),
        Type::Bool => Ok(Ty::Bool),
        Type::None => Ok(Ty::None),
        Type::Str => Ok(Ty::Str),
        Type::Range => Ok(Ty::Range),
        Type::Exception => Ok(Ty::Exc),
        Type::Var(v) => subst
            .get(v)
            .copied()
            .ok_or_else(|| unsupported(span, "a value whose type depends on generic code")),
    }
}

pub(super) fn unsupported(span: Span, what: &str) -> Diagnostic {
    Diagnostic {
        span,
        message: format!("the native backend does not support {what} yet"),
        kind: DiagKind::Backend,
    }
}

pub(super) fn internal(span: Span, what: &str) -> Diagnostic {
    Diagnostic {
        span,
        message: format!("internal: {what}"),
        kind: DiagKind::Backend,
    }
}
