//! Cranelift backend: an [`ir::Program`] to a relocatable object file.
//!
//! One Cranelift function per IR function; IR locals become
//! `cranelift_frontend` variables, which build the SSA form. The module's
//! top-level code is exported as `pythrust_main`, which `pythrust-rt`'s
//! `main` calls; everything else has local linkage. A runtime error
//! calls `pt_fail` with its diagnostic, rendered at compile time and
//! stored as read-only data. `None` has no machine representation: a
//! function returning it returns nothing.
//!
//! Calls are counted in a module-private depth counter so recursion past
//! the interpreter's limit is a `RecursionError`, as in `run`, rather
//! than a native stack overflow.

use std::collections::HashMap;

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    types, AbiParam, InstBuilder, MemFlagsData, Signature, TrapCode, Value,
};
use cranelift_codegen::{isa, settings, settings::Configurable, Context};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext, Variable};
use cranelift_module::{
    default_libcall_names, DataDescription, DataId, FuncId as ClifFunc, Linkage, Module,
};
use cranelift_object::{ObjectBuilder, ObjectModule};

mod num;
mod seq;

use crate::diag::{DiagKind, Diagnostic, Result};
use crate::ir::{
    self, BlockId, CmpOp, Compound, Func, Inst, IntOp, LocalId, Operand, Program, Rvalue, Term, Ty,
};
use crate::span::Span;

/// Symbol the runtime's `main` calls.
pub const ENTRY: &str = "pythrust_main";

/// Compiles `program` for the host. `render` turns a runtime error into
/// the exact text the program prints to stderr when it happens.
pub fn emit(program: &Program, render: &dyn Fn(&Diagnostic) -> String) -> Result<Vec<u8>> {
    let mut cg = Codegen::new(render)?;
    cg.declare_globals(&program.globals)?;
    let ids = cg.declare(program)?;
    for (func, id) in program.funcs.iter().zip(&ids) {
        cg.define(func, *id, &ids, &program.types)?;
    }
    cg.module
        .finish()
        .emit()
        .map_err(|e| backend(format!("cannot write object: {e}")))
}

/// Runtime functions generated code calls.
#[derive(Clone, Copy)]
enum Rt {
    WriteInt,
    WriteBool,
    WriteNone,
    WriteByte,
    WriteStr,
    Fail,
    StrNew,
    StrInc,
    StrDec,
    StrConcat,
    StrRepeat,
    StrIndex,
    StrLen,
    StrCmp,
    StrContains,
    StrFromInt,
    StrFromBool,
    StrNone,
    StrRepr,
    FailMid,
    WriteFloat,
    StrFromFloat,
    FloatFloorDiv,
    FloatMod,
    FloatPow,
    IntPow,
    PowHint,
    FloatParse,
    IntParse,
    SeqNew,
    SeqPush,
    SeqLen,
    SeqGet,
    ListSet,
    SeqInc,
    SeqDec,
    SeqEq,
    SeqCmp,
    ListContains,
    WriteSeq,
    StrFromSeq,
}

impl Rt {
    const ALL: [Rt; 41] = [
        Rt::WriteInt,
        Rt::WriteBool,
        Rt::WriteNone,
        Rt::WriteByte,
        Rt::WriteStr,
        Rt::Fail,
        Rt::StrNew,
        Rt::StrInc,
        Rt::StrDec,
        Rt::StrConcat,
        Rt::StrRepeat,
        Rt::StrIndex,
        Rt::StrLen,
        Rt::StrCmp,
        Rt::StrContains,
        Rt::StrFromInt,
        Rt::StrFromBool,
        Rt::StrNone,
        Rt::StrRepr,
        Rt::FailMid,
        Rt::WriteFloat,
        Rt::StrFromFloat,
        Rt::FloatFloorDiv,
        Rt::FloatMod,
        Rt::FloatPow,
        Rt::IntPow,
        Rt::PowHint,
        Rt::FloatParse,
        Rt::IntParse,
        Rt::SeqNew,
        Rt::SeqPush,
        Rt::SeqLen,
        Rt::SeqGet,
        Rt::ListSet,
        Rt::SeqInc,
        Rt::SeqDec,
        Rt::SeqEq,
        Rt::SeqCmp,
        Rt::ListContains,
        Rt::WriteSeq,
        Rt::StrFromSeq,
    ];

    fn symbol(self) -> &'static str {
        match self {
            Rt::WriteInt => "pt_write_int",
            Rt::WriteBool => "pt_write_bool",
            Rt::WriteNone => "pt_write_none",
            Rt::WriteByte => "pt_write_byte",
            Rt::WriteStr => "pt_write_str",
            Rt::Fail => "pt_fail",
            Rt::StrNew => "pt_str_new",
            Rt::StrInc => "pt_str_inc",
            Rt::StrDec => "pt_str_dec",
            Rt::StrConcat => "pt_str_concat",
            Rt::StrRepeat => "pt_str_repeat",
            Rt::StrIndex => "pt_str_index",
            Rt::StrLen => "pt_str_len",
            Rt::StrCmp => "pt_str_cmp",
            Rt::StrContains => "pt_str_contains",
            Rt::StrFromInt => "pt_str_from_int",
            Rt::StrFromBool => "pt_str_from_bool",
            Rt::StrNone => "pt_str_none",
            Rt::StrRepr => "pt_str_repr",
            Rt::FailMid => "pt_fail_mid",
            Rt::WriteFloat => "pt_write_float",
            Rt::StrFromFloat => "pt_str_from_float",
            Rt::FloatFloorDiv => "pt_float_floordiv",
            Rt::FloatMod => "pt_float_mod",
            Rt::FloatPow => "pt_float_pow",
            Rt::IntPow => "pt_int_pow",
            Rt::PowHint => "pt_pow_hint",
            Rt::FloatParse => "pt_float_parse",
            Rt::IntParse => "pt_int_parse",
            Rt::SeqNew => "pt_seq_new",
            Rt::SeqPush => "pt_seq_push",
            Rt::SeqLen => "pt_seq_len",
            Rt::SeqGet => "pt_seq_get",
            Rt::ListSet => "pt_list_set",
            Rt::SeqInc => "pt_seq_inc",
            Rt::SeqDec => "pt_seq_dec",
            Rt::SeqEq => "pt_seq_eq",
            Rt::SeqCmp => "pt_seq_cmp",
            Rt::ListContains => "pt_list_contains",
            Rt::WriteSeq => "pt_write_seq",
            Rt::StrFromSeq => "pt_str_from_seq",
        }
    }

    /// Parameter and result types; `ptr` is a pointer, such as a string.
    fn sig(self, ptr: types::Type) -> (Vec<types::Type>, Option<types::Type>) {
        use types::{F64, I64, I8};
        match self {
            Rt::WriteInt => (vec![I64], None),
            Rt::WriteBool | Rt::WriteByte => (vec![I8], None),
            Rt::WriteNone => (vec![], None),
            Rt::WriteStr | Rt::StrInc | Rt::StrDec => (vec![ptr], None),
            Rt::Fail => (vec![ptr, ptr], None),
            Rt::StrNew | Rt::StrConcat => (vec![ptr, ptr], Some(ptr)),
            Rt::StrRepeat | Rt::StrIndex => (vec![ptr, I64], Some(ptr)),
            Rt::StrLen => (vec![ptr], Some(I64)),
            Rt::StrCmp => (vec![ptr, ptr], Some(I64)),
            Rt::StrContains => (vec![ptr, ptr], Some(I8)),
            Rt::StrFromInt => (vec![I64], Some(ptr)),
            Rt::StrFromBool => (vec![I8], Some(ptr)),
            Rt::StrNone => (vec![], Some(ptr)),
            Rt::StrRepr => (vec![ptr], Some(ptr)),
            Rt::FailMid => (vec![ptr; 5], None),
            Rt::WriteFloat => (vec![F64], None),
            Rt::StrFromFloat => (vec![F64], Some(ptr)),
            Rt::FloatFloorDiv | Rt::FloatMod => (vec![F64, F64], Some(F64)),
            Rt::FloatPow => (vec![F64, F64, ptr], Some(F64)),
            Rt::IntPow => (vec![I64, I64, ptr], Some(I64)),
            Rt::PowHint => (vec![I64, I64], Some(ptr)),
            Rt::FloatParse => (vec![ptr, ptr], Some(F64)),
            Rt::IntParse => (vec![ptr, ptr], Some(I64)),
            Rt::SeqNew => (vec![ptr, ptr, I64], Some(ptr)),
            Rt::SeqPush => (vec![ptr, I64], None),
            Rt::SeqLen => (vec![ptr], Some(I64)),
            Rt::SeqGet => (vec![ptr, I64, ptr], Some(I64)),
            Rt::ListSet => (vec![ptr, I64, I64], Some(I8)),
            Rt::SeqInc | Rt::SeqDec | Rt::WriteSeq => (vec![ptr], None),
            Rt::SeqEq => (vec![ptr, ptr], Some(I8)),
            Rt::SeqCmp => (vec![ptr, ptr], Some(I64)),
            Rt::ListContains => (vec![ptr, I64], Some(I8)),
            Rt::StrFromSeq => (vec![ptr], Some(ptr)),
        }
    }
}

struct Codegen<'r> {
    module: ObjectModule,
    render: &'r dyn Fn(&Diagnostic) -> String,
    rt: HashMap<&'static str, ClifFunc>,
    /// The `i64` count of active Pythrust calls.
    depth: DataId,
    /// Read-only byte strings (failure messages, literals), deduplicated.
    bytes: HashMap<String, (DataId, usize)>,
    ptr: types::Type,
    /// `ir::Program::globals`: each a zeroed, module-private word.
    globals: Vec<(DataId, Ty)>,
}

impl<'r> Codegen<'r> {
    fn new(render: &'r dyn Fn(&Diagnostic) -> String) -> Result<Self> {
        let mut flags = settings::builder();
        // PIC, because system linkers default to position-independent
        // executables.
        for (name, value) in [("opt_level", "speed"), ("is_pic", "true")] {
            flags.set(name, value).map_err(|e| backend(e.to_string()))?;
        }
        let isa = isa::lookup(target_lexicon::Triple::host())
            .map_err(|e| backend(format!("unsupported host: {e}")))?
            .finish(settings::Flags::new(flags))
            .map_err(|e| backend(e.to_string()))?;
        let builder =
            ObjectBuilder::new(isa, "pythrust", default_libcall_names()).map_err(module_err)?;
        let mut module = ObjectModule::new(builder);
        let depth = module
            .declare_data("pythrust_depth", Linkage::Local, true, false)
            .map_err(module_err)?;
        let mut zero = DataDescription::new();
        zero.define_zeroinit(8);
        module.define_data(depth, &zero).map_err(module_err)?;
        let ptr = module.target_config().pointer_type();
        let mut cg = Codegen {
            module,
            render,
            depth,
            rt: HashMap::new(),
            bytes: HashMap::new(),
            ptr,
            globals: Vec::new(),
        };
        for rt in Rt::ALL {
            let mut sig = cg.module.make_signature();
            let (params, ret) = rt.sig(ptr);
            sig.params.extend(params.into_iter().map(AbiParam::new));
            sig.returns.extend(ret.map(AbiParam::new));
            let id = cg
                .module
                .declare_function(rt.symbol(), Linkage::Import, &sig)
                .map_err(module_err)?;
            cg.rt.insert(rt.symbol(), id);
        }
        Ok(cg)
    }

    fn declare_globals(&mut self, tys: &[Ty]) -> Result<()> {
        for ty in tys {
            let id = self
                .module
                .declare_anonymous_data(true, false)
                .map_err(module_err)?;
            let mut zero = DataDescription::new();
            zero.define_zeroinit(8);
            self.module.define_data(id, &zero).map_err(module_err)?;
            self.globals.push((id, *ty));
        }
        Ok(())
    }

    fn declare(&mut self, program: &Program) -> Result<Vec<ClifFunc>> {
        let mut ids = Vec::new();
        for (i, f) in program.funcs.iter().enumerate() {
            let sig = self.signature(f);
            let (name, linkage) = match i == program.main.0 {
                true => (ENTRY.to_string(), Linkage::Export),
                false => (format!("py{i}_{}", f.name), Linkage::Local),
            };
            ids.push(
                self.module
                    .declare_function(&name, linkage, &sig)
                    .map_err(module_err)?,
            );
        }
        Ok(ids)
    }

    fn signature(&self, f: &Func) -> Signature {
        let mut sig = self.module.make_signature();
        for p in &f.params {
            sig.params.push(AbiParam::new(self.clif_ty(f.locals[p.0])));
        }
        if f.ret != Ty::None {
            sig.returns.push(AbiParam::new(self.clif_ty(f.ret)));
        }
        sig
    }

    fn clif_ty(&self, ty: Ty) -> types::Type {
        match ty {
            Ty::Int => types::I64,
            Ty::Float => types::F64,
            Ty::Bool | Ty::None => types::I8,
            Ty::Str | Ty::List(_) | Ty::Tuple(_) => self.ptr,
        }
    }

    /// The data object holding `text`, and its length.
    fn bytes(&mut self, text: String) -> Result<(DataId, usize)> {
        if let Some(found) = self.bytes.get(&text) {
            return Ok(*found);
        }
        let id = self
            .module
            .declare_anonymous_data(false, false)
            .map_err(module_err)?;
        let mut data = DataDescription::new();
        data.define(text.clone().into_bytes().into_boxed_slice());
        self.module.define_data(id, &data).map_err(module_err)?;
        let entry = (id, text.len());
        self.bytes.insert(text, entry);
        Ok(entry)
    }

    fn define(
        &mut self,
        func: &Func,
        id: ClifFunc,
        ids: &[ClifFunc],
        types: &[Compound],
    ) -> Result<()> {
        let mut ctx = Context::new();
        ctx.func.signature = self.signature(func);
        FnGen::build(
            self,
            Ctx { func, ids, types },
            &mut ctx,
            &mut FunctionBuilderContext::new(),
        )?;
        self.module
            .define_function(id, &mut ctx)
            .map_err(module_err)
    }
}

/// What translating one function reads.
struct Ctx<'c> {
    func: &'c Func,
    ids: &'c [ClifFunc],
    types: &'c [Compound],
}

/// Translates one function.
struct FnGen<'b, 'c, 'r> {
    cg: &'c mut Codegen<'r>,
    b: FunctionBuilder<'b>,
    func: &'c Func,
    ids: &'c [ClifFunc],
    types: &'c [Compound],
    vars: Vec<Variable>,
    blocks: Vec<cranelift_codegen::ir::Block>,
}

impl<'b, 'c, 'r> FnGen<'b, 'c, 'r> {
    fn build(
        cg: &'c mut Codegen<'r>,
        Ctx { func, ids, types }: Ctx<'c>,
        ctx: &'b mut Context,
        fctx: &'b mut FunctionBuilderContext,
    ) -> Result<()> {
        let mut b = FunctionBuilder::new(&mut ctx.func, fctx);
        let blocks: Vec<_> = func.blocks.iter().map(|_| b.create_block()).collect();
        let vars: Vec<_> = func
            .locals
            .iter()
            .map(|t| b.declare_var(cg.clif_ty(*t)))
            .collect();
        let entry = *blocks
            .first()
            .ok_or_else(|| backend(format!("`{}` has no blocks", func.name)))?;
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let params = b.block_params(entry).to_vec();
        for (i, var) in vars.iter().enumerate() {
            let v = match func.params.iter().position(|p| p.0 == i) {
                Some(k) => params[k],
                None if func.locals[i] == Ty::Float => b.ins().f64const(0.0),
                None => b.ins().iconst(cg.clif_ty(func.locals[i]), 0),
            };
            b.def_var(*var, v);
        }
        let mut g = FnGen {
            cg,
            b,
            func,
            ids,
            types,
            vars,
            blocks,
        };
        for (i, block) in func.blocks.iter().enumerate() {
            if i > 0 {
                g.b.switch_to_block(g.blocks[i]);
            }
            for inst in &block.insts {
                g.inst(inst)?;
            }
            g.term(&block.term)?;
        }
        g.b.seal_all_blocks();
        let config = g.cg.module.target_config();
        g.b.finalize(config);
        Ok(())
    }

    fn inst(&mut self, inst: &Inst) -> Result<()> {
        match inst {
            Inst::Assign { dest, value, span } => {
                let v = self.rvalue(value, *dest, *span)?;
                self.b.def_var(self.vars[dest.0], v);
            }
            Inst::Print { args, sep, end } => {
                for (i, op) in args.iter().enumerate() {
                    if i > 0 {
                        self.write_text(*sep, b' ');
                    }
                    self.write(*op);
                }
                self.write_text(*end, b'\n');
            }
            Inst::Retain(l) => {
                let (inc, _) = rc_fns(self.func.locals[l.0]);
                let s = self.b.use_var(self.vars[l.0]);
                self.call_rt(inc, &[s]);
            }
            Inst::Release(l) => {
                let (_, dec) = rc_fns(self.func.locals[l.0]);
                let s = self.b.use_var(self.vars[l.0]);
                self.call_rt(dec, &[s]);
                self.forget(*l);
            }
            Inst::Forget(l) => self.forget(*l),
            Inst::Append { list, value } => {
                let (s, v) = (self.operand(*list), self.slot(*value));
                self.call_rt(Rt::SeqPush, &[s, v]);
            }
            Inst::SetItem {
                list,
                index,
                value,
                span,
            } => self.set_item(*list, *index, *value, *span)?,
            Inst::SetGlobal { global, value } => {
                let v = self.operand(*value);
                self.set_global(*global, v)?;
            }
            Inst::DropGlobal(global) => {
                let null = self.b.ins().iconst(self.cg.ptr, 0);
                self.set_global(*global, null)?;
            }
        }
        Ok(())
    }

    /// The address and type of global `g`.
    fn global(&mut self, g: ir::GlobalId) -> Result<(Value, Ty)> {
        let (data, ty) = *self
            .cg
            .globals
            .get(g.0)
            .ok_or_else(|| backend("an unknown global".to_string()))?;
        let gv = self.cg.module.declare_data_in_func(data, self.b.func);
        Ok((self.b.ins().symbol_value(self.cg.ptr, gv), ty))
    }

    /// Stores `v` in global `g`: a reference-counted global takes a new
    /// reference to `v` (null: none) and drops its old one.
    fn set_global(&mut self, g: ir::GlobalId, v: Value) -> Result<()> {
        let (addr, ty) = self.global(g)?;
        let flags = MemFlagsData::trusted();
        if !ty.is_rc() {
            self.b.ins().store(flags, v, addr, 0);
            return Ok(());
        }
        let (inc, dec) = rc_fns(ty);
        self.call_rt(inc, &[v]);
        let old = self.b.ins().load(self.cg.ptr, flags, addr, 0);
        self.b.ins().store(flags, v, addr, 0);
        self.call_rt(dec, &[old]);
        Ok(())
    }

    fn forget(&mut self, l: LocalId) {
        let null = self.b.ins().iconst(self.cg.ptr, 0);
        self.b.def_var(self.vars[l.0], null);
    }

    fn write(&mut self, op: Operand) {
        let (rt, args) = match self.func.ty(op) {
            Ty::Int => (Rt::WriteInt, vec![self.operand(op)]),
            Ty::Bool => (Rt::WriteBool, vec![self.operand(op)]),
            Ty::None => (Rt::WriteNone, vec![]),
            Ty::Str => (Rt::WriteStr, vec![self.operand(op)]),
            Ty::Float => (Rt::WriteFloat, vec![self.operand(op)]),
            Ty::List(_) | Ty::Tuple(_) => (Rt::WriteSeq, vec![self.operand(op)]),
        };
        self.call_rt(rt, &args);
    }

    /// Writes the string `text`, or `default` if there is none.
    fn write_text(&mut self, text: Option<Operand>, default: u8) {
        match text {
            Some(s) => self.write(s),
            None => self.write_byte(default),
        }
    }

    fn write_byte(&mut self, byte: u8) {
        let v = self.b.ins().iconst(types::I8, i64::from(byte));
        self.call_rt(Rt::WriteByte, &[v]);
    }

    /// Calls `rt`, giving its result if it has one.
    fn call_rt(&mut self, rt: Rt, args: &[Value]) -> Option<Value> {
        let id = self.cg.rt[rt.symbol()];
        let fref = self.cg.module.declare_func_in_func(id, self.b.func);
        let call = self.b.ins().call(fref, args);
        self.b.inst_results(call).first().copied()
    }

    /// Calls `rt`, which has a result.
    fn call_rt_value(&mut self, rt: Rt, args: &[Value]) -> Result<Value> {
        self.call_rt(rt, args)
            .ok_or_else(|| backend(format!("`{}` has no result", rt.symbol())))
    }

    /// Calls `rt` on `ops`, which has a result.
    fn rt_on(&mut self, rt: Rt, ops: &[Operand]) -> Result<Value> {
        let args: Vec<Value> = ops.iter().map(|op| self.operand(*op)).collect();
        self.call_rt_value(rt, &args)
    }

    /// Calls `rt` on `ops`, failing at `span` if it gives null.
    fn rt_nonnull(&mut self, rt: Rt, ops: &[Operand], span: Span, message: &str) -> Result<Value> {
        let s = self.rt_on(rt, ops)?;
        let null = self.b.ins().icmp_imm_s(IntCC::Equal, s, 0);
        self.fail_if(null, span, message)?;
        Ok(s)
    }

    fn operand(&mut self, op: Operand) -> Value {
        match op {
            Operand::Local(LocalId(i)) => self.b.use_var(self.vars[i]),
            Operand::Int(n) => self.b.ins().iconst(types::I64, n),
            Operand::Bool(v) => self.b.ins().iconst(types::I8, i64::from(v)),
            Operand::None => self.b.ins().iconst(types::I8, 0),
            Operand::Float(f) => self.b.ins().f64const(f),
        }
    }

    fn rvalue(&mut self, value: &Rvalue, dest: LocalId, span: Span) -> Result<Value> {
        match value {
            Rvalue::Use(op) => Ok(self.operand(*op)),
            Rvalue::Int(op, a, b) => {
                let (a, b) = (self.operand(*a), self.operand(*b));
                self.int_op(*op, a, b, span)
            }
            Rvalue::Neg(a) if self.func.ty(*a) == Ty::Float => {
                let a = self.operand(*a);
                Ok(self.b.ins().fneg(a))
            }
            Rvalue::Neg(a) => {
                let a = self.operand(*a);
                let min = self.b.ins().icmp_imm_s(IntCC::Equal, a, i64::MIN);
                self.fail_if(min, span, OVERFLOW)?;
                Ok(self.b.ins().ineg(a))
            }
            Rvalue::Cmp(op, a, b) if self.func.ty(*a) == Ty::Str => {
                let order = self.rt_on(Rt::StrCmp, &[*a, *b])?;
                Ok(self.b.ins().icmp_imm_s(int_cc(*op), order, 0))
            }
            Rvalue::Cmp(op, a, b) if is_seq(self.func.ty(*a)) => self.seq_cmp(*op, *a, *b),
            Rvalue::Cmp(op, a, b) if self.func.ty(*a) == Ty::Float => {
                let (a, b) = (self.operand(*a), self.operand(*b));
                Ok(self.b.ins().fcmp(num::float_cc(*op), a, b))
            }
            Rvalue::Cmp(op, a, b) => {
                let (a, b) = (self.operand(*a), self.operand(*b));
                Ok(self.b.ins().icmp(int_cc(*op), a, b))
            }
            Rvalue::AddOverflows(a, b) => {
                let (a, b) = (self.operand(*a), self.operand(*b));
                Ok(self.b.ins().sadd_overflow(a, b).1)
            }
            Rvalue::Call(callee, args) => self.call(*callee, args, dest, span),
            Rvalue::StrLit(text) => self.str_lit(text),
            Rvalue::Concat(a, b) => self.rt_on(Rt::StrConcat, &[*a, *b]),
            Rvalue::Repeat(s, n) => self.rt_nonnull(Rt::StrRepeat, &[*s, *n], span, TOO_LARGE),
            Rvalue::Index(s, i) if is_seq(self.func.ty(*s)) => self.item(*s, *i, dest, span),
            Rvalue::Index(s, i) => self.rt_nonnull(Rt::StrIndex, &[*s, *i], span, STR_INDEX),
            Rvalue::Len(s) if is_seq(self.func.ty(*s)) => self.rt_on(Rt::SeqLen, &[*s]),
            Rvalue::Len(s) => self.rt_on(Rt::StrLen, &[*s]),
            Rvalue::Contains { hay, needle } if is_seq(self.func.ty(*hay)) => {
                let (s, v) = (self.operand(*hay), self.slot(*needle));
                self.call_rt_value(Rt::ListContains, &[s, v])
            }
            Rvalue::Contains { hay, needle } => self.rt_on(Rt::StrContains, &[*hay, *needle]),
            Rvalue::ToStr(op) => match self.func.ty(*op) {
                Ty::Int => self.rt_on(Rt::StrFromInt, &[*op]),
                Ty::Bool => self.rt_on(Rt::StrFromBool, &[*op]),
                Ty::None => self.rt_on(Rt::StrNone, &[]),
                Ty::Float => self.rt_on(Rt::StrFromFloat, &[*op]),
                Ty::List(_) | Ty::Tuple(_) => self.rt_on(Rt::StrFromSeq, &[*op]),
                Ty::Str => Err(backend("`str` of a string".to_string())),
            },
            Rvalue::NewSeq(items) => self.new_seq(self.func.locals[dest.0], items),
            Rvalue::Field(t, k) => self.field(*t, *k, dest),
            Rvalue::Global(g) => {
                let (addr, ty) = self.global(*g)?;
                let v = self
                    .b
                    .ins()
                    .load(self.cg.clif_ty(ty), MemFlagsData::trusted(), addr, 0);
                if ty.is_rc() {
                    self.call_rt(rc_fns(ty).0, &[v]);
                }
                Ok(v)
            }
            Rvalue::Float(op, a, b) => self.float_op(*op, *a, *b, span),
            Rvalue::IntPow(a, b) => self.int_pow(*a, *b, span),
            Rvalue::ToFloat(a) => Ok(self.int_as_float(*a)),
            Rvalue::FloatToInt(a) => self.float_to_int(*a, span),
            Rvalue::ParseFloat(s) => self.parse_float(*s, span),
            Rvalue::ParseInt(s) => self.parse_int(*s, span),
        }
    }

    /// A new string holding `text`, copied from read-only data.
    fn str_lit(&mut self, text: &str) -> Result<Value> {
        let (addr, len) = self.data(text.to_string())?;
        self.call_rt_value(Rt::StrNew, &[addr, len])
    }

    /// The address and length of `text` in read-only data.
    fn data(&mut self, text: String) -> Result<(Value, Value)> {
        let (data, len) = self.cg.bytes(text)?;
        let gv = self.cg.module.declare_data_in_func(data, self.b.func);
        let addr = self.b.ins().symbol_value(self.cg.ptr, gv);
        let len = self.b.ins().iconst(self.cg.ptr, len as i64);
        Ok((addr, len))
    }

    /// Calls `callee`, failing at `span` if `MAX_DEPTH` calls are active.
    fn call(
        &mut self,
        callee: ir::FuncId,
        args: &[Operand],
        dest: LocalId,
        span: Span,
    ) -> Result<Value> {
        let id = *self
            .ids
            .get(callee.0)
            .ok_or_else(|| backend("call to an unknown function".to_string()))?;
        let fref = self.cg.module.declare_func_in_func(id, self.b.func);
        let args: Vec<Value> = args.iter().map(|op| self.operand(*op)).collect();
        let ptr = self.cg.module.target_config().pointer_type();
        let gv = self
            .cg
            .module
            .declare_data_in_func(self.cg.depth, self.b.func);
        let addr = self.b.ins().symbol_value(ptr, gv);
        let flags = MemFlagsData::trusted();
        let depth = self.b.ins().load(types::I64, flags, addr, 0);
        let full = self
            .b
            .ins()
            .icmp_imm_s(IntCC::SignedGreaterThanOrEqual, depth, MAX_DEPTH);
        self.fail_if(full, span, RECURSION)?;
        let deeper = self.b.ins().iadd_imm_s(depth, 1);
        self.b.ins().store(flags, deeper, addr, 0);
        let call = self.b.ins().call(fref, &args);
        self.b.ins().store(flags, depth, addr, 0);
        match self.b.inst_results(call).first() {
            Some(v) => Ok(*v),
            None => Ok(self
                .b
                .ins()
                .iconst(self.cg.clif_ty(self.func.locals[dest.0]), 0)),
        }
    }

    /// Checked `a op b` with Python's floor semantics for `//` and `%`.
    fn int_op(&mut self, op: IntOp, a: Value, b: Value, span: Span) -> Result<Value> {
        let (v, overflow) = match op {
            IntOp::Add => self.b.ins().sadd_overflow(a, b),
            IntOp::Sub => self.b.ins().ssub_overflow(a, b),
            IntOp::Mul => self.b.ins().smul_overflow(a, b),
            IntOp::FloorDiv => return self.floor_div(a, b, span),
            IntOp::Mod => return self.modulo(a, b, span),
        };
        self.fail_if(overflow, span, OVERFLOW)?;
        Ok(v)
    }

    fn floor_div(&mut self, a: Value, b: Value, span: Span) -> Result<Value> {
        self.check_divisor(b, span)?;
        let min = self.b.ins().icmp_imm_s(IntCC::Equal, a, i64::MIN);
        let neg1 = self.b.ins().icmp_imm_s(IntCC::Equal, b, -1);
        let overflow = self.b.ins().band(min, neg1);
        self.fail_if(overflow, span, OVERFLOW)?;
        let q = self.b.ins().sdiv(a, b);
        let r = self.b.ins().srem(a, b);
        let adjust = self.floor_adjust(r, b);
        let adjust = self.b.ins().uextend(types::I64, adjust);
        Ok(self.b.ins().isub(q, adjust))
    }

    fn modulo(&mut self, a: Value, b: Value, span: Span) -> Result<Value> {
        self.check_divisor(b, span)?;
        // `i64::MIN % -1` traps in hardware; any `x % -1` is 0, as is `x % 1`.
        let neg1 = self.b.ins().icmp_imm_s(IntCC::Equal, b, -1);
        let one = self.b.ins().iconst(types::I64, 1);
        let safe = self.b.ins().select(neg1, one, b);
        let r = self.b.ins().srem(a, safe);
        let adjust = self.floor_adjust(r, safe);
        let zero = self.b.ins().iconst(types::I64, 0);
        let add = self.b.ins().select(adjust, safe, zero);
        Ok(self.b.ins().iadd(r, add))
    }

    /// Whether a truncating remainder `r` of a division by `b` has the
    /// wrong sign for flooring: nonzero, and of the opposite sign to `b`.
    fn floor_adjust(&mut self, r: Value, b: Value) -> Value {
        let nonzero = self.b.ins().icmp_imm_s(IntCC::NotEqual, r, 0);
        let signs = self.b.ins().bxor(r, b);
        let differ = self.b.ins().icmp_imm_s(IntCC::SignedLessThan, signs, 0);
        self.b.ins().band(nonzero, differ)
    }

    fn check_divisor(&mut self, b: Value, span: Span) -> Result<()> {
        let zero = self.b.ins().icmp_imm_s(IntCC::Equal, b, 0);
        self.fail_if(zero, span, ZERO_DIV)
    }

    /// Branches to a cold failure block when `cond` is nonzero.
    fn fail_if(&mut self, cond: Value, span: Span, message: &str) -> Result<()> {
        let (fail, ok) = (self.b.create_block(), self.b.create_block());
        self.b.set_cold_block(fail);
        self.b.ins().brif(cond, fail, &[], ok, &[]);
        self.b.switch_to_block(fail);
        self.fail(&runtime(span, message))?;
        self.b.switch_to_block(ok);
        Ok(())
    }

    fn fail(&mut self, diag: &Diagnostic) -> Result<()> {
        let (addr, len) = self.data((self.cg.render)(diag))?;
        self.call_rt(Rt::Fail, &[addr, len]);
        self.b.ins().trap(TrapCode::unwrap_user(1));
        Ok(())
    }

    fn term(&mut self, term: &Term) -> Result<()> {
        match term {
            Term::Goto(to) => {
                let to = self.block(*to);
                self.b.ins().jump(to, &[]);
            }
            Term::Branch { cond, then, orelse } => {
                let c = self.operand(*cond);
                let (t, e) = (self.block(*then), self.block(*orelse));
                self.b.ins().brif(c, t, &[], e, &[]);
            }
            Term::Return(op) => {
                let vals = match self.func.ret {
                    Ty::None => vec![],
                    _ => vec![self.operand(*op)],
                };
                self.b.ins().return_(&vals);
            }
            Term::Fail(diag) => self.fail(diag)?,
            Term::Unreachable => {
                self.b.ins().trap(TrapCode::unwrap_user(2));
            }
        }
        Ok(())
    }

    fn block(&self, id: BlockId) -> cranelift_codegen::ir::Block {
        self.blocks[id.0]
    }
}

const OVERFLOW: &str = "OverflowError: integer overflow";
const RECURSION: &str = "RecursionError: maximum recursion depth exceeded";
/// Active calls allowed, as in the interpreter.
const MAX_DEPTH: i64 = crate::interp::MAX_DEPTH as i64;
const ZERO_DIV: &str = "ZeroDivisionError: integer division or modulo by zero";
const TOO_LARGE: &str = "MemoryError: repeated sequence is too large";
const STR_INDEX: &str = "IndexError: string index out of range";

/// Whether `ty` is a list or tuple.
fn is_seq(ty: Ty) -> bool {
    matches!(ty, Ty::List(_) | Ty::Tuple(_))
}

/// The runtime functions adding and dropping a reference to a `ty`.
fn rc_fns(ty: Ty) -> (Rt, Rt) {
    match is_seq(ty) {
        true => (Rt::SeqInc, Rt::SeqDec),
        false => (Rt::StrInc, Rt::StrDec),
    }
}

fn int_cc(op: CmpOp) -> IntCC {
    match op {
        CmpOp::Eq => IntCC::Equal,
        CmpOp::Ne => IntCC::NotEqual,
        CmpOp::Lt => IntCC::SignedLessThan,
        CmpOp::Le => IntCC::SignedLessThanOrEqual,
        CmpOp::Gt => IntCC::SignedGreaterThan,
        CmpOp::Ge => IntCC::SignedGreaterThanOrEqual,
    }
}

fn runtime(span: Span, message: &str) -> Diagnostic {
    Diagnostic {
        span,
        message: message.to_string(),
        kind: DiagKind::Runtime,
    }
}

fn backend(message: String) -> Diagnostic {
    Diagnostic {
        span: Span { start: 0, end: 0 },
        message: format!("internal: {message}"),
        kind: DiagKind::Backend,
    }
}

fn module_err(e: cranelift_module::ModuleError) -> Diagnostic {
    backend(e.to_string())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::{infer, lexer, lower, parser};

    /// Compiles `src`, returning the object and every diagnostic rendered.
    fn compile(src: &str) -> (Vec<u8>, Vec<String>) {
        let rendered = RefCell::new(Vec::new());
        let render = |d: &Diagnostic| {
            rendered.borrow_mut().push(d.message.clone());
            d.message.clone()
        };
        let module = parser::parse(&lexer::lex(src).expect("lex")).expect("parse");
        let types = infer::infer(&module).expect("infer");
        let program = lower::lower(&module, &types).expect("lower");
        let object = emit(&program, &render).expect("emit");
        (object, rendered.into_inner())
    }

    #[test]
    fn emits_an_object_with_the_entry_symbol() {
        let (object, _) = compile("def f(n):\n    return n * 2\nprint(f(21), True, None)\n");
        assert!(object.len() > 64);
        let entry = ENTRY.as_bytes();
        assert!(object.windows(entry.len()).any(|w| w == entry));
    }

    #[test]
    fn renders_each_failure_site_once_at_compile_time() {
        let (_, rendered) = compile("x = 5\nprint(x + 1, x // 2, -x)\n");
        let count = |m: &str| rendered.iter().filter(|r| r.as_str() == m).count();
        // `+`, `//` and `-` can overflow; `//` can also divide by zero.
        assert_eq!(count(OVERFLOW), 3);
        assert_eq!(count(ZERO_DIV), 1);
    }

    #[test]
    fn run_time_message_parts_are_rendered_around_a_marker() {
        let (_, rendered) = compile("n = -2\nprint(2 ** n, float('x'), 1.5 / n)\n");
        let marked = rendered.iter().filter(|r| r.contains('\0')).count();
        assert_eq!(marked, 2, "{rendered:?}");
        assert!(rendered
            .iter()
            .any(|r| r == "ZeroDivisionError: division by zero"));
    }

    #[test]
    fn string_failures_are_rendered_and_literals_stored_once() {
        let (object, rendered) = compile("s = 'needle'\nprint(s[5], s * 3, 'needle')\n");
        let count = |m: &str| rendered.iter().filter(|r| r.as_str() == m).count();
        assert_eq!((count(STR_INDEX), count(TOO_LARGE)), (1, 1));
        let lit = b"needle";
        assert_eq!(object.windows(lit.len()).filter(|w| w == lit).count(), 1);
    }
}
