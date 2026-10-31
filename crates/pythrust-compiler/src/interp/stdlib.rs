//! Module functions ([`crate::stdlib`]) that need the interpreter's
//! state.

use super::builtins::Args;
use super::value::Value;
use super::{rt_err, Exec, Interp};
use crate::stdlib::StdFn;

impl<'m> Interp<'m, '_> {
    pub(super) fn std(&mut self, f: StdFn, args: &Args<'m>) -> Exec<'m, Value<'m>> {
        if !args.positional.is_empty() || !args.keywords.is_empty() {
            let msg = format!("internal: `{}` takes no arguments", f.name());
            return Err(rt_err(args.span, msg).into());
        }
        Ok(match f {
            StdFn::Argv => Value::list(self.argv.iter().map(|a| Value::str(a)).collect()),
        })
    }
}
