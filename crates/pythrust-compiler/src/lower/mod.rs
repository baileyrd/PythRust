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

#[cfg(test)]
mod tests;

use std::collections::{BTreeSet, HashMap};

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
    let mut b = FnBuilder::new(&mut cx, Subst::new(), scope::assigned(&module.body));
    b.block(&module.body)?;
    let func = b.finish("<module>", Vec::new(), Ty::None);
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
        let mut b = FnBuilder::new(self, job.subst.clone(), scope);
        let mut params = Vec::new();
        for (p, ty) in f.params.iter().zip(&job.args) {
            let id = b.new_local(*ty);
            b.names.insert(p.name.name.clone(), id);
            params.push(id);
        }
        b.block(&f.body)?;
        Ok(b.finish(&f.name.name, params, job.ret))
    }
}

/// Binds the variables of `t` so that it becomes `ty`. False on a clash.
fn bind(t: &Type, ty: Ty, subst: &mut Subst) -> bool {
    match (t, ty) {
        (Type::Var(v), _) => *subst.entry(*v).or_insert(ty) == ty,
        (Type::Int, Ty::Int) | (Type::Bool, Ty::Bool) | (Type::None, Ty::None) => true,
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
}

/// Builds one function's IR.
struct FnBuilder<'c, 'a> {
    cx: &'c mut Cx<'a>,
    subst: Subst,
    /// Names local to this function, as Python decides them.
    scope: BTreeSet<String>,
    names: HashMap<String, LocalId>,
    locals: Vec<Ty>,
    blocks: Vec<Pending>,
    cur: BlockId,
    loops: Vec<Loop>,
}

impl<'c, 'a> FnBuilder<'c, 'a> {
    fn new(cx: &'c mut Cx<'a>, subst: Subst, scope: BTreeSet<String>) -> Self {
        let mut b = FnBuilder {
            cx,
            subst,
            scope,
            names: HashMap::new(),
            locals: Vec::new(),
            blocks: Vec::new(),
            cur: BlockId(0),
            loops: Vec::new(),
        };
        b.cur = b.new_block();
        b
    }

    /// Closes the current block (falling off the end returns `None`) and
    /// any unreachable ones, and assembles the function.
    fn finish(mut self, name: &str, params: Vec<LocalId>, ret: Ty) -> Func {
        let end = match ret {
            Ty::None => Term::Return(Operand::None),
            _ => Term::Unreachable,
        };
        self.terminate(end);
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
        self.cur = self.new_block();
    }

    fn emit(&mut self, inst: Inst) {
        self.blocks[self.cur.0].insts.push(inst);
    }

    fn assign(&mut self, dest: LocalId, value: Rvalue, span: Span) {
        self.emit(Inst::Assign { dest, value, span });
    }

    /// A fresh temporary holding `value`.
    fn temp(&mut self, ty: Ty, value: Rvalue, span: Span) -> Operand {
        let dest = self.new_local(ty);
        self.assign(dest, value, span);
        Operand::Local(dest)
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
            Type::Var(v) => self
                .subst
                .get(v)
                .copied()
                .ok_or_else(|| unsupported(span, "a value whose type depends on generic code")),
            other => Err(unsupported(span, &format!("values of type `{other}`"))),
        }
    }

    /// The local for variable `name`, created on first use with the type
    /// of the node at `span`.
    fn var(&mut self, name: &str, span: Span) -> Result<LocalId> {
        if let Some(id) = self.names.get(name) {
            return Ok(*id);
        }
        if !self.scope.contains(name) {
            let what = format!("reading `{name}`, a global, from a function");
            return Err(unsupported(span, &what));
        }
        let ty = self.ty_at(span)?;
        let id = self.new_local(ty);
        self.names.insert(name.to_string(), id);
        Ok(id)
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
