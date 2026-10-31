//! Module functions ([`crate::stdlib`]): runtime calls.

use cranelift_codegen::ir::Value;

use super::{FnGen, Rt};
use crate::diag::Result;
use crate::ir::{Operand, Ty};
use crate::stdlib::StdFn;

impl FnGen<'_, '_, '_> {
    /// `f`'s result, of type `ty`, from `args`.
    pub(super) fn std(&mut self, f: StdFn, _args: &[Operand], ty: Ty) -> Result<Value> {
        match f {
            StdFn::Argv => {
                let (desc, len) = self.descriptor(ty)?;
                self.call_rt_value(Rt::Argv, &[desc, len])
            }
        }
    }
}
