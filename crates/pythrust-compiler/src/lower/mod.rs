//! Lowering: a typed `ast::Module` to an [`ir::Program`].
//!
//! Generic functions are monomorphized: each call site's concrete
//! argument and result types pick (or create) one instance, whose type
//! variables are bound by matching the generic signature against them.
//! Instances are lowered from a worklist, so only reachable code is
//! compiled. Anything outside the subset the IR covers is a
//! `DiagKind::Backend` error naming the construct, never a guess.

mod expr;
mod stmt;
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

/// Lowers `module`, typed by `types`, to IR.
pub fn lower(module: &Module, types: &Types) -> Result<Program> {
    let mut cx = Cx {
        types,
        defs: HashMap::new(),
        instances: HashMap::new(),
        funcs: Vec::new(),
        queue: Vec::new(),
    };
    for s in &module.body {
        if let StmtKind::FuncDef(f) = &s.kind {
            cx.defs.insert(f.name.name.as_str(), f);
        }
    }
    let main = cx.reserve();
    let mut b = FnBuilder::new(
        &mut cx,
        Subst::new(),
        scope::assigned(&module.body),
        Ty::None,
    );
    b.defined = Some(HashSet::new());
    b.block(&module.body)?;
    let func = b.finish("<module>", Vec::new());
    cx.funcs[main.0] = Some(func);
    while let Some(job) = cx.queue.pop() {
        let func = cx.instance_body(&job)?;
        cx.funcs[job.id.0] = Some(func);
    }
    let funcs = cx.funcs.into_iter().collect::<Option<Vec<Func>>>();
    let funcs = funcs.ok_or_else(|| internal(module.span, "an instance was never lowered"))?;
    Ok(Program { funcs, main })
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
}

/// An instance waiting to be lowered.
struct Job<'a> {
    id: FuncId,
    def: &'a FuncDef,
    subst: Subst,
    args: Vec<Ty>,
    ret: Ty,
}

impl<'a> Cx<'a> {
    fn reserve(&mut self) -> FuncId {
        self.funcs.push(None);
        FuncId(self.funcs.len() - 1)
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
        if !bound.into_iter().all(|(t, ty)| bind(t, *ty, &mut subst)) {
            return Err(internal(span, &format!("call does not match `{name}`")));
        }
        let id = self.reserve();
        self.instances.insert(key, id);
        self.queue.push(Job {
            id,
            def,
            subst,
            args,
            ret,
        });
        Ok(id)
    }

    fn instance_body(&mut self, job: &Job<'a>) -> Result<Func> {
        let f = job.def;
        let mut scope = scope::assigned(&f.body);
        scope.extend(f.params.iter().map(|p| p.name.name.clone()));
        let mut b = FnBuilder::new(self, job.subst.clone(), scope, job.ret);
        let mut params = Vec::new();
        for (p, ty) in f.params.iter().zip(&job.args) {
            let id = b.new_local(*ty);
            b.names.insert(
                p.name.name.clone(),
                vars::Var {
                    local: id,
                    flag: None,
                },
            );
            params.push(id);
        }
        b.block(&f.body)?;
        Ok(b.finish(&f.name.name, params))
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
fn bind(t: &Type, ty: Ty, subst: &mut Subst) -> bool {
    match (t, ty) {
        (Type::Var(v), _) => *subst.entry(*v).or_insert(ty) == ty,
        (Type::Int, Ty::Int)
        | (Type::Bool, Ty::Bool)
        | (Type::None, Ty::None)
        | (Type::Str, Ty::Str) => true,
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

/// Builds one function's IR.
struct FnBuilder<'c, 'a> {
    cx: &'c mut Cx<'a>,
    subst: Subst,
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
    /// Where `return` leaves its value, created by the first `return`.
    ret_slot: Option<LocalId>,
    /// Where every `return` goes: it releases the function's strings.
    exit: BlockId,
    /// Per statement (innermost last), the `Str` temporaries it owns,
    /// released when it ends.
    pools: Vec<Vec<LocalId>>,
}

impl<'c, 'a> FnBuilder<'c, 'a> {
    fn new(cx: &'c mut Cx<'a>, subst: Subst, scope: BTreeSet<String>, ret: Ty) -> Self {
        let mut b = FnBuilder {
            cx,
            subst,
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
            ret_slot: None,
            exit: BlockId(1),
            pools: vec![Vec::new()],
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
            _ => Term::Unreachable,
        };
        self.terminate(end);
        let ret = self.ret;
        let value = match self.ret_slot {
            Some(slot) if ret != Ty::None => Operand::Local(slot),
            _ => Operand::None,
        };
        let strs = (0..self.locals.len()).map(LocalId);
        let owned = strs.filter(|l| self.locals[l.0] == Ty::Str && Some(*l) != self.ret_slot);
        let exit = &mut self.blocks[self.exit.0];
        exit.insts = owned.map(Inst::Release).collect();
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
            blocks: blocks.collect(),
        }
    }

    fn new_block(&mut self) -> BlockId {
        self.blocks.push(Pending {
            insts: Vec::new(),
            term: None,
        });
        BlockId(self.blocks.len() - 1)
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
        if ty == Ty::Str {
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
        if self.ty(op) != Ty::Str || self.is_temp(op) {
            return op;
        }
        let t = self.temp(Ty::Str, Rvalue::Use(op), span);
        if let Operand::Local(l) = t {
            self.emit(Inst::Retain(l));
        }
        t
    }

    /// `dest = v`, where `dest` holds no reference: moves `v`'s
    /// reference if `v` is a temporary, else adds one.
    fn move_into(&mut self, dest: LocalId, v: Operand, span: Span) {
        self.assign(dest, Rvalue::Use(v), span);
        if self.ty(v) != Ty::Str {
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
    fn ty_at(&self, span: Span) -> Result<Ty> {
        let t = self
            .cx
            .types
            .at(span)
            .ok_or_else(|| internal(span, "node has no type"))?;
        self.concrete(t, span)
    }

    fn concrete(&self, t: &Type, span: Span) -> Result<Ty> {
        match t {
            Type::Int => Ok(Ty::Int),
            Type::Bool => Ok(Ty::Bool),
            Type::None => Ok(Ty::None),
            Type::Str => Ok(Ty::Str),
            Type::Var(v) => self
                .subst
                .get(v)
                .copied()
                .ok_or_else(|| unsupported(span, "a value whose type depends on generic code")),
            other => Err(unsupported(span, &format!("values of type `{other}`"))),
        }
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
