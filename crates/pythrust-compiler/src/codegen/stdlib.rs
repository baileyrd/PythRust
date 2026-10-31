//! Module functions, `open` and a file's methods ([`crate::stdlib`]):
//! runtime calls. One that fails
//! raises at `span`'s site and gives a placeholder; lowering tests the
//! flag next.

use cranelift_codegen::ir::{types, InstBuilder, Value};

use super::{backend, FnGen, Rt};
use crate::diag::Result;
use crate::ir::{Operand, Ty};
use crate::span::Span;
use crate::stdlib::StdFn;

impl FnGen<'_, '_, '_> {
    /// `f`'s result, of type `ty`, from `args`. Lowering gave `os.listdir`
    /// its path, `os.path.join` at least two parts and `open` its path
    /// alone; a file method's one argument is the file.
    pub(super) fn std(&mut self, f: StdFn, args: &[Operand], ty: Ty, span: Span) -> Result<Value> {
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
            (StdFn::Open, [p]) => {
                let site = self.site(span)?;
                self.call_rt_value(Rt::Open, &[[*p].as_slice(), &site].concat())
            }
            (StdFn::Read, [f]) => {
                let site = self.site(span)?;
                self.call_rt_value(Rt::FileRead, &[[*f].as_slice(), &site].concat())
            }
            (StdFn::Sha256, [b]) => self.call_rt_value(Rt::Sha256Hex, &[*b]),
            (StdFn::Close, [f]) => {
                self.call_rt(Rt::FileClose, &[*f]);
                Ok(self.b.ins().iconst(types::I8, 0))
            }
            _ => Err(backend(format!(
                "`{}` with {} arguments",
                f.name(),
                args.len()
            ))),
        }
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

    /// `os.path.isdir` (`which` 0), `isfile` (1) or `islink` (2).
    fn path_is(&mut self, p: Value, which: i64) -> Result<Value> {
        let which = self.b.ins().iconst(types::I8, which);
        self.call_rt_value(Rt::OsPathIs, &[p, which])
    }
}
