//! Tasks (M6c): `submit` and `result()` as calls into
//! `pythrust-rt/src/task.rs`, and a thunk per function a task runs.
//!
//! What crosses to a worker, all marked first (`pt_mark`) so every
//! thread counts it atomically: the task's arguments, and the module
//! values its function may load or replace, directly or through the
//! functions it calls or submits ([`reads`]). Nothing else does: a
//! literal is made where it is used, a default is filled in at the call
//! site as an argument, and a future or executor never leaves the
//! function that made it. The outcome is made on the worker, unmarked,
//! and handed over under the future's lock (`task.rs`'s module doc).

use std::collections::{BTreeSet, HashMap};

use cranelift_codegen::ir::{
    types, AbiParam, InstBuilder, MemFlagsData, Signature, StackSlotData, StackSlotKind, Value,
};
use cranelift_codegen::Context;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId as ClifFunc, Linkage, Module};

use super::seq::{from_slot, to_slot};
use super::{backend, module_err, rc_dec, rc_inc, Codegen, FnGen, Rt};
use crate::diag::Result;
use crate::ir::{FuncId, GlobalId, Inst, LocalId, Operand, Program, Rvalue};

impl Codegen<'_> {
    /// Declares a thunk for each function a task runs, and finds the
    /// module values each may reach.
    pub(super) fn declare_thunks(&mut self, program: &Program) -> Result<()> {
        for f in submitted(program) {
            let sig = self.thunk_signature();
            let name = format!("pt_task{}_{}", f.0, program.funcs[f.0].name);
            let id = self
                .module
                .declare_function(&name, Linkage::Local, &sig)
                .map_err(module_err)?;
            self.thunks.insert(f, id);
            self.reads.insert(f, reads(program, f));
        }
        Ok(())
    }

    /// `unsafe extern "C" fn(ctx, args) -> u64`, `pythrust-rt`'s `Thunk`.
    fn thunk_signature(&self) -> Signature {
        let mut sig = self.module.make_signature();
        sig.params
            .extend([AbiParam::new(self.ptr), AbiParam::new(self.ptr)]);
        sig.returns.push(AbiParam::new(types::I64));
        sig
    }

    /// Defines each declared thunk: it unpacks the argument slots, calls
    /// the function with the worker's context, drops the arguments the
    /// function borrowed, and returns the result as a slot.
    pub(super) fn define_thunks(&mut self, program: &Program, ids: &[ClifFunc]) -> Result<()> {
        let mut thunks: Vec<(FuncId, ClifFunc)> =
            self.thunks.iter().map(|(f, t)| (*f, *t)).collect();
        thunks.sort();
        for (f, thunk) in thunks {
            let callee = *ids
                .get(f.0)
                .ok_or_else(|| backend("a task of an unknown function".to_string()))?;
            let mut ctx = Context::new();
            ctx.func.signature = self.thunk_signature();
            self.thunk_body(program, f, callee, &mut ctx)?;
            self.define_function(thunk, &mut ctx)?;
        }
        Ok(())
    }

    fn thunk_body(
        &mut self,
        program: &Program,
        f: FuncId,
        callee: ClifFunc,
        ctx: &mut Context,
    ) -> Result<()> {
        let func = &program.funcs[f.0];
        let mut fctx = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut ctx.func, &mut fctx);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let (cx, args) = match b.block_params(entry) {
            [cx, args] => (*cx, *args),
            _ => return Err(backend("a thunk without its two parameters".to_string())),
        };
        let flags = MemFlagsData::trusted();
        let mut slots = Vec::new();
        let mut call_args = vec![cx];
        for (k, p) in func.params.iter().enumerate() {
            let slot = b.ins().load(types::I64, flags, args, 8 * k as i32);
            call_args.push(from_slot(&mut b, slot, self.clif_ty(func.locals[p.0])));
            slots.push(slot);
        }
        let fref = self.module.declare_func_in_func(callee, b.func);
        let call = b.ins().call(fref, &call_args);
        let result = match b.inst_results(call).first().copied() {
            Some(v) => to_slot(&mut b, v),
            None => b.ins().iconst(types::I64, 0),
        };
        for (p, slot) in func.params.iter().zip(slots) {
            if !func.borrowed.contains(p) {
                continue;
            }
            let id = self.rt[rc_dec(&program.types, func.locals[p.0]).symbol()];
            let dec = self.module.declare_func_in_func(id, b.func);
            b.ins().call(dec, &[slot]);
        }
        b.ins().return_(&[result]);
        b.seal_all_blocks();
        b.finalize(self.module.target_config());
        Ok(())
    }
}

impl FnGen<'_, '_, '_> {
    /// `exec.submit(func, *args)`: marks what crosses, gives the task a
    /// reference to each argument, and queues it, giving its future.
    pub(super) fn submit(
        &mut self,
        exec: Operand,
        func: FuncId,
        args: &[Operand],
    ) -> Result<Value> {
        let thunk = *self
            .cg
            .thunks
            .get(&func)
            .ok_or_else(|| backend("a task without a thunk".to_string()))?;
        let reads = self
            .cg
            .reads
            .get(&func)
            .cloned()
            .ok_or_else(|| backend("a task without its module values".to_string()))?;
        for g in reads {
            self.mark_global(g)?;
        }
        let mut slots = Vec::new();
        for op in args {
            slots.push(self.cross(*op)?);
        }
        let packed = self.pack(&slots)?;
        let exec = self.operand(exec);
        let fref = self.cg.module.declare_func_in_func(thunk, self.b.func);
        let thunk = self.b.ins().func_addr(self.cg.ptr, fref);
        let n = self.b.ins().iconst(self.cg.ptr, slots.len() as i64);
        let ret = self.ret_of(func)?;
        let (desc, len) = self.descriptor(ret)?;
        self.call_rt_value(Rt::Submit, &[exec, thunk, packed, n, desc, len])
    }

    /// `fut.result()`, into `dest`: 0 or null when it raised.
    pub(super) fn task_result(&mut self, fut: Operand, dest: LocalId) -> Result<Value> {
        let fut = self.operand(fut);
        let v = self.call_rt_value(Rt::TaskResult, &[fut])?;
        Ok(self.unslot(v, self.func.locals[dest.0]))
    }

    /// The next of `map`'s results ([`Rvalue::NextResult`]).
    pub(super) fn next_result(&mut self, rs: Operand, dest: LocalId) -> Result<Value> {
        let rs = self.operand(rs);
        let v = self.call_rt_value(Rt::ResultsNext, &[rs])?;
        Ok(self.unslot(v, self.func.locals[dest.0]))
    }

    /// `op` as a slot a task takes: marked, with a reference of its own.
    fn cross(&mut self, op: Operand) -> Result<Value> {
        let ty = self.func.ty(op);
        let slot = self.slot(op);
        if !ty.is_rc() {
            return Ok(slot);
        }
        let (desc, len) = self.descriptor(ty)?;
        self.call_rt(Rt::Mark, &[desc, len, slot]);
        let inc = rc_inc(self.types, ty)?;
        let v = self.operand(op);
        self.call_rt(inc, &[v]);
        Ok(slot)
    }

    /// Marks module value `g`, if reference-counted and set.
    fn mark_global(&mut self, g: GlobalId) -> Result<()> {
        let (addr, ty) = self.global(g)?;
        if !ty.is_rc() {
            return Ok(());
        }
        let v = self
            .b
            .ins()
            .load(types::I64, MemFlagsData::trusted(), addr, 0);
        let (desc, len) = self.descriptor(ty)?;
        self.call_rt(Rt::Mark, &[desc, len, v]);
        Ok(())
    }

    /// `slots` in a frame array, its address; null for none.
    fn pack(&mut self, slots: &[Value]) -> Result<Value> {
        if slots.is_empty() {
            return Ok(self.b.ins().iconst(self.cg.ptr, 0));
        }
        let size = u32::try_from(8 * slots.len())
            .map_err(|_| backend("too many task arguments".to_string()))?;
        let data = StackSlotData::new(StackSlotKind::ExplicitSlot, size, 3);
        let ss = self.b.create_sized_stack_slot(data);
        for (k, v) in slots.iter().enumerate() {
            self.b.ins().stack_store(self.cg.ptr, *v, ss, 8 * k as i32);
        }
        Ok(self.b.ins().stack_addr(self.cg.ptr, ss, 0))
    }

    fn ret_of(&self, func: FuncId) -> Result<crate::ir::Ty> {
        self.cg
            .rets
            .get(func.0)
            .copied()
            .ok_or_else(|| backend("a task of an unknown function".to_string()))
    }
}

/// The functions some `submit` runs.
fn submitted(program: &Program) -> BTreeSet<FuncId> {
    let insts = program
        .funcs
        .iter()
        .flat_map(|f| &f.blocks)
        .flat_map(|b| &b.insts);
    insts
        .filter_map(|i| match i {
            Inst::Assign {
                value: Rvalue::Submit { func, .. },
                ..
            } => Some(*func),
            _ => None,
        })
        .collect()
}

/// The module values a task of `f` may load or replace: `f`'s, and
/// those of every function it may call or submit.
fn reads(program: &Program, f: FuncId) -> Vec<GlobalId> {
    let mut seen = BTreeSet::from([f]);
    let mut todo = vec![f];
    let mut out = BTreeSet::new();
    while let Some(id) = todo.pop() {
        for inst in program.funcs[id.0].blocks.iter().flat_map(|b| &b.insts) {
            let (global, callee) = touches(inst);
            out.extend(global);
            if let Some(c) = callee.filter(|c| seen.insert(*c)) {
                todo.push(c);
            }
        }
    }
    out.into_iter().collect()
}

/// The module value `inst` loads or replaces, and the function it calls
/// or submits.
fn touches(inst: &Inst) -> (Option<GlobalId>, Option<FuncId>) {
    match inst {
        Inst::Assign { value, .. } => match value {
            Rvalue::Global(g) => (Some(*g), None),
            Rvalue::Call(f, _) | Rvalue::Submit { func: f, .. } => (None, Some(*f)),
            _ => (None, None),
        },
        Inst::SetGlobal { global, .. } | Inst::DropGlobal(global) => (Some(*global), None),
        _ => (None, None),
    }
}

/// The module values each function's tasks reach, by function.
pub(super) type Reads = HashMap<FuncId, Vec<GlobalId>>;
