//! Exceptions: objects the sequence runtime holds, and raising them
//! (`pythrust-rt/src/exc.rs`). Where a `raise` happened is rendered
//! here, at compile time, around the line the runtime fills in if
//! nothing catches it.

use cranelift_codegen::ir::{types, InstBuilder, Value};

use super::num::MID;
use super::seq::descriptor;
use super::{backend, runtime, FnGen, Rt};
use crate::diag::Result;
use crate::exc::ExcKind;
use crate::ir::Operand;
use crate::span::Span;

impl FnGen<'_, '_, '_> {
    /// A new exception of `kind`, holding `arg` if there is one. Its
    /// descriptor is `x`, the kind's [`ExcKind::code_char`], and `arg`'s type.
    pub(super) fn new_exc(&mut self, kind: ExcKind, arg: Option<Operand>) -> Result<Value> {
        let mut text = format!("x{}", kind.code_char());
        if let Some(a) = arg {
            descriptor(self.types, self.func.ty(a), &mut text)?;
        }
        let (desc, len) = self.data(text)?;
        let cap = self.b.ins().iconst(types::I64, i64::from(arg.is_some()));
        let e = self.call_rt_value(Rt::SeqNew, &[desc, len, cap])?;
        if let Some(a) = arg {
            let v = self.slot(a);
            self.call_rt(Rt::SeqPush, &[e, v]);
        }
        Ok(e)
    }

    /// Whether the pending exception is one of `classes`. The runtime
    /// knows no hierarchy: it gets the set of the codes of `classes` and
    /// every class derived from one, a bit per code.
    pub(super) fn pending_is(&mut self, classes: &[ExcKind]) -> Result<Value> {
        let codes = ExcKind::ALL
            .into_iter()
            .filter(|k| classes.iter().any(|c| k.is_a(*c)))
            .fold(0u32, |set, k| set | 1 << k.code());
        let codes = self.b.ins().iconst(types::I32, i64::from(codes));
        self.call_rt_value(Rt::PendingIs, &[codes])
    }

    /// `raise exc` at `span`.
    pub(super) fn raise(&mut self, exc: Operand, span: Span) -> Result<()> {
        let e = self.operand(exc);
        let site = self.site(span)?;
        self.call_rt(Rt::Raise, &[[e].as_slice(), &site].concat());
        Ok(())
    }

    /// Raises the `ValueError` a malformed `int(s)` or `float(s)` gives
    /// when `cond` holds: `prefix`, then `repr` of the string `text`.
    pub(super) fn raise_value_if(
        &mut self,
        cond: Value,
        text: Value,
        prefix: &str,
        span: Span,
    ) -> Result<()> {
        let (raise, ok) = (self.b.create_block(), self.b.create_block());
        self.b.set_cold_block(raise);
        self.b.ins().brif(cond, raise, &[], ok, &[]);
        self.b.switch_to_block(raise);
        let (p, p_len) = self.data(prefix.to_string())?;
        let site = self.site(span)?;
        self.call_rt(
            Rt::RaiseValue,
            &[[p, p_len, text].as_slice(), &site].concat(),
        );
        self.after_raise(ok);
        self.b.switch_to_block(ok);
        Ok(())
    }

    /// Raises `kind` holding `message` when `cond` holds: a catchable
    /// fault (a division by zero, an index out of range). The code after
    /// runs either way, on a placeholder result, and lowering tests the
    /// flag next.
    pub(super) fn raise_if(
        &mut self,
        cond: Value,
        kind: ExcKind,
        message: &str,
        span: Span,
    ) -> Result<()> {
        let (raise, ok) = (self.b.create_block(), self.b.create_block());
        self.b.set_cold_block(raise);
        self.b.ins().brif(cond, raise, &[], ok, &[]);
        self.b.switch_to_block(raise);
        let code = self.b.ins().iconst(types::I8, i64::from(kind.code()));
        let (m, m_len) = self.data(message.to_string())?;
        let site = self.site(span)?;
        self.call_rt(Rt::RaiseMsg, &[[code, m, m_len].as_slice(), &site].concat());
        self.after_raise(ok);
        self.b.switch_to_block(ok);
        Ok(())
    }

    /// The diagnostic of an exception raised at `span` and never caught,
    /// rendered before and after its line: address and length of each.
    pub(super) fn site(&mut self, span: Span) -> Result<[Value; 4]> {
        let text = (self.cg.render)(&runtime(span, &MID.to_string()));
        let Some((head, tail)) = text.split_once(MID) else {
            return Err(backend(
                "a rendered diagnostic lost its message".to_string(),
            ));
        };
        let (head, tail) = (head.to_string(), tail.to_string());
        let (h, h_len) = self.data(head)?;
        let (t, t_len) = self.data(tail)?;
        Ok([h, h_len, t, t_len])
    }
}
