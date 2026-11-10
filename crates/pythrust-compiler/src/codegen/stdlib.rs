//! Module functions, `open` and a file's methods ([`crate::stdlib`]):
//! runtime calls. One that fails
//! raises at `span`'s site and gives a placeholder; lowering tests the
//! flag next. Also calls to Rust bindings' shims ([`crate::interop`]).

use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{types, AbiParam, InstBuilder, Signature, Value};
use cranelift_module::{Linkage, Module};

use super::{backend, foreign_param, module_err, FnGen, Rt};
use crate::diag::Result;
use crate::interop::gen::symbol;
use crate::ir::{LocalId, Operand, Ty};
use crate::span::Span;
use crate::stdlib::StdFn;

/// The head of the fault `ord` of a string of length other than 1
/// gives, before the length (D56).
const ORD_HEAD: &str = "TypeError: ord() expected a character, but string of length ";

impl FnGen<'_, '_, '_> {
    /// `xs.index(x)`, raising its `ValueError` at `span`'s site, the
    /// position `xs.remove(x)` removes (-1 when absent), or
    /// `xs.count(x)` (D70): `x` is passed as a slot of the list's
    /// element type.
    fn list_search(&mut self, f: StdFn, s: Operand, x: Operand, span: Span) -> Result<Value> {
        let (s, x) = (self.operand(s), self.slot(x));
        if f == StdFn::ListCount {
            return self.call_rt_value(Rt::ListCount, &[s, x]);
        }
        let raise = self.flag(f == StdFn::ListIndex);
        let site = self.site(span)?;
        self.call_rt_value(Rt::ListIndex, &[[s, x, raise].as_slice(), &site].concat())
    }

    /// `f`'s result, of type `ty`, from `args`. Lowering gave `os.listdir`
    /// its path, `os.path.join` at least two parts and `open` its path
    /// alone; a file method's one argument is the file.
    pub(super) fn std(&mut self, f: StdFn, args: &[Operand], ty: Ty, span: Span) -> Result<Value> {
        if let (StdFn::ListIndex | StdFn::ListFind | StdFn::ListCount, [s, x]) = (f, args) {
            return self.list_search(f, *s, *x, span);
        }
        // What `write` writes: a `str` or `bytes`.
        let written = match args.get(1) {
            Some(Operand::Local(l)) => self.func.locals[l.0],
            _ => Ty::None,
        };
        let args: Vec<Value> = args.iter().map(|a| self.operand(*a)).collect();
        match (f, args.as_slice()) {
            (StdFn::Argv, []) => {
                let (desc, len) = self.descriptor(ty)?;
                self.call_rt_value(Rt::Argv, &[desc, len])
            }
            (StdFn::Listdir, [p]) => {
                let (desc, len) = self.descriptor(ty)?;
                let site = self.site(span)?;
                self.call_rt_value(Rt::OsListdir, &[[*p, desc, len].as_slice(), &site].concat())
            }
            (StdFn::Join, [first, rest @ ..]) => self.join(*first, rest),
            (StdFn::Isdir, [p]) => self.path_is(*p, 0),
            (StdFn::Isfile, [p]) => self.path_is(*p, 1),
            (StdFn::Islink, [p]) => self.path_is(*p, 2),
            (StdFn::Getsize, [p]) => {
                let site = self.site(span)?;
                self.call_rt_value(Rt::OsPathGetsize, &[[*p].as_slice(), &site].concat())
            }
            (StdFn::Open, [p, mode, encoding]) => {
                let site = self.site(span)?;
                let args = [[*p, *mode, *encoding].as_slice(), &site].concat();
                self.call_rt_value(Rt::Open, &args)
            }
            (StdFn::Read | StdFn::Readline, [file]) => {
                let rt = match (f, ty) {
                    (StdFn::Readline, _) => Rt::FileReadline,
                    (_, Ty::Str) => Rt::FileReadText,
                    _ => Rt::FileRead,
                };
                let site = self.site(span)?;
                self.call_rt_value(rt, &[[*file].as_slice(), &site].concat())
            }
            (StdFn::Write, [file, data]) => {
                let rt = match written {
                    Ty::Str => Rt::FileWriteStr,
                    _ => Rt::FileWriteBytes,
                };
                let site = self.site(span)?;
                self.call_rt_value(rt, &[[*file, *data].as_slice(), &site].concat())
            }
            (StdFn::Flush | StdFn::Close, [file]) => {
                let rt = match f {
                    StdFn::Flush => Rt::FileFlush,
                    _ => Rt::FileClose,
                };
                let site = self.site(span)?;
                self.call_rt(rt, &[[*file].as_slice(), &site].concat());
                Ok(self.b.ins().iconst(types::I8, 0))
            }
            (StdFn::Sha256, [b]) => self.call_rt_value(Rt::Sha256Hex, &[*b]),
            (StdFn::Executor, []) => self.call_rt_value(Rt::ExecNew, &[]),
            (StdFn::Split, [s, rest @ ..]) => {
                let sep = match rest {
                    [sep] => *sep,
                    _ => self.null(),
                };
                let (desc, len) = self.descriptor(ty)?;
                let site = self.site(span)?;
                let args = [[*s, sep, desc, len].as_slice(), &site].concat();
                self.call_rt_value(Rt::StrSplit, &args)
            }
            (StdFn::Strip | StdFn::Lstrip | StdFn::Rstrip, [s, rest @ ..]) => {
                let chars = match rest {
                    [chars] => *chars,
                    _ => self.null(),
                };
                let side = match f {
                    StdFn::Lstrip => 1,
                    StdFn::Rstrip => 2,
                    _ => 0,
                };
                let side = self.b.ins().iconst(types::I8, side);
                self.call_rt_value(Rt::StrStrip, &[*s, chars, side])
            }
            (StdFn::Replace, [s, old, new]) => {
                self.call_rt_value(Rt::StrReplace, &[*s, *old, *new])
            }
            (StdFn::StrJoin, [sep, xs]) => self.call_rt_value(Rt::StrJoin, &[*sep, *xs]),
            (StdFn::Startswith | StdFn::Endswith, [s, affix]) => {
                let end = self.flag(f == StdFn::Endswith);
                self.call_rt_value(Rt::StrAffix, &[*s, *affix, end])
            }
            (StdFn::Find | StdFn::Rfind | StdFn::Index | StdFn::Rindex, [s, sub]) => {
                let right = self.flag(matches!(f, StdFn::Rfind | StdFn::Rindex));
                let raise = self.flag(matches!(f, StdFn::Index | StdFn::Rindex));
                let site = self.site(span)?;
                let args = [[*s, *sub, right, raise].as_slice(), &site].concat();
                self.call_rt_value(Rt::StrFind, &args)
            }
            (StdFn::Count, [s, sub]) => self.call_rt_value(Rt::StrCount, &[*s, *sub]),
            (StdFn::Upper | StdFn::Lower, [s]) => {
                let upper = self.flag(f == StdFn::Upper);
                self.call_rt_value(Rt::StrCase, &[*s, upper])
            }
            (StdFn::Ord, [s]) => {
                let code = self.call_rt_value(Rt::StrOrd, &[*s])?;
                let bad = self.b.ins().icmp_imm_s(IntCC::SignedLessThan, code, 0);
                let message = format!("{}\0 found", ORD_HEAD);
                self.fail_mid_if(bad, span, &message, |g| {
                    let n = g.b.ins().bnot(code);
                    g.call_rt_value(Rt::StrFromInt, &[n])
                })?;
                Ok(code)
            }
            (StdFn::Chr, [n]) => {
                let site = self.site(span)?;
                self.call_rt_value(Rt::StrChr, &[[*n].as_slice(), &site].concat())
            }
            (StdFn::FloatSum, [xs, start]) => self.call_rt_value(Rt::SeqFsum, &[*xs, *start]),
            _ => Err(backend(format!(
                "`{}` with {} arguments",
                f.name(),
                args.len()
            ))),
        }
    }

    /// A call to the shim of the stub `name` (`binding.func`), whose
    /// result goes to `dest`: the arguments as generated code holds them,
    /// then the call's site, where a raise or a panic is reported.
    pub(super) fn rust(
        &mut self,
        name: &str,
        args: &[Operand],
        dest: LocalId,
        span: Span,
    ) -> Result<Value> {
        let params: Vec<types::Type> = args
            .iter()
            .map(|a| self.cg.clif_ty(self.func.ty(*a)))
            .collect();
        let ret = self.cg.clif_ty(self.func.locals[dest.0]);
        let sig = shim_sig(self.cg.module.make_signature(), &params, ret, self.cg.ptr);
        let id = self
            .cg
            .module
            .declare_function(&symbol(name), Linkage::Import, &sig)
            .map_err(module_err)?;
        let fref = self.cg.module.declare_func_in_func(id, self.b.func);
        let mut values: Vec<Value> = args.iter().map(|a| self.operand(*a)).collect();
        values.extend(self.site(span)?);
        let call = self.b.ins().call(fref, &values);
        self.b
            .inst_results(call)
            .first()
            .copied()
            .ok_or_else(|| backend(format!("`{name}` has no result")))
    }

    /// `os.path.join` of two or more parts, pairwise; each partial result
    /// is released once the next is made.
    fn join(&mut self, first: Value, rest: &[Value]) -> Result<Value> {
        let Some((second, more)) = rest.split_first() else {
            return Err(backend("`os.path.join` of one part".to_string()));
        };
        let mut out = self.call_rt_value(Rt::OsPathJoin, &[first, *second])?;
        for part in more {
            let next = self.call_rt_value(Rt::OsPathJoin, &[out, *part])?;
            self.call_rt(Rt::StrDec, &[out]);
            out = next;
        }
        Ok(out)
    }

    /// A runtime call's `u8` switch.
    fn flag(&mut self, on: bool) -> Value {
        self.b.ins().iconst(types::I8, i64::from(on))
    }

    /// A null pointer: an argument left out.
    fn null(&mut self) -> Value {
        self.b.ins().iconst(self.cg.ptr, 0)
    }

    /// `os.path.isdir` (`which` 0), `isfile` (1) or `islink` (2).
    fn path_is(&mut self, p: Value, which: i64) -> Result<Value> {
        let which = self.b.ins().iconst(types::I8, which);
        self.call_rt_value(Rt::OsPathIs, &[p, which])
    }
}

/// `sig` given a binding shim's `params` and result `ret`, then the four
/// pointers of the call's site.
fn shim_sig(
    mut sig: Signature,
    params: &[types::Type],
    ret: types::Type,
    ptr: types::Type,
) -> Signature {
    sig.params.extend(params.iter().copied().map(foreign_param));
    sig.params.extend([AbiParam::new(ptr); 4]);
    sig.returns.push(foreign_param(ret));
    sig
}

#[cfg(test)]
mod tests {
    use cranelift_codegen::ir::ArgumentExtension;
    use cranelift_codegen::isa::CallConv;

    use super::*;

    #[test]
    fn narrow_shim_arguments_and_results_are_zero_extended() {
        // A shim compiled by rustc reads a `bool` from the whole register.
        let (i8, i64) = (types::I8, types::I64);
        let sig = shim_sig(Signature::new(CallConv::SystemV), &[i8, i64], i8, i64);
        let ext: Vec<ArgumentExtension> = sig
            .params
            .iter()
            .chain(&sig.returns)
            .map(|p| p.extension)
            .collect();
        let (u, n) = (ArgumentExtension::Uext, ArgumentExtension::None);
        assert_eq!(ext, [u, n, n, n, n, n, u]);
    }
}
