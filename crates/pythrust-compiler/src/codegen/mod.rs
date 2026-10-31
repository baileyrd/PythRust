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
//! A raised exception is the runtime's pending one (`exc.rs`); the IR
//! spells out every test of it.
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

mod exc;
mod num;
mod opt;
mod seq;
mod stdlib;

use crate::diag::{DiagKind, Diagnostic, Result};
use crate::exc::ExcKind;
use crate::ir::{
    self, BlockId, CmpOp, Compound, FloatOp, Func, Inst, IntOp, LocalId, Operand, Program, Rvalue,
    Term, Ty,
};
use crate::span::Span;

/// Symbol the runtime's `main` calls.
pub const ENTRY: &str = "pythrust_main";

/// Compiles `program` for the host. `render` turns a runtime error into
/// the exact text the program prints to stderr when it happens.
pub fn emit(program: &Program, render: &dyn Fn(&Diagnostic) -> String) -> Result<Vec<u8>> {
    refuse_gaps(program)?;
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
    ListCopy,
    WriteSeq,
    StrFromSeq,
    RangeLen,
    RangeGet,
    RangeContains,
    RangeContainsFloat,
    ExcStr,
    Raise,
    RaiseValue,
    RaiseMsg,
    PendingIs,
    Catch,
    Uncaught,
    DictSet,
    DictGet,
    SetAdd,
    TableContains,
    TableEntry,
    ViewNew,
    ListOf,
    ListSort,
    Argv,
    OsListdir,
    OsPathJoin,
    OsPathIs,
    OsPathGetsize,
    ObjInc,
    ObjDec,
    ObjLen,
    ObjEq,
    ObjStr,
    WriteObj,
    Open,
    FileRead,
    FileClose,
    Sha256Hex,
}

impl Rt {
    const ALL: [Rt; 76] = [
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
        Rt::ListCopy,
        Rt::WriteSeq,
        Rt::StrFromSeq,
        Rt::RangeLen,
        Rt::RangeGet,
        Rt::RangeContains,
        Rt::RangeContainsFloat,
        Rt::ExcStr,
        Rt::Raise,
        Rt::RaiseValue,
        Rt::RaiseMsg,
        Rt::PendingIs,
        Rt::Catch,
        Rt::Uncaught,
        Rt::DictSet,
        Rt::DictGet,
        Rt::SetAdd,
        Rt::TableContains,
        Rt::TableEntry,
        Rt::ViewNew,
        Rt::ListOf,
        Rt::ListSort,
        Rt::Argv,
        Rt::OsListdir,
        Rt::OsPathJoin,
        Rt::OsPathIs,
        Rt::OsPathGetsize,
        Rt::ObjInc,
        Rt::ObjDec,
        Rt::ObjLen,
        Rt::ObjEq,
        Rt::ObjStr,
        Rt::WriteObj,
        Rt::Open,
        Rt::FileRead,
        Rt::FileClose,
        Rt::Sha256Hex,
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
            Rt::ListCopy => "pt_list_copy",
            Rt::WriteSeq => "pt_write_seq",
            Rt::StrFromSeq => "pt_str_from_seq",
            Rt::RangeLen => "pt_range_len",
            Rt::RangeGet => "pt_range_get",
            Rt::RangeContains => "pt_range_contains",
            Rt::RangeContainsFloat => "pt_range_contains_float",
            Rt::ExcStr => "pt_exc_str",
            Rt::Raise => "pt_raise",
            Rt::RaiseValue => "pt_raise_value",
            Rt::RaiseMsg => "pt_raise_msg",
            Rt::PendingIs => "pt_pending_is",
            Rt::Catch => "pt_catch",
            Rt::Uncaught => "pt_uncaught",
            Rt::DictSet => "pt_dict_set",
            Rt::DictGet => "pt_dict_get",
            Rt::SetAdd => "pt_set_add",
            Rt::TableContains => "pt_table_contains",
            Rt::TableEntry => "pt_table_entry",
            Rt::ViewNew => "pt_view_new",
            Rt::ListOf => "pt_list_of",
            Rt::ListSort => "pt_list_sort",
            Rt::Argv => "pt_argv",
            Rt::OsListdir => "pt_os_listdir",
            Rt::OsPathJoin => "pt_os_path_join",
            Rt::OsPathIs => "pt_os_path_is",
            Rt::OsPathGetsize => "pt_os_path_getsize",
            Rt::ObjInc => "pt_obj_inc",
            Rt::ObjDec => "pt_obj_dec",
            Rt::ObjLen => "pt_obj_len",
            Rt::ObjEq => "pt_obj_eq",
            Rt::ObjStr => "pt_obj_str",
            Rt::WriteObj => "pt_write_obj",
            Rt::Open => "pt_open",
            Rt::FileRead => "pt_file_read",
            Rt::FileClose => "pt_file_close",
            Rt::Sha256Hex => "pt_sha256_hex",
        }
    }

    /// Parameter and result types; `ptr` is a pointer, such as a string.
    fn sig(self, ptr: types::Type) -> (Vec<types::Type>, Option<types::Type>) {
        use types::{F64, I32, I64, I8};
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
            Rt::StrFromSeq | Rt::ListCopy => (vec![ptr], Some(ptr)),
            Rt::RangeLen => (vec![ptr], Some(I64)),
            Rt::RangeGet => (vec![ptr, I64, ptr], Some(I64)),
            Rt::RangeContains => (vec![ptr, I64], Some(I8)),
            Rt::RangeContainsFloat => (vec![ptr, F64], Some(I8)),
            Rt::ExcStr => (vec![ptr], Some(ptr)),
            Rt::Raise => (vec![ptr; 5], None),
            Rt::RaiseValue => (vec![ptr; 7], None),
            Rt::RaiseMsg => (vec![I8, ptr, ptr, ptr, ptr, ptr, ptr], None),
            Rt::PendingIs => (vec![I32], Some(I8)),
            Rt::Catch => (vec![], Some(ptr)),
            Rt::Uncaught => (vec![], None),
            Rt::DictSet => (vec![ptr, I64, I64], None),
            Rt::DictGet => (vec![ptr, I64, ptr], Some(I64)),
            Rt::SetAdd => (vec![ptr, I64], None),
            Rt::TableContains => (vec![ptr, I64], Some(I8)),
            Rt::TableEntry => (vec![ptr, I64, I64], Some(I64)),
            Rt::ViewNew | Rt::ListOf => (vec![ptr, ptr, ptr], Some(ptr)),
            Rt::ListSort => (vec![ptr], None),
            Rt::Argv => (vec![ptr, ptr], Some(ptr)),
            Rt::OsListdir => (vec![ptr; 7], Some(ptr)),
            Rt::OsPathJoin => (vec![ptr, ptr], Some(ptr)),
            Rt::OsPathIs => (vec![ptr, I8], Some(I8)),
            Rt::OsPathGetsize => (vec![ptr; 5], Some(I64)),
            Rt::ObjInc | Rt::ObjDec | Rt::WriteObj | Rt::FileClose => (vec![ptr], None),
            Rt::ObjLen => (vec![ptr], Some(I64)),
            Rt::ObjEq => (vec![ptr, ptr], Some(I8)),
            Rt::ObjStr | Rt::Sha256Hex => (vec![ptr], Some(ptr)),
            Rt::Open | Rt::FileRead => (vec![ptr; 5], Some(ptr)),
        }
    }
}

struct Codegen<'r> {
    module: ObjectModule,
    render: &'r dyn Fn(&Diagnostic) -> String,
    rt: HashMap<&'static str, ClifFunc>,
    /// The `i64` count of active Pythrust calls.
    depth: DataId,
    /// The runtime's pending-exception flag byte, read only through
    /// [`FnGen::pending_flag`].
    pending: DataId,
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
        let pending = module
            .declare_data("pt_pending_flag", Linkage::Import, true, false)
            .map_err(module_err)?;
        let ptr = module.target_config().pointer_type();
        let mut cg = Codegen {
            module,
            render,
            depth,
            pending,
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
            Ty::Str
            | Ty::List(_)
            | Ty::Tuple(_)
            | Ty::Opt(_)
            | Ty::Range
            | Ty::Exc
            | Ty::Dict(_)
            | Ty::Set(_)
            | Ty::View(_)
            | Ty::Bytes
            | Ty::File => self.ptr,
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
    /// While translating a site whose flag test is fused ([`fused`]):
    /// where its raise goes, and the local it would have defined.
    raise_to: Option<(BlockId, Option<LocalId>)>,
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
            raise_to: None,
        };
        for (i, block) in func.blocks.iter().enumerate() {
            if i > 0 {
                g.b.switch_to_block(g.blocks[i]);
            }
            g.body(block)?;
        }
        g.b.seal_all_blocks();
        let config = g.cg.module.target_config();
        g.b.finalize(config);
        Ok(())
    }

    /// A block's instructions and terminator. A catchable fault site
    /// followed by the flag test lowering puts after it is fused: the
    /// site's cold raise branch goes straight where the test would, so
    /// the path that does not raise neither reads the flag nor rejoins.
    fn body(&mut self, block: &ir::Block) -> Result<()> {
        let Some(f) = fused(block) else {
            block.insts.iter().try_for_each(|i| self.inst(i))?;
            return self.term(&block.term);
        };
        let (before, site) = block.insts.split_at(f.site);
        before.iter().try_for_each(|i| self.inst(i))?;
        self.raise_to = Some((f.raised, f.dest));
        let done = self.inst(&site[0]);
        self.raise_to = None;
        done?;
        let ok = self.block(f.ok);
        self.b.ins().jump(ok, &[]);
        Ok(())
    }

    /// Ends a cold raise branch: back to `ok`, where lowering's flag test
    /// follows, or, fused, to the raise path, with the site's local
    /// defined as zero (null) so that path releases nothing stale.
    fn after_raise(&mut self, ok: cranelift_codegen::ir::Block) {
        let Some((raised, dest)) = self.raise_to else {
            self.b.ins().jump(ok, &[]);
            return;
        };
        if let Some(d) = dest {
            let zero = self.zero(self.func.locals[d.0]);
            self.b.def_var(self.vars[d.0], zero);
        }
        let raised = self.block(raised);
        self.b.ins().jump(raised, &[]);
    }

    /// What to compute with after `raise_if(raised, ..)`: `v`, or `safe`
    /// when it raised and the raise rejoins this path. A fused raise
    /// never does, so then `v` needs no select.
    fn go_on_with(&mut self, raised: Value, safe: Value, v: Value) -> Value {
        match self.raise_to {
            Some(_) => v,
            None => self.b.ins().select(raised, safe, v),
        }
    }

    fn zero(&mut self, ty: Ty) -> Value {
        match ty {
            Ty::Float => self.b.ins().f64const(0.0),
            _ => self.b.ins().iconst(self.cg.clif_ty(ty), 0),
        }
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
                    self.write(*op)?;
                }
                self.write_text(*end, b'\n');
            }
            Inst::Retain(l) => {
                let (inc, _) = rc_fns(self.types, self.func.locals[l.0]);
                let s = self.b.use_var(self.vars[l.0]);
                self.call_rt(inc, &[s]);
            }
            Inst::Release(l) => {
                let (_, dec) = rc_fns(self.types, self.func.locals[l.0]);
                let s = self.b.use_var(self.vars[l.0]);
                self.call_rt(dec, &[s]);
                self.forget(*l);
            }
            Inst::Forget(l) => self.forget(*l),
            Inst::Append { list, value, .. } => {
                let rt = match self.func.ty(*list) {
                    Ty::Set(_) => Rt::SetAdd,
                    _ => Rt::SeqPush,
                };
                let (s, v) = (self.operand(*list), self.slot(*value));
                self.call_rt(rt, &[s, v]);
            }
            Inst::SetItem {
                list, index, value, ..
            } if matches!(self.func.ty(*list), Ty::Dict(_)) => {
                let (d, k, v) = (self.operand(*list), self.slot(*index), self.slot(*value));
                self.call_rt(Rt::DictSet, &[d, k, v]);
            }
            Inst::Sort(list) => {
                let s = self.operand(*list);
                self.call_rt(Rt::ListSort, &[s]);
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
            Inst::Raise { exc, span } => self.raise(*exc, *span)?,
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
        let (inc, dec) = rc_fns(self.types, ty);
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

    /// Writes `op` as `print` does. Lowering renders a value that may
    /// hold an exception with `str` first, which can fail.
    fn write(&mut self, op: Operand) -> Result<()> {
        let ty = self.func.ty(op);
        if ir::holds(self.types, ty, ir::is_exc) {
            return Err(backend(
                "printing an exception not rendered first".to_string(),
            ));
        }
        if let Ty::Opt(_) = ty {
            return self.write_opt(op);
        }
        let v = self.operand(op);
        self.write_value(ty, v)
    }

    /// Writes `v`, a value of `ty` but not an optional.
    fn write_value(&mut self, ty: Ty, v: Value) -> Result<()> {
        let rt = match ty {
            Ty::Int => Rt::WriteInt,
            Ty::Bool => Rt::WriteBool,
            Ty::None => {
                self.write_none();
                return Ok(());
            }
            Ty::Exc => return Err(backend("printing an unrendered exception".to_string())),
            Ty::Str => Rt::WriteStr,
            Ty::Float => Rt::WriteFloat,
            Ty::List(_)
            | Ty::Tuple(_)
            | Ty::Opt(_)
            | Ty::Range
            | Ty::Dict(_)
            | Ty::Set(_)
            | Ty::View(_) => Rt::WriteSeq,
            Ty::Bytes | Ty::File => Rt::WriteObj,
        };
        self.call_rt(rt, &[v]);
        Ok(())
    }

    fn write_none(&mut self) {
        self.call_rt(Rt::WriteNone, &[]);
    }

    /// Writes the string `text`, or `default` if there is none.
    fn write_text(&mut self, text: Option<Operand>, default: u8) {
        match text {
            Some(s) => {
                let v = self.operand(s);
                self.call_rt(Rt::WriteStr, &[v]);
            }
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
            Rvalue::Cmp(op, a, b)
                if is_seq(self.func.ty(*a))
                    || is_table(self.func.ty(*a))
                    || self.func.ty(*a) == Ty::Range =>
            {
                self.seq_cmp(*op, *a, *b)
            }
            Rvalue::Cmp(op, a, b) if self.func.ty(*a) == Ty::Bytes => {
                let eq = self.rt_on(Rt::ObjEq, &[*a, *b])?;
                match op {
                    CmpOp::Eq => Ok(eq),
                    CmpOp::Ne => Ok(self.b.ins().icmp_imm_s(IntCC::Equal, eq, 0)),
                    _ => Err(backend("ordering bytes".to_string())),
                }
            }
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
            Rvalue::Std(f, args) => self.std(*f, args, self.func.locals[dest.0], span),
            Rvalue::Rust(name, args) => self.rust(name, args, dest),
            Rvalue::Concat(a, b) => self.rt_on(Rt::StrConcat, &[*a, *b]),
            Rvalue::Repeat(s, n) => self.rt_nonnull(Rt::StrRepeat, &[*s, *n], span, TOO_LARGE),
            Rvalue::Index(d, k) if is_table(self.func.ty(*d)) => self.dict_get(*d, *k, dest),
            Rvalue::Index(s, i) if is_seq(self.func.ty(*s)) => self.item(*s, *i, dest, span),
            Rvalue::Index(s, i) if self.func.ty(*s) == Ty::Range => self.range_item(*s, *i, span),
            Rvalue::Index(s, i) => self.str_item(*s, *i, span),
            Rvalue::Len(s) if is_seq(self.func.ty(*s)) || is_table(self.func.ty(*s)) => {
                self.rt_on(Rt::SeqLen, &[*s])
            }
            Rvalue::Entry(t, i, part) => self.entry(*t, *i, *part, dest),
            Rvalue::View(d) => self.view(*d, self.func.locals[dest.0]),
            Rvalue::Contains { hay, needle } if is_table(self.func.ty(*hay)) => {
                let (s, v) = (self.operand(*hay), self.slot(*needle));
                self.call_rt_value(Rt::TableContains, &[s, v])
            }
            Rvalue::Len(s) if self.func.ty(*s) == Ty::Range => self.rt_on(Rt::RangeLen, &[*s]),
            Rvalue::Len(s) if self.func.ty(*s) == Ty::Bytes => self.rt_on(Rt::ObjLen, &[*s]),
            Rvalue::Len(s) => self.rt_on(Rt::StrLen, &[*s]),
            Rvalue::Contains { hay, needle } if is_seq(self.func.ty(*hay)) => {
                let (s, v) = (self.operand(*hay), self.slot(*needle));
                self.call_rt_value(Rt::ListContains, &[s, v])
            }
            Rvalue::Contains { hay, needle } if self.func.ty(*hay) == Ty::Range => {
                self.range_contains(*hay, *needle)
            }
            Rvalue::Contains { hay, needle } => self.rt_on(Rt::StrContains, &[*hay, *needle]),
            Rvalue::ToStr(op) => match self.func.ty(*op) {
                Ty::Opt(_) => self.opt_to_str(*op, dest, span),
                Ty::Str => Err(backend("`str` of a string".to_string())),
                ty => {
                    let v = self.operand(*op);
                    self.str_of_value(ty, v, span)
                }
            },
            Rvalue::Wrap(op) => self.wrap(*op, self.func.locals[dest.0]),
            Rvalue::Unwrap(op) => self.unwrap(*op, dest),
            Rvalue::IsNone(op) => {
                let v = self.operand(*op);
                Ok(self.b.ins().icmp_imm_s(IntCC::Equal, v, 0))
            }
            Rvalue::CopyList(s) if matches!(self.func.ty(*s), Ty::List(_)) => {
                self.rt_on(Rt::ListCopy, &[*s])
            }
            Rvalue::CopyList(s) => self.list_of(*s, self.func.locals[dest.0]),
            Rvalue::NewSeq(items) => self.new_seq(self.func.locals[dest.0], items),
            Rvalue::Field(t, k) => self.field(*t, *k, dest),
            Rvalue::Global(g) => {
                let (addr, ty) = self.global(*g)?;
                let v = self
                    .b
                    .ins()
                    .load(self.cg.clif_ty(ty), MemFlagsData::trusted(), addr, 0);
                if ty.is_rc() {
                    self.call_rt(rc_fns(self.types, ty).0, &[v]);
                }
                Ok(v)
            }
            Rvalue::Float(op, a, b) => self.float_op(*op, *a, *b, span),
            Rvalue::IntPow(a, b) => self.int_pow(*a, *b, span),
            Rvalue::ToFloat(a) => Ok(self.int_as_float(*a)),
            Rvalue::BoolToInt(a) => {
                let v = self.operand(*a);
                Ok(self.b.ins().uextend(types::I64, v))
            }
            Rvalue::FloatToInt(a) => self.float_to_int(*a, span),
            Rvalue::ParseFloat(s) => self.parse_float(*s, span),
            Rvalue::ParseInt(s) => self.parse_int(*s, span),
            Rvalue::NewExc(kind, arg) => self.new_exc(*kind, *arg),
            Rvalue::Raised => Ok(self.pending_flag()),
            Rvalue::PendingIs(classes) => self.pending_is(classes),
            Rvalue::Catch => self.call_rt_value(Rt::Catch, &[]),
        }
    }

    /// `str(v)`, a new string, for `v` of `ty`; a string gains a
    /// reference. Rendering an exception fails at `span` when it nests
    /// deeper than `run` allows.
    fn str_of_value(&mut self, ty: Ty, v: Value, span: Span) -> Result<Value> {
        let rt = match ty {
            Ty::Int => Rt::StrFromInt,
            Ty::Bool => Rt::StrFromBool,
            Ty::None => return self.call_rt_value(Rt::StrNone, &[]),
            Ty::Float => Rt::StrFromFloat,
            Ty::List(_)
            | Ty::Tuple(_)
            | Ty::Opt(_)
            | Ty::Range
            | Ty::Dict(_)
            | Ty::Set(_)
            | Ty::View(_) => Rt::StrFromSeq,
            Ty::Bytes | Ty::File => Rt::ObjStr,
            Ty::Exc => Rt::ExcStr,
            Ty::Str => {
                self.call_rt(Rt::StrInc, &[v]);
                return Ok(v);
            }
        };
        let s = self.call_rt_value(rt, &[v])?;
        if ir::holds(self.types, ty, ir::is_exc) {
            let null = self.b.ins().icmp_imm_s(IntCC::Equal, s, 0);
            self.fail_if(null, span, TOO_DEEP)?;
        }
        Ok(s)
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

    /// Whether an exception is pending, an `i8` 1 or 0: the runtime's
    /// flag byte, read inline rather than through a call. Every read of
    /// the flag goes through here, so it can become per task (phase 5).
    fn pending_flag(&mut self) -> Value {
        let gv = self
            .cg
            .module
            .declare_data_in_func(self.cg.pending, self.b.func);
        let addr = self.b.ins().symbol_value(self.cg.ptr, gv);
        self.b
            .ins()
            .load(types::I8, MemFlagsData::trusted(), addr, 0)
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
        let b = self.check_divisor(b, span)?;
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
        let b = self.check_divisor(b, span)?;
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

    /// Raises `ZeroDivisionError` when `b` is 0; gives the divisor to
    /// compute with, 1 in its place then, since the code after runs on.
    fn check_divisor(&mut self, b: Value, span: Span) -> Result<Value> {
        let zero = self.b.ins().icmp_imm_s(IntCC::Equal, b, 0);
        self.raise_if(zero, ExcKind::ZeroDivision, ZERO_DIV, span)?;
        let one = self.b.ins().iconst(types::I64, 1);
        Ok(self.go_on_with(zero, one, b))
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
            Term::Uncaught => {
                self.call_rt(Rt::Uncaught, &[]);
                self.b.ins().trap(TrapCode::unwrap_user(1));
            }
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
const ZERO_DIV: &str = "integer division or modulo by zero";
const TOO_LARGE: &str = "MemoryError: repeated sequence is too large";
const TOO_DEEP: &str =
    "RecursionError: maximum recursion depth exceeded while getting the repr of an object";

/// A fault site fused with the flag test after it ([`FnGen::body`]).
struct Fused {
    /// The site's index in its block.
    site: usize,
    /// The local it defines, if any.
    dest: Option<LocalId>,
    /// Where the test branches when an exception is pending, and not.
    raised: BlockId,
    ok: BlockId,
}

/// The fusable site of `block`: one whose every raise goes through
/// [`FnGen::after_raise`], followed only by the test of the flag, which
/// the block then branches on.
fn fused(block: &ir::Block) -> Option<Fused> {
    let [.., site, Inst::Assign {
        dest: flag,
        value: Rvalue::Raised,
        ..
    }] = block.insts.as_slice()
    else {
        return None;
    };
    let Term::Branch {
        cond: Operand::Local(cond),
        then,
        orelse,
    } = block.term
    else {
        return None;
    };
    let dest = match site {
        Inst::SetItem { .. } => None,
        Inst::Assign { dest, value, .. } if raises_inline(value) => Some(*dest),
        _ => return None,
    };
    (cond == *flag).then_some(Fused {
        site: block.insts.len() - 2,
        dest,
        raised: then,
        ok: orelse,
    })
}

/// Whether `value` raises only through [`FnGen::after_raise`]: not a
/// call, whose callee leaves the exception pending.
fn raises_inline(value: &Rvalue) -> bool {
    matches!(
        value,
        Rvalue::Index(..)
            | Rvalue::Int(IntOp::FloorDiv | IntOp::Mod, ..)
            | Rvalue::Float(
                FloatOp::Div | FloatOp::IntDiv | FloatOp::FloorDiv | FloatOp::Mod,
                ..
            )
            | Rvalue::ParseInt(_)
            | Rvalue::ParseFloat(_)
    )
}

/// Whether `ty` is a list, tuple or dict view: slots read by position.
fn is_seq(ty: Ty) -> bool {
    matches!(ty, Ty::List(_) | Ty::Tuple(_) | Ty::View(_))
}

/// Whether `ty` is a dict or set: read by key.
fn is_table(ty: Ty) -> bool {
    matches!(ty, Ty::Dict(_) | Ty::Set(_))
}

/// The runtime functions adding and dropping a reference to a `ty`. An
/// optional's are its present type's, or a sequence's for a boxed one.
fn rc_fns(types: &[Compound], ty: Ty) -> (Rt, Rt) {
    let ty = match ty {
        Ty::Opt(id) => match types.get(id.0) {
            Some(Compound::Opt(present)) => *present,
            _ => ty,
        },
        _ => ty,
    };
    match ty {
        Ty::Str => (Rt::StrInc, Rt::StrDec),
        Ty::Bytes | Ty::File => (Rt::ObjInc, Rt::ObjDec),
        _ => (Rt::SeqInc, Rt::SeqDec),
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

/// The native limitation an exception that may close a reference cycle
/// meets: refcounting has no cycle collector.
const CYCLE_GAP: &str = "the native backend does not support an exception holding a list \
                         or dict that may hold exceptions yet: it could form a reference \
                         cycle, and native code has no cycle collector";

/// Rejects what lowering expresses but this backend does not: an
/// exception whose argument holds a list that may hold an exception,
/// whose reference counts could form a cycle (a list holding an
/// exception holding that list). See [`may_cycle`].
fn refuse_gaps(program: &Program) -> Result<()> {
    for func in &program.funcs {
        for inst in func.blocks.iter().flat_map(|b| &b.insts) {
            let Inst::Assign {
                value: Rvalue::NewExc(_, Some(arg)),
                span,
                ..
            } = inst
            else {
                continue;
            };
            if may_cycle(&program.types, func.ty(*arg)) {
                return Err(Diagnostic {
                    span: *span,
                    message: CYCLE_GAP.into(),
                    kind: DiagKind::Backend,
                });
            }
        }
    }
    Ok(())
}

/// Whether an exception's argument of type `ty` may close a reference
/// cycle: it holds, outside any exception, a list whose elements, or
/// a dict whose values, may hold an exception (a set's elements never
/// can: they are hashable). Every other value is acyclic: a value is built
/// from values that exist, so a cycle needs a list changed afterwards
/// to reach back, and only an exception hides its argument's type. The
/// last exception before that list on the cycle holds it by its type.
/// "An exception" is any `Ty::Exc`, whatever its kind, so a future
/// `list[Exception]` counts; a future `dyn` must count as one too.
fn may_cycle(types: &[Compound], ty: Ty) -> bool {
    let (Ty::List(id) | Ty::Tuple(id) | Ty::Opt(id) | Ty::Dict(id) | Ty::Set(id) | Ty::View(id)) =
        ty
    else {
        return false;
    };
    match types.get(id.0) {
        Some(Compound::List(t) | Compound::Set(t)) => ir::holds(types, *t, ir::is_exc),
        Some(Compound::Dict(_, v)) => ir::holds(types, *v, ir::is_exc),
        Some(Compound::Opt(t) | Compound::View(_, t)) => may_cycle(types, *t),
        Some(Compound::Tuple(fields)) => fields.iter().any(|t| may_cycle(types, *t)),
        None => false,
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

    /// Whether `emit` refuses `src` as an exception that may close a
    /// reference cycle.
    fn refused_as_cycle(src: &str) -> bool {
        let module = parser::parse(&lexer::lex(src).expect("lex")).expect("parse");
        let types = infer::infer(&module).expect("infer");
        let program = lower::lower(&module, &types).expect("lower");
        let render = |d: &Diagnostic| d.message.clone();
        match emit(&program, &render) {
            Ok(_) => false,
            Err(d) => {
                assert_eq!(d.message, CYCLE_GAP);
                true
            }
        }
    }

    #[test]
    fn every_inline_fault_test_is_fused() {
        let src = "def f(xs: list[int], s: str, i: int, d: float) -> float:\n    \
                   xs[i] = xs[i] // i + xs[i] % i\n    a, b = xs\n    \
                   return len(s[i]) + i / i + d / d + d // d + d % d + int(s) + float(s)\n\
                   print(f([1, 2], \"3\", 1, 2.0))\n";
        let module = parser::parse(&lexer::lex(src).expect("lex")).expect("parse");
        let types = infer::infer(&module).expect("infer");
        let program = crate::rc::elide(lower::lower(&module, &types).expect("lower"));
        let blocks = program.funcs.iter().flat_map(|f| &f.blocks);
        // A block ending in the flag test, but for the test after a call:
        // what comes before it, past the releases of the call's arguments.
        let tested = |b: &&ir::Block| {
            let mut back = b.insts.iter().rev();
            let raised = |i: &Inst| {
                matches!(
                    i,
                    Inst::Assign {
                        value: Rvalue::Raised,
                        ..
                    }
                )
            };
            let call = |i: &Inst| {
                matches!(
                    i,
                    Inst::Assign {
                        value: Rvalue::Call(..),
                        ..
                    }
                )
            };
            let released = |i: &&Inst| matches!(i, Inst::Release(_) | Inst::Forget(_));
            back.next().is_some_and(raised) && !back.find(|i| !released(i)).is_some_and(call)
        };
        let tests: Vec<&ir::Block> = blocks.filter(tested).collect();
        // The store, three reads, `//`, `%` and `/` on ints, three float
        // divisions and two parses.
        assert_eq!(tests.len(), 12);
        assert!(tests.iter().all(|b| fused(b).is_some()));
    }

    #[test]
    fn exceptions_holding_lists_build() {
        for src in [
            "e = ValueError([1])\nprint(e)\n",
            "e = IndexError([['a'], []])\nprint(e)\n",
            "e = ValueError(([1], ValueError(2)))\nprint(e)\n",
            "e = ValueError((ValueError(0), IndexError(1)))\nprint(e)\n",
            "xs: list[int] | None = [1]\ne = ValueError(xs)\nprint(e)\n",
            "es = [ValueError(1)]\nprint(ValueError((1, 2)), es)\n",
        ] {
            assert!(!refused_as_cycle(src), "{src}");
        }
    }

    #[test]
    fn an_exception_holding_a_list_that_may_hold_one_is_refused() {
        for src in [
            "e = ValueError([ValueError(0)])\n",
            "e = ValueError([[IndexError(0)]])\n",
            "e = ValueError((1, [[ValueError(0)]]))\n",
            "e = ValueError([(1, ValueError(0))])\n",
            "x: Exception | None = None\ne = ValueError([x])\n",
        ] {
            assert!(refused_as_cycle(src), "{src}");
        }
    }

    /// Dicts and sets are containers too: a value or element that may hold
    /// an exception is refused; keys never can, being hashable.
    #[test]
    fn an_exception_holding_a_table_that_may_hold_one_is_refused() {
        for src in [
            "d: dict[str, Exception] = {}\ne = ValueError(d)\n",
            "ds = [{'a': [ValueError(0)]}]\ne = ValueError(ds)\n",
            "d = {1: [ValueError(0)]}\ne = ValueError(d.values())\n",
            "d = {1: (2, [IndexError(0)])}\ne = ValueError(d.items())\n",
        ] {
            assert!(refused_as_cycle(src), "{src}");
        }
        for src in [
            "e = ValueError({'a': [1]})\nprint(e)\n",
            "e = ValueError({1, 2})\nprint(e)\n",
            "e = ValueError({'a': 1}.keys())\nprint(e)\n",
            "es = {'a': (1, ValueError(1))}\nprint(ValueError(es.items()))\nprint(ValueError({(1, 'x'): 2}), es)\n",
        ] {
            assert!(!refused_as_cycle(src), "{src}");
        }
    }

    /// The refusal is by type, and every class's value is an `Exception`,
    /// so it holds for every class, outer and inner (B4).
    #[test]
    fn the_cycle_refusal_covers_every_exception_class() {
        for outer in ExcKind::ALL {
            for inner in ExcKind::ALL {
                let (o, i) = (outer.name(), inner.name());
                let src = format!("es: list[Exception] = [{i}(0)]\ne = {o}(es)\n");
                assert!(refused_as_cycle(&src), "{src}");
            }
        }
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
        // `+`, `//` and `-` can overflow. Dividing by zero raises instead,
        // so only its raise site is rendered, around a marker.
        assert_eq!(count(OVERFLOW), 3);
        assert!(!rendered.iter().any(|r| r.contains("ZeroDivisionError")));
    }

    #[test]
    fn run_time_message_parts_are_rendered_around_a_marker() {
        let (_, rendered) = compile("n = -2\nprint(2 ** n, float('x'), 1.5 / n)\n");
        // `2 ** n`'s hint, and the raise sites of `float('x')` and `/`.
        let marked = rendered.iter().filter(|r| r.contains('\0')).count();
        assert_eq!(marked, 3, "{rendered:?}");
    }

    #[test]
    fn string_failures_are_rendered_and_literals_stored_once() {
        let (object, rendered) = compile("s = 'needle'\nprint(s[5], s * 3, 'needle')\n");
        let count = |m: &str| rendered.iter().filter(|r| r.as_str() == m).count();
        assert_eq!(count(TOO_LARGE), 1);
        let stored = |lit: &[u8]| object.windows(lit.len()).filter(|w| *w == lit).count();
        // The index error's message is data its raise hands the runtime.
        assert_eq!(
            (stored(b"needle"), stored(b"string index out of range")),
            (1, 1)
        );
    }
}
