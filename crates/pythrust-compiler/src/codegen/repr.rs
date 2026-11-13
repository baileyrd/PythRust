//! `__repr__` of plain classes for the runtime's `repr` walker (D170): a
//! thunk per class the program renders inside a container, registered
//! with the runtime when the program starts. The walker calls it with
//! the thread's context and a borrowed object; it counts the call as any
//! other and gives the string, or null if the call raised or ran out of
//! depth. No function pointer lives in a data object.

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{types, AbiParam, InstBuilder, MemFlagsData, Signature};
use cranelift_codegen::Context;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId as ClifFunc, Linkage, Module};

use super::{backend, module_err, rc_inc, Codegen, FnGen, Rt, CTX_DEPTH, MAX_DEPTH};
use crate::diag::Result;
use crate::ir::{FuncId, Program, Ty};

/// The most classes a descriptor can name: two 7-bit bytes.
const MAX_CLASSES: usize = 1 << 14;

/// A registered `__repr__`: the class's number, its thunk and function.
pub(super) struct Thunk {
    pub(super) class: u16,
    thunk: ClifFunc,
    func: FuncId,
}

impl Codegen<'_> {
    /// Declares the thunk of each `__repr__` lowering found.
    pub(super) fn declare_reprs(&mut self, program: &Program) -> Result<()> {
        for &f in &program.reprs {
            let func = &program.funcs[f.0];
            let class = match func.params.first().map(|p| func.locals[p.0]) {
                Some(Ty::Obj(id)) if id.0 < MAX_CLASSES => id.0 as u16,
                _ => return Err(backend("a `__repr__` of no registrable class".to_string())),
            };
            let sig = self.repr_signature();
            let name = format!("pt_repr{}", f.0);
            let thunk = self
                .module
                .declare_function(&name, Linkage::Local, &sig)
                .map_err(module_err)?;
            self.reprs.push(Thunk {
                class,
                thunk,
                func: f,
            });
        }
        Ok(())
    }

    /// `pythrust-rt`'s `ReprFn`: (context, object) -> string.
    fn repr_signature(&self) -> Signature {
        let mut sig = self.module.make_signature();
        sig.params.extend([AbiParam::new(self.ptr); 2]);
        sig.returns.push(AbiParam::new(self.ptr));
        sig
    }

    /// Defines each declared thunk.
    pub(super) fn define_reprs(&mut self, program: &Program, ids: &[ClifFunc]) -> Result<()> {
        for k in 0..self.reprs.len() {
            let (thunk, f) = (self.reprs[k].thunk, self.reprs[k].func);
            let callee = *ids
                .get(f.0)
                .ok_or_else(|| backend("a `__repr__` of an unknown function".to_string()))?;
            let mut ctx = Context::new();
            ctx.func.signature = self.repr_signature();
            self.repr_body(program, f, callee, &mut ctx)?;
            self.define_function(thunk, &mut ctx)?;
        }
        Ok(())
    }

    fn repr_body(
        &mut self,
        program: &Program,
        f: FuncId,
        callee: ClifFunc,
        ctx: &mut Context,
    ) -> Result<()> {
        let func = &program.funcs[f.0];
        let param = func.params[0];
        let mut fctx = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut ctx.func, &mut fctx);
        let (entry, deep, go) = (b.create_block(), b.create_block(), b.create_block());
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let (cx, obj) = match b.block_params(entry) {
            [cx, obj] => (*cx, *obj),
            _ => return Err(backend("a thunk without its two parameters".to_string())),
        };
        let flags = MemFlagsData::trusted();
        let depth = b.ins().load(types::I64, flags, cx, CTX_DEPTH);
        let full = b
            .ins()
            .icmp_imm_s(IntCC::SignedGreaterThanOrEqual, depth, MAX_DEPTH);
        b.ins().brif(full, deep, &[], go, &[]);
        b.switch_to_block(deep);
        let null = b.ins().iconst(self.ptr, 0);
        b.ins().return_(&[null]);
        b.switch_to_block(go);
        if !func.borrowed.contains(&param) {
            let inc = self.rt[rc_inc(&program.types, func.locals[param.0])?.symbol()];
            let inc = self.module.declare_func_in_func(inc, b.func);
            b.ins().call(inc, &[obj]);
        }
        let deeper = b.ins().iadd_imm_s(depth, 1);
        b.ins().store(flags, deeper, cx, CTX_DEPTH);
        let fref = self.module.declare_func_in_func(callee, b.func);
        let call = b.ins().call(fref, &[cx, obj]);
        b.ins().store(flags, depth, cx, CTX_DEPTH);
        let text = b.inst_results(call).first().copied();
        let text = text.ok_or_else(|| backend("a `__repr__` without a result".to_string()))?;
        b.ins().return_(&[text]);
        b.seal_all_blocks();
        b.finalize(self.module.target_config());
        Ok(())
    }
}

impl FnGen<'_, '_, '_> {
    /// Registers the recursive records' descriptors and every `__repr__`
    /// thunk with the runtime.
    pub(super) fn register_reprs(&mut self) -> Result<()> {
        for k in 0..self.cg.descs.len() {
            let (id, text) = self.cg.descs[k].clone();
            let (addr, len) = self.data(text)?;
            let id = self.b.ins().iconst(types::I64, i64::from(id));
            self.call_rt(Rt::RegisterDesc, &[id, addr, len]);
        }
        for k in 0..self.cg.reprs.len() {
            let (class, thunk) = (self.cg.reprs[k].class, self.cg.reprs[k].thunk);
            let class = self.b.ins().iconst(types::I64, i64::from(class));
            let fref = self.cg.module.declare_func_in_func(thunk, self.b.func);
            let addr = self.b.ins().func_addr(self.cg.ptr, fref);
            self.call_rt(Rt::RegisterRepr, &[class, addr]);
        }
        Ok(())
    }
}
