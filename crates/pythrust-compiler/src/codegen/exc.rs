//! Exceptions: objects the sequence runtime holds, and raising them
//! (`pythrust-rt/src/exc.rs`). Where a `raise` happened is rendered
//! here, at compile time, around the line the runtime fills in if
//! nothing catches it.

use cranelift_codegen::ir::{types, InstBuilder, Value};

use super::num::MID;
use super::seq::{descriptor, push_name};
use super::{backend, runtime, FnGen, Rt};
use crate::diag::{DiagKind, Diagnostic, Result};
use crate::exc::{Class, ExcKind};
use crate::ir::{Inst, Operand, Rvalue};
use crate::span::Span;

impl FnGen<'_, '_, '_> {
    /// A new exception of `kind`, holding `arg` if there is one. Its
    /// descriptor is `x`, the kind's code ([`code_char`]), and `arg`'s
    /// type; a program's own class has its builtin root's code and its two names
    /// before the type (D143).
    pub(super) fn new_exc(&mut self, kind: &Class, arg: Option<Operand>) -> Result<Value> {
        let mut text = format!("x{}", code_char(kind)?);
        if let Class::User(u) = kind {
            text.push(u.base.builtin().code_char());
            let what = "an exception class name of over 127 bytes";
            push_name(kind.shown(), what, &mut text)?;
            push_name(kind.name(), what, &mut text)?;
        }
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
    ///
    /// Lowering lists every program class a handler catches, so only the
    /// builtin ones' subclasses are added here.
    pub(super) fn pending_is(&mut self, classes: &[Class]) -> Result<Value> {
        let builtin = ExcKind::ALL
            .into_iter()
            .filter(|k| classes.iter().any(|c| Class::from(*k).is_a(c)))
            .map(Class::from);
        let mut codes = 0u64;
        for c in builtin.chain(classes.iter().cloned()) {
            code_char(&c)?;
            codes |= 1 << c.code();
        }
        let codes = self.b.ins().iconst(types::I64, codes.cast_signed());
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

/// The most kind codes the runtime's handler test holds, a bit each.
const MAX_CODES: u16 = 64;

/// A program with more classes than the runtime's handler test holds is
/// a backend gap (D143), refused where `inst` makes or tests for one.
pub(super) fn refuse_codes(inst: &Inst) -> Result<()> {
    let Inst::Assign { value, span, .. } = inst else {
        return Ok(());
    };
    let classes = match value {
        Rvalue::NewExc(class, _) => std::slice::from_ref(class),
        Rvalue::PendingIs(classes) => classes.as_slice(),
        _ => return Ok(()),
    };
    if classes.iter().all(|c| c.code() < MAX_CODES) {
        return Ok(());
    }
    let most = usize::from(MAX_CODES) - ExcKind::ALL.len();
    Err(Diagnostic {
        span: *span,
        message: format!(
            "the native backend does not support more than {most} exception classes yet"
        ),
        kind: DiagKind::Backend,
    })
}

/// `class`'s code as one descriptor byte, `'0'` plus the code, which
/// [`refuse_codes`] keeps under [`MAX_CODES`].
fn code_char(class: &Class) -> Result<char> {
    let code = u8::try_from(class.code())
        .ok()
        .filter(|c| u16::from(*c) < MAX_CODES)
        .ok_or_else(|| backend("an exception code past the runtime's".to_string()))?;
    Ok(char::from(b'0' + code))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{code_char, refuse_codes, MAX_CODES};
    use crate::exc::{Class, ExcKind, UserClass};
    use crate::ir::{Inst, LocalId, Rvalue};
    use crate::span::Span;

    fn user(code: u16) -> Class {
        let name = format!("E{code}");
        let base = ExcKind::Value.into();
        Class::User(Arc::new(UserClass { name, base, code }))
    }

    fn new_exc(class: Class) -> Inst {
        Inst::Assign {
            dest: LocalId(0),
            value: Rvalue::NewExc(class, None),
            span: Span { start: 0, end: 0 },
        }
    }

    #[test]
    fn the_last_code_the_handler_test_holds_builds_and_the_next_is_a_gap() {
        assert_eq!(code_char(&user(MAX_CODES - 1)).ok(), Some('o'));
        assert!(refuse_codes(&new_exc(user(MAX_CODES - 1))).is_ok());
        let msg = refuse_codes(&new_exc(user(MAX_CODES)))
            .err()
            .map(|d| d.message);
        let msg = msg.unwrap_or_default();
        assert!(msg.contains("more than 47 exception classes"), "{msg}");
        let caught = Inst::Assign {
            dest: LocalId(0),
            value: Rvalue::PendingIs(vec![Class::from(ExcKind::Value), user(MAX_CODES)]),
            span: Span { start: 0, end: 0 },
        };
        assert!(refuse_codes(&caught).is_err());
        assert!(code_char(&user(MAX_CODES)).is_err());
    }
}
