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
//! Every generated function takes the running thread's context
//! (`pythrust-rt`'s `PtCtx`) as a hidden first parameter, and passes it
//! on. It holds the pending-exception flag, read inline, and the count of
//! active calls, so recursion past the interpreter's limit is a
//! `RecursionError`, as in `run`, rather than a native stack overflow.
//! Both are per thread, so tasks (M6c) each count from zero.

use std::collections::{BTreeMap, HashMap};

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

mod concat;
mod constarg;
mod exc;
mod num;
mod opt;
mod pair;
mod seq;
mod stdlib;
mod task;

use crate::diag::{DiagKind, Diagnostic, Result};
use crate::exc::ExcKind;
use crate::ir::{
    self, BlockId, CmpOp, Compound, FloatOp, Func, Inst, IntOp, LocalId, Operand, Program, Rvalue,
    Term, Ty,
};
use crate::span::Span;

/// Symbol the runtime's `main` calls.
pub const ENTRY: &str = "pythrust_main";

/// Byte offsets of the fields of `pythrust-rt`'s `PtCtx`, which a
/// runtime test pins: the pending-exception flag and the call depth.
const CTX_PENDING: i32 = 0;
const CTX_DEPTH: i32 = 8;

/// Compiles `program` for the host. `render` turns a runtime error into
/// the exact text the program prints to stderr when it happens.
pub fn emit(program: &Program, render: &dyn Fn(&Diagnostic) -> String) -> Result<Vec<u8>> {
    refuse_gaps(program)?;
    let mut cg = Codegen::new(render)?;
    cg.declare_globals(&program.globals)?;
    let ids = cg.declare(program)?;
    cg.rets = program.funcs.iter().map(|f| f.ret).collect();
    let plans = pair::plans(program);
    cg.declare_pairs(program, &plans)?;
    cg.declare_clones(program, constarg::plan(program, &plans))?;
    cg.declare_thunks(program)?;
    for (i, (func, id)) in program.funcs.iter().zip(&ids).enumerate() {
        let what = |scalar_return, bound| Ctx {
            func,
            ids: &ids,
            types: &program.types,
            plan: &plans[i],
            scalar_return,
            bound,
        };
        cg.define(*id, what(false, None))?;
        if let Some(clone) = cg.pairs.get(&ir::FuncId(i)).copied() {
            cg.define(clone, what(true, None))?;
        }
    }
    let clones: Vec<_> = cg.clones.iter().map(|(c, id)| (*c, *id)).collect();
    for (c, id) in clones {
        let (func, plan) = (&program.funcs[c.callee.0], &plans[c.callee.0]);
        let param = func
            .params
            .get(c.arg)
            .copied()
            .ok_or_else(|| backend("clone of a missing parameter".into()))?;
        let bound = Some((param, c.value));
        let what = Ctx {
            func,
            ids: &ids,
            types: &program.types,
            plan,
            scalar_return: c.scalar,
            bound,
        };
        cg.define(id, what)?;
    }
    cg.define_thunks(program, &ids)?;
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
    StrConcat3,
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
    IntListGet,
    SeqHold,
    SeqReleaseHeld,
    SeqLenHeld,
    SeqGetHeld,
    SeqItemHeld,
    ListSet,
    IntListSet,
    SeqInc,
    SeqDec,
    SeqEq,
    SeqCmp,
    ListContains,
    ListCopy,
    ListSlice,
    StrSlice,
    ListExtend,
    ListScale,
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
    Stash,
    Restore,
    Unstash,
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
    Mark,
    ExecNew,
    ExecEnd,
    Submit,
    TaskResult,
    FutInc,
    FutDec,
    ResultsNext,
    ListReverse,
    StrSplit,
    StrStrip,
    StrReplace,
    StrJoin,
    StrAffix,
    StrFind,
    StrCount,
    StrCase,
    StrOrd,
    StrChr,
    SeqFsum,
    ListInsert,
    ListRemove,
    TableRemove,
    ListIndex,
    ListCount,
}

impl Rt {
    const ALL: [Rt; 116] = [
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
        Rt::StrConcat3,
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
        Rt::IntListGet,
        Rt::SeqHold,
        Rt::SeqReleaseHeld,
        Rt::SeqLenHeld,
        Rt::SeqGetHeld,
        Rt::SeqItemHeld,
        Rt::ListSet,
        Rt::IntListSet,
        Rt::SeqInc,
        Rt::SeqDec,
        Rt::SeqEq,
        Rt::SeqCmp,
        Rt::ListContains,
        Rt::ListCopy,
        Rt::ListSlice,
        Rt::StrSlice,
        Rt::ListExtend,
        Rt::ListScale,
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
        Rt::Stash,
        Rt::Restore,
        Rt::Unstash,
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
        Rt::Mark,
        Rt::ExecNew,
        Rt::ExecEnd,
        Rt::Submit,
        Rt::TaskResult,
        Rt::FutInc,
        Rt::FutDec,
        Rt::ResultsNext,
        Rt::ListReverse,
        Rt::StrSplit,
        Rt::StrStrip,
        Rt::StrReplace,
        Rt::StrJoin,
        Rt::StrAffix,
        Rt::StrFind,
        Rt::StrCount,
        Rt::StrCase,
        Rt::StrOrd,
        Rt::StrChr,
        Rt::SeqFsum,
        Rt::ListInsert,
        Rt::ListRemove,
        Rt::TableRemove,
        Rt::ListIndex,
        Rt::ListCount,
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
            Rt::StrConcat3 => "pt_str_concat3",
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
            Rt::IntListGet => "pt_int_list_get",
            Rt::SeqHold => "pt_seq_hold",
            Rt::SeqReleaseHeld => "pt_seq_release_held",
            Rt::SeqLenHeld => "pt_seq_len_held",
            Rt::SeqGetHeld => "pt_seq_get_held",
            Rt::SeqItemHeld => "pt_seq_item_held",
            Rt::ListSet => "pt_list_set",
            Rt::IntListSet => "pt_int_list_set",
            Rt::SeqInc => "pt_seq_inc",
            Rt::SeqDec => "pt_seq_dec",
            Rt::SeqEq => "pt_seq_eq",
            Rt::SeqCmp => "pt_seq_cmp",
            Rt::ListContains => "pt_list_contains",
            Rt::ListCopy => "pt_list_copy",
            Rt::ListSlice => "pt_list_slice",
            Rt::StrSlice => "pt_str_slice",
            Rt::ListExtend => "pt_list_extend",
            Rt::ListScale => "pt_list_scale",
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
            Rt::Stash => "pt_stash",
            Rt::Restore => "pt_restore",
            Rt::Unstash => "pt_unstash",
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
            Rt::StrSplit => "pt_str_split",
            Rt::StrStrip => "pt_str_strip",
            Rt::StrReplace => "pt_str_replace",
            Rt::StrJoin => "pt_str_join",
            Rt::StrAffix => "pt_str_affix",
            Rt::StrFind => "pt_str_find",
            Rt::StrCount => "pt_str_count",
            Rt::StrCase => "pt_str_case",
            Rt::StrOrd => "pt_str_ord",
            Rt::StrChr => "pt_str_chr",
            Rt::SeqFsum => "pt_seq_fsum",
            Rt::Mark => "pt_mark",
            Rt::ExecNew => "pt_exec_new",
            Rt::ExecEnd => "pt_exec_end",
            Rt::Submit => "pt_submit",
            Rt::TaskResult => "pt_result",
            Rt::FutInc => "pt_fut_inc",
            Rt::FutDec => "pt_fut_dec",
            Rt::ResultsNext => "pt_results_next",
            Rt::ListReverse => "pt_list_reverse",
            Rt::ListInsert => "pt_list_insert",
            Rt::ListRemove => "pt_list_remove",
            Rt::TableRemove => "pt_table_remove",
            Rt::ListIndex => "pt_list_index",
            Rt::ListCount => "pt_list_count",
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
            Rt::StrConcat3 => (vec![ptr, ptr, ptr], Some(ptr)),
            Rt::StrRepeat | Rt::StrIndex => (vec![ptr, I64], Some(ptr)),
            Rt::StrSlice | Rt::ListSlice => (vec![ptr, I64, I64, I64, I64], Some(ptr)),
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
            Rt::SeqLen | Rt::SeqLenHeld => (vec![ptr], Some(I64)),
            Rt::SeqGet | Rt::IntListGet | Rt::SeqGetHeld | Rt::SeqItemHeld => {
                (vec![ptr, I64, ptr], Some(I64))
            }
            Rt::SeqHold => (vec![ptr], Some(ptr)),
            Rt::ListSet | Rt::IntListSet => (vec![ptr, I64, I64], Some(I8)),
            Rt::SeqInc | Rt::SeqDec | Rt::SeqReleaseHeld | Rt::WriteSeq => (vec![ptr], None),
            Rt::SeqEq => (vec![ptr, ptr], Some(I8)),
            Rt::SeqCmp => (vec![ptr, ptr], Some(I64)),
            Rt::ListContains | Rt::ListScale => (vec![ptr, I64], Some(I8)),
            Rt::ListExtend => (vec![ptr, ptr], None),
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
            Rt::Catch | Rt::Unstash => (vec![], Some(ptr)),
            Rt::Stash | Rt::Restore => (vec![], None),
            Rt::Uncaught => (vec![], None),
            Rt::DictSet => (vec![ptr, I64, I64], None),
            Rt::DictGet => (vec![ptr, I64, ptr], Some(I64)),
            Rt::SetAdd => (vec![ptr, I64], None),
            Rt::TableContains => (vec![ptr, I64], Some(I8)),
            Rt::TableEntry => (vec![ptr, I64, I64], Some(I64)),
            Rt::ViewNew | Rt::ListOf => (vec![ptr, ptr, ptr], Some(ptr)),
            Rt::ListSort => (vec![ptr, I8], None),
            Rt::ListInsert => (vec![ptr, I64, I64], None),
            Rt::ListRemove | Rt::TableRemove => (vec![ptr, I64], None),
            Rt::ListIndex => (vec![ptr, I64, I8, ptr, ptr, ptr, ptr], Some(I64)),
            Rt::ListCount => (vec![ptr, I64], Some(I64)),
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
            Rt::Mark => (vec![ptr, ptr, I64], None),
            Rt::ExecNew => (vec![], Some(ptr)),
            Rt::ExecEnd | Rt::FutInc | Rt::FutDec | Rt::ListReverse => (vec![ptr], None),
            Rt::Submit => (vec![ptr; 6], Some(ptr)),
            Rt::TaskResult | Rt::ResultsNext => (vec![ptr], Some(I64)),
            Rt::StrSplit => (vec![ptr; 8], Some(ptr)),
            Rt::StrStrip => (vec![ptr, ptr, I8], Some(ptr)),
            Rt::StrReplace => (vec![ptr; 3], Some(ptr)),
            Rt::StrJoin => (vec![ptr, ptr], Some(ptr)),
            Rt::StrAffix => (vec![ptr, ptr, I8], Some(I8)),
            Rt::StrFind => (vec![ptr, ptr, I8, I8, ptr, ptr, ptr, ptr], Some(I64)),
            Rt::StrCount => (vec![ptr, ptr], Some(I64)),
            Rt::StrCase => (vec![ptr, I8], Some(ptr)),
            Rt::StrOrd => (vec![ptr], Some(I64)),
            Rt::StrChr => (vec![I64, ptr, ptr, ptr, ptr], Some(ptr)),
            Rt::SeqFsum => (vec![ptr, F64], Some(F64)),
        }
    }
}

struct Codegen<'r> {
    module: ObjectModule,
    render: &'r dyn Fn(&Diagnostic) -> String,
    rt: HashMap<&'static str, ClifFunc>,
    /// Read-only byte strings (failure messages, literals), deduplicated.
    bytes: HashMap<String, (DataId, usize)>,
    ptr: types::Type,
    /// `ir::Program::globals`: each a zeroed, module-private word.
    globals: Vec<(DataId, Ty)>,
    /// Each function's result type, by `ir::FuncId`.
    rets: Vec<Ty>,
    /// Private scalar-return variants; ordinary ids and all ABIs stay boxed.
    pairs: HashMap<ir::FuncId, ClifFunc>,
    /// Private constant-divisor clones (D69); calls only, never ABIs.
    clones: BTreeMap<constarg::Clone, ClifFunc>,
    /// The thunk of each function a task runs ([`task`]).
    thunks: HashMap<ir::FuncId, ClifFunc>,
    /// The module values each such function's tasks may reach.
    reads: task::Reads,
}

impl<'r> Codegen<'r> {
    fn new(render: &'r dyn Fn(&Diagnostic) -> String) -> Result<Self> {
        let mut flags = settings::builder();
        let triple = target_lexicon::Triple::host();
        // AArch64 COFF has no GOT/page relocations in cranelift-object
        // 0.137. Use absolute addresses there; MSVC emits PE base
        // relocations for ASLR. Other hosts keep PIC for their PIE linkers.
        let pic = if arm64_coff(&triple) { "false" } else { "true" };
        for (name, value) in [("opt_level", "speed"), ("is_pic", pic)] {
            flags.set(name, value).map_err(|e| backend(e.to_string()))?;
        }
        let isa = isa::lookup(triple)
            .map_err(|e| backend(format!("unsupported host: {e}")))?
            .finish(settings::Flags::new(flags))
            .map_err(|e| backend(e.to_string()))?;
        let builder =
            ObjectBuilder::new(isa, "pythrust", default_libcall_names()).map_err(module_err)?;
        let module = ObjectModule::new(builder);
        let ptr = module.target_config().pointer_type();
        let mut cg = Codegen {
            module,
            render,
            rt: HashMap::new(),
            bytes: HashMap::new(),
            ptr,
            globals: Vec::new(),
            rets: Vec::new(),
            pairs: HashMap::new(),
            clones: BTreeMap::new(),
            thunks: HashMap::new(),
            reads: HashMap::new(),
        };
        for rt in Rt::ALL {
            let sig = runtime_sig(cg.module.make_signature(), rt, ptr);
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
        sig.params.push(AbiParam::new(self.ptr));
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
            | Ty::File
            | Ty::Exec
            | Ty::Future
            | Ty::Results
            | Ty::Held(_) => self.ptr,
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

    fn define(&mut self, id: ClifFunc, what: Ctx<'_>) -> Result<()> {
        let mut ctx = Context::new();
        ctx.func.signature = if what.scalar_return {
            self.pair_signature(what.func)
        } else {
            self.signature(what.func)
        };
        FnGen::build(self, what, &mut ctx, &mut FunctionBuilderContext::new())?;
        self.define_function(id, &mut ctx)
    }

    /// Ordinary functions, pair/constant-divisor clones and task thunks
    /// all use the same relocation policy before object emission.
    fn define_function(&mut self, id: ClifFunc, ctx: &mut Context) -> Result<()> {
        far_coff_symbols(self.module.isa().triple(), &mut ctx.func);
        self.module.define_function(id, ctx).map_err(module_err)
    }
}

fn arm64_coff(triple: &target_lexicon::Triple) -> bool {
    matches!(
        triple.architecture,
        target_lexicon::Architecture::Aarch64(_)
    ) && triple.binary_format == target_lexicon::BinaryFormat::Coff
}

/// Non-PIC near addresses also need unsupported AArch64 page relocations.
/// Far addresses use Abs8, which the COFF writer and PE loader support.
fn far_coff_symbols(triple: &target_lexicon::Triple, func: &mut cranelift_codegen::ir::Function) {
    if !arm64_coff(triple) {
        return;
    }
    for value in func.global_values.values_mut() {
        if let cranelift_codegen::ir::GlobalValueData::Symbol { colocated, .. } = value {
            *colocated = false;
        }
    }
    for ext in func.dfg.ext_funcs.values_mut() {
        ext.colocated = false;
    }
}

/// What translating one function reads.
struct Ctx<'c> {
    func: &'c Func,
    ids: &'c [ClifFunc],
    types: &'c [Compound],
    plan: &'c pair::Plan,
    scalar_return: bool,
    /// In a constant-divisor clone (D69): the parameter bound at entry
    /// and its value; the caller's argument is ignored.
    bound: Option<(LocalId, i64)>,
}

/// Translates one function.
struct FnGen<'b, 'c, 'r> {
    cg: &'c mut Codegen<'r>,
    b: FunctionBuilder<'b>,
    func: &'c Func,
    ids: &'c [ClifFunc],
    types: &'c [Compound],
    vars: Vec<Variable>,
    /// Proven pair temporaries (and, in a clone, its return chain).
    pair_vars: HashMap<LocalId, [Variable; 2]>,
    scalar_return: bool,
    blocks: Vec<cranelift_codegen::ir::Block>,
    /// The running thread's `PtCtx`, the hidden first parameter.
    ctx: Value,
    /// While translating a site whose flag test is fused ([`fused`]):
    /// where its raise goes, and the local it would have defined.
    raise_to: Option<(BlockId, Option<LocalId>)>,
}

impl<'b, 'c, 'r> FnGen<'b, 'c, 'r> {
    fn build(
        cg: &'c mut Codegen<'r>,
        Ctx {
            func,
            ids,
            types,
            plan,
            scalar_return,
            bound,
        }: Ctx<'c>,
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
        let (ctx_param, params) = match b.block_params(entry).split_first() {
            Some((c, rest)) => (*c, rest.to_vec()),
            None => return Err(backend(format!("`{}` has no context", func.name))),
        };
        for (i, var) in vars.iter().enumerate() {
            let v = match func.params.iter().position(|p| p.0 == i) {
                Some(_) if bound.is_some_and(|(p, _)| p.0 == i) => {
                    let value = bound.map_or(0, |(_, c)| c);
                    b.ins().iconst(types::I64, value)
                }
                Some(k) => params[k],
                None if func.locals[i] == Ty::Float => b.ins().f64const(0.0),
                None => b.ins().iconst(cg.clif_ty(func.locals[i]), 0),
            };
            b.def_var(*var, v);
        }
        let mut pair_locals = plan.calls.clone();
        if scalar_return {
            pair_locals.extend(&plan.returns);
        }
        let pair_vars = pair::variables(&mut b, pair_locals);
        let mut g = FnGen {
            cg,
            b,
            func,
            ids,
            types,
            vars,
            pair_vars,
            scalar_return,
            blocks,
            ctx: ctx_param,
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
            self.concat_insts(&block.insts)?;
            return self.term(&block.term);
        };
        let (before, site) = block.insts.split_at(f.site);
        self.concat_insts(before)?;
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
        if self.pair_inst(inst)? {
            return Ok(());
        }
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
                let inc = rc_inc(self.types, self.func.locals[l.0])?;
                let s = self.b.use_var(self.vars[l.0]);
                self.call_rt(inc, &[s]);
            }
            Inst::Release(l) => {
                let dec = rc_dec(self.types, self.func.locals[l.0]);
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
            Inst::Sort { list, reverse, .. } => {
                let (s, r) = (self.operand(*list), self.operand(*reverse));
                self.call_rt(Rt::ListSort, &[s, r]);
            }
            Inst::Insert {
                list, index, value, ..
            } => {
                let (s, i, v) = (self.operand(*list), self.operand(*index), self.slot(*value));
                self.call_rt(Rt::ListInsert, &[s, i, v]);
            }
            Inst::Remove { list, index, .. } => {
                let (rt, i) = match self.func.ty(*list) {
                    Ty::Dict(_) => (Rt::TableRemove, self.slot(*index)),
                    _ => (Rt::ListRemove, self.operand(*index)),
                };
                let s = self.operand(*list);
                self.call_rt(rt, &[s, i]);
            }
            Inst::Extend { list, other, .. } => {
                let (s, t) = (self.operand(*list), self.operand(*other));
                self.call_rt(Rt::ListExtend, &[s, t]);
            }
            Inst::Scale { list, count, span } => {
                let (s, n) = (self.operand(*list), self.operand(*count));
                let failed = self.call_rt_value(Rt::ListScale, &[s, n])?;
                self.fail_if(failed, *span, TOO_LARGE)?;
            }
            Inst::Reverse(list) => {
                let s = self.operand(*list);
                self.call_rt(Rt::ListReverse, &[s]);
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
            Inst::Stash => {
                self.call_rt(Rt::Stash, &[]);
            }
            Inst::Restore => {
                self.call_rt(Rt::Restore, &[]);
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
        let inc = rc_inc(self.types, ty)?;
        self.call_rt(inc, &[v]);
        let old = self.b.ins().load(self.cg.ptr, flags, addr, 0);
        self.b.ins().store(flags, v, addr, 0);
        self.call_rt(rc_dec(self.types, ty), &[old]);
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
            Ty::Exec | Ty::Future | Ty::Results | Ty::Held(_) => {
                return Err(backend(
                    "printing a task's handle or a loop's hold".to_string(),
                ))
            }
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
            Rvalue::Int(op @ (IntOp::FloorDiv | IntOp::Mod), a, Operand::Int(2))
                if self.func.ty(*a) == Ty::Int =>
            {
                // For every signed i64, arithmetic shift floors by positive two;
                // its low bit is the corresponding nonnegative remainder. The
                // numerator is already evaluated by lowering, including faults.
                let a = self.operand(*a);
                Ok(match op {
                    IntOp::FloorDiv => self.b.ins().sshr_imm_s(a, 1),
                    _ => self.b.ins().band_imm_s(a, 1),
                })
            }
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
            Rvalue::Rust(name, args) => self.rust(name, args, dest, span),
            Rvalue::Concat(a, b) => self.rt_on(Rt::StrConcat, &[*a, *b]),
            Rvalue::Repeat(s, n) => self.rt_nonnull(Rt::StrRepeat, &[*s, *n], span, TOO_LARGE),
            Rvalue::Index(d, k) if is_table(self.func.ty(*d)) => self.dict_get(*d, *k, dest),
            Rvalue::Index(s, i) if is_seq(self.func.ty(*s)) => {
                self.item(Rt::SeqGet, *s, *i, dest, span)
            }
            Rvalue::Index(s, i) if matches!(self.func.ty(*s), Ty::Held(_)) => {
                self.item(Rt::SeqGetHeld, *s, *i, dest, span)
            }
            Rvalue::Item(s, i) if matches!(self.func.ty(*s), Ty::Held(_)) => {
                self.item(Rt::SeqItemHeld, *s, *i, dest, span)
            }
            Rvalue::Item(..) => Err(backend(
                "an uncounted element read of a list no loop holds".to_string(),
            )),
            Rvalue::Index(s, i) if self.func.ty(*s) == Ty::Range => self.range_item(*s, *i, span),
            Rvalue::Index(s, i) => self.str_item(*s, *i, span),
            Rvalue::Len(s)
                if is_seq(self.func.ty(*s))
                    || is_table(self.func.ty(*s))
                    || self.func.ty(*s) == Ty::Results =>
            {
                self.rt_on(Rt::SeqLen, &[*s])
            }
            Rvalue::Len(s) if matches!(self.func.ty(*s), Ty::Held(_)) => {
                self.rt_on(Rt::SeqLenHeld, &[*s])
            }
            Rvalue::Hold(s) => self.rt_on(Rt::SeqHold, &[*s]),
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
            Rvalue::Slice {
                source,
                bounds,
                present,
            } => {
                let rt = if self.func.ty(*source) == Ty::Str {
                    Rt::StrSlice
                } else {
                    Rt::ListSlice
                };
                self.rt_on(
                    rt,
                    &[
                        *source,
                        bounds[0],
                        bounds[1],
                        bounds[2],
                        Operand::Int(i64::from(*present)),
                    ],
                )
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
                    self.call_rt(rc_inc(self.types, ty)?, &[v]);
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
            Rvalue::Unstash => self.call_rt_value(Rt::Unstash, &[]),
            Rvalue::Submit { exec, func, args } => self.submit(*exec, *func, args),
            Rvalue::TaskResult(fut) => self.task_result(*fut, dest),
            Rvalue::NextResult(rs) => self.next_result(*rs, dest),
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
            Ty::Exec | Ty::Future | Ty::Results | Ty::Held(_) => {
                return Err(backend(
                    "`str` of a task's handle or a loop's hold".to_string(),
                ))
            }
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

    /// Whether an exception is pending on this thread, an `i8` 1 or 0:
    /// the context's flag byte, read inline rather than through a call.
    /// Every read of the flag goes through here.
    fn pending_flag(&mut self) -> Value {
        self.b
            .ins()
            .load(types::I8, MemFlagsData::trusted(), self.ctx, CTX_PENDING)
    }

    /// A planned constant-divisor clone of `callee` for these arguments.
    fn clone_target(&self, callee: ir::FuncId, args: &[Operand], scalar: bool) -> Option<ClifFunc> {
        constarg::target(&self.cg.clones, callee, args, scalar)
    }

    /// Calls `callee`, failing at `span` if `MAX_DEPTH` calls are active.
    fn call(
        &mut self,
        callee: ir::FuncId,
        args: &[Operand],
        dest: LocalId,
        span: Span,
    ) -> Result<Value> {
        let scalar = self.pair_vars.contains_key(&dest);
        let id = match self.clone_target(callee, args, scalar) {
            Some(clone) => Some(clone),
            None if scalar => self.cg.pairs.get(&callee).copied(),
            None => self.ids.get(callee.0).copied(),
        }
        .ok_or_else(|| backend("call to an unknown function".to_string()))?;
        let fref = self.cg.module.declare_func_in_func(id, self.b.func);
        let args: Vec<Value> = std::iter::once(self.ctx)
            .chain(args.iter().map(|op| self.operand(*op)))
            .collect();
        let (addr, flags) = (self.ctx, MemFlagsData::trusted());
        let depth = self.b.ins().load(types::I64, flags, addr, CTX_DEPTH);
        let full = self
            .b
            .ins()
            .icmp_imm_s(IntCC::SignedGreaterThanOrEqual, depth, MAX_DEPTH);
        self.fail_if(full, span, RECURSION)?;
        let deeper = self.b.ins().iadd_imm_s(depth, 1);
        self.b.ins().store(flags, deeper, addr, CTX_DEPTH);
        let call = self.b.ins().call(fref, &args);
        self.b.ins().store(flags, depth, addr, CTX_DEPTH);
        if scalar {
            let vals = self.b.inst_results(call);
            let vals = match vals {
                [a, b] => [*a, *b],
                _ => return Err(backend("scalar call without two results".into())),
            };
            self.set_pair(dest, vals)?;
            return Ok(self.b.ins().iconst(self.cg.ptr, 0));
        }
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
            Term::Return(Operand::Local(l)) if self.scalar_return => {
                let vals = self.pair_values(*l)?;
                self.b.ins().return_(&vals);
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
            | Rvalue::Item(..)
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

/// The runtime call taking a new reference to a `ty` value. An executor
/// has one owner and is never copied, nor is a loop's hold.
fn rc_inc(types: &[Compound], ty: Ty) -> Result<Rt> {
    match counted(types, ty) {
        Ty::Str => Ok(Rt::StrInc),
        Ty::Bytes | Ty::File => Ok(Rt::ObjInc),
        Ty::Future => Ok(Rt::FutInc),
        Ty::Exec => Err(backend("copying an executor".to_string())),
        Ty::Held(_) => Err(backend("copying a loop's hold".to_string())),
        _ => Ok(Rt::SeqInc),
    }
}

/// The runtime call dropping a reference to a `ty` value; an executor's
/// one reference ends it.
fn rc_dec(types: &[Compound], ty: Ty) -> Rt {
    match counted(types, ty) {
        Ty::Str => Rt::StrDec,
        Ty::Bytes | Ty::File => Rt::ObjDec,
        Ty::Future => Rt::FutDec,
        Ty::Exec => Rt::ExecEnd,
        Ty::Held(_) => Rt::SeqReleaseHeld,
        _ => Rt::SeqDec,
    }
}

/// The type whose count a `ty` value carries: an optional's present type,
/// or the optional itself (a sequence's count) for a boxed one.
fn counted(types: &[Compound], ty: Ty) -> Ty {
    match ty {
        Ty::Opt(id) => match types.get(id.0) {
            Some(Compound::Opt(present)) => *present,
            _ => ty,
        },
        _ => ty,
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

/// `sig` given the parameters and result of the runtime function `rt`.
fn runtime_sig(mut sig: Signature, rt: Rt, ptr: types::Type) -> Signature {
    let (params, ret) = rt.sig(ptr);
    sig.params.extend(params.into_iter().map(foreign_param));
    sig.returns.extend(ret.map(foreign_param));
    sig
}

/// A parameter or result of a function compiled by rustc (the runtime, a
/// binding shim): an integer narrower than 32 bits is zero-extended, since
/// an optimized callee may read a `u8` or `bool` from the whole register,
/// whose upper bits Cranelift otherwise leaves undefined.
pub(super) fn foreign_param(t: types::Type) -> AbiParam {
    match t.is_int() && t.bits() < 32 {
        true => AbiParam::new(t).uext(),
        false => AbiParam::new(t),
    }
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

    /// Inspect pre-optimization CLIF, independent of target instructions.
    fn arithmetic_clif(expression: &str) -> String {
        let src =
            format!("def f(n: int, d: int) -> int:\n    return {expression}\nprint(f(-7, 2))\n");
        let module = parser::parse(&lexer::lex(&src).expect("lex")).expect("parse");
        let types = infer::infer(&module).expect("infer");
        let program = crate::rc::elide(lower::lower(&module, &types).expect("lower"));
        let render = |d: &Diagnostic| d.message.clone();
        let mut cg = Codegen::new(&render).expect("codegen");
        cg.declare_globals(&program.globals).expect("globals");
        let ids = cg.declare(&program).expect("declare");
        cg.rets = program.funcs.iter().map(|f| f.ret).collect();
        let func = program.funcs.iter().find(|f| f.name == "f").expect("f");
        let mut ctx = Context::new();
        ctx.func.signature = cg.signature(func);
        FnGen::build(
            &mut cg,
            Ctx {
                func,
                ids: &ids,
                types: &program.types,
                plan: &pair::Plan::default(),
                scalar_return: false,
                bound: None,
            },
            &mut ctx,
            &mut FunctionBuilderContext::new(),
        )
        .expect("build CLIF");
        ctx.func.display().to_string()
    }

    /// Inspect the actual runtime references imported by a generated function.
    fn list_calls(source: &str) -> Vec<&'static str> {
        let module = parser::parse(&lexer::lex(source).expect("lex")).expect("parse");
        let types = infer::infer(&module).expect("infer");
        let program = crate::rc::elide(lower::lower(&module, &types).expect("lower"));
        let render = |d: &Diagnostic| d.message.clone();
        let mut cg = Codegen::new(&render).expect("codegen");
        cg.declare_globals(&program.globals).expect("globals");
        let ids = cg.declare(&program).expect("declare");
        cg.rets = program.funcs.iter().map(|f| f.ret).collect();
        let func = program.funcs.iter().find(|f| f.name == "f").expect("f");
        let mut ctx = Context::new();
        ctx.func.signature = cg.signature(func);
        FnGen::build(
            &mut cg,
            Ctx {
                func,
                ids: &ids,
                types: &program.types,
                plan: &pair::Plan::default(),
                scalar_return: false,
                bound: None,
            },
            &mut ctx,
            &mut FunctionBuilderContext::new(),
        )
        .expect("build CLIF");
        let clif = ctx.func.display().to_string();
        cg.rt
            .iter()
            .filter_map(|(&name, id)| {
                clif.contains(&format!("u0:{} ", id.as_u32()))
                    .then_some(name)
            })
            .collect()
    }

    #[test]
    fn integer_list_calls_are_selected_by_the_receiver_type() {
        for (annotation, value, typed) in [
            (": list[int]", "7", true),
            ("", "7", true),
            (": list[float]", "7.0", false),
            ("", "7.0", false),
            (": list[bool]", "True", false),
            (": list[str]", "\"a\"", false),
            (": list[list[int]]", "[7]", false),
            (": list[int | None]", "None", false),
        ] {
            let src = format!(
                "def f(xs{annotation}):\n    xs[-1] = {value}\n    print(xs[-1])\nf([{value}])\n"
            );
            let calls = list_calls(&src);
            for (symbol, want) in [
                ("pt_int_list_get", typed),
                ("pt_int_list_set", typed),
                ("pt_seq_get", !typed),
                ("pt_list_set", !typed),
            ] {
                assert_eq!(calls.contains(&symbol), want, "{src}: {calls:?}");
            }
        }
        let held =
            list_calls("def f(xs: list[int]):\n    for x in xs:\n        print(x)\nf([7])\n");
        assert!(!held.contains(&"pt_int_list_get"), "{held:?}");
        assert!(
            held.contains(&"pt_seq_get_held") || held.contains(&"pt_seq_item_held"),
            "{held:?}"
        );
        let tuple = list_calls("def f(xs: tuple[int, int]):\n    print(xs[0])\nf((1, 2))\n");
        assert!(tuple.contains(&"pt_seq_get"), "{tuple:?}");
        assert!(!tuple.contains(&"pt_int_list_get"), "{tuple:?}");
    }

    #[test]
    fn only_proven_positive_literal_two_uses_half_instructions() {
        for (expr, instruction) in [("n // 2", "sshr"), ("n % 2", "band")] {
            let clif = arithmetic_clif(expr);
            assert!(clif.contains(instruction), "{clif}");
            assert!(!clif.contains("sdiv") && !clif.contains("srem"), "{clif}");
        }
        for divisor in ["d", "0", "-1", "-2", "1", "3", "10"] {
            for op in ["//", "%"] {
                let clif = arithmetic_clif(&format!("n {op} {divisor}"));
                assert!(clif.contains("srem"), "{clif}");
                assert_eq!(clif.contains("sdiv"), op == "//", "{clif}");
            }
        }
    }

    /// Optimized machine code of `f(n, d)` returning `body`, with `d`
    /// bound to `divisor` as a constant-divisor clone would (D69).
    fn machine_code(body: &str, divisor: Option<i64>) -> String {
        let src = format!("def f(n: int, d: int) -> int:\n    return {body}\nprint(f(-7, 3))\n");
        let module = parser::parse(&lexer::lex(&src).expect("lex")).expect("parse");
        let types = infer::infer(&module).expect("infer");
        let program = crate::rc::elide(lower::lower(&module, &types).expect("lower"));
        let render = |d: &Diagnostic| d.message.clone();
        let mut cg = Codegen::new(&render).expect("codegen");
        cg.declare_globals(&program.globals).expect("globals");
        let ids = cg.declare(&program).expect("declare");
        cg.rets = program.funcs.iter().map(|f| f.ret).collect();
        let func = program.funcs.iter().find(|f| f.name == "f").expect("f");
        let mut ctx = Context::new();
        ctx.func.signature = cg.signature(func);
        ctx.set_disasm(true);
        FnGen::build(
            &mut cg,
            Ctx {
                func,
                ids: &ids,
                types: &program.types,
                plan: &pair::Plan::default(),
                scalar_return: false,
                bound: divisor.map(|c| (func.params[1], c)),
            },
            &mut ctx,
            &mut FunctionBuilderContext::new(),
        )
        .expect("build CLIF");
        let isa = cg.module.isa();
        ctx.compile(isa, &mut Default::default()).expect("compile");
        ctx.compiled_code()
            .and_then(|c| c.vcode.clone())
            .expect("disassembly")
    }

    #[test]
    fn a_bound_literal_divisor_needs_no_hardware_division() {
        // x86-64 `idiv` or its checked `srem` sequence, AArch64 `sdiv`.
        let divides = |code: &str| ["idiv", "sdiv", "srem"].iter().any(|i| code.contains(i));
        for body in ["n // d", "n % d", "(n // d) + (n % d)"] {
            let code = machine_code(body, None);
            assert!(divides(&code), "{body}:\n{code}");
            for c in [10, 3, 7, -10, i64::MAX, i64::MIN] {
                let code = machine_code(body, Some(c));
                assert!(!divides(&code), "{body} by {c}:\n{code}");
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
    fn coff_relocations_are_target_specific() {
        let render = |_: &Diagnostic| String::new();
        let mut cg = Codegen::new(&render).unwrap();
        let mut func = cranelift_codegen::ir::Function::new();
        let data = cg.module.declare_anonymous_data(false, false).unwrap();
        let gv = cg.module.declare_data_in_func(data, &mut func);
        let sig = cg.module.make_signature();
        let local = cg
            .module
            .declare_function("local", Linkage::Local, &sig)
            .unwrap();
        let fref = cg.module.declare_func_in_func(local, &mut func);
        for target in [
            "aarch64-pc-windows-msvc",
            "x86_64-pc-windows-msvc",
            "aarch64-unknown-linux-gnu",
            "x86_64-unknown-linux-gnu",
            "aarch64-apple-darwin",
            "x86_64-apple-darwin",
        ] {
            let triple = target.parse().unwrap();
            let far = target == "aarch64-pc-windows-msvc";
            assert_eq!(arm64_coff(&triple), far, "{target}");
            let mut f = func.clone();
            far_coff_symbols(&triple, &mut f);
            assert_eq!(f.dfg.ext_funcs[fref].colocated, !far, "{target}");
            let cranelift_codegen::ir::GlobalValueData::Symbol { colocated, .. } =
                f.global_values[gv]
            else {
                panic!("expected a data symbol");
            };
            assert_eq!(colocated, !far, "{target}");
        }
    }

    #[test]
    fn emits_globals_literals_and_task_function_addresses() {
        let (object, _) = compile(
            "import concurrent.futures\ngreeting = 'hello'\ndef work(n):\n    return greeting + str(n)\ndef start():\n    with concurrent.futures.ThreadPoolExecutor() as ex:\n        f = ex.submit(work, 42)\n        print(f.result())\nstart()\n",
        );
        assert!(object.len() > 64);
    }

    #[test]
    fn emits_both_constant_divisor_clone_variants_with_data_references() {
        let src = "greeting = 'hello'\ndef split(n: int, d: int) -> tuple[int, int]:\n    print(greeting)\n    return n // d, n % d\na, b = split(42, 10)\nprint(a, b)\nboxed = split(42, 10)\nprint(boxed)\n";
        let module = parser::parse(&lexer::lex(src).unwrap()).unwrap();
        let types = infer::infer(&module).unwrap();
        let p = crate::rc::elide(lower::lower(&module, &types).unwrap());
        let clones = constarg::plan(&p, &pair::plans(&p));
        assert!(clones.iter().any(|c| c.scalar));
        assert!(clones.iter().any(|c| !c.scalar));
        let (object, _) = compile(src);
        assert!(object.len() > 64);
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
        // `+` and `-` can overflow; division by positive literal two cannot.
        // The specialized site needs neither an overflow nor a zero-divisor
        // diagnostic. Generic divisions are covered by the CLIF controls.
        assert_eq!(count(OVERFLOW), 2);
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
    #[test]
    fn narrow_runtime_arguments_and_results_are_zero_extended() {
        use cranelift_codegen::ir::ArgumentExtension;
        use cranelift_codegen::isa::CallConv;
        // An optimized runtime reads a `u8` or `bool` from the whole
        // register, whose upper bits Cranelift leaves undefined.
        let mut narrow = 0;
        for rt in Rt::ALL {
            let sig = runtime_sig(Signature::new(CallConv::SystemV), rt, types::I64);
            for p in sig.params.iter().chain(&sig.returns) {
                let want = match p.value_type == types::I8 {
                    true => ArgumentExtension::Uext,
                    false => ArgumentExtension::None,
                };
                assert_eq!(p.extension, want, "{}", rt.symbol());
                narrow += usize::from(p.value_type == types::I8);
            }
        }
        assert!(narrow > 0);
    }
}
